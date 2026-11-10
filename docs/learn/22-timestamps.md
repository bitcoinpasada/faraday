# Timestamps

## What a timestamp proof is

An OpenTimestamps proof is a file ending `.ots` that shows a file existed by the time a Bitcoin block was mined. It holds a list of steps, each a hash or an append, that take the hash of your file to a value. Whoever made the proof sent that value to a calendar, a server that gathers many of them into one tree and puts the tree's root in a Bitcoin transaction. The block that holds the transaction commits to the root through its merkle root.

The proof does not hold the file, and it says nothing about who made the file or what is in it. It says only that these bytes existed before that block.

## What the device checks

Check a timestamp takes the proof, the file (or its SHA-256) and the 80-byte header of the block, and does the steps itself. The file's hash must be the one the proof starts from. Each path of steps must reach the header's merkle root. The result gives the block's hash, its height and the time in its header.

The device is offline and has no copy of the chain. A match says the header commits to the file; it does not say the header is in the chain.

## The header must come from a node you trust

Get the header from your own node, or from one you trust, on a computer other than the one that gave you the proof. Anyone can make a header that looks right for a block that was never mined at that difficulty only by doing the work, and the device refuses a header whose hash is above its own target. A header made with little work is still refused only when it fails that test on the network chosen, so a header from an untrusted source proves little. The time in a header is what its miner wrote, within about two hours of true time.

## What is shown and not counted

A path counts only when the file's hash and every hash on the path are SHA-256. A path through SHA-1, RIPEMD-160 or Keccak-256 is shown with the hash named and whether it reaches the header, and is not counted. SHA-1 is broken, and the other two give a shorter or different guarantee than the one Bitcoin's own work rests on. Many older proofs go through the calendar's RIPEMD-160, so they show as reaching a block through a named hash.

A pending attestation names a calendar that has not yet seen its block: it proves nothing until the proof is upgraded on a computer with a network. An attestation for Litecoin or Ethereum is listed by its height, and an unknown kind by its tag. A Bitcoin attestation whose header does not match is listed as not matching.

## What is refused

A file that is not a proof, a version other than 1, a proof over 64 KiB, a tree over its limits, a header that is not 80 bytes, and a header whose hash is above its own target. A refused proof shows the reason in one line.
