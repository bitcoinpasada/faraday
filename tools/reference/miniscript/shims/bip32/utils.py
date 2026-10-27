"""Written here, not upstream: python-bip32's path reader, as
python-bip380 calls it for a key origin's path ("m/48h/1'/0"): one child
number per step, hardened when it ends in h, H or '."""

from . import HARDENED_INDEX


def _deriv_path_str_to_list(strpath):
    parts = strpath.split("/")
    if parts[0] != "m":
        raise ValueError("a path starts at m")
    out = []
    for p in parts[1:]:
        hard = p[-1:] in ("h", "H", "'")
        n = int(p[:-1] if hard else p)
        if not 0 <= n < HARDENED_INDEX:
            raise ValueError("a child number out of range")
        out.append(n + HARDENED_INDEX if hard else n)
    return out
