"""What the `vault` area reads of a KDBX file through pykeepass, what
`keepassxc-cli` shows of it, what Faraday is asked to put in one, and the
files pykeepass and `keepassxc-cli` make (`docs/VAULT.md` §7, §7.1).

A view is plain data (lists and text), the same shape as Faraday's own
answer to `vault.open`:

    {"label", "id", "custom": [[key, value]],
     "entries": [{"uuid", "path": [group names], "fields": [[key, value, protected]],
                  "custom": [[key, value]]}]}   sorted by UUID

A deep view adds what Faraday does not report and a seal must keep: each
entry's tags, icon, attachments (name and bytes) and history, and the groups.
"""

import hashlib
import os
import tempfile
import unicodedata
import uuid as uuidlib

from readers import open_kdbx

XPUB = ("[73c5da0a/84h/0h/0h]xpub6CatWdiZiodmUeTDp8LT5or8nmbKNcuyvz7WyksVFkKB4RHwCD3XyuvPEbvqAQY3rAPsh"
        "WcMLoP2fMFMKHPJ4ZeZXYVUhLv1VMrjPC7PW6V")
KINDS = ("entry", "note", "seed_words", "seed_hex", "wallet_policy", "wallet_descriptor", "sheet",
         "gpg", "secure_boot", "wallet_share", "passphrase", "share", "plan", "signed")
# The group each kind is put in (§7).
GROUPS = {"note": "Notes", "seed_words": "Bitcoin keys", "seed_hex": "Bitcoin keys",
          "wallet_policy": "Wallets", "wallet_descriptor": "Wallets", "sheet": "Recovery sheets",
          "gpg": "GPG keys", "secure_boot": "Secure Boot", "wallet_share": "Wallet shares",
          "passphrase": "Passphrases", "share": "Codex32 shares"}
SHARE = "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn"
SHARE_KEY = "fab6868a"


def text(element):
    return "" if element is None or element.text is None else element.text


def entry_view(e, path):
    """The part of an entry Faraday reports."""
    return {
        "uuid": e.uuid.hex,
        "path": path,
        "fields": [[text(s.find("Key")), text(s.find("Value")),
                    s.find("Value").get("Protected") == "True"]
                   for s in e._element.xpath("String")],
        "custom": [[text(i.find("Key")), text(i.find("Value"))]
                   for i in e._element.xpath("CustomData/Item")],
    }


def view(kp, deep=False):
    """pykeepass's reading of a database as a view, with the groups when
    `deep` and then each entry's tags, icon, attachments and history."""
    entries, groups = [], []

    def walk(g, path):
        for e in g.entries:
            v = entry_view(e, path)
            if deep:
                v["tags"] = sorted(e.tags or [])
                v["icon"] = e.icon
                v["attachments"] = sorted([a.filename, a.data.hex()] for a in e.attachments)
                v["history"] = [[[text(s.find("Key")), text(s.find("Value"))]
                                 for s in h._element.xpath("String")] for h in e.history]
            entries.append(v)
        for sub in g.subgroups:
            groups.append(path + [sub.name])
            walk(sub, path + [sub.name])

    walk(kp.root_group, [])
    out = {
        "label": kp.database_name or "",
        "id": kp.root_group.uuid.hex,
        "custom": [[text(i.find("Key")), text(i.find("Value"))]
                   for i in kp.tree.xpath("/KeePassFile/Meta/CustomData/Item")],
        "entries": sorted(entries, key=lambda v: v["uuid"]),
    }
    if deep:
        out["groups"] = sorted(groups)
    return out


def read_view(data, passphrase, deep=False):
    """The view of `data` under `passphrase`, or None when pykeepass will not open it."""
    kp = open_kdbx(data, passphrase)
    return None if kp is None else view(kp, deep)


def faraday_view(ans):
    """Faraday's `vault.open` answer cut to a view."""
    return {
        "label": ans["label"], "id": ans["id"], "custom": ans["custom"],
        "entries": sorted(({"uuid": e["uuid"], "path": e["path"], "fields": e["fields"],
                            "custom": e["custom"]} for e in ans["entries"]),
                          key=lambda v: v["uuid"]),
    }


def plain(v):
    """A view without what Faraday does not report: it reads the custom data items that
    begin `faraday.` and keeps the rest unread."""
    def mine(items):
        return [i for i in items if i[0].startswith("faraday.")]
    return {"label": v["label"], "id": v["id"], "custom": mine(v["custom"])} | {
        "entries": [{k: e[k] for k in ("uuid", "path", "fields")} | {"custom": mine(e["custom"])}
                    for e in v["entries"]]}


def fields_of(entry):
    return {f[0]: f[1] for f in entry["fields"]}


def kpxc_path(entry):
    return "/".join(entry["path"] + [fields_of(entry)["Title"]])


def kpxc_listing(v):
    """What `ls -R -f` lists: every group and entry by path."""
    groups = set()
    for e in v["entries"]:
        for i in range(1, len(e["path"]) + 1):
            groups.add("/".join(e["path"][:i]) + "/")
    for g in v.get("groups", []):
        for i in range(1, len(g) + 1):
            groups.add("/".join(g[:i]) + "/")
    return sorted(groups | {kpxc_path(e) for e in v["entries"]})


def kpxc_differs(k, passphrase, v):
    """What `keepassxc-cli` shows of the file against the view `v`: the
    listing, and every entry's fields in order. Gives the first difference as
    (what, shown, expected), or None."""
    got = k.entries(passphrase)
    if got != kpxc_listing(v):
        return ("ls -R -f", got, kpxc_listing(v))
    for e in v["entries"]:
        names = [f[0] for f in e["fields"]]
        want = "".join(f[1] + "\n" for f in e["fields"])
        shown = k.show(passphrase, kpxc_path(e), names)
        if shown != want:
            return (f"show -s {kpxc_path(e)}", shown, want)
    return None


# ----- what Faraday is asked to put in ---------------------------------------


def uuid_of(n):
    """The UUID the Rust command gives the n-th item of a request."""
    return bytes([(n & 0xFF) + 1] * 16).hex()


def expected(n, item):
    """(group path, the fields the vault must hold) of an item, a part of
    §7's layout. Nothing for a plan or the signed amounts (vault custom data)."""
    k = item["kind"]
    if k == "entry":
        f = {"Title": item["title"], "UserName": item["user"], "Password": item["password"],
             "URL": item["url"], "Notes": item["notes"]}
        if "totp" in item:
            f["otp"] = item["totp"]
        return [], f
    f = {"note": lambda: {"Title": "Note", "Notes": item["text"]},
         "seed_words": lambda: {"Title": item["label"]} | (
             {"passphrase": item["passphrase"]} if "passphrase" in item else {}),
         "seed_hex": lambda: {"Title": item["title"], "hex": item["seed"]},
         "wallet_policy": lambda: {"Title": item["name"], "policy": "wpkh(@0/**)\n" + XPUB},
         "wallet_descriptor": lambda: {"Title": item["name"],
                                       "descriptor": f"wpkh({XPUB}/<0;1>/*)"},
         "sheet": lambda: {"Title": item["name"], "descriptor": item["descriptor"],
                           "Notes": item["note"]},
         "gpg": lambda: {"Title": item["title"], "key": item["key"]}
         | {x: item[x] for x in ("public", "revocation", "revoked") if x in item},
         "secure_boot": lambda: {"Title": "Secure Boot keys",
                                 "owner": str(uuidlib.UUID(bytes_le=bytes.fromhex(item["owner"])))},
         "wallet_share": lambda: {"Title": item["name"], "Password": item["text"],
                                  "index": str(item["index"]), "of": str(item["of"])}
         | ({"wallet": item["wallet"]} if "wallet" in item else {}),
         "passphrase": lambda: {"Title": "Passphrase for " + item["key"], "Password": item["text"],
                                "for": item["key"]},
         "share": lambda: {"Title": "Codex32 share for " + SHARE_KEY, "Password": SHARE,
                           "for": SHARE_KEY}}.get(k)
    if f is None:
        return None
    return ([GROUPS[k]] if k in GROUPS else []), f()


def custom_expected(item, plans):
    """The vault custom data an item puts (keys), as (key, text that must be in the value)."""
    if item["kind"] == "plan":
        return [(f"faraday.plan.{plans}", item["place"])]
    if item["kind"] == "signed":
        return [("faraday.signed." + item["tx"], f"{item['first']},{item['second']}")]
    return []


# ----- random text -----------------------------------------------------------

POOL = ("abcdefghijklmnopqrstuvwxyz ABCXYZ 0123456789 <>&\"' é ß ñ ü ç å ø 日本語 ключ 🔑 "
        ",.;:!?()[]{}-_=+*#@%^~|")
LINE_POOL = POOL + "\n\t\n"
LETTERS = "abcdefghijklmnopqrstuvwxyzABCXYZ0123456789éßñü日本ключ"


def word(rng, lo, hi, lines=False):
    """Text of up to `hi` characters (none when `hi` is 0 or, from `lo` 0, now and then)
    that begins with a letter or digit (never a command line option) and has no space
    or line end at the end; line ends and tabs inside when `lines`."""
    if hi == 0 or (lo == 0 and rng.random() < 0.2):
        return ""
    n = rng.randint(max(lo, 1), max(hi, 1))
    pool = LINE_POOL if lines else POOL
    out = rng.choice(LETTERS) + "".join(rng.choice(pool) for _ in range(n - 1))
    return out.rstrip(" \n\t") or "x"


def passphrase(rng):
    """A passphrase in NFC (§3), with letters NFD writes differently, and no
    line end or space at either end."""
    parts = ["café", "Zoë", "naïve", "ǖ", "Å", "한글", "ḉ", "ệ", "ñandú", "über", "résumé"]
    p = " ".join(rng.choice(parts) for _ in range(rng.randint(1, 3)))
    p += word(rng, 0, 6) if rng.random() < 0.5 else ""
    return unicodedata.normalize("NFC", p)


# ----- the files pykeepass makes ---------------------------------------------


def py_database(rng, passphrase_, path):
    """A KDBX 4 database made by pykeepass at the lowest cost Faraday accepts
    (8 MiB; the changes of `tools/vectors/kdbx/make.py`), of a random cipher,
    KDF and compression, with groups, entries, tags, icons, custom properties,
    attachments, history and custom data. Returns the model of its entries:
    {(group path, title): {field: value}}."""
    import argon2
    from lxml import etree
    from pykeepass import create_database

    kp = create_database(path, password=passphrase_)
    groups = [kp.root_group]
    for i in range(rng.choice([0, 1, 2, 3])):
        parent = rng.choice(groups)
        groups.append(kp.add_group(parent, f"{word(rng, 1, 12)} g{i}", icon=rng.choice([None, "48"])))
    model = {}
    for i in range(rng.randint(1, 6)):
        g = rng.choice(groups)
        title = f"{word(rng, 1, 14)} e{i}"
        fields = {"UserName": word(rng, 0, 10), "Password": word(rng, 0, 20),
                  "URL": rng.choice(["", "https://example.org/" + word(rng, 1, 5)]),
                  "Notes": word(rng, 0, 80, lines=True)}
        # add_entry looks the title and user name up by xpath (a quote of each kind
        # breaks it, even with force_creation): they are set after.
        e = kp.add_entry(g, f"t{i}", "u", fields["Password"],
                         url=fields["URL"] or None, notes=fields["Notes"] or None,
                         tags=rng.choice([None, ["one"], ["alpha", "beta;gamma"]]),
                         icon=rng.choice([None, "0", "12", "30"]),
                         force_creation=True)
        e.title = title
        e.username = fields["UserName"]
        for j in range(rng.choice([0, 0, 1, 2])):
            e.set_custom_property(f"prop{j}", word(rng, 0, 30, lines=True),
                                  protect=rng.random() < 0.5)
        for j in range(rng.choice([0, 0, 1, 2])):
            e.add_attachment(kp.add_binary(rng.randbytes(rng.choice([0, 5, 300, 3000]))),
                             f"file{j}{rng.choice(['.bin', '.txt', ' é'])}")
        if rng.random() < 0.3:
            e.save_history()
            e.password = word(rng, 1, 12)
        if rng.random() < 0.3:
            cd = etree.SubElement(e._element, "CustomData")
            item = etree.SubElement(cd, "Item")
            etree.SubElement(item, "Key").text = "other.program"
            etree.SubElement(item, "Value").text = word(rng, 1, 20)
        model[(tuple(x.name for x in _chain(kp, g)), title)] = fields | {"Password": e.password}
    if rng.random() < 0.5:
        meta = kp.tree.xpath("/KeePassFile/Meta")[0]
        cd = meta.find("CustomData")
        if cd is None:
            cd = etree.SubElement(meta, "CustomData")
        item = etree.SubElement(cd, "Item")
        etree.SubElement(item, "Key").text = "other.vault"
        etree.SubElement(item, "Value").text = word(rng, 1, 20)
    # The cost and the shapes, as make.py does.
    header = kp.kdbx.header.value
    dyn = header.dynamic_header
    cipher = rng.choice(["aes256", "chacha20"])
    dyn.cipher_id.data = cipher
    if cipher == "chacha20":
        dyn.encryption_iv.data = rng.randbytes(12)
    dyn.compression_flags.data.compression = rng.random() < 0.5
    params = dyn.kdf_parameters.data.dict
    argon2id = rng.random() < 0.5
    params["M"].value = 8 * 1024 * 1024
    params["I"].value = rng.choice([1, 2, 3])
    params["P"].value = rng.choice([1, 2])
    if argon2id:
        params["$UUID"].value = b"\x9e\x29\x8b\x19\x56\xdb\x47\x73\xb2\x3d\xfc\x3e\xc6\xf0\xa1\xe6"
    key = hashlib.sha256(hashlib.sha256(passphrase_.encode()).digest()).digest()
    transformed = argon2.low_level.hash_secret_raw(
        secret=key, salt=params["S"].value, time_cost=params["I"].value,
        memory_cost=params["M"].value // 1024, parallelism=params["P"].value, hash_len=32,
        type=argon2.low_level.Type.ID if argon2id else argon2.low_level.Type.D, version=0x13)
    for f in ("data", "length", "offset1", "offset2"):
        kp.kdbx.header.pop(f, None)
    kp.save(transformed_key=transformed)
    return model


def _chain(kp, g):
    out = []
    while g.parentgroup is not None:
        out.append(g)
        g = g.parentgroup
    return reversed(out)


def py_file(rng, passphrase_):
    """(file bytes, the model of its entries) made by pykeepass."""
    fd, path = tempfile.mkstemp(suffix=".kdbx")
    os.close(fd)
    try:
        model = py_database(rng, passphrase_, path)
        with open(path, "rb") as f:
            return f.read(), model
    finally:
        os.unlink(path)


# ----- the files keepassxc-cli makes -----------------------------------------


def kpxc_edit(rng, k, passphrase_):
    """Changes the file in `k` the way a person uses `keepassxc-cli`: groups, entries,
    notes and attachments. Returns the model: {(group path, title): {field: value,
    "attachments": {name: bytes}}}."""
    groups = [[]]
    for i in range(rng.choice([0, 1, 2])):
        parent = rng.choice(groups)
        path = parent + [f"{word(rng, 1, 10)} g{i}"]
        if not k.mkdir(passphrase_, "/".join(path)):
            raise AssertionError(f"keepassxc-cli mkdir {path} failed")
        groups.append(path)
    model = {}
    for i in range(rng.randint(1, 4)):
        g = rng.choice(groups)
        title = f"{word(rng, 1, 14)} e{i}"
        f = {"UserName": word(rng, 1, 10), "Password": word(rng, 1, 20),
             "URL": "https://example.org/" + word(rng, 1, 5), "Notes": ""}
        entry = "/".join(g + [title])
        if not k.add(passphrase_, entry, f["UserName"], f["Password"], f["URL"]):
            raise AssertionError(f"keepassxc-cli add {entry} failed")
        if rng.random() < 0.6:
            f["Notes"] = word(rng, 1, 80, lines=True)
            if not k.edit_notes(passphrase_, entry, f["Notes"]):
                raise AssertionError(f"keepassxc-cli edit {entry} failed")
        files = {}
        for j in range(rng.choice([0, 0, 1, 2])):
            name = f"att{j}{rng.choice(['.bin', '.txt'])}"
            files[name] = rng.randbytes(rng.choice([1, 40, 3000]))
            src = os.path.join(k.dir.name, "source")
            with open(src, "wb") as h:
                h.write(files[name])
            if not k.attach(passphrase_, entry, name, src):
                raise AssertionError(f"keepassxc-cli attachment-import {entry} failed")
            os.unlink(src)
        model[(tuple(g), title)] = f | {"attachments": files}
    return model
