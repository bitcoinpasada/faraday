// A KDBX 4 database locked with a passphrase and a YubiKey's HMAC-SHA1 secret,
// made by kdbxweb (KeeWeb's library, MIT), which Faraday's authors did not
// write. kdbxweb takes the YubiKey as a function from the challenge to the
// 20-byte response; here the function is HMAC-SHA1 under a fixed secret, so no
// hardware is needed. Argon2 is hash-wasm's.
//
//   node make-yubikey.mjs OUT.kdbx
//
// Run in a directory where kdbxweb 2.1.1 and hash-wasm 4.13.0 are installed
// (`make-yubikey.sh` installs them, and puts the file in the folder it is given). Entries: `Example` (user `alice`, password
// `hunter2`) and `Other` (user `bob`, password `secret5`).
import { createHmac } from "node:crypto";
import { writeFileSync } from "node:fs";
import kdbxweb from "kdbxweb";
import { argon2d, argon2id } from "hash-wasm";

const PASSPHRASE = "keyed vector passphrase";
const SECRET = Buffer.from("000102030405060708090a0b0c0d0e0f10111213", "hex");

kdbxweb.CryptoEngine.setArgon2Impl(async (password, salt, memory, iterations, length, parallelism, type, version) => {
  const f = type === kdbxweb.CryptoEngine.Argon2TypeArgon2id ? argon2id : argon2d;
  const out = await f({
    password: new Uint8Array(password),
    salt: new Uint8Array(salt),
    parallelism,
    iterations,
    memorySize: memory,
    hashLength: length,
    outputType: "binary",
  });
  return out.buffer.slice(out.byteOffset, out.byteOffset + out.byteLength);
});

const response = async (challenge) => new Uint8Array(createHmac("sha1", SECRET).update(Buffer.from(challenge)).digest());
const credentials = new kdbxweb.Credentials(kdbxweb.ProtectedValue.fromString(PASSPHRASE), null, response);
const db = kdbxweb.Kdbx.create(credentials, "keyed");
db.setKdf(kdbxweb.Consts.KdfId.Argon2d);
// Argon2 at 8 MiB, 2 passes, 2 lanes, as the other vectors (kdbxweb's own 1 MiB is below Faraday's floor).
const kdf = db.header.kdfParameters;
kdf.set("M", kdbxweb.VarDictionary.ValueType.UInt64, new kdbxweb.Int64(8 * 1024 * 1024));
kdf.set("I", kdbxweb.VarDictionary.ValueType.UInt64, new kdbxweb.Int64(2));
kdf.set("P", kdbxweb.VarDictionary.ValueType.UInt32, 2);
for (const [title, user, password] of [["Example", "alice", "hunter2"], ["Other", "bob", "secret5"]]) {
  const entry = db.createEntry(db.getDefaultGroup());
  entry.fields.set("Title", title);
  entry.fields.set("UserName", user);
  entry.fields.set("Password", kdbxweb.ProtectedValue.fromString(password));
}
writeFileSync(process.argv[2], Buffer.from(await db.save()));
