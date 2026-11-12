# Blank templates

**Kind:** Reference
**Status:** Checked against the app
**Starter page:** `Templates`, `TemplateWords`, `TemplateCopy`, `TemplatePlaces`, `TemplateForm`, `TemplateLength`, `TemplateWorksheets`, `TemplateSize`
**Key:** starter-templates

A sheet to copy a seed onto by hand, made with no wallet, key or seed. Each row opens the page of its choices, which goes back with **Continue**.

## Blank templates

**Key:** starter-templates.templates

The rows depend on **Form**. **Print** opens the sheet as a PDF in this computer's viewer. **Save the PDF** saves it in Downloads, never over a file, and shows Saved with its place and **Open it**.

## Form

**Key:** starter-templates.template-form

**BIP-39 words** gives the rows **Words**, **Copy** and **Places**. **codex32** gives **Length**, **Copy**, **Worksheets** and **Places**. **Krux code** gives **Size** and **Places**.

## Words

**Key:** starter-templates.template-words

The number of words of the seed, **12** or **24**, 24 at first.

## Copy

**Key:** starter-templates.template-copy

For words, **Words** is the word lines, **SeedQR** the SeedQR grid and **Both** the two on one sheet. For codex32, **String** is the written string, **Codex32QR** its grid, **Both** the two and **Four Codex32QR grids** four smaller grids on one page; all but **String** are offered at 48 characters only.

## Length

**Key:** starter-templates.template-length

The codex32 string's length: **48 characters**, **74 characters** or **127 characters**. A choice of copy or worksheets that the new length does not offer goes back to **String** or **None**.

## Worksheets

**Key:** starter-templates.template-worksheets

**None** prints the card alone. **Dice, checksum and translation** adds the booklet's dice, checksum and translation worksheets; it is not offered at 127 characters.

## Size

**Key:** starter-templates.template-size

The Krux code's side in squares: **25 × 25**, **29 × 29**, **33 × 33** or **37 × 37**. Under each size are the numbers of words and the QR versions that give it.

## Places

**Key:** starter-templates.template-places

How many places the sheet has to write in, 2 at first. **Fewer** and **More** change it between 1 and 13.
