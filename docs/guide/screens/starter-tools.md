# Faraday Starter: Tools

**Kind:** Spec
**Status:** Written from the app's code and strings (`faraday/faraday-starter/src/draw/home.rs`, `faraday/faraday-starter/src/draw.rs`, `faraday/faraday-starter/src/template.rs`, `faraday/faraday-starter/src/lib.rs`); not yet read beside the running app
**Open:** the pictures, drawn once after the theming has landed
**Starter page:** `Tools`, `Templates`, `TemplateForm`, `TemplateWords`, `TemplateLength`, `TemplateCopy`, `TemplateWorksheets`, `TemplatePlaces`
**Group:** Starter

**Opened from:** **Tools** on the rail or the bar.

## Tools

*Picture to come: `starter-tools.png`, Tools with its two cards.*

| Card | Line | Button |
|---|---|---|
| **Transfer** | **Send and Receive** | **Open Transfer**, the primary button: opens Faraday's own [Transfer](qr-and-scan.md#transfer) under the Starter's top app bar, which reads **Transfer** with the way back to **Tools** |
| **Blank templates** | **Words, SeedQR and codex32** | **Open blank templates**: opens [Blank templates](#blank-templates) |

## Blank templates

*Picture to come: `starter-tools-blank-templates.png`, Blank templates with the BIP-39 words rows.*

Printable sheets to copy a seed onto by hand. A blank template holds
nothing of any wallet: no wallet, key or seed is asked for or shown.
Its way back is **Tools**.

A row for each choice, its value at the right; a press opens the
choice's page. The rows depend on **Form**:

| Row | With **BIP-39 words** | With **codex32** |
|---|---|---|
| **Form** | **BIP-39 words** | **codex32** |
| **Words** | **24** at first, or **12** | — |
| **Length** | — | "48 characters" at first, "74 characters" or "127 characters" |
| **Copy** | **Words** at first, **SeedQR** or **Both** | **String** at first, **Codex32QR**, **Both** or **Four Codex32QR grids** |
| **Worksheets** | — | **None** at first, or **Dice, checksum and translation** |
| **Places** | 2 at first | 2 at first |

| Control | What it does |
|---|---|
| **Print** | the primary button: makes the sheet as a PDF and opens it in this computer's PDF viewer, to print from there. Nothing is sent to a printer unasked |
| **Save the PDF** | saves the same PDF into Downloads, never over a file: a second copy is named "(2)" |
| "Saved …" | in the ok tone, after a save: where the file went |
| **Open it** | under it: opens the saved PDF in the viewer |
| the reason | in the error tone, when the PDF could not be saved or the viewer could not be opened |

A choice changed clears the "Saved …" line.

### A choice's page

Each choice is a page of its own, named as its row (**Form**,
**Words**, **Length**, **Copy**, **Worksheets**, **Places**), with its
way back to **Blank templates**. A ring is filled on the option chosen;
a press chooses it. **Continue** goes back to Blank templates.

| Page | Options |
|---|---|
| **Form** | **BIP-39 words**, **codex32** |
| **Words** | **12**, **24** |
| **Length** | "48 characters", "74 characters", "127 characters". A length that no longer allows the copy or the worksheets chosen puts them back to **String** and **None** |
| **Copy** | with BIP-39 words: **Words** (the word lines), **SeedQR** (the SeedQR grid), **Both** (both on one sheet). With codex32: **String**; at 48 characters only, also **Codex32QR**, **Both** and **Four Codex32QR grids** |
| **Worksheets** | **None**; **Dice, checksum and translation**, not at 127 characters |
| **Places** | the count of places to fill in by hand, at the top as a row (**Places** and the number); **Fewer** and **More** change it, never below one |

Faraday's backup checklist asks for these templates: a 12-word one in
[Back it up](../walk/back-up.md), a codex32 one in [codex32
shares](../walk/codex32.md).
