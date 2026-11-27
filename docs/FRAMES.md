# Frames: what the runtime draws, keeps and hands out

**Kind:** Spec
**Open:** The runtime builds `View` and `Show`, calls the root's frame, keeps the hit targets, the Tab order, the pointer state and the laid facts and sends the laid facts once; the painting of the kit's call list into the buffer, the motion store's moves (scrolling, `Fx::View` orders, page offsets), the frames kept for fades and sheets and the hold control's completion are the kit's and wait for it (spec 04 §7, §8).

This is how the runtime draws a frame (spec 05 §2.5) and what it keeps between frames. The kit draws
(spec 04); the runtime owns what outlives a frame.

## 1. One frame

1. `View` and `Show` are built from shared borrows (05 §4.1): `Show` carries the wording, the memo
   and the note on show, and goes to `present` alone.
2. A `Ui` is built over the canvas with the last frame's pointer state, the motion store and the
   theme from `view.settings`; `Root::frame` presents then draws. The screen model is a local of
   `Root::frame` and is dropped when it returns.
3. `Ui::finish` gives the `FrameOut`: hits, the Tab (`reach`) order, anchors, scrolled regions, laid
   facts, the `secret` flag, the pace and the page's and sheets' anchors.
4. The canvas replays its call list into the frame buffer (wiping, P13), recording the changed
   rectangles.
5. The laid facts are compared with the last one sent for each anchor. Each that differs, or is the
   first, becomes the root's `press(press)` and is handled as one event after the frame; one more
   frame is drawn if that dispatched anything. A fact that changes again in that frame is sent after
   the next event; a debug build asserts that layout settles in one frame.

## 2. What the runtime keeps

- The last frame's `FrameOut`, the pointer (position, down, the focused press) and the focus
  order. No model or state holds any of them (V3).
- A press is hit-tested against the last frame handed out, the one the person saw: the top layer
  drawn takes presses and, of its hits, the last drawn wins. The press is sent when the contact
  lifts on the hit it went down on, then passes `refuses` against the current state (R9).
- Tab and Back-tab move the focus along `reach`; Enter presses the focused control; any other key
  is the root's `input`.
- Notes queue; one comes up when the one before has been up for the note interval by the runtime's
  clock, and the change draws a frame.

## 3. At a lock

The frame buffer is zeroed, the call list and the frame's output are dropped and the waiting view
orders are dropped (spec 05 §7.2, step 7), so no pixel of the screen before the hide stays in the
runtime.
