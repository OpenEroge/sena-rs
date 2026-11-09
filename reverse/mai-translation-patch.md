# `mai` translation patch

Scope: the proxy `mai.dll` (458240 bytes, SHA-256
`7fa6cd925165552e934ae54e3d6fb792f2396a5ce7428a679e8c83600a77f5d6`) shipped
with a fan translation of the Iroha (`trenka.exe`) build, together with the
`mai/` directory next to the executable. Addresses are only valid for that
DLL (image base 0x10000000). Nothing here applies to an install without the
patch, and Sena only applies it when asked to (`--patch mai`).

## Native evidence

- The patched executable differs from `trenka.exe` only in its import
  table: `COMDLG32.dll!GetSaveFileNameA` is renamed to
  `mai.dll!MetSaveFileNameA`. `mai.dll` forwards that export and
  `MmmAssociateContext` (IMM32) and does all of its work from `DllMain`.
- `DllMain` (0x10001080) detours `LoadLibraryExA` to 0x10004020 and the GDI
  font entry points (0x10004970). When `PAL.DLL` is loaded, 0x10004020
  detours:
  - `PalFileCreateEx` → 0x10003B40: if `<exe dir>\mai\chg\<name>.mai`
    exists, that file is opened instead. `name` is the string the engine
    passed in.
  - `PalSpriteLoad` → 0x100038A0: after the original load succeeds, it
    looks for `<exe dir>\mai\img\<name>.mai` and decodes that PNG through
    `libfusionex.dll`. It then `memcpy`s width × height × 4 bytes over the
    locked texture. The sprite keeps its placement, and the copy assumes
    equal dimensions.
  - `PalFontSetType` → 0x10003880: type 4 (the bitmap font) becomes 1.
  - `exe+0x4B470` (the text-record reader) → 0x10003F60: it runs the
    original reader, then replaces the copied record with the record of
    `mai\sc\text.mai` that has the same id. Ids 0–4 are left alone; the
    table starts with the GDI face names.
  - It also renames a loose `movie` directory to `movies` on disk. That is
    an install-time side effect and has no Sena counterpart.
- `CreateFontA` → 0x10004580 forces the face to `SimHei` (`@SimHei` for a
  vertical face) and the charset to `GB2312_CHARSET`. `GetGlyphOutlineA`
  converts each code to UTF-16 through the table built below and draws it
  with `GetGlyphOutlineW`.

### Files

- `text.mai`: `$TEXT_LIST__`, encrypted like `TEXT.DAT` from offset 0x10
  (rotate-left 4,5,6,7,0,… then XOR `0xF7D5859D`; 0x10003EB6). Records are
  `u32 id` + NUL-terminated text. 0x10003F24 indexes them by id.
- `chg/*.mai`, `img/*.mai`: plain files (CSV, MPEG-PS, PNG) named
  `<original name>.mai`.
- `word.mai`: 65536 bytes, one flag per UTF-16 code unit. A non-zero flag
  marks a character that the translated text uses.

### Code-point remap (0x10004220, 0x100041C0)

The translated text stays in Shift-JIS byte form. Characters that cp932
cannot encode are parked on kanji codes that the translation does not use:

1. Candidate codes: lead 0x81–0x9F or 0xE0–0xEF, and trail 0x40–0x7E,
   0x80–0x9E or 0xA0–0xFC (trail 0x9F is never used). Every code below
   0x889F is removed.
2. A flagged character that cp932 can encode keeps its canonical code,
   which is removed from the candidates. The canonical code is the table at
   0x1003F120: for each character, the highest code that decodes to it,
   excluding the user-defined rows 0xF0–0xF9.
3. The remaining flagged characters, in ascending code-unit order, take the
   free candidates in ascending order.
4. The glyph table inverts only the two-byte codes. Everything else
   decodes to `?`.

## Implementation

- `pal_asset::MaiCharset` builds the remap from `word.mai`. The canonical
  table is regenerated from encoding_rs Shift_JIS decoding, not copied from
  the DLL. `Nls::Mai` decodes and encodes through it. Codes the patch does
  not assign fall back to plain cp932, so untranslated Japanese (old save
  data, records 0–4) stays readable instead of turning into `?`. Encoding
  rejects a character whose cp932 code the patch reassigned.
- `pal_asset::MaiPatch` indexes `chg/` and `img/` case-insensitively and
  parses `sc/text.mai`. `ResourceManager::set_mai_patch` lets `chg` files
  shadow every search path. `ResourceManager::replacement_image` serves
  `img` PNGs. A script name without a suffix also matches `<name>.PGD.mai`,
  the suffix the loader appends.
- `CoreAssets::apply_text_patch` maps the patch records onto the original
  `Text.dat` record offsets by id. Only real record starts are mapped, and
  script and resource strings resolve through it.
- `decode_asset_image` copies the patch pixels over any decoded image of
  the same size. A PNG of a different size, which would overrun the native
  texture, is logged and the original is kept.
- `PalFontSystem::set_outline_text_only` reproduces the type 4 → 1 rewrite.
  `ScriptRuntime::apply_translation_font_policy` keeps a `--font-file` or
  `fonts/` face, and otherwise loads the first host stand-in for SimHei
  (`SIMHEI_SUBSTITUTE_CANDIDATES`).
- `--patch mai` (`TranslationPatch::Mai`) reads `<game root>/mai` and
  replaces `--nls` with the patch charset before any asset is decoded.

## Runtime validation

Against the shipped patch (3 `chg` files, 661 images, 25793 text records,
2996 flagged characters):

- The Rust remap matches a simulation of 0x10004220 run on the DLL's own
  table for all 2996 characters.
- 25788 `Text.dat` records are replaced; records 0–4 are kept.
- 655 of the 661 images match their original's dimensions. Two TIPS
  thumbnails are larger than their originals and are rejected. Four have no
  original, one of them because its name lacks the dot before `PGD`.
- Headless: boot shows the patched CAUTION images and the `chg` LOGO movie.
  ADV text, name boxes, the SYSTEM screen (patched images plus drawn
  dropdown text), the favourite-voice screen and an in-game TIPS window
  built from `TIPS.CSV.mai` render the translation.
