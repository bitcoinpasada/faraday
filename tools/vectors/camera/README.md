# Camera: known answers

The known answers of the camera's addition (F158;
`tools/reference/entropy/README.md`): frames are committed to as
SHA-256 over their SHA-256 digests, for a mix or for the session's
randomness, never a key alone. The procedure is Faraday's own and
carries the label "own construction, not yet independently reviewed"
(F160). A reproduction by an outside party is asked for in
`docs/AUDIT-SUBMISSIONS.md` §10.

Faraday's tests read `vectors.json` (`core/faraday-entropy/src/camera.rs`):
the frames are pushed into `CameraNoise` and what it takes, refuses,
commits to and makes must be these.

| Key | What |
|---|---|
| `frames_needed_per_128_bits` | three frames for each 128 bits, rounded up: a guard against a still scene, not an estimate of entropy |
| `vectors` | three frames of 512 luma bytes, frame f's byte i being (7i + 3f) mod 256: their digests, the commitment, the entropy at 128 bits, and the refusal at 256 bits for too few frames (6 needed) |
| `frames` | a frame taken or refused alone: 4096 bytes of one value, refused for fewer than 32 distinct values; 256 bytes of i mod 32, the least a frame may carry, taken |

## The procedure

- A frame is its luma bytes. It is refused when it holds fewer than 32
  distinct byte values, or when its SHA-256 is one already taken.
- The commitment is SHA-256 over the concatenated SHA-256 digests of
  the frames taken, in the order taken.
- The entropy at a strength is the commitment's first strength / 8
  bytes, once ⌈strength / 128⌉ × 3 frames are taken; fewer are refused.

## To reproduce

From the top of the repository, with Python 3's standard library:

    import json
    from hashlib import sha256
    v = json.load(open("tools/vectors/camera/vectors.json"))
    per = v["frames_needed_per_128_bits"]
    for case in v["vectors"]:
        frames = [bytes.fromhex(f) for f in case["frames"]]
        assert all(len(set(f)) >= 32 for f in frames)
        digests = [sha256(f).digest() for f in frames]
        assert [d.hex() for d in digests] == case["frame_digests"]
        c = sha256(b"".join(digests)).digest()
        assert c.hex() == case["commitment"]
        assert len(frames) >= -(-case["strength"] // 128) * per
        assert c[:case["strength"] // 8].hex() == case["entropy"]
        few = case["too_few"]
        assert few["have"] == len(frames) < few["need"] == -(-few["strength"] // 128) * per
    for f in v["frames"]:
        frame = bytes.fromhex(f["frame"])
        assert len(set(frame)) == f["distinct"]
        assert (f["distinct"] >= 32) == f["taken"]
    print("all agree")

The answers were computed with Python's `hashlib` from the procedure
above. The cross-check's `entropy` area
(`tools/crosscheck/area_entropy.py`) compares Faraday's Rust with a
model written by Faraday's own authors; it is not an outside
implementation.
