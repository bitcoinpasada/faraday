# Wiping a stick

**Kind:** History
**Status:** The Learn page's text as it was edited before the in-app guide (`docs/guide/`) replaced it

## What Faraday writes

Faraday writes only into a stick's files partition. The stick's other partitions, its partition table and any space outside a partition are not touched, so zeroing the files partition reaches everything Faraday ever wrote to the stick.

## Why only the files partition

The process that reads sticks takes in whatever a stick brings, so it is given the files partition and nothing else. It can never write a boot partition or a partition table, where a bad stick could leave code for the next computer to boot.

## What zeros do not reach

No zeroing is a certain erase. A USB stick or SD card can keep old copies of a file's blocks where no write reaches: its controller moves data around inside the chip. Zeros stop recovery tools; a lab that reads the chip itself may still find them. Only destroying the stick is certain.
