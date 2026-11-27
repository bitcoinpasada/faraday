"""A small reader of Rust source for the lints of `tools/` (`lint-layers`, `lint-test-headers`).

It is not a parser. It cuts a file into tokens with their line numbers (comments and the insides
of strings gone, doc comments kept on request), drops the items a `#[cfg(test)]` guards, reads
`use` trees into a map from the name a file gives to the path it means, and lists the paths a
file writes. That is enough to ask "does this module name that path" by what is written, not by
a substring. Standard library only; load it with `importlib` (the lints run as `python3 -I -B`).
"""

import re
from typing import NamedTuple


class Tok(NamedTuple):
    kind: str  # id | lt | str | chr | num | p | doc (outer) | idoc (inner)
    text: str
    line: int


_IDSTART = re.compile(r"[A-Za-z_]")
_IDREST = re.compile(r"[A-Za-z0-9_]*")
_NUM = re.compile(r"[0-9][0-9A-Za-z_]*")
_PUNCT2 = ("::", "->", "=>", "&&", "||", "==", "!=", "<=", ">=", "..", "<<", ">>")


def _raw_string_end(src: str, i: int):
    """`src[i:]` starts after an `r`: return (end index, text) of a raw string, or None."""
    j = i
    while j < len(src) and src[j] == "#":
        j += 1
    if j >= len(src) or src[j] != '"':
        return None
    hashes = j - i
    close = '"' + "#" * hashes
    end = src.find(close, j + 1)
    if end < 0:
        return len(src), src[j + 1 :]
    return end + len(close), src[j + 1 : end]


def tokenize(src: str, docs: bool = False) -> list:
    """The tokens of `src`. Comments are dropped; with `docs`, `///`, `//!` and `/** */` stay as
    `doc` tokens whose text is the comment without its marker."""
    toks = []
    i, n, line = 0, len(src), 1
    while i < n:
        c = src[i]
        if c == "\n":
            line += 1
            i += 1
        elif c in " \t\r":
            i += 1
        elif src.startswith("//", i):
            end = src.find("\n", i)
            end = n if end < 0 else end
            text = src[i:end]
            if docs and text.startswith("///") and not text.startswith("////"):
                toks.append(Tok("doc", text[3:], line))
            elif docs and text.startswith("//!"):
                toks.append(Tok("idoc", text[3:], line))
            i = end
        elif src.startswith("/*", i):
            depth, j, start, start_line = 1, i + 2, i, line
            while j < n and depth:
                if src.startswith("/*", j):
                    depth += 1
                    j += 2
                elif src.startswith("*/", j):
                    depth -= 1
                    j += 2
                else:
                    if src[j] == "\n":
                        line += 1
                    j += 1
            text = src[start:j]
            if docs and text.startswith("/**") and not text.startswith("/***") and text != "/**/":
                toks.append(Tok("doc", text[3:-2], start_line))
            elif docs and text.startswith("/*!"):
                toks.append(Tok("idoc", text[3:-2], start_line))
            i = j
        elif c == '"' or (c == "b" and src.startswith('b"', i)):
            j = i + (2 if c == "b" else 1)
            start = j
            while j < n and src[j] != '"':
                if src[j] == "\\":
                    j += 1
                if j < n and src[j] == "\n":
                    line += 1
                j += 1
            toks.append(Tok("str", src[start:j], line))
            i = j + 1
        elif c in "rb" and (
            (c == "r" and i + 1 < n and src[i + 1] in '"#')
            or (c == "b" and src.startswith("br", i) and i + 2 < n and src[i + 2] in '"#')
        ):
            k = i + (2 if c == "b" else 1)
            raw = _raw_string_end(src, k)
            if raw is None:  # `r#name`, a raw identifier
                m = _IDREST.match(src, k + 1)
                toks.append(Tok("id", src[k + 1 : m.end()], line))
                i = m.end()
            else:
                end, text = raw
                toks.append(Tok("str", text, line))
                line += src.count("\n", i, end)
                i = end
        elif c == "'":
            # A lifetime ('a) or a character literal ('a', '\n', '\'').
            if i + 1 < n and src[i + 1] == "\\":
                j = i + 2
                while j < n and src[j] != "'":
                    j += 1
                toks.append(Tok("chr", src[i : j + 1], line))
                i = j + 1
            elif i + 2 < n and src[i + 2] == "'":
                toks.append(Tok("chr", src[i : i + 3], line))
                i += 3
            else:
                m = _IDREST.match(src, i + 1)
                toks.append(Tok("lt", src[i : m.end()], line))
                i = max(m.end(), i + 1)
        elif _IDSTART.match(c):
            m = _IDREST.match(src, i + 1)
            toks.append(Tok("id", src[i : m.end()], line))
            i = m.end()
        elif c.isdigit():
            m = _NUM.match(src, i)
            toks.append(Tok("num", m.group(), line))
            i = m.end()
        else:
            two = src[i : i + 2]
            if two in _PUNCT2:
                toks.append(Tok("p", two, line))
                i += 2
            else:
                toks.append(Tok("p", c, line))
                i += 1
    return toks


_OPEN, _CLOSE = "([{", ")]}"


def _match_close(toks, i):
    """`toks[i]` opens a bracket: the index of its closing one (len(toks) if none)."""
    depth = 0
    for j in range(i, len(toks)):
        t = toks[j]
        if t.kind == "p":
            if t.text in _OPEN:
                depth += 1
            elif t.text in _CLOSE:
                depth -= 1
                if depth == 0:
                    return j
    return len(toks)


def _is_test_cfg(attr) -> bool:
    """The tokens inside `#[...]`: a `cfg(...)` that holds `test` and no `not`."""
    if not attr or attr[0].text != "cfg" or len(attr) < 2 or attr[1].text != "(":
        return False
    names = [t.text for t in attr if t.kind == "id"]
    return "test" in names and "not" not in names


def _item_end(toks, i):
    """The index just after the item that starts at `toks[i]` (after its attributes)."""
    n = len(toks)
    j = i
    while j < n:
        t = toks[j]
        if t.kind == "p" and t.text == ";":
            return j + 1
        if t.kind == "p" and t.text == "{":
            return _match_close(toks, j) + 1
        if t.kind == "p" and t.text in "([":
            j = _match_close(toks, j)
        j += 1
    return n


def drop_test_items(toks: list) -> list:
    """`toks` without the items a `#[cfg(test)]` guards, and empty if the file is `#![cfg(test)]`."""
    out, i, n = [], 0, len(toks)
    while i < n:
        t = toks[i]
        if t.kind == "p" and t.text == "#":
            j = i + 1
            inner = j < n and toks[j].text == "!"
            if inner:
                j += 1
            if j < n and toks[j].text == "[":
                close = _match_close(toks, j)
                if _is_test_cfg(toks[j + 1 : close]):
                    if inner:
                        return []
                    k = close + 1
                    # further attributes of the same item
                    while k < n and toks[k].text == "#" and k + 1 < n and toks[k + 1].text == "[":
                        k = _match_close(toks, k + 1) + 1
                    i = _item_end(toks, k)
                    continue
        out.append(t)
        i += 1
    return out


class Use(NamedTuple):
    path: tuple  # full path, `*` last for a glob
    name: str  # the name it gives in the file ("" for a glob or `_`)
    vis: str  # "" or the visibility text ("pub", "pub(crate)")
    line: int


def _expand(toks, i, end, prefix, out, vis, line):
    """Read a use tree in toks[i:end] after `prefix`, appending Use to `out`."""
    seg = list(prefix)
    first = i
    while i < end:
        t = toks[i]
        if t.kind == "p" and t.text == "{":
            close = _match_close(toks, i)
            j = i + 1
            depth_start = j
            k = j
            parts = []
            depth = 0
            while k < close:
                tk = toks[k]
                if tk.kind == "p" and tk.text in _OPEN:
                    depth += 1
                elif tk.kind == "p" and tk.text in _CLOSE:
                    depth -= 1
                elif tk.kind == "p" and tk.text == "," and depth == 0:
                    parts.append((depth_start, k))
                    depth_start = k + 1
                k += 1
            if depth_start < close:
                parts.append((depth_start, close))
            for a, b in parts:
                _expand(toks, a, b, seg, out, vis, line)
            return
        if t.kind == "p" and t.text == "*":
            out.append(Use(tuple(seg) + ("*",), "", vis, line))
            return
        if t.kind == "id" and t.text == "as":
            alias = toks[i + 1].text if i + 1 < end else ""
            out.append(Use(tuple(seg), "" if alias == "_" else alias, vis, line))
            return
        if t.kind == "id":
            if t.text == "self" and seg and i == first and (i + 1 >= end or toks[i + 1].text != "::"):
                # `a::{self}` names `a` itself
                out.append(Use(tuple(seg), seg[-1], vis, line))
                return
            seg.append(t.text)
        i += 1
    if seg:
        out.append(Use(tuple(seg), seg[-1], vis, line))


def split_uses(toks: list):
    """(uses, rest): the `use` declarations of a token list, and the tokens that are not in one."""
    uses, rest, i, n = [], [], 0, len(toks)
    while i < n:
        t = toks[i]
        if t.kind == "id" and t.text == "use":
            # visibility is the tokens just before: pub, pub(crate), pub(in path)
            vis = ""
            k = len(rest)
            if k and rest[-1].text == ")":
                depth, m = 0, k - 1
                while m >= 0:
                    if rest[m].text == ")":
                        depth += 1
                    elif rest[m].text == "(":
                        depth -= 1
                        if depth == 0:
                            break
                    m -= 1
                if m > 0 and rest[m - 1].text == "pub":
                    vis = "pub" + "".join(x.text for x in rest[m : k])
                    del rest[m - 1 :]
            elif k and rest[-1].text == "pub":
                vis = "pub"
                rest.pop()
            end = i + 1
            depth = 0
            while end < n:
                te = toks[end]
                if te.text in _OPEN and te.kind == "p":
                    depth += 1
                elif te.text in _CLOSE and te.kind == "p":
                    depth -= 1
                elif te.text == ";" and te.kind == "p" and depth == 0:
                    break
                end += 1
            start = i + 1
            if start < end and toks[start].text == "::":
                start += 1
            _expand(toks, start, end, [], uses, vis, t.line)
            i = end + 1
            continue
        rest.append(t)
        i += 1
    return uses, rest


def alias_map(uses) -> dict:
    """Name a file's `use`s give -> the full path it stands for."""
    return {u.name: u.path for u in uses if u.name and u.path[-1] != "*"}


def globs(uses) -> list:
    """The paths a file imports with `*`, without the `*`."""
    return [u.path[:-1] for u in uses if u.path[-1] == "*"]


def paths(toks: list):
    """Every written path in `toks` (outside `use`): (segments tuple, line). A path is identifiers
    joined by `::`; a generic argument ends it."""
    out, i, n = [], 0, len(toks)
    while i < n:
        t = toks[i]
        if t.kind == "id":
            # A path is not continued from a `.field` or a lifetime; a method call is `.name(`.
            if i > 0 and toks[i - 1].text == "." and toks[i - 1].kind == "p":
                i += 1
                continue
            segs = [t.text]
            line = t.line
            j = i + 1
            while j + 1 < n and toks[j].text == "::" and toks[j + 1].kind == "id":
                segs.append(toks[j + 1].text)
                j += 2
            out.append((tuple(segs), line))
            i = j
        else:
            i += 1
    return out


def resolve(segs: tuple, amap: dict) -> tuple:
    """A written path with its first segment replaced by what a `use` made it."""
    if segs and segs[0] in amap:
        return tuple(amap[segs[0]]) + tuple(segs[1:])
    return segs


def has_seq(path: tuple, seq: tuple) -> bool:
    """Does `path` hold `seq` as consecutive segments?"""
    m = len(seq)
    return any(path[k : k + m] == seq for k in range(len(path) - m + 1))


def crate_ident(name: str) -> str:
    return name.replace("-", "_")


def all_paths(toks: list):
    """(uses, resolved paths): the file's uses, and every path it names, uses included, each
    resolved through the file's own `use`s: [(resolved tuple, line, via_use)]."""
    uses, rest = split_uses(toks)
    amap = alias_map(uses)
    named = [(tuple(u.path), u.line, True) for u in uses]
    named += [(resolve(p, amap), line, False) for p, line in paths(rest)]
    return uses, named


def outer_attrs_before(toks: list, i: int):
    """The attributes (as token lists inside `#[...]`) that sit just before the item at `toks[i]`."""
    out = []
    j = i
    while j > 0 and toks[j - 1].text == "]":
        depth, k = 0, j - 1
        while k >= 0:
            if toks[k].text == "]":
                depth += 1
            elif toks[k].text == "[":
                depth -= 1
                if depth == 0:
                    break
            k -= 1
        if k > 0 and toks[k - 1].text == "#":
            out.append(toks[k + 1 : j - 1])
            j = k - 1
        else:
            break
    return list(reversed(out))
