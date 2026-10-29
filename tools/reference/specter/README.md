# Numbered text parts, as SeedSigner reads them

Specter Desktop shows a PSBT or a wallet's descriptor too long for one
QR code as a run of codes `p<n>of<m> <text>`, and SeedSigner reads
them. No document describes the format. The cross-check's `bbqr` area
(`tools/crosscheck/area_bbqr.py`) checks `faraday_qr::specter` against
SeedSigner's own reader, `SpecterWalletQrDecoder` in `decode_qr.py`,
vendored unchanged with what it imports in `tools/reference/seedsigner/`
(its README has the commit, the licence and every file's SHA-256).

| Source | URL | Commit | File | SHA-256 |
|---|---|---|---|---|
| SeedSigner | <https://github.com/SeedSigner/seedsigner> | `85cd9a0211eeb22962a5e65ba349ef6efa91bb57` | `src/seedsigner/models/decode_qr.py` | `ca35d03c9ede201893a01e5faceb96d58449e1600a28e564ae29d6a2b0248912` |

The part's number and count, `SpecterWalletQrDecoder`:

```python
    def current_segment_num(self, segment) -> int:
        if re.search(r'^p(\d+)of(\d+) ', segment, re.IGNORECASE) != None:
            return int(re.search(r'^p(\d+)of(\d+) ', segment, re.IGNORECASE).group(1))
    ...
    def parse_segment(self, segment) -> str:
        try:
            return re.search(r'^p(\d+)of(\d+) (.+$)', segment, re.IGNORECASE).group(3)
```

and how parts are put together, `BaseAnimatedQrDecoder.add`:

```python
        if self.total_segments == None:
            self.total_segments = self.total_segment_nums(segment)
            self.segments = [None] * self.total_segments
        elif self.total_segments != self.total_segment_nums(segment):
            raise Exception('Segment total changed unexpectedly')

        current_segment_num = self.current_segment_num(segment)
        if self.segments[current_segment_num - 1] == None:
            self.segments[current_segment_num - 1] = self.parse_segment(segment)
```

with the text the parts joined in order (`"".join(self.segments)`).

The area runs that decoder, `add` and its `segments`, over every set
and part, and refuses in front of it where SeedSigner's would fail or
misread: a part number of 0 (which SeedSigner files as the last part) or
past the count, a part arriving twice with other text (SeedSigner keeps
the first of), a part with no header (SeedSigner takes it as the whole
text). Faraday refuses those too, and two more the area counts as known
differences: an upper-case `P` or `OF`, which SeedSigner matches and
Faraday does not, and a count over 1,000, Faraday's limit.
