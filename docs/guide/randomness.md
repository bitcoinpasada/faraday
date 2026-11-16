# Add randomness

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Randomness`
**Key:** randomness

Add randomness lets you add randomness of your own to the system's, for every key and seal made later in the session. Yours is mixed with the system's and never replaces it, and the page opens from Settings or from the **Randomness** row above a press that draws the system's randomness.

## Status

**Key:** randomness.status

The status line reads **System:** with the source the system names, then **your own:** with what you have added so far, counted by source, or **none**. After a batch is added, a line gives how many times something was added and the last batch. Nothing is rated: the counts show what was taken, not how strong it is.

## Sources

**Key:** randomness.sources

**Add randomness to the pool** offers one source at a time, and nothing starts until you choose one.

**Camera** is offered where a camera is present. **Take a picture** adds that frame at once; a picture with too little variation is refused. **Dice** takes the faces 1 to 6. **Coins** takes **Heads** and **Tails**. **Cards** takes a rank, then a suit, from a deck; a card already drawn from it is refused, and the next batch starts from a full deck. **Pointer and keys** is offered on the desktop app only. It takes pointer movement over its area and typing in it; keys typed elsewhere are not taken.

## Entries

**Key:** randomness.entries

For dice, coins and cards, **In this batch** counts the entries not yet added. **Undo** takes the last entry back, **Clear** drops the batch, and **Add** puts it in the pool and empties it. Dice show **Enough for 256 bits on its own** from 99 rolls.

On a keyboard, dice take the digits 1 to 6, coins H and T, cards a rank and then S, H, D or C; Backspace undoes, Enter adds, and Space takes a camera picture.
