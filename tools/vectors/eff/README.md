# EFF diceware wordlists

The three lists the Electronic Frontier Foundation publishes for
passphrases rolled with dice, verbatim. Each line is the dice roll then
the word, tab-separated.

| File | Dice | Words | Use |
|---|---|---|---|
| `eff_large_wordlist.txt` | five d6 | 7776 | the long list: 12.9 bits per word, six words for 77 bits |
| `eff_short_wordlist_1.txt` | four d6 | 1296 | the short list: 10.3 bits per word, short words |
| `eff_short_wordlist_2_0.txt` | four d6 | 1296 | the second short list: unique three-letter prefixes, an edit distance of at least 3 between any two words |

Source: `https://www.eff.org/files/2016/07/18/eff_large_wordlist.txt`,
`https://www.eff.org/files/2016/09/08/eff_short_wordlist_1.txt`,
`https://www.eff.org/files/2016/09/08/eff_short_wordlist_2_0.txt`,
downloaded 2026-09-12. SHA-256:

- large: `addd35536511597a02fa0a9ff1e5284677b8883b83e986e43f15a3db996b903e`
- short 1: `8f5ca830b8bffb6fe39c9736c024a00a6a6411adb3f83a9be8bfeeb6e067ae69`
- short 2.0: `22b45c52e0bd0bbf03aa522240b111eb4c7c0c1d86c4e518e1be2a7eb2a625e4`

To refresh, download again and check that a changed digest has a
reason. The lists are the input of the Tools › Dice passphrase tool; a
test checks the baked lists against these files.
