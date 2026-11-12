#!/usr/bin/env python3
"""Guide lint (docs/guide/).

Keeps the user guide checked against the app, so that a string renamed in
the app fails `just lint` until the guide follows.

1. Names. Every bold span in docs/guide/**/*.md (outside
   fenced code) is joined onto one line and split on " -> " and " . "
   (the arrow and the middle dot the app writes). Each part must be an
   en.rs string, a filled-in en.rs template (`{}` and `{name}` read as
   any text), or a line of tools/lint-guide.allow (test values and place
   names, each with its reason).
2. Links. Every image and every relative link resolves to a file, and a
   `#anchor` to a heading of that file (GitHub's anchors).
3. Keys (docs/GUIDE-IN-APP.md §1.3). The `**Key:**` lines of
   docs/guide/<place>.md: a key written twice fails, a key no place
   reaches fails where `guide_key` maps the place by step, tab or
   category (it returns a part key of it; the code's keys are read from
   `guide_key` in faraday/faraday-core/src/learn.rs), and a place the code
   reaches that has no page and is not in docs/guide/TO-WRITE.txt fails. A
   section keyed as a reached part with a word more (`<part>-<word>`) is
   read after that part and counts as reached. A
   section of a place that shows its parts at once is found from the
   contents and its address, and needs no place to reach it.

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
    files = []
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
            if re.match(r"^(Kind|Status|Open|Screen|Group|Starter page|Starter sheet):$", span):
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


KEYS_SRC = "faraday/faraday-core/src/learn.rs"
KEYS_TO_WRITE = "docs/guide/TO-WRITE.txt"
KEY_LITERAL = re.compile(r'"([a-z][a-z0-9-]*(?:\.[a-z0-9-]+)?)"')


def code_keys(root):
    """The keys `guide_key` works out, read from its source."""
    src = os.path.join(root, KEYS_SRC)
    if not os.path.exists(src):
        return None
    with open(src, encoding="utf-8") as f:
        text = f.read()
    start = text.find("pub fn guide_key")
    if start < 0:
        return None
    end = text.find("pub enum GuideAim", start)
    return KEY_LITERAL.findall(text[start:end])


def page_keys(root):
    """{key: [file:line, ...]} written in the pages docs/guide/<place>.md
    (the `**Key:**` lines), and the places that have a page."""
    folder = os.path.join(root, "docs/guide")
    keys, paged = {}, set()
    for n in sorted(os.listdir(folder)):
        path = os.path.join(folder, n)
        if not n.endswith(".md") or not os.path.isfile(path) or n == "README.md":
            continue
        paged.add(n[:-3])
        for num, line in prose_lines(path):
            m = re.match(r"^\*\*Key:\*\*\s*(\S+)\s*$", line)
            if m:
                keys.setdefault(m.group(1), []).append(f"docs/guide/{n}:{num}")
    return keys, paged


def sub_part(key, reached):
    """A section that follows a part reached and carries its key with a
    word more (`vault-contents.gpg-keys-term` after `vault-contents.gpg-keys`)
    is read in that part's place: the guide opens at the part and the
    section is next on the page."""
    return any(key.startswith(k + "-") for k in reached if "." in k)


def check_keys(root, errors):
    """docs/GUIDE-IN-APP.md §1.3: a key is written once, every key written
    is one a place reaches, and every place the app reaches has a page or
    is listed in docs/guide/TO-WRITE.txt."""
    reached = code_keys(root)
    if reached is None:
        return
    written, paged = page_keys(root)
    by_part = {k.split(".")[0] for k in reached if "." in k}
    for key, places in written.items():
        if len(places) > 1:
            errors.append(f"{places[1]}: key `{key}` is written twice: {', '.join(places)}")
        elif key not in reached and key.split(".")[0] in by_part and not sub_part(key, reached):
            errors.append(f"{places[0]}: key `{key}` is reached by no place (guide_key in {KEYS_SRC})")
    todo = {}
    path = os.path.join(root, KEYS_TO_WRITE)
    if os.path.exists(path):
        with open(path, encoding="utf-8") as f:
            for num, line in enumerate(f, 1):
                if line.strip() and not line.startswith("#"):
                    todo[line.strip()] = num
    places = {k.split(".")[0] for k in reached}
    for place in sorted(places):
        if place not in paged and place not in todo:
            errors.append(f"{KEYS_TO_WRITE}: place `{place}` has no page and is not listed")
    for place, num in todo.items():
        if place not in places:
            errors.append(f"{KEYS_TO_WRITE}:{num}: `{place}` is no place the app reaches")
        elif place in paged:
            errors.append(f"{KEYS_TO_WRITE}:{num}: `{place}` has a page; take it off the list")


def starter_variants(root):
    """The variants of the Starter's `Page` enum, or None without the source."""
    src = os.path.join(root, "faraday/faraday-starter/src/lib.rs")
    if not os.path.exists(src):
        return None
    with open(src, encoding="utf-8") as f:
        text = f.read()
    m = re.search(r"pub enum Page \{(.*?)\n\}", text, re.S)
    if not m:
        return None
    body = re.sub(r"//[^\n]*", "", m.group(1))
    return re.findall(r"^\s{4}([A-Z]\w*)\s*[,{(]", body, re.M)


# Starter pages with no guide page of their own: the guide itself, and
# About, which holds the version, the build's facts and the self-test
# (docs/GUIDE-IN-APP.md §5.4).
STARTER_NO_PAGE = {"Guide", "About"}


def check_starter_pages(root, errors):
    """The `**Starter page:**` lines of docs/guide/starter-*.md name every
    variant of the Starter's `Page` once, except STARTER_NO_PAGE, and nothing
    else."""
    variants = starter_variants(root)
    if variants is None:
        return
    folder = os.path.join(root, "docs/guide")
    named = {}
    for n in sorted(os.listdir(folder)):
        if not (n.startswith("starter-") and n.endswith(".md")):
            continue
        for num, line in prose_lines(os.path.join(folder, n)):
            m = re.match(r"^\*\*Starter page:\*\*\s*(.*)$", line)
            if m:
                for v in re.findall(r"`([^`]+)`", m.group(1)):
                    named.setdefault(v, []).append(f"docs/guide/{n}:{num}")
    for v in variants:
        where = named.get(v, [])
        if v in STARTER_NO_PAGE:
            if where:
                errors.append(f"Starter page `{v}` has no guide page, but {where[0]} names it")
            continue
        if not where:
            errors.append(f"Starter page `{v}` is named by no docs/guide/starter-*.md page")
        elif len(where) > 1:
            errors.append(f"Starter page `{v}` is named more than once: {', '.join(where)}")
    for v, where in sorted(named.items()):
        if v not in variants:
            errors.append(f"{where[0]}: `{v}` is no variant of the Starter's Page")


def run(root):
    errors = []
    files = guide_files(root)
    count = check_names(root, files, errors)
    check_links(root, files, errors)
    check_keys(root, errors)
    check_starter_pages(root, errors)
    return count, errors


def self_test():
    with tempfile.TemporaryDirectory() as root:

        def put(rel, text):
            p = os.path.join(root, rel)
            os.makedirs(os.path.dirname(p), exist_ok=True)
            with open(p, "w", encoding="utf-8") as f:
                f.write(text)

        put(ALLOW, "# test values\nPlace 1  # place name\n")
        put("docs/guide/g.md", "# G\n\nPress **Wallets**, then **Make a new wallet** and **Key 2 · Place 1**.\n")
        put("docs/guide/a.md", "# A\n\n![x](p.png)\n\n[up](g.md#g)\n")
        put("docs/guide/p.png", "")
        en = 'strings! {\n    a: "Wallets",\n    b: "Make a new wallet",\n    c: "Key {}",\n}\n'
        put(STRINGS, en)
        _, errors = run(root)
        assert not errors, errors
        put(STRINGS, en.replace("Make a new wallet", "Make a wallet"))
        _, errors = run(root)
        assert len(errors) == 1 and "Make a new wallet" in errors[0] and errors[0].startswith("docs/guide/g.md:3:"), errors
        put(STRINGS, en)
        put("docs/guide/a.md", "# A\n\n![x](gone.png)\n\n[up](g.md#nope)\n")
        _, errors = run(root)
        assert len(errors) == 2, errors
        put("docs/guide/a.md", "# A\n")
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
