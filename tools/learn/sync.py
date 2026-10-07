#!/usr/bin/env python3
"""Learn pages as Markdown, round-tripped with `strings/en.rs`.

The Learn pages are `LearnPage` literals in
`opensigner/opensigner-core/src/strings/en.rs`. Editing prose inside Rust
string literals is miserable, so each page also lives as one Markdown file
in `docs/learn/`, which is where the text is written and revised:

    docs/learn/01-words.md
    # Words                      ← the page title
    ## What the words are        ← a section heading
    A paragraph.                 ← paragraphs, blank-line separated

    A second paragraph.

`en.rs` stays the only place the device reads strings from; the Markdown
is the editing form. The two are kept identical by the `check` command,
which `just` runs as a lint.

Commands, from the repository root:

    python3 tools/learn/sync.py export   # en.rs → docs/learn/*.md
    python3 tools/learn/sync.py import   # docs/learn/*.md → en.rs, printing word counts
    python3 tools/learn/sync.py check    # fail if the two differ

Pages are matched by the field name in `en.rs` (`learn_words` ↔
`01-words.md`); the number prefix is the order `learn_pages()` lists them
in. Adding or removing a page is a code change (the struct field,
`LEARN_PAGES` and `learn_pages()`), not something this script does: it
refuses a file with no field and a field with no file.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EN_RS = ROOT / "opensigner" / "opensigner-core" / "src" / "strings" / "en.rs"
DOCS = ROOT / "docs" / "learn"

# ----- en.rs -----------------------------------------------------------------

FIELD_RE = re.compile(r"^    (learn_[a-z_]+): LearnPage \{$")
ORDER_RE = re.compile(r"^\s+&self\.(learn_[a-z_]+),$", re.M)


def rust_unescape(s: str) -> str:
    """The escapes the strings file uses: \\" \\\\ and \\u{…}."""
    out = []
    i = 0
    while i < len(s):
        c = s[i]
        if c != "\\":
            out.append(c)
            i += 1
            continue
        n = s[i + 1]
        if n == "u":
            j = s.index("}", i)
            out.append(chr(int(s[i + 3 : j], 16)))
            i = j + 1
        elif n == "n":
            out.append("\n")
            i += 2
        else:
            out.append(n)
            i += 2
    return "".join(out)


def rust_escape(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"')


def page_order(text: str) -> list[str]:
    """Field names in the order `learn_pages()` lists them."""
    start = text.index("pub fn learn_pages(")
    end = text.index("\n    }", start)
    order = ORDER_RE.findall(text[start:end] + "\n")
    if not order:
        raise SystemExit("en.rs: learn_pages() lists no pages")
    return order


def page_blocks(lines: list[str]) -> dict[str, tuple[int, int]]:
    """Field name → (first line, line after the closing `    },`)."""
    blocks = {}
    i = 0
    while i < len(lines):
        m = FIELD_RE.match(lines[i])
        if m:
            j = i + 1
            while lines[j] != "    },":
                j += 1
            blocks[m.group(1)] = (i, j + 1)
            i = j + 1
        else:
            i += 1
    return blocks


STRING_RE = re.compile(r'^\s*(?:title|heading): "((?:[^"\\]|\\.)*)",$')
PARA_RE = re.compile(r'^\s*"((?:[^"\\]|\\.)*)",$')


def parse_block(lines: list[str]) -> tuple[str, list[tuple[str, list[str]]]]:
    """A `LearnPage` literal → (title, [(heading, [paragraphs])])."""
    title = None
    sections: list[tuple[str, list[str]]] = []
    mode = None
    for line in lines:
        s = line.strip()
        if s.startswith("title:"):
            title = rust_unescape(STRING_RE.match(line).group(1))
        elif s.startswith("heading:"):
            sections.append((rust_unescape(STRING_RE.match(line).group(1)), []))
            mode = None
        elif s.startswith("paragraphs:"):
            # rustfmt puts a short array on one line: `paragraphs: &["…"],`.
            inline = re.findall(r'"((?:[^"\\]|\\.)*)"', s)
            sections[-1][1].extend(rust_unescape(p) for p in inline)
            mode = None if s.endswith("],") else "para"
        elif mode == "para" and s.startswith('"'):
            sections[-1][1].append(rust_unescape(PARA_RE.match(line).group(1)))
        elif s == "],":
            mode = None
    assert title is not None
    return title, sections


def render_block(field: str, title: str, sections: list[tuple[str, list[str]]]) -> list[str]:
    out = [f"    {field}: LearnPage {{", f'        title: "{rust_escape(title)}",', "        sections: &["]
    for heading, paras in sections:
        out += [
            "            LearnSection {",
            f'                heading: "{rust_escape(heading)}",',
            "                paragraphs: &[",
        ]
        out += [f'                    "{rust_escape(p)}",' for p in paras]
        out += ["                ],", "            },"]
    out += ["        ],", "    },"]
    return out


# ----- Markdown --------------------------------------------------------------


def to_markdown(title: str, sections: list[tuple[str, list[str]]]) -> str:
    parts = [f"# {title}", ""]
    for heading, paras in sections:
        parts += [f"## {heading}", ""]
        for p in paras:
            parts += [p, ""]
    return "\n".join(parts).rstrip("\n") + "\n"


def from_markdown(text: str, name: str) -> tuple[str, list[tuple[str, list[str]]]]:
    title = None
    sections: list[tuple[str, list[str]]] = []
    para: list[str] = []

    def flush():
        if para:
            if not sections:
                raise SystemExit(f"{name}: a paragraph before the first '## ' heading")
            sections[-1][1].append(" ".join(para))
            para.clear()

    for raw in text.splitlines():
        line = raw.rstrip()
        if line.startswith("# "):
            if title is not None:
                raise SystemExit(f"{name}: two '# ' titles")
            title = line[2:].strip()
        elif line.startswith("## "):
            flush()
            sections.append((line[3:].strip(), []))
        elif line.strip() == "":
            flush()
        elif line.startswith("#"):
            raise SystemExit(f"{name}: only '# ' and '## ' headings are allowed: {line!r}")
        else:
            para.append(line.strip())
    flush()
    if title is None:
        raise SystemExit(f"{name}: no '# ' title")
    if not sections:
        raise SystemExit(f"{name}: no sections")
    return title, sections


def word_count(title: str, sections: list[tuple[str, list[str]]]) -> int:
    n = len(title.split())
    for heading, paras in sections:
        n += len(heading.split()) + sum(len(p.split()) for p in paras)
    return n


def file_for(field: str, index: int) -> Path:
    return DOCS / f"{index + 1:02d}-{field.removeprefix('learn_').replace('_', '-')}.md"


# ----- commands --------------------------------------------------------------


def load_en() -> tuple[list[str], list[str], dict[str, tuple[int, int]]]:
    text = EN_RS.read_text(encoding="utf-8")
    lines = text.split("\n")
    order = page_order(text)
    blocks = page_blocks(lines)
    missing = [f for f in order if f not in blocks]
    if missing:
        raise SystemExit(f"en.rs: listed in learn_pages() but no literal: {missing}")
    return lines, order, blocks


def markdown_of(lines, order, blocks) -> dict[Path, str]:
    out = {}
    for i, field in enumerate(order):
        a, b = blocks[field]
        title, sections = parse_block(lines[a:b])
        out[file_for(field, i)] = to_markdown(title, sections)
    return out


def cmd_export() -> None:
    lines, order, blocks = load_en()
    DOCS.mkdir(parents=True, exist_ok=True)
    for path, md in markdown_of(lines, order, blocks).items():
        path.write_text(md, encoding="utf-8")
        print(f"wrote {path.relative_to(ROOT)}")
    stray = sorted(set(DOCS.glob("*.md")) - set(markdown_of(lines, order, blocks)))
    for s in stray:
        print(f"note: {s.relative_to(ROOT)} matches no page in learn_pages()")


def cmd_import() -> None:
    lines, order, blocks = load_en()
    expected = {file_for(f, i): f for i, f in enumerate(order)}
    present = set(DOCS.glob("*.md"))
    for p in sorted(present - set(expected)):
        raise SystemExit(f"{p.relative_to(ROOT)}: no such page; adding a page is a code change")
    for p in sorted(set(expected) - present):
        raise SystemExit(f"{p.relative_to(ROOT)}: missing; every page listed in learn_pages() needs its file")
    # Replace from the bottom so earlier line numbers stay valid.
    for path, field in sorted(expected.items(), key=lambda kv: blocks[kv[1]][0], reverse=True):
        title, sections = from_markdown(path.read_text(encoding="utf-8"), path.name)
        a, b = blocks[field]
        lines[a:b] = render_block(field, title, sections)
        print(f"{path.name:28} {word_count(title, sections):4} words")
    EN_RS.write_text("\n".join(lines), encoding="utf-8")
    print(f"wrote {EN_RS.relative_to(ROOT)}; run `cargo fmt --all` and `just`")


def cmd_check() -> None:
    lines, order, blocks = load_en()
    want = markdown_of(lines, order, blocks)
    bad = []
    for path, md in want.items():
        if not path.exists():
            bad.append(f"{path.relative_to(ROOT)}: missing (run `just learn-export`)")
        elif path.read_text(encoding="utf-8") != md:
            bad.append(f"{path.relative_to(ROOT)}: differs from en.rs (run `just learn-import` or `just learn-export`)")
    for p in sorted(set(DOCS.glob("*.md")) - set(want)):
        bad.append(f"{p.relative_to(ROOT)}: matches no page in learn_pages()")
    if bad:
        print("\n".join(bad))
        raise SystemExit(1)
    print("lint-learn: ok")


def main() -> None:
    cmds = {"export": cmd_export, "import": cmd_import, "check": cmd_check}
    if len(sys.argv) != 2 or sys.argv[1] not in cmds:
        raise SystemExit(f"usage: {sys.argv[0]} {{{'|'.join(cmds)}}}")
    cmds[sys.argv[1]]()


if __name__ == "__main__":
    main()
