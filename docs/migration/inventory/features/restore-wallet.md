# Restoring a wallet

Restore a wallet (`Screen::Restore`) brings an existing wallet back onto the device. It can start from:

- typed seeds;
- a wallet description or multisig configuration (a file in the Inbox, or a scan);
- split share sheets;
- a vault record.

It then matches or types each seed, and checks and finishes the wallet. It is for the person at the offline device. It is opened from:

- the Wallets start page;
- a loaded key's "From this key" (`KeyWallet`);
- Check an address and Decode (`RestoreFor(Check::Address|Decode)`, which come back through `restore_return`);
- share files in the Inbox (Files' Restore from shares);
- a vault stick;
- a words or seed-part file (`RestoreSeedFile(k)`).

## Screens and sheets

Logic: `faraday/faraday-core/src/restore.rs`. Drawing: `faraday/faraday-core/src/screens/restore.rs`.

- `Screen::Restore` steps (`rstep` / `RSTEPS`):
  1. Kind.
  2. Quorum (multisig kinds only).
  3. Description.
  4. Seeds.
  5. Check.
  6. Done.
- Back goes to Catalog, Decode, Home, Backups, Wallets or WalletsStart. The fallback is WalletsStart.
- Kind card: the kinds of `NewKind::ALL`.
- Description card (`restore_description`, `faraday/faraday-core/src/screens/restore.rs:~225`):
  - "I have the seed" (single key);
  - "I do not have it" (multisig, `RSeeds`);
  - pick share files (`RShare(k)`) and `RRebuild`;
  - shortcuts `ScanFor(Ask::Description)`, `RFromFiles` and `RUse(k)`.
- Seeds card (`restore_seeds`, `faraday/faraday-core/src/screens/restore.rs:~324`). There is one slot per key, with its fingerprint and state. An open slot takes:
  - words (`RSlotWords`);
  - a scan (`RSlotScan`);
  - other forms (`RSlotForm(code)`, `RSlotForms`, or `EntryOtherForms(Opening::RestoreSlot)`);
  - a vault's seed;
  - a loaded key;
  - a cosigner's xpub (`Seeds(S::Open(Focus::Cosigner(k)))`).
- The Seeds card also has "Not here" (`RAbsent(k)`, multisig only) and `Seeds(S::Make)`.
- Sources (`restore_sources` and `restore_sources_compact`, `faraday/faraday-core/src/screens/restore.rs:~656, ~758`):
  - Copy files from stick (`VisitFrom(Screen::Restore)`);
  - Lock;
  - unlock a vault (`Vault(OpenFrom(k, Screen::Restore))`);
  - a wallet from an open vault (`RFromVault(v, r)`).
- Check step: `QrWallet(i)`, and "It matches" / "Nothing to compare" (`RNext`).
- Done card: `backup::wallet_done` with `From::Restore`.
- `Screen::Entry` is used for typed seeds. `Sheet::OtherForms` is used for other forms.
- The Sticks-and-vaults board is shared with Create, including the Use vaults card.

## Actions

Load the description
- From a descriptor or configuration file, in the Inbox or by camera (`RUse`). `restore_arrived` takes only the Wallet and Share kinds.
- From split share files: `RShare` toggles a file, and `RRebuild` merges the ticked shares plus the open vaults' share records.
- From a vault record (`RFromVault`).

Seeds
- Match or type each seed: `RSlotWords`, `RSlotForms`, `RSlotForm`, `RSlotScan`.
- `RAbsent` marks a slot Not here. It is only on described wallets.

Vaults and files
- `RExistingVault(on)` brings one vault stick in and unlocks it before the seeds.
- `RestoreSeedFile(k)` opens Seeds with a words or seed-part file.

Steps
- `RNext`, Kind changes (`restore_kind`), Quorum changes.

## Effects

- Wallets are added to the session (`session.add_wallet`).
- Keys are added from typed seeds, from vault seeds (the `vault_load` job, `record_ticks`, `swap_skip`), from scans and from seed files.
- `refresh_spend`, `explore_hide`, `address_check::reopen` and `decode.reread_ours` run on return (`faraday/faraday-core/src/restore.rs:1984, 1987`).
- The stick is read: Copy files from stick, or bring in one vault stick.
- The pool cache sits in `kept` under a 64-bit hash key (`DefaultHasher | 1`). Key 0 means no Restore and drops all kept pools.

## State and lock

- `RestoreState` lives in `RestoreArea.state: Option`. The flow exists only while it is open.
- It holds seed fingerprints, and the cosigner box's text through `seeds::SeedsState`.
- Typed seed words and xprvs are passed as arguments, not stored. The passphrase is read from `entry` without being copied.
- Lock: `RestoreArea::lock` calls `clear()`, which drops the state. `holds` is typed when cosigner text is typed (`faraday/faraday-core/src/restore.rs:~1999-2010`).
- The process restarts at every lock (`faraday/faraday-core/src/vault/seal.rs:933-946`).
- The flow ends on `restore_return`, on `wallet_left` (its wallet removed), or at lock. Leaving the screen does not drop it.
- A wallet made from anything but typed seeds returns to the check that opened Restore.
- A description's keys are cleared from the seeds; typed seeds keep them.
- With a stick put in during Restore, the screen says what holds a secret and offers Lock (fix 66737c616).
- `restore::Pool` (`faraday/faraday-core/src/restore.rs:132-140`) holds multisig config share texts and what they merge to: fingerprints, paths and xpubs, nothing secret. They are `Text`, which wipes on drop.

## Reach

- Opens:
  - Add a key (`Screen::Entry`, Other forms);
  - the scanner;
  - Files (copy from stick);
  - vaults (unlock, load);
  - Back up (the Done card);
  - Check an address and Decode (on return).
- Reads `inbox`, `vault`, `wallet_backup::{sheets_format, split_share, multisig_config}`, `create` (`wallet_flow_kind`), `seeds` (the seeds piece), `refresh_spend` and `explore_hide`.
- Used by the backup's public files: `restore::checked_config` and `restore::sheets_rebuild`. Used by the backup through `restore::repeats_a_seed` (`faraday/faraday-core/src/wallet_backup.rs:183`).

## Rules and refusals
- Restore's pool of shares is cached under one key, a 64-bit hash (low bit set) of the shares chosen, the Inbox and vault edit counts and the open vault ids; one pool is kept and a different key recomputes it (faraday/faraday-core/src/restore.rs:1632-1663; drawing/kept.rs:188-209).

Description and configuration (`faraday/faraday-core/src/restore.rs`)
- Derivation text (`~48-63`, `~89-91`):
  - `m`-form, with 1 to 255 steps;
  - digits, no leading zero, below 2^31;
  - one `'` or `h` per step;
  - maximum length 1 + 255×12.
- A config line is refused (`~69-119`) when it has:
  - no `:`;
  - a `Policy:` that is not `M of N`;
  - a key fingerprint that is not 8 hex digits, or an xpub that does not decode;
  - a bad derivation;
  - no key lines;
  - more key lines than n.
- At most 15 keys (`multisig_config::MAX_KEYS`, `core/faraday-bip/src/multisig_config.rs:50`). The parser stops at that count (`faraday/faraday-core/src/restore.rs:98`). A policy over it, or more key lines than n, is refused (`faraday/faraday-core/src/restore.rs:109`).
- A setup file with no Format line is read as P2SH. A SLIP-132 key naming another script is refused (fix 953f62e77).
- A sheet whose Derivation is not a plain path is not offered (fix dbb38f02b).
- `merge` (`faraday/faraday-core/src/restore.rs:309-345`) refuses:
  - text that is not multisig (`restore_not_a_multisig`);
  - a share naming one fingerprint twice (`restore_key_twice`);
  - shares that disagree on m, n or Format, or on Name (trimmed);
  - a fingerprint with a second xpub or derivation;
  - more keys than n (`restore_these_shares`);
  - no shares.
- A share sheet with more than one xpub or path is refused at merge (fix 2bb9d3a1e).
- Shares are joined only with those of the same seed (fix 09a60d6d0).
- The rebuild, `tr(NUMS, sortedmulti_a(..))` for P2TR or a Coldcard config, happens only once all keys are in hand. It keeps the name the sheets carry (fix 690835427).
- `rebuilds` requires the same first receive and change address.
- `sheets_rebuild` (`~455-485`) checks every m-subset when `choose(n,m) <= 10`, else the first m and the last m.
- `repeats_a_seed` is true when two keys of one policy share a fingerprint.
- `RRebuild` with fewer keys than n shows `act_of_keys_in_hand`.
- Two complete wallets sharing a checksum are refused (`restore_two_wallets`).

Kind
- `restore_kind` ignores an index outside `NewKind::ALL`, and ignores any change on a described wallet, without notice (`faraday/faraday-core/src/restore.rs:1198-1214`).
- A multisig kind resets n to 3; a single kind sets it to 1.
- Pressing Kind moves on only when the wallet is not described (`faraday/faraday-core/src/screens/restore.rs:~142-146`).

Seeds
- A described wallet refuses (`NOT_A_KEY`) a fingerprint that is not in its slots. With `key_at`, a slot matches only if the whole xpub matches at its path (`faraday/faraday-core/src/restore.rs:1232-1245`). A seed matches a slot only by its whole extended key, not by fingerprint (fix 070ceab53).
- A seeds wallet refuses (`restore_refusal_every`) a fingerprint already in, or when there is no empty slot.
- The same check runs on typed words plus passphrase before anything loads (`~1262-1279`).
- A refused key stays refused with its words, parts and passphrase. Nothing refused is loaded (fix 8ee16fe78). A refusal is cleared when its cause changes (fix fcfa15964).
- A tick past the free slots is refused and drawn disabled (fix c813939cc).
- Only loaded keys outside the set count. Ticks follow presence (fix 44dfaf28f).
- A seed loaded in Restore clears Not here on every slot of its fingerprint (fix 9288d196a).
- A finished wallet takes no loose key (fix 0e1315252).
- The open slot refuses `Way::Words`, `Aezeed`, `Holder` and `AddAKey` (`faraday/faraday-core/src/screens/restore.rs:609-613`).
- `open_slot_key` is None unless the entry's back is Restore (`faraday/faraday-core/src/screens/restore.rs:855-858`).
- "Not here" is only for multi kinds (`faraday/faraday-core/src/screens/restore.rs:~424, ~445-455, ~506-518`).
- `ready` means all slots are filled and the wallet is described or its descriptor builds (`~533-535`).
- Path rows show only when the wallet is not described and a seed exists (`~538-540`).
- These need `may_load_keys()` (`faraday/faraday-core/src/restore.rs:~1081, ~1399, ~1574`):
  - `RSlotScan`;
  - `restore_open_on_seed_file`;
  - `restore_from_vault`'s key loading.
- So Restore loads no seed with a stick attached or before the self-test passes (fix 8d1fbefa3).
- Seed file:
  - only the Words and SeedPart kinds;
  - a bad SeedPart toasts `seed_check_not_seed`.

Make and check
- `restore_described_make` (`faraday/faraday-core/src/restore.rs:1505-1518`) refuses while a slot is open and not Not here (`restore_described_make_still`, with the count).
- A Restore from typed seeds is finished as Make finishes it (fix e4347e8d4).
- The Check step says "make the wallet" when there is no wallet or SEEDS is not done (`faraday/faraday-core/src/screens/restore.rs:~166-203`).
- "Nothing to compare" shows only when the wallet is not described and seeds exist (`~175-189`).
- The end card shows only with a wallet and SEEDS done (`~207`).
- Back and Done return to the opening screen. A wallet from a loaded key ends on "Wallet made" (fix 067eb5949).
- A wallet restored from a vault goes the vault route in Spend, whatever its name (fix 7502785d2).

Drawing (`faraday/faraday-core/src/screens/restore.rs`)
- Nothing is drawn without restore state or a step flow (`~22-25`, `~57-59`).
- "I have the seed" shows only for single-key kinds; "I do not have it" only for multisig (`~229-238`).
- Share rows show only `FileKind::Share`. The description shortcut shows only `FileKind::Wallet` (`~79`, `~308`).
- Rebuild is primary only when the merged description is whole (`~255-285`). A merge error has the Error tone.
- Sources (`~665, ~673-677, ~692, ~710, ~722-723, ~789, ~814`):
  - the stick is offered only when present and no secret is held;
  - Lock only when a secret is held and a stick is present;
  - vault files for unlock only when the vault is closed;
  - wallets only from open vaults.
- Auto-advancing choices draw no Continue. An unanswered closed row shows no Change (fix ab1d7d6ce).
- Files' Restore from shares stands for the share files and ticks one wallet's. A scanned share ticks that share. The wallet is made at once when the shares hold every key (fix f863cb34f).

## Domain calls

- `faraday_bip`: `multisig_config`, `slip132`, `WalletPolicy`.
- `faraday_keys`: identity fingerprint, `xpub_at`.
- `faraday_vault` records.
- `faraday_crypto::work::Compute`.

## Fixtures

None of its own. The multisig configs and wallet files of `faraday/faraday-core/tests/vectors/interop/` (listed under wallets) are read through the same intake.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 8ee16fe78 | A refused key (typed words, shares, SLIP-39 or Seed XOR parts) was not kept refused: the form was cleared or the refusal not shown; a loaded key could leak into a Restore slot. | A refused key stays refused with its words, parts and passphrase; nothing refused is loaded. |
| 067eb5949 | Restore's Back and Done went to a fixed screen; a wallet from a loaded key ended on the restore title. | Back and Done return to the opening screen; a loaded-key wallet ends on "Wallet made". |
| f863cb34f | Files' Restore from shares merged every share file that ended on Wallets, or toasted. | The button stands for the share files and ticks one wallet's; a scanned share ticks that share; the wallet is made at once when shares hold every key. |
| 66737c616 | A stick put in during Restore gave no account of what holds a secret and no Lock. | The stick state is stated and Lock offered. |
| 44dfaf28f | A loaded key outside the seed set was miscounted; a tick stayed on a removed key or after a kind change; a described wallet listed loaded keys it does not use. | Only loaded keys outside the set count; ticks follow presence. |
| fcfa15964 | A key refusal stayed on screen after the slot count changed or another step opened. | The refusal is cleared when its cause changes. |
| c813939cc | Restore could tick more keys than free slots. | A tick past free slots is refused and drawn disabled; a slot filled another way takes off the latest ticks. |
| 0e1315252 | Restore from typed seeds showed Keys 3 of 3 wrongly; a loose key could still be added to a finished wallet; a description read first skipped the vaults question. | A finished wallet takes no loose key; closed cards and counts read what the setup did. |
| 7502785d2 | A wallet restored from a vault was filed as Named, so Spend routed it as Paper. | A wallet from a vault goes the vault route, whatever its name. |
| 866f4847d | The multisig share parser accepted a sixteenth key line, a policy over 15 keys, more key lines than the policy names, an over-long derivation, and shares adding up to more keys than the policy. | A config holds at most 15 keys; key lines never exceed n; derivation capped at the BIP-32 maximum; merged shares never exceed n. |
| ab1d7d6ce | Restore's Use vaults card drew a Continue although Yes and No move on by themselves; a closed step with no answer showed Change. | Auto-advancing choices draw no Continue; an unanswered closed row shows no Change. |
| 070ceab53 | A seed was matched to a slot by its 4-byte fingerprint, so a different key with the same fingerprint could take the slot. | A seed matches a slot only by its whole extended key. |
| 953f62e77 | A multisig setup file with no Format line was refused or misread. | No-Format files are read as P2SH, as Coldcard reads them; a key naming a different script is refused. |
| 2bb9d3a1e | A share sheet with more than one xpub or path was merged. | A sheet with more than one key or path is refused at merge. |
| 690835427 | A Taproot multisig rebuilt from share sheets came back as "Restored wallet". | The rebuilt wallet keeps the name the sheets carry. |
| dbb38f02b | A sheet whose Derivation was not a plain path was still offered. | Non-plain derivations are not offered. |
| 9288d196a | A seed loaded in Restore cleared Not here only on the first slot of its fingerprint. | Every slot of the fingerprint is cleared. |
| 8d1fbefa3 | Restore from a vault loaded a seed with a stick attached or before the self-test passed. | Restore loads no seed with a stick attached or before the self-test. |
| 42e7ced21 | A multisig config's 16th cosigner line was decoded; a SLIP-39 set with iteration exponent up to 15 was read and computed. | A multisig config is read to 15 keys only; an exponent above 5 is refused before any round. |
| 09a60d6d0 | Restore and codex32 could combine shares of different seeds or wallets. | Shares are joined only with those of the same seed, grouped by the key their records name. |
| c36e2a142 | A codex32 string typed back was checked only in part; a share naming a key twice, or two keys for one fingerprint, was accepted. | The whole string is checked; duplicate keys in a share set are refused. |

## Doubts

- Rows given to this feature but left out as not about it:
  - 9d1dd6af2, 48689014d, 7c45ea8d2 (My seed makes my wallet; carried in seed-checks);
  - cd2e42596, 083d29bc5 (carried in create-wallet);
  - 367d9df3c, 4e38374d4, 6bd234da3 (Spend from a backup);
  - 90a8dd9e2, f3638bd23 (Krux slot in Create);
  - 064e4307a (Make a new key);
  - 3d56431d1, 9e01117be, ff368d14b (Add a key and its stick);
  - a6a7c4001, 29888ccd0, 933e0ba65 (Convert);
  - 626edb6e9, 84d1e5c70 (backup plan);
  - bdb2fe102, 458fc9a41 (Back up);
  - 2f7768c51 (codex32 worksheet);
  - 67856c674 (GPG key restore).
- One source gave the key cap as 16. The code says 15 (`core/faraday-bip/src/multisig_config.rs:50`), and 15 is written here.
- `sheets_rebuild`'s `1 << n` is unbounded for n ≥ 32. In practice `MAX_KEYS` bounds it.
- Dead or test-only:
  - `restore_share_texts` is used by tests only;
  - `readable` has no caller found;
  - `hide_shown` is a no-op;
  - `still_fits` is always true.
- Drawing calls `restore::merge` while it draws. Whether the result is cached was not checked.
