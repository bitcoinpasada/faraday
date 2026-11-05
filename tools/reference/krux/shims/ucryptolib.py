"""Stands in for the firmware's `ucryptolib` module, which krux's `kef.py`
uses as `aes(key, mode, ...)`: AES-256 in ECB, CBC, CTR and GCM. Written
for the cross-check, not krux's, over PyCA cryptography (OpenSSL).

The calls `kef.py` makes: `aes(key, MODE_ECB)`, `aes(key, MODE_CBC, iv)`,
`aes(key, MODE_CTR, nonce=iv)` (a 12-byte nonce, then a 4-byte big-endian
counter from 0), `aes(key, MODE_GCM, iv, mac_len=n)`; each object's
`encrypt(data)` and `decrypt(data)`, and for GCM `digest()` after
encrypting (the tag, of which `kef.py` takes the first `mac_len` bytes) and
`verify(tag)` after decrypting (it raises when the tag is not the object's).
"""

from cryptography.exceptions import InvalidTag
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes

MODE_ECB = 1
MODE_CBC = 2
MODE_CTR = 6
MODE_GCM = 7


class aes:
    def __init__(self, key, mode, iv=None, *, nonce=None, mac_len=16):
        self.mode = mode
        self.mac_len = mac_len
        self.tag = None
        if mode == MODE_ECB:
            m = modes.ECB()
        elif mode == MODE_CBC:
            m = modes.CBC(iv)
        elif mode == MODE_CTR:
            m = modes.CTR(bytes(nonce) + b"\x00\x00\x00\x00")
        elif mode == MODE_GCM:
            m = modes.GCM(iv, min_tag_length=4)
        else:
            raise ValueError("unknown mode")
        self.cipher = Cipher(algorithms.AES(key), m)
        self.coder = None

    def encrypt(self, data):
        c = self.cipher.encryptor()
        out = c.update(data)
        if self.mode == MODE_GCM:
            out += c.finalize()
            self.tag = c.tag
        return out

    def decrypt(self, data):
        self.coder = self.cipher.decryptor()
        return self.coder.update(data)

    def digest(self):
        return self.tag

    def verify(self, tag):
        try:
            self.coder.finalize_with_tag(bytes(tag))
        except InvalidTag as e:
            raise ValueError("tag mismatch") from e
