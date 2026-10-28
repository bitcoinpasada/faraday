#!/usr/bin/env python3
"""Guide lint (docs/GUIDE.md, docs/guide/).

Keeps the user guide checked against the app, so that a string renamed in
the app fails `just lint` until the guide follows.

1. Names. Every bold span in docs/GUIDE.md and docs/guide/**/*.md (outside
   fenced code) is joined onto one line and split on " -> " and " . "
   (the arrow and the middle dot the app writes). Each part must be an
   en.rs string, a filled-in en.rs template (`{}` and `{name}` read as
   any text), or a line of tools/lint-guide.allow (test values and place
   names, each with its reason).
2. Links. Every image and every relative link resolves to a file, and a
   `#anchor` to a heading of that file (GitHub's anchors).
3. Screens. When docs/guide/screens/ exists, every `Screen` and `Sheet`
   variant is named by the `**Screen:**` line of exactly one page there,
   or listed under "## To write" in its README.md (a line
   "- `Screen`: `A`, `B`"), never both; a name in either that is no
   variant fails.

The allow list holds one name per line (`re:<pattern>` matches a whole
name), then `  # reason`. Standard library only; no build. `--self-test` shows a renamed string
failing. Errors are `file:line: message`.
"""
import os
import re
import sys
import tempfile

STRINGS = "faraday/faraday-strings/src/en.rs"
ALLOW = "tools/lint-guide.allow"
LEARN = "faraday/faraday-learn/src/pages/en.rs"
SEPARATORS = re.compile(r" → | · ")
LITERAL = re.compile(r'"((?:[^"\\]|\\.)*)"', re.S)


def unescape(s):
    s = re.sub(r"\\\n\s*", "", s)
    out, i = [], 0
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
        else:
            out.append({"n": "\n", "t": "\t", "r": "\r"}.get(n, n))
            i += 2
    return "".join(out)


def load_strings(root):
    """Literals and templates of en.rs, comments left out."""
    with open(os.path.join(root, STRINGS), encoding="utf-8") as f:
        text = f.read()
    text = "\n".join(l for l in text.split("\n") if not l.lstrip().startswith("//"))
    exact, templates = set(), []
    for m in LITERAL.finditer(text):
        v = unescape(m.group(1))
        exact.add(v)
        for line in v.split("\n"):
            exact.add(line.strip())
        # A string the app joins from parts with " · " is named by a part.
        exact.update(SEPARATORS.split(v))
        if "{" in v:
            parts = re.split(r"\{\{|\}\}|\{[a-z_0-9:.]*\}", v)
            if len(parts) > 1 and any(p.strip() for p in parts):
                pat = ".+".join(re.escape(p) for p in parts)
                templates.append(re.compile("^" + pat + "$", re.S))
    # Learn page titles are named too (`title: "..."` in faraday-learn).
    learn = os.path.join(root, LEARN)
    if os.path.exists(learn):
        with open(learn, encoding="utf-8") as f:
            exact.update(unescape(t) for t in re.findall(r'title: "((?:[^"\\]|\\.)*)"', f.read()))
    return exact, templates


def load_allow(root):
    allowed = set()
    with open(os.path.join(root, ALLOW), encoding="utf-8") as f:
        for line in f:
            line = line.rstrip("\n")
            if not line.strip() or line.startswith("#"):
                continue
            allowed.add(line.split("  # ")[0].strip())
    return allowed


def allowed_match(part, allowed):
    """`re:` entries are patterns that must match the whole name."""
    return part in allowed or any(
        a.startswith("re:") and re.fullmatch(a[3:], part) for a in allowed
    )


def guide_files(root):
    files = [os.path.join(root, "docs/GUIDE.md")]
    for d, _, names in os.walk(os.path.join(root, "docs/guide")):
        files += [os.path.join(d, n) for n in names if n.endswith(".md")]
    return sorted(f for f in files if os.path.exists(f))


def prose_lines(path):
    """(number, text) of the lines outside fenced code."""
    fenced = False
    with open(path, encoding="utf-8") as f:
        for n, line in enumerate(f, 1):
            if line.lstrip().startswith("```"):
                fenced = not fenced
                continue
            if not fenced:
                yield n, line.rstrip("\n")


def bold_spans(path):
    """(line, text) of each bold span; a span may wrap across lines of one
    paragraph. The line is the one the span starts on."""
    para, out = [], []

    def flush():
        text, starts = "", []
        for n, l in para:
            starts.append((len(text), n))
            text += l.strip() + " "
        for m in re.finditer(r"\*\*(.+?)\*\*", text):
            line = [n for off, n in starts if off <= m.start()][-1]
            out.append((line, m.group(1)))
        para.clear()

    for n, line in prose_lines(path):
        if not line.strip():
            flush()
            continue
        para.append((n, re.sub(r"`[^`]*`", "``", line)))
    flush()
    return out


def known(part, exact, templates, allowed):
    """Whether a name is an app string, a filled template, or allowed. A
    leading "· " (the join the app writes between two strings) and a
    trailing ":" (a label before its value) are not part of the name."""
    for p in (part, part.removeprefix("· "), part.removesuffix(":")):
        if p in exact or allowed_match(p, allowed) or any(t.match(p) for t in templates):
            return True
    if part.endswith(":"):
        return any(s.startswith(part) for s in exact)
    return False


def check_names(root, files, errors):
    exact, templates = load_strings(root)
    allowed = load_allow(root)
    count = 0
    for path in files:
        rel = os.path.relpath(path, root)
        for line, span in bold_spans(path):
            if re.match(r"^(Kind|Status|Open|Screen|Group):$", span):
                continue
            whole = " ".join(span.split())
            parts = [whole] + SEPARATORS.split(span) if SEPARATORS.search(span) else [whole]
            for i, part in enumerate(parts):
                part = " ".join(part.split())
                if not part:
                    continue
                if i == 0 and len(parts) > 1:
                    if known(whole, exact, templates, allowed):
                        count += 1
                        break
                    continue
                count += 1
                if known(part, exact, templates, allowed):
                    continue
                errors.append(f"{rel}:{line}: bold name not in {STRINGS} or {ALLOW}: **{part}**")
    return count


def slug(heading):
    h = re.sub(r"`|\*|_", "", heading.strip().lower())
    h = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", h)
    h = re.sub(r"[^\w\- ]", "", h)
    return h.replace(" ", "-")


def anchors(path, cache={}):
    if path not in cache:
        seen, out = {}, set()
        for _, line in prose_lines(path):
            m = re.match(r"^#{1,6}\s+(.*?)\s*#*$", line)
            if m:
                s = slug(m.group(1))
                k = seen.get(s, 0)
                seen[s] = k + 1
                out.add(s if k == 0 else f"{s}-{k}")
        cache[path] = out
    return cache[path]


def check_links(root, files, errors):
    for path in files:
        rel = os.path.relpath(path, root)
        for n, line in prose_lines(path):
            line = re.sub(r"`[^`]*`", "``", line)
            for m in re.finditer(r"!?\[[^\]]*\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)", line):
                target = m.group(1)
                if re.match(r"^[a-z][a-z0-9+.-]*:", target):
                    continue
                file_part, _, frag = target.partition("#")
                dest = path if not file_part else os.path.normpath(
                    os.path.join(os.path.dirname(path), file_part)
                )
                if not os.path.exists(dest):
                    errors.append(f"{rel}:{n}: link does not resolve: {target}")
                elif frag and dest.endswith(".md") and frag not in anchors(dest):
                    errors.append(f"{rel}:{n}: no heading for anchor: {target}")


def enum_variants(root, name):
    """Variant names of `pub enum <name>` in faraday-core."""
    base = os.path.join(root, "faraday/faraday-core/src")
    for d, _, names in os.walk(base):
        for n in names:
            if not n.endswith(".rs"):
                continue
            with open(os.path.join(d, n), encoding="utf-8") as f:
                text = f.read()
            m = re.search(r"pub enum %s\s*\{" % name, text)
            if not m:
                continue
            depth, i, body = 1, m.end(), []
            while depth:
                c = text[i]
                depth += c in "{(" and 1 or (c in "})" and -1 or 0)
                body.append(c)
                i += 1
            flat = re.sub(r"//[^\n]*", "", "".join(body[:-1]))
            out, depth, cur = [], 0, ""
            for c in flat:
                if c in "{(":
                    depth += 1
                elif c in "})":
                    depth -= 1
                elif depth == 0:
                    cur += c
            for v in cur.split(","):
                v = re.sub(r"#\[[^\]]*\]", "", v).strip()
                if re.match(r"^[A-Z]\w*$", v):
                    out.append(v)
            return out
    return []


SCREENS = "docs/guide/screens"
TO_WRITE = "docs/guide/screens/README.md"


def to_write(root):
    """{variant: (enum, line)} listed under "## To write" in the screens'
    README.md: the variants whose page is still to be written."""
    path = os.path.join(root, TO_WRITE)
    out = {}
    if not os.path.exists(path):
        return out
    inside = False
    for num, line in prose_lines(path):
        if line.startswith("#"):
            inside = re.match(r"^##\s+To write\s*$", line) is not None
            continue
        m = re.match(r"^- `(Screen|Sheet)`:\s*(.*)$", line) if inside else None
        if m:
            for v in re.findall(r"`([^`]+)`", m.group(2)):
                out[v] = (m.group(1), num)
    return out


def check_screens(root, errors):
    folder = os.path.join(root, SCREENS)
    if not os.path.isdir(folder):
        return
    named = {}
    for n in sorted(os.listdir(folder)):
        if n.endswith(".md") and n != "README.md":
            for num, line in prose_lines(os.path.join(folder, n)):
                m = re.match(r"^\*\*Screen:\*\*\s*(.*)$", line)
                if m:
                    for v in re.split(r"[,\s]+", m.group(1).replace("`", "")):
                        if v:
                            named.setdefault(v, []).append(f"{SCREENS}/{n}:{num}")
    todo = to_write(root)
    variants = {enum: enum_variants(root, enum) for enum in ("Screen", "Sheet")}
    every = {v for vs in variants.values() for v in vs}
    for v, places in named.items():
        if v not in every:
            errors.append(f"{places[0]}: `{v}` is not a Screen or Sheet variant")
        elif len(places) > 1:
            errors.append(f"{places[1]}: `{v}` is named by more than one page: {', '.join(places)}")
    for v, (enum, num) in todo.items():
        if v not in variants[enum]:
            errors.append(f"{TO_WRITE}:{num}: `{v}` is not a {enum} variant")
        elif v in named:
            errors.append(f"{TO_WRITE}:{num}: `{v}` has a page ({named[v][0]}); take it off To write")
    for enum, vs in variants.items():
        for v in vs:
            if v not in named and v not in todo:
                errors.append(
                    f"{SCREENS}: no page names the {enum} variant {v}, and To write does not list it"
                )


def run(root):
    errors = []
    files = guide_files(root)
    count = check_names(root, files, errors)
    check_links(root, files, errors)
    check_screens(root, errors)
    return count, errors


def self_test():
    with tempfile.TemporaryDirectory() as root:

        def put(rel, text):
            p = os.path.join(root, rel)
            os.makedirs(os.path.dirname(p), exist_ok=True)
            with open(p, "w", encoding="utf-8") as f:
                f.write(text)

        put(ALLOW, "# test values\nPlace 1  # place name\n")
        put("docs/GUIDE.md", "# G\n\nPress **Wallets**, then **Make a new wallet** and **Key 2 · Place 1**.\n")
        put("docs/guide/walk/a.md", "# A\n\n![x](../p.png)\n\n[up](../../GUIDE.md#g)\n")
        put("docs/guide/p.png", "")
        en = 'strings! {\n    a: "Wallets",\n    b: "Make a new wallet",\n    c: "Key {}",\n}\n'
        put(STRINGS, en)
        _, errors = run(root)
        assert not errors, errors
        put(STRINGS, en.replace("Make a new wallet", "Make a wallet"))
        _, errors = run(root)
        assert len(errors) == 1 and "Make a new wallet" in errors[0] and errors[0].startswith("docs/GUIDE.md:3:"), errors
        put(STRINGS, en)
        put("docs/guide/walk/a.md", "# A\n\n![x](../gone.png)\n\n[up](../../GUIDE.md#nope)\n")
        _, errors = run(root)
        assert len(errors) == 2, errors
        put("docs/guide/walk/a.md", "# A\n")
        # The screens: each variant on one page, or on the To write list.
        put(
            "faraday/faraday-core/src/lib.rs",
            "pub enum Screen {\n    Home,\n    Files,\n}\npub enum Sheet {\n    Power,\n}\n",
        )
        put(SCREENS + "/home.md", "# Home\n\n**Screen:** `Home`, `Power`\n")
        put(TO_WRITE, "# S\n\n## To write\n\n- `Screen`: `Files`\n")
        _, errors = run(root)
        assert not errors, errors
        put(TO_WRITE, "# S\n\n## To write\n\n- `Screen`: `Home`\n")
        _, errors = run(root)
        assert len(errors) == 2 and "take it off" in errors[0] and "Files" in errors[1], errors
        put(TO_WRITE, "# S\n\n## To write\n\n- `Screen`: `Files`, `Gone`\n")
        put(SCREENS + "/power.md", "# Power\n\n**Screen:** `Power`\n")
        _, errors = run(root)
        assert len(errors) == 2 and "more than one page" in errors[0] and "Gone" in errors[1], errors
    print("lint-guide self-test: ok")


def main():
    if "--self-test" in sys.argv:
        self_test()
        return 0
    root = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
    count, errors = run(root)
    for e in errors:
        print(e)
    if errors:
        print(f"lint-guide: {len(errors)} problem(s) in {count} bold names")
        return 1
    print(f"lint-guide: {count} bold names checked, ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
