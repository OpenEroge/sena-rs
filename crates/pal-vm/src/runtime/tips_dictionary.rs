//! trenka.exe dictionary ("TIPS") table, save-folder string helpers and the
//! global word table (category 18 indices 50, 59, 72, 73, 79, 82..87).
//!
//! Native 0x41C880 parses a four-column CSV (name, reading, small image,
//! large image) into 0x6C-byte records that also keep a kana row index and
//! two flag bytes; 0x41D650 sets the flags and 0x41D790 writes them to the
//! script-chosen file as ASCII digit pairs. The script builds that path with
//! 18:50 (save folder) and 18:59 (string append). These coordinates are only
//! dispatched for builds whose wrappers were audited (`NativeBuild`).
use super::*;

/// Kana rows searched for a reading's first character (0x6296A0..0x6298A8).
/// The byte offset of the match divided by ten (five two-byte kana) is the
/// row: 0 for the a-row through 9 for wa/n.
const KANA_ROWS: [&str; 6] = [
    "あいうえおかきくけこさしすせそたちつてとなにぬねのはひふへほまみむめもや　ゆ　よらりるれろわゐゑを",
    "　　　　　がぎぐげござじずぜぞだぢづでど　　　　　ばびぶべぼ　　　　　　　　　　　　　　　　　　ん",
    "ぁぃぅぇぉ　　　　　　　　　　　　　　　　　　　　ぱぴぷぺぽ　　　　　ゃ　ゅ　ょ　　　　　ゎ　　ん",
    "アイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤ　ユ　ヨラリルレロワヰヱヲ",
    "　　ヴ　　ガギグゲゴザジズゼゾダヂヅデド　　　　　バビブベボ　　　　　　　　　　　　　　　　　　ン",
    "ァィゥェォ　　　　　　　　　　　　　　　　　　　　パピプペポ　　　　　ャ　ュ　ョ　　　　　ヮ　　ン",
];

/// Folder 18:50 reports; file helpers map it back onto the save directory.
const SAVE_FOLDER: &str = "save";

#[derive(Clone, Debug, Default, PartialEq)]
struct TipsEntry {
    fields: [String; 4],
    row: i32,
    flags: [u8; 2],
}

/// Entries of the unsaved global table at 0x82EFE4 (18:72/18:73).
const GLOBAL_WORDS: usize = 0x1000;

#[derive(Clone, Debug, Default)]
pub(super) struct TipsDictionary {
    entries: Vec<TipsEntry>,
    flag_path: String,
    globals: Vec<i32>,
}

/// Map a script path below the reported save folder to the real save
/// directory. Absolute paths, parent components and other roots are refused.
pub(super) fn save_folder_path(root: &Path, native: &str) -> Option<PathBuf> {
    let mut parts = native.split(['\\', '/']).filter(|part| !part.is_empty() && *part != ".");
    if !parts.next()?.eq_ignore_ascii_case(SAVE_FOLDER) {
        return None;
    }
    let mut path = pal_asset::vfs::save_directory(root);
    let mut any = false;
    for part in parts {
        if part == ".." || part.contains(':') {
            return None;
        }
        path.push(part);
        any = true;
    }
    any.then_some(path)
}

/// Split the native tokenizer's quoted, comma-separated fields.
fn csv_fields(text: &str) -> Vec<String> {
    let mut fields = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut field = String::new();
        let mut quoted = false;
        for ch in line.chars() {
            match ch {
                '"' => quoted = !quoted,
                ',' if !quoted => fields.push(std::mem::take(&mut field)),
                _ => field.push(ch),
            }
        }
        fields.push(field);
    }
    fields
}

fn kana_row(reading: &str, nls: Nls) -> i32 {
    let Some(first) = reading.chars().next() else { return -1; };
    let Ok(needle) = nls.encode(first.encode_utf8(&mut [0; 4])) else { return -1; };
    if needle.is_empty() {
        return -1;
    }
    KANA_ROWS
        .iter()
        .filter_map(|row| nls.encode(row).ok())
        .find_map(|table| table.windows(needle.len()).position(|window| window == needle.as_slice()))
        .map_or(-1, |offset| (offset / 10) as i32)
}

impl TipsDictionary {
    fn parse(text: &str, nls: Nls) -> Option<Vec<TipsEntry>> {
        let fields = csv_fields(text);
        let entries: Vec<TipsEntry> = fields
            .chunks_exact(4)
            .map(|chunk| TipsEntry {
                fields: [chunk[0].clone(), chunk[1].clone(), chunk[2].clone(), chunk[3].clone()],
                row: kana_row(&chunk[1], nls),
                flags: [0; 2],
            })
            .collect();
        // Native rejects the whole table when a reading is empty.
        entries.iter().all(|entry| !entry.fields[1].is_empty()).then_some(entries)
    }

    fn apply_flags(&mut self, bytes: &[u8]) {
        for (entry, pair) in self.entries.iter_mut().zip(bytes.chunks(2)) {
            entry.flags[0] = pair[0].wrapping_sub(b'0');
            if let Some(&second) = pair.get(1) {
                entry.flags[1] = second.wrapping_sub(b'0');
            }
        }
    }

    fn encode_flags(&self) -> Vec<u8> {
        self.entries
            .iter()
            .flat_map(|entry| entry.flags.map(|flag| flag.wrapping_add(b'0')))
            .collect()
    }
}

impl ScriptRuntime {
    /// Category 18 coordinates that only some audited builds define.
    pub(super) fn dispatch_build_system_ext(
        &mut self,
        index: u16,
        assets: &CoreAssets,
        nls: Nls,
        resource_manager: Option<&mut ResourceManager>,
    ) -> ExtCallOutcome {
        if !assets.native_build.has_tips_dictionary() {
            return ExtCallOutcome::Skip;
        }
        match index {
            50 => self.ext_get_save_folder(),
            72 => {
                // 0x41BBC0: pop index then value; no destination write.
                let args = self.pop_ext_args(2);
                let [index, value] = args[..] else { return ExtCallOutcome::Block; };
                if (index as u32 as usize) < GLOBAL_WORDS {
                    let globals = &mut self.tips_dictionary.globals;
                    globals.resize(GLOBAL_WORDS, 0);
                    globals[index as usize] = value;
                }
                ExtCallOutcome::Continue
            }
            73 => {
                // 0x41BC70: out-of-range indexes leave the destination alone.
                let args = self.pop_ext_args(1);
                let Some(&index) = args.first() else { return ExtCallOutcome::Block; };
                if (index as u32 as usize) < GLOBAL_WORDS {
                    ExtCallOutcome::Value(self.tips_dictionary.globals.get(index as usize).copied().unwrap_or(0))
                } else {
                    ExtCallOutcome::Continue
                }
            }
            79 => {
                // 0x41BD00 stores a word (0x832FE4) no audited wrapper reads.
                if self.pop_ext_args(1).is_empty() {
                    return ExtCallOutcome::Block;
                }
                ExtCallOutcome::Continue
            }
            59 => {
                // 0x41BAA0 appends without writing the destination slot.
                let _ = self.ext_string_append(assets, nls);
                ExtCallOutcome::Continue
            }
            82 => self.ext_tips_read(assets, nls, resource_manager),
            83 => self.ext_tips_get(),
            84 => self.ext_tips_search(assets, nls),
            86 => self.ext_tips_set_flag(),
            87 => self.ext_tips_save_flags(resource_manager.as_deref()),
            _ => ExtCallOutcome::Skip,
        }
    }

    /// 0x419F40: copy the save folder into a string slot and return 1.
    fn ext_get_save_folder(&mut self) -> ExtCallOutcome {
        let args = self.pop_ext_args(1);
        let Some(&dst) = args.first() else { return ExtCallOutcome::Block; };
        if is_dynamic_string_handle(dst) {
            self.replace_dynamic_string(dst, SAVE_FOLDER.to_owned());
            ExtCallOutcome::Value(1)
        } else {
            ExtCallOutcome::Continue
        }
    }

    /// 0x41C880: pop the CSV resource and the flag file path; return the
    /// entry count, or 0 when the CSV is missing or malformed.
    fn ext_tips_read(
        &mut self,
        assets: &CoreAssets,
        nls: Nls,
        resource_manager: Option<&mut ResourceManager>,
    ) -> ExtCallOutcome {
        let args = self.pop_ext_args(2);
        let [csv, path] = args[..] else { return ExtCallOutcome::Block; };
        self.tips_dictionary.flag_path = self.resolve_script_string(path, assets, nls).unwrap_or_default();
        self.tips_dictionary.entries.clear();
        let Some(manager) = resource_manager else { return ExtCallOutcome::Value(0); };
        let Some(name) = self.resolve_resource_string(csv, assets, nls) else { return ExtCallOutcome::Value(0); };
        let text = match manager.open(&name).map_err(anyhow::Error::from)
            .and_then(|asset| Ok(manager.nls().decode(&asset.bytes)?))
        {
            Ok(text) => text,
            Err(error) => {
                log::warn!("[trace-tips] tips_csv_read {name:?} failed: {error}");
                return ExtCallOutcome::Value(0);
            }
        };
        let Some(entries) = TipsDictionary::parse(&text, nls) else {
            log::warn!("[trace-tips] tips_csv_read {name:?} has an entry without a reading");
            return ExtCallOutcome::Value(0);
        };
        self.tips_dictionary.entries = entries;
        let flag_file = save_folder_path(manager.root(), &self.tips_dictionary.flag_path);
        if let Some(bytes) = flag_file.and_then(|path| std::fs::read(path).ok()) {
            self.tips_dictionary.apply_flags(&bytes);
        }
        log::debug!("[trace-tips] tips_csv_read {name:?} entries={}", self.tips_dictionary.entries.len());
        ExtCallOutcome::Value(self.tips_dictionary.entries.len() as i32)
    }

    /// 0x41C4B0: pop index, field and destination. A string destination
    /// receives fields 0..3 appended and the slot gets the entry index;
    /// fields 4, 5 and 6 return the kana row and the two flags.
    fn ext_tips_get(&mut self) -> ExtCallOutcome {
        let args = self.pop_ext_args(3);
        let [index, field, dst] = args[..] else { return ExtCallOutcome::Block; };
        let Some(entry) = usize::try_from(index).ok().and_then(|index| self.tips_dictionary.entries.get(index)) else {
            log::debug!("[trace-tips] tips_csv_get error index={index} field={field}");
            return ExtCallOutcome::Value(-1);
        };
        match field {
            4 => ExtCallOutcome::Value(entry.row),
            5 => ExtCallOutcome::Value(i32::from(entry.flags[0] as i8)),
            6 => ExtCallOutcome::Value(i32::from(entry.flags[1] as i8)),
            0..=3 if is_dynamic_string_handle(dst) => {
                let suffix = entry.fields[field as usize].clone();
                let mut value = dynamic_string_index(dst)
                    .and_then(|slot| self.dynamic_strings.get(slot).cloned())
                    .unwrap_or_default();
                value.push_str(&suffix);
                self.replace_dynamic_string(dst, value);
                ExtCallOutcome::Value(index)
            }
            _ => ExtCallOutcome::Value(index),
        }
    }

    /// 0x41C320: exact name match; -1 when absent.
    fn ext_tips_search(&mut self, assets: &CoreAssets, nls: Nls) -> ExtCallOutcome {
        let args = self.pop_ext_args(1);
        let Some(&value) = args.first() else { return ExtCallOutcome::Block; };
        let name = self.resolve_script_string(value, assets, nls).unwrap_or_default();
        let found = self.tips_dictionary.entries.iter().position(|entry| entry.fields[0] == name);
        if found.is_none() {
            log::debug!("[trace-tips] tips_csv_search not found {name:?}");
        }
        ExtCallOutcome::Value(found.map_or(-1, |index| index as i32))
    }

    /// 0x41D650: pop index, selector and value. Selectors 0 and 5 write the
    /// first flag, 1 and 6 the second; the slot always receives 0.
    fn ext_tips_set_flag(&mut self) -> ExtCallOutcome {
        let args = self.pop_ext_args(3);
        let [index, selector, value] = args[..] else { return ExtCallOutcome::Block; };
        if let Some(entry) = usize::try_from(index).ok().and_then(|index| self.tips_dictionary.entries.get_mut(index)) {
            match selector {
                0 | 5 => entry.flags[0] = value as u8,
                1 | 6 => entry.flags[1] = value as u8,
                _ => {}
            }
        }
        ExtCallOutcome::Value(0)
    }

    /// 0x41D790: write the flags to the file chosen by 18:82.
    fn ext_tips_save_flags(&mut self, resource_manager: Option<&ResourceManager>) -> ExtCallOutcome {
        let Some(path) = resource_manager
            .and_then(|manager| save_folder_path(manager.root(), &self.tips_dictionary.flag_path))
        else {
            log::debug!("[trace-tips] tips_acc_save has no writable path {:?}", self.tips_dictionary.flag_path);
            return ExtCallOutcome::Continue;
        };
        let bytes = self.tips_dictionary.encode_flags();
        let result = path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(&path, bytes));
        if let Err(error) = result {
            log::warn!("[trace-tips] tips_acc_save {} failed: {error}", path.display());
        }
        ExtCallOutcome::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_folder_paths_stay_inside_the_save_directory() {
        let root = Path::new("/game");
        let save = pal_asset::vfs::save_directory(root);
        assert_eq!(save_folder_path(root, "save\\\\tips.dat"), Some(save.join("tips.dat")));
        assert_eq!(save_folder_path(root, "SAVE/a/b.dat"), Some(save.join("a").join("b.dat")));
        assert_eq!(save_folder_path(root, "save"), None);
        assert_eq!(save_folder_path(root, "save\\..\\x"), None);
        assert_eq!(save_folder_path(root, "C:\\save\\x"), None);
        assert_eq!(save_folder_path(root, "data\\x"), None);
    }

    #[test]
    fn csv_entries_keep_fields_rows_and_flag_pairs() {
        let text = "\"A\",\"か\",\"s1\",\"b1\"\r\n\"B\",\"ン\",\"s2\",\"b2\"\r\n";
        let mut dictionary = TipsDictionary {
            entries: TipsDictionary::parse(text, Nls::ShiftJis).unwrap(),
            ..Default::default()
        };
        assert_eq!(dictionary.entries[0].fields, ["A", "か", "s1", "b1"].map(String::from));
        assert_eq!(dictionary.entries[0].row, 1);
        assert_eq!(dictionary.entries[1].row, 9);
        dictionary.apply_flags(b"1001");
        assert_eq!(dictionary.entries[0].flags, [1, 0]);
        assert_eq!(dictionary.entries[1].flags, [0, 1]);
        assert_eq!(dictionary.encode_flags(), b"1001");
        assert!(TipsDictionary::parse("\"A\",\"\",\"s\",\"b\"", Nls::ShiftJis).is_none());
    }
}
