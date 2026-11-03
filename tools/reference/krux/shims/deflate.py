"""Stands in for MicroPython's `deflate` module (`import deflate`), which
krux's `bbqr.py` uses as `DeflateIO(stream)` to write and as
`DeflateIO(BytesIO(data))` to read, raw deflate. Written for the
cross-check, not krux's, over the standard library's zlib.

krux calls `DeflateIO(stream)` with no format and no window, which
MicroPython's documentation (library/deflate.rst in the MicroPython repository, "DeflateIO")
gives as a raw stream compressed with a 256-byte window (wbits 8) and, for
a raw stream, read with a 256-byte window. zlib's smallest raw window is
512 bytes (wbits 9: it refuses 8), so this writes with that, the nearest
the standard library has; every stream a 256-byte window writes is valid
in it, and what it writes is read by any reader of a window of 512 bytes
or more (Coinkite's reader uses 1,024). Reading accepts the largest
window, where a device reads 256 bytes and refuses a stream of Coinkite's
that reaches further back (a named difference of the area).
"""

import zlib


class DeflateIO:
    def __init__(self, stream):
        self.stream = stream
        self.buffer = b""

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        if not exc[0] and self.buffer is not None and self.writing:
            c = zlib.compressobj(wbits=-9)
            self.stream.write(c.compress(self.buffer) + c.flush())
        return False

    writing = False

    def write(self, data):
        self.writing = True
        self.buffer += bytes(data)

    def read(self):
        try:
            return zlib.decompress(self.stream.read(), -15)
        except zlib.error as e:
            raise ValueError(str(e))
