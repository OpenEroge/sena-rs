//! The `mai` translation-patch overlay.
//!
//! A `mai` patch ships a proxy `mai.dll` plus a `mai/` directory next to the
//! game executable. The DLL hooks a handful of PAL entry points; this module
//! reproduces their data side so the engine can apply the same directory:
//!
//! * `word.mai`: one byte per UTF-16 code unit, non-zero for every character
//!   the translated text uses. [`MaiCharset`] rebuilds the code-point remap the
//!   DLL derives from it: the translation stays in Shift-JIS byte form, and
//!   characters that cp932 cannot encode are parked on unused kanji codes.
//! * `sc/text.mai`: a `$TEXT_LIST__` table encrypted like `TEXT.DAT`. Its
//!   records replace the original ones that carry the same record id.
//! * `chg/<name>.mai`: whole-file replacements for files the game opens by
//!   name (CSV tables, movies).
//! * `img/<name>.mai`: PNG pixels for sprites, keyed by the loaded image name.
//!
//! Lookups match file names case-insensitively, like the Windows file system
//! the DLL relies on.

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use encoding_rs::SHIFT_JIS;

use crate::decrypt::decrypt_pal_dollar_file;
use crate::error::{AssetError, Result};
use crate::vfs;

const CODE_SPACE: usize = 0x1_0000;
const UNMAPPED: u16 = 0;

/// Shift-JIS code space with the patch's extra characters assigned.
///
/// `encode` maps a UTF-16 code unit to its patched code and `decode` maps a
/// two-byte code back. Codes the patch leaves alone fall back to plain cp932,
/// so untranslated Japanese text still reads correctly.
pub struct MaiCharset {
    encode: Box<[u16]>,
    decode: Box<[u16]>,
}

impl MaiCharset {
    /// Build the remap from the 64 KiB `word.mai` flag table.
    ///
    /// mai.dll 0x10004220: every two-byte code whose lead is 0x81-0x9F or
    /// 0xE0-0xEF and whose trail is 0x40-0x7E, 0x80-0x9E or 0xA0-0xFC is a
    /// candidate, except the non-kanji rows below 0x889F. A flagged character
    /// that cp932 can encode keeps its canonical code, which stops being a
    /// candidate. The remaining flagged characters then take the free
    /// candidates in ascending order of both code unit and code.
    pub fn from_word_table(flags: &[u8]) -> Result<Self> {
        if flags.len() != CODE_SPACE {
            return Err(AssetError::InvalidMaiPatch {
                reason: format!("word.mai is {} bytes, expected {CODE_SPACE}", flags.len()),
            });
        }
        let canonical = canonical_cp932_codes();
        let mut free = vec![false; CODE_SPACE];
        for lead in (0x81..=0x9F).chain(0xE0..=0xEF) {
            for trail in (0x40..=0x7E).chain(0x80..=0x9E).chain(0xA0..=0xFC) {
                free[(lead << 8) | trail] = true;
            }
        }
        free[..0x889F].fill(false);

        let mut encode = vec![UNMAPPED; CODE_SPACE];
        for (unit, slot) in encode.iter_mut().enumerate().take(0x80) {
            *slot = unit as u16;
        }
        let mut pending = Vec::new();
        for unit in 0x80..CODE_SPACE {
            if flags[unit] == 0 {
                continue;
            }
            match canonical[unit] {
                UNMAPPED => pending.push(unit),
                code => {
                    encode[unit] = code;
                    free[usize::from(code)] = false;
                }
            }
        }
        let mut next = 0;
        for unit in pending {
            let Some(offset) = free[next..].iter().position(|&slot| slot) else {
                return Err(AssetError::InvalidMaiPatch {
                    reason: "word.mai needs more codes than Shift-JIS has free".into(),
                });
            };
            next += offset;
            encode[unit] = next as u16;
            next += 1;
        }

        // mai.dll 0x100041C0 inverts only the two-byte codes.
        let mut decode = vec![UNMAPPED; CODE_SPACE];
        for (unit, &code) in encode.iter().enumerate().skip(0x80) {
            if code & 0xFF00 != 0 {
                decode[usize::from(code)] = unit as u16;
            }
        }
        Ok(Self {
            encode: encode.into_boxed_slice(),
            decode: decode.into_boxed_slice(),
        })
    }

    pub fn decode(&self, bytes: &[u8]) -> String {
        let mut out = String::with_capacity(bytes.len());
        let mut index = 0;
        while index < bytes.len() {
            let lead = bytes[index];
            let width = if is_lead_byte(lead) && index + 1 < bytes.len() { 2 } else { 1 };
            let raw = &bytes[index..index + width];
            let patched = (width == 2)
                .then(|| self.decode[usize::from(u16::from_be_bytes([raw[0], raw[1]]))])
                .filter(|&unit| unit != UNMAPPED)
                .and_then(|unit| char::from_u32(u32::from(unit)));
            match patched {
                Some(ch) => out.push(ch),
                None => out.push_str(&SHIFT_JIS.decode_without_bom_handling(raw).0),
            }
            index += width;
        }
        out
    }

    /// Encode `text`, or `None` when a character has no code that decodes
    /// back to it.
    pub fn encode(&self, text: &str) -> Option<Vec<u8>> {
        let mut out = Vec::with_capacity(text.len());
        let mut buffer = [0u8; 4];
        for ch in text.chars() {
            let unit = u32::from(ch);
            let patched = usize::try_from(unit)
                .ok()
                .and_then(|unit| self.encode.get(unit).copied())
                .filter(|&code| code != UNMAPPED || unit == 0);
            if let Some(code) = patched {
                if code > 0xFF {
                    out.extend_from_slice(&code.to_be_bytes());
                } else {
                    out.push(code as u8);
                }
                continue;
            }
            let (bytes, _, unmappable) = SHIFT_JIS.encode(ch.encode_utf8(&mut buffer));
            if unmappable || self.decode(&bytes).chars().ne(std::iter::once(ch)) {
                return None;
            }
            out.extend_from_slice(&bytes);
        }
        Some(out)
    }
}

impl fmt::Debug for MaiCharset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let assigned = self.decode.iter().filter(|&&unit| unit != UNMAPPED).count();
        f.debug_struct("MaiCharset").field("assigned_codes", &assigned).finish()
    }
}

/// Every [`crate::Nls::Mai`] value refers to the one charset built for the
/// process, so identity is the right equality.
impl PartialEq for MaiCharset {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for MaiCharset {}

fn is_lead_byte(byte: u8) -> bool {
    matches!(byte, 0x81..=0x9F | 0xE0..=0xFC)
}

/// The cp932 code the patch assigns to each code unit: the highest code that
/// decodes to it, the user-defined rows 0xF0-0xF9 excluded (the encoder table
/// at mai.dll 0x1003F120).
fn canonical_cp932_codes() -> Vec<u16> {
    let mut table = vec![UNMAPPED; CODE_SPACE];
    let mut record = |code: u16, bytes: &[u8]| {
        let (text, malformed) = SHIFT_JIS.decode_without_bom_handling(bytes);
        let mut chars = text.chars();
        if let (false, Some(ch), None) = (malformed, chars.next(), chars.next()) {
            if let Ok(unit) = usize::try_from(u32::from(ch)) {
                if (0x80..CODE_SPACE).contains(&unit) {
                    table[unit] = code;
                }
            }
        }
    };
    for byte in 0xA1..=0xDFu8 {
        record(u16::from(byte), &[byte]);
    }
    for lead in (0x81..=0x9Fu8).chain(0xE0..=0xEF).chain(0xFA..=0xFC) {
        for trail in 0x40..=0xFCu8 {
            record(u16::from_be_bytes([lead, trail]), &[lead, trail]);
        }
    }
    table
}

/// `sc/text.mai`: replacement text records keyed by record id.
#[derive(Debug, Default)]
pub struct MaiText {
    records: HashMap<u32, Vec<u8>>,
}

impl MaiText {
    pub fn parse(mut bytes: Vec<u8>) -> Result<Self> {
        if !bytes.starts_with(b"$TEXT_LIST__") || bytes.len() < 0x10 {
            return Err(AssetError::InvalidMaiPatch {
                reason: "sc/text.mai is not a $TEXT_LIST__ table".into(),
            });
        }
        decrypt_pal_dollar_file("text.mai", &mut bytes)?;
        let count = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        let mut records = HashMap::new();
        let mut offset = 0x10;
        for _ in 0..count {
            let Some(header) = bytes.get(offset..offset + 4) else { break };
            let id = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
            let start = offset + 4;
            let Some(len) = bytes[start..].iter().position(|&byte| byte == 0) else { break };
            records.insert(id, bytes[start..start + len].to_vec());
            offset = start + len + 1;
        }
        Ok(Self { records })
    }

    /// Replacement bytes for record `id`, without the terminator.
    ///
    /// mai.dll 0x10003F60 leaves records 0-4 alone; the table starts with the
    /// face names the engine asks GDI for.
    pub fn record(&self, id: u32) -> Option<&[u8]> {
        (id > 4).then(|| self.records.get(&id)).flatten().map(Vec::as_slice)
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

impl FromIterator<(u32, Vec<u8>)> for MaiText {
    fn from_iter<I: IntoIterator<Item = (u32, Vec<u8>)>>(records: I) -> Self {
        Self { records: records.into_iter().collect() }
    }
}

/// A `mai/` patch directory.
#[derive(Debug)]
pub struct MaiPatch {
    charset: &'static MaiCharset,
    files: HashMap<String, PathBuf>,
    images: HashMap<String, PathBuf>,
    text: Option<MaiText>,
}

impl MaiPatch {
    /// Open the patch at `dir` (the game's `mai/` directory).
    ///
    /// The charset is leaked: [`crate::Nls`] is `Copy` and carries it by
    /// reference, and a process applies at most one patch.
    pub fn open(dir: &Path) -> Result<Self> {
        let flags = vfs::read_all(&dir.join("word.mai"))?;
        let charset = Box::leak(Box::new(MaiCharset::from_word_table(&flags)?));
        let text_path = dir.join("sc").join("text.mai");
        let text = if vfs::is_file(&text_path) {
            Some(MaiText::parse(vfs::read_all(&text_path)?)?)
        } else {
            None
        };
        Ok(Self {
            charset,
            files: index_directory(&dir.join("chg")),
            images: index_directory(&dir.join("img")),
            text,
        })
    }

    pub fn charset(&self) -> &'static MaiCharset {
        self.charset
    }

    /// mai.dll 0x10003B40 (PalFileCreateEx): `chg/<name>.mai` replaces the
    /// file. Only the leaf name takes part, as the game opens these by name.
    pub fn replacement_file(&self, name: &str) -> Option<&Path> {
        self.files.get(&overlay_key(name)).map(PathBuf::as_path)
    }

    /// mai.dll 0x100038A0 (PalSpriteLoad): `img/<name>.mai` supplies the
    /// pixels of a loaded sprite. Script names omit the `.PGD` suffix the
    /// loader appends, so the suffixed name is tried as well.
    pub fn replacement_image(&self, name: &str) -> Option<&Path> {
        let leaf = leaf_name(name);
        let key = overlay_key(leaf);
        self.images
            .get(&key)
            .or_else(|| {
                (!leaf.contains('.'))
                    .then(|| self.images.get(&overlay_key(&format!("{leaf}.PGD"))))
                    .flatten()
            })
            .map(PathBuf::as_path)
    }

    pub fn text(&self) -> Option<&MaiText> {
        self.text.as_ref()
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn image_count(&self) -> usize {
        self.images.len()
    }
}

fn leaf_name(name: &str) -> &str {
    name.rsplit(['/', '\\']).next().unwrap_or(name)
}

fn overlay_key(name: &str) -> String {
    format!("{}.MAI", leaf_name(name).to_ascii_uppercase())
}

fn index_directory(dir: &Path) -> HashMap<String, PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return HashMap::new();
    };
    entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| vfs::is_file(path))
        .filter_map(|path| {
            let name = path.file_name()?.to_str()?.to_ascii_uppercase();
            Some((name, path))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn charset(units: &[u16]) -> MaiCharset {
        let mut flags = vec![0u8; CODE_SPACE];
        for &unit in units {
            flags[usize::from(unit)] = 1;
        }
        MaiCharset::from_word_table(&flags).unwrap()
    }

    #[test]
    fn canonical_codes_prefer_the_highest_duplicate() {
        let table = canonical_cp932_codes();
        // U+2160 ROMAN NUMERAL ONE: NEC row 13 (0x8754) and IBM (0xFA4A).
        assert_eq!(table[0x2160], 0xFA4A);
        // U+3042 HIRAGANA A has one code.
        assert_eq!(table[0x3042], 0x82A0);
        // Halfwidth katakana are single bytes.
        assert_eq!(table[0xFF71], 0xB1);
        // The user-defined rows never take part.
        assert_eq!(table[0xE000], UNMAPPED);
    }

    #[test]
    fn encodable_characters_keep_their_codes_and_others_take_free_kanji_codes() {
        // U+4E48 and U+4EEC are simplified Chinese characters cp932 lacks.
        let set = charset(&[0x3042, 0x4E9C, 0x4E48, 0x4EEC]);
        assert_eq!(set.encode("あ").unwrap(), [0x82, 0xA0]);
        // U+4E9C is the first JIS level-1 kanji, code 0x889F, so the first
        // free code is the next one.
        assert_eq!(set.encode("亜").unwrap(), [0x88, 0x9F]);
        assert_eq!(set.encode("么").unwrap(), [0x88, 0xA0]);
        assert_eq!(set.encode("们").unwrap(), [0x88, 0xA1]);
        assert_eq!(set.decode(&[0x88, 0xA0, 0x88, 0xA1, b'!']), "么们!");
    }

    #[test]
    fn trail_byte_0x9f_is_never_assigned() {
        let units: Vec<u16> = (0x4E00..0x4E00 + 0x60).filter(|&u| canonical_cp932_codes()[usize::from(u)] == UNMAPPED).collect();
        let set = charset(&units);
        for &unit in &units {
            let code = set.encode[usize::from(unit)];
            assert_ne!(code & 0xFF, 0x9F, "U+{unit:04X} took 0x{code:04X}");
            assert!(code >= 0x889F);
        }
    }

    #[test]
    fn unpatched_codes_fall_back_to_cp932_both_ways() {
        let set = charset(&[0x4EEC]);
        // The assigned code decodes to the patched character, not the kanji
        // cp932 puts there.
        let code = set.encode("们").unwrap();
        assert_eq!(set.decode(&code), "们");
        // Unflagged Japanese still round-trips through cp932.
        assert_eq!(set.decode(&[0x82, 0xA0]), "あ");
        assert_eq!(set.encode("あ").unwrap(), [0x82, 0xA0]);
        // The kanji whose code the patch took cannot be encoded any more.
        let displaced = SHIFT_JIS.decode_without_bom_handling(&code).0.into_owned();
        assert_eq!(set.encode(&displaced), None);
    }

    #[test]
    fn text_records_skip_the_reserved_ids() {
        let mut plain = b"$TEXT_LIST__".to_vec();
        plain.extend_from_slice(&2u32.to_le_bytes());
        for (id, text) in [(3u32, &b"face"[..]), (9, b"line")] {
            plain.extend_from_slice(&id.to_le_bytes());
            plain.extend_from_slice(text);
            plain.push(0);
        }
        // Encrypt with the inverse of the dollar-file transform.
        let mut encrypted = plain.clone();
        for (rotate, chunk) in (4u32..).zip(encrypted[0x10..].chunks_exact_mut(4)) {
            let value = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) ^ 0xF7D5_859D;
            chunk.copy_from_slice(&value.to_le_bytes());
            chunk[0] = chunk[0].rotate_right(rotate & 7);
        }
        let text = MaiText::parse(encrypted).unwrap();
        assert_eq!(text.len(), 2);
        assert_eq!(text.record(3), None);
        assert_eq!(text.record(9), Some(&b"line"[..]));
    }

    #[test]
    fn patch_lookups_ignore_case_and_add_the_pgd_suffix() {
        let dir = std::env::temp_dir().join(format!("mai-patch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("chg")).unwrap();
        std::fs::create_dir_all(dir.join("img")).unwrap();
        std::fs::write(dir.join("word.mai"), vec![0u8; CODE_SPACE]).unwrap();
        std::fs::write(dir.join("chg").join("TIPS.CSV.mai"), b"x").unwrap();
        std::fs::write(dir.join("img").join("BTN_A.PGD.mai"), b"x").unwrap();
        let patch = MaiPatch::open(&dir).unwrap();
        assert!(patch.replacement_file("data\\tips.csv").is_some());
        assert!(patch.replacement_file("TIPS").is_none());
        assert!(patch.replacement_image("btn_a").is_some());
        assert!(patch.replacement_image("BTN_A.pgd").is_some());
        assert!(patch.replacement_image("BTN_B").is_none());
        assert!(patch.text().is_none());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
