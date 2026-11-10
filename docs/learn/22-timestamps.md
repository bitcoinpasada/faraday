# Timestamps

## What a timestamp proof is

An OpenTimestamps proof is a file ending `.ots` that shows a file existed by the time a Bitcoin block was mined. It holds a list of steps, each a hash, an append, a prepend, a reverse or a hexlify, that take the hash of your file to a value. Whoever made the proof sent that value to a calendar, a server that gathers many of them into one tree and puts the tree's root in a Bitcoin transaction. The block that holds the transaction commits to the root through its merkle root.

The proof does not hold the file, and it says nothing about who made the file or what is in it. It says only that these bytes existed before that block.

## What the device checks

Check a timestamp takes the proof, the file (or its SHA-256) and the 80-byte header of the block, and does the steps itself. The file's hash must be the one the proof starts from. One path of steps that reaches the header's merkle root, through SHA-256 alone, is enough. The result gives the block's hash, the height the proof states and the time in the header.

The height is written in the proof and nothing checks it: the header holds no height. Only the block hash is worth comparing, with a node you trust.

The device is offline and has no copy of the chain. A match says the header commits to the file; it does not say the header is in the chain.

## The header must come from a node you trust

Get the header from your own node, or from one you trust, on a computer other than the one that gave you the proof. The device refuses a header whose hash is above its own target, but a network's easiest allowed target is cheap to meet: on mainnet it is difficulty 1, and a header at that limit can be made with a few billion hashes. So a header from an untrusted source proves little, and the header must come from a node you trust. The time in a header is what its miner wrote; it cannot be more than about two hours ahead of true time, and how far behind it can be is only bounded by the blocks before it.

## What is shown and not counted

A path counts only when the file's hash and every hash on the path are SHA-256. A path through SHA-1, RIPEMD-160 or Keccak-256 is shown with the hash named and whether it reaches the header, and is not counted. SHA-1 is broken, and the other two give a shorter or different guarantee than the one Bitcoin's own work rests on. Many older proofs go through the calendar's RIPEMD-160, so they show as reaching a block through a named hash.

A pending attestation names a calendar that has not yet seen its block: it proves nothing until the proof is upgraded on a computer with a network. An attestation for Litecoin or Ethereum is listed by its height, and an unknown kind by its tag. A Bitcoin attestation whose header does not match is listed as not matching.

## What is refused

A file that is not a proof, a version other than 1, a number written in more than 9 bytes, the same step twice in one place, Keccak-256 as the file's hash, a proof over 64 KiB, a tree over its limits, a header that is not 80 bytes, and a header whose hash is above its own target. A refused proof shows the reason in one line.
