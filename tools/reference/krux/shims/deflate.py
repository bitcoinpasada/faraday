"""Stands in for MicroPython's `deflate` module (`import deflate`), which
krux's `bbqr.py` uses as `DeflateIO(stream)` to write and as
`DeflateIO(BytesIO(data))` to read, raw deflate. Written for the
cross-check, not krux's, over the standard library's zlib.

krux calls `DeflateIO(stream)` with no format and no window, which the
firmware it pins reads as a raw stream with a 1,024-byte window
(selfcustody/micropython `b11923f020e0caf2572e5874957f29128da2bb45`,
`extmod/moddeflate.c`, `DEFLATEIO_DEFAULT_WBITS` 10; MicroPython's own
documentation gives 256 bytes). This writes and reads raw deflate with
zlib's `wbits` -10, the window the firmware uses.

Reading stops where the firmware's does: `DEFLATEIO_MAX_DECOMPRESSED_SIZE`
is 100 * 1024 bytes, and `deflateio_read` caps each read to what is left
and raises `ValueError` once none is left and the stream has not ended.
`read()` takes the stream 256 bytes at a time (`stream_readall`), and the
decoder stops at the end of a read without reading the end-of-block code, so
a stream of exactly 102,400 bytes is read to its limit with the stream not
yet ended, and the next read raises: the firmware opens up to 102,399 bytes.
"""

import zlib

MAX_DECOMPRESSED_SIZE = 100 * 1024


class DeflateIO:
    def __init__(self, stream):
        self.stream = stream
        self.buffer = b""

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        if not exc[0] and self.buffer is not None and self.writing:
            c = zlib.compressobj(wbits=-10)
            self.stream.write(c.compress(self.buffer) + c.flush())
        return False

    writing = False

    def write(self, data):
        self.writing = True
        self.buffer += bytes(data)

    def read(self):
        try:
            # One byte at a time: zlib counts what a single call has written
            # as available besides its window, so only a call that gives one
            # byte holds a match to the 1,024 bytes of the window, as the
            # firmware's decoder does.
            d = zlib.decompressobj(-10)
            data = self.stream.read()
            out = bytearray()
            while not d.eof:
                before = len(data)
                piece = d.decompress(data, 1)
                data = d.unconsumed_tail
                out += piece
                if len(out) >= MAX_DECOMPRESSED_SIZE:
                    raise ValueError("decompressed data exceeds size limit")
                if not piece and len(data) == before and not d.eof:
                    raise ValueError("incomplete stream")
            return bytes(out)
        except zlib.error as e:
            raise ValueError(str(e))
