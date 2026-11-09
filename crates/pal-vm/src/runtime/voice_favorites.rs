//! trenka.exe favourite voices (category 18 indices 60..68) and the global
//! word table they share (72, 73, 79).
//!
//! Native keeps up to 0x400 records of 0x224 bytes at 0x7A4FE0: id, name,
//! tag-stripped text, voice and a flag byte. 18:79 stores the configured
//! limit (`FAV_MAX`) that adds must stay below; 18:60 clears the records and
//! the 4096-word table at 0x82EFE4 and remembers the file path. The file
//! holds `%06d,%1d,"name","voice","text"` lines in the game's NLS.
use super::*;
use tips_dictionary::save_folder_path;

const CAPACITY: usize = 0x400;
const GLOBAL_WORDS: usize = 0x1000;
/// Bytes each string field keeps (0xFF plus the terminator).
const FIELD_BYTES: usize = 0xFF;

#[derive(Clone, Debug, Default, PartialEq)]
struct FavoriteVoice {
    id: i32,
    name: String,
    voice: String,
    text: String,
    flag: i8,
}

#[derive(Clone, Debug, Default)]
pub(super) struct VoiceFavorites {
    path: String,
    records: Vec<FavoriteVoice>,
    limit: i32,
    globals: Vec<i32>,
}

fn strip_tags(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch == '<' {
            chars.by_ref().find(|&skipped| skipped == '>');
        } else {
            out.push(ch);
        }
    }
    out
}

/// Keep the longest prefix whose encoded form fits the native field.
fn clip(text: &str, nls: Nls) -> String {
    let mut used = 0;
    let mut out = String::new();
    for ch in text.chars() {
        let width = nls
            .encode(ch.encode_utf8(&mut [0; 4]))
            .map_or(2, |bytes| bytes.len());
        if used + width > FIELD_BYTES {
            break;
        }
        used += width;
        out.push(ch);
    }
    out
}

fn parse_line(line: &str) -> Option<FavoriteVoice> {
    let mut parts = line.splitn(3, ',');
    let id = parts.next()?.trim().parse().ok()?;
    let flag = parts.next()?.trim().parse::<i32>().ok()? as i8;
    let mut quoted = parts.next()?.split('"').skip(1).step_by(2);
    Some(FavoriteVoice {
        id,
        flag,
        name: quoted.next()?.to_owned(),
        voice: quoted.next()?.to_owned(),
        text: quoted.next()?.to_owned(),
    })
}

impl VoiceFavorites {
    fn contains(&self, slot: i32) -> bool {
        (slot as u32 as usize) < self.records.len()
    }

    /// 0x4146B0: append a record; a negative id takes the next free one.
    fn add(&mut self, requested: i32, name: &str, voice: &str, text: &str, nls: Nls) -> i32 {
        if self.records.len() >= CAPACITY || self.records.len() as i32 >= self.limit {
            return -1;
        }
        let id = if requested >= 0 {
            requested
        } else if self.records.is_empty() {
            0
        } else {
            self.records.iter().fold(requested, |best, record| {
                if best > record.id {
                    best
                } else {
                    record.id.wrapping_add(1)
                }
            })
        };
        self.records.push(FavoriteVoice {
            id,
            name: clip(name, nls),
            voice: voice.to_owned(),
            text: clip(&strip_tags(text), nls),
            flag: 0,
        });
        id
    }

    fn serialize(&self) -> String {
        self.records
            .iter()
            .filter(|record| record.id >= 0)
            .map(|record| {
                format!(
                    "{:06},{},\"{}\",\"{}\",\"{}\"\r\n",
                    record.id, record.flag, record.name, record.voice, record.text
                )
            })
            .collect()
    }
}

impl ScriptRuntime {
    pub(super) fn dispatch_voice_favorites(
        &mut self,
        index: u16,
        assets: &CoreAssets,
        nls: Nls,
        resource_manager: Option<&mut ResourceManager>,
    ) -> ExtCallOutcome {
        let root = resource_manager
            .as_deref()
            .map(|manager| manager.root().to_path_buf());
        match index {
            60 => {
                // 0x414CA0: -1 clears without changing the path.
                let args = self.pop_ext_args(1);
                let Some(&path) = args.first() else {
                    return ExtCallOutcome::Block;
                };
                let path = (path != -1).then(|| {
                    self.resolve_script_string(path, assets, nls)
                        .unwrap_or_default()
                });
                self.voice_favorites.records.clear();
                self.voice_favorites.globals.clear();
                if let Some(path) = path {
                    self.voice_favorites.path = path;
                }
                ExtCallOutcome::Continue
            }
            61 => {
                let args = self.pop_ext_args(4);
                let [id, name, voice, text] = args[..] else {
                    return ExtCallOutcome::Block;
                };
                let (name, voice, text) = if name == 0 {
                    // Without strings native copies the current line (ctx
                    // +0x10FC name, +0x2200 voice, +0x1200 text).
                    let current = self
                        .history_state
                        .records
                        .len()
                        .checked_sub(1)
                        .map(|last| self.history_record_text(last, assets, nls))
                        .unwrap_or_default();
                    (current.name, current.voice, current.body)
                } else {
                    let resolve = |value| {
                        self.resolve_script_string(value, assets, nls)
                            .unwrap_or_default()
                    };
                    (resolve(name), resolve(voice), resolve(text))
                };
                let added = self.voice_favorites.add(id, &name, &voice, &text, nls);
                log::debug!("[trace-favorites] favadd voice={voice:?} -> {added}");
                ExtCallOutcome::Value(added)
            }
            62 => {
                let args = self.pop_ext_args(1);
                let Some(&slot) = args.first() else {
                    return ExtCallOutcome::Block;
                };
                if !self.voice_favorites.contains(slot) {
                    return ExtCallOutcome::Value(0);
                }
                self.voice_favorites.records[slot as usize] = FavoriteVoice {
                    id: -1,
                    ..Default::default()
                };
                ExtCallOutcome::Value(1)
            }
            63 => {
                let args = self.pop_ext_args(2);
                let [first, second] = args[..] else {
                    return ExtCallOutcome::Block;
                };
                if !self.voice_favorites.contains(first) || !self.voice_favorites.contains(second) {
                    return ExtCallOutcome::Value(0);
                }
                self.voice_favorites
                    .records
                    .swap(first as usize, second as usize);
                ExtCallOutcome::Value(1)
            }
            64 => {
                // 0x415390 returns the loaded count, 0 on error, -1 without a path.
                if self.voice_favorites.path.is_empty() {
                    return ExtCallOutcome::Value(-1);
                }
                let Some(file) =
                    root.and_then(|root| save_folder_path(&root, &self.voice_favorites.path))
                else {
                    return ExtCallOutcome::Value(0);
                };
                let Ok(bytes) = std::fs::read(&file) else {
                    return ExtCallOutcome::Value(0);
                };
                let Ok(text) = nls.decode(&bytes) else {
                    return ExtCallOutcome::Value(0);
                };
                self.voice_favorites.records =
                    text.lines().filter_map(parse_line).take(CAPACITY).collect();
                ExtCallOutcome::Value(self.voice_favorites.records.len() as i32)
            }
            65 => {
                let file =
                    root.and_then(|root| save_folder_path(&root, &self.voice_favorites.path));
                let Some(file) = file.filter(|_| !self.voice_favorites.path.is_empty()) else {
                    return ExtCallOutcome::Value(0);
                };
                let written = nls
                    .encode(&self.voice_favorites.serialize())
                    .map_err(|error| std::io::Error::other(error.to_string()))
                    .and_then(|bytes| {
                        file.parent().map_or(Ok(()), std::fs::create_dir_all)?;
                        std::fs::write(&file, bytes)
                    });
                log::debug!(
                    "[trace-favorites] favsave {} records={} ok={}",
                    file.display(),
                    self.voice_favorites.records.len(),
                    written.is_ok()
                );
                if let Err(error) = &written {
                    log::warn!(
                        "[trace-favorites] favsave {} failed: {error}",
                        file.display()
                    );
                }
                ExtCallOutcome::Value(i32::from(written.is_ok()))
            }
            66 => self.ext_favorite_get(),
            67 => {
                let args = self.pop_ext_args(1);
                let Some(&slot) = args.first() else {
                    return ExtCallOutcome::Block;
                };
                ExtCallOutcome::Value(if self.voice_favorites.contains(slot) {
                    i32::from(self.voice_favorites.records[slot as usize].flag)
                } else {
                    0
                })
            }
            68 => {
                let args = self.pop_ext_args(2);
                let [slot, value] = args[..] else {
                    return ExtCallOutcome::Block;
                };
                if !self.voice_favorites.contains(slot) {
                    return ExtCallOutcome::Value(0);
                }
                self.voice_favorites.records[slot as usize].flag = value as i8;
                ExtCallOutcome::Value(1)
            }
            72 => {
                // 0x41BBC0: pop index then value; no destination write.
                let args = self.pop_ext_args(2);
                let [index, value] = args[..] else {
                    return ExtCallOutcome::Block;
                };
                if (index as u32 as usize) < GLOBAL_WORDS {
                    self.voice_favorites.globals.resize(GLOBAL_WORDS, 0);
                    self.voice_favorites.globals[index as usize] = value;
                }
                ExtCallOutcome::Continue
            }
            73 => {
                // 0x41BC70: out-of-range indexes leave the destination alone.
                let args = self.pop_ext_args(1);
                let Some(&index) = args.first() else {
                    return ExtCallOutcome::Block;
                };
                if (index as u32 as usize) < GLOBAL_WORDS {
                    ExtCallOutcome::Value(
                        self.voice_favorites
                            .globals
                            .get(index as usize)
                            .copied()
                            .unwrap_or(0),
                    )
                } else {
                    ExtCallOutcome::Continue
                }
            }
            79 => {
                // 0x41BD00 stores the favourite limit (0x832FE4).
                let args = self.pop_ext_args(1);
                let Some(&limit) = args.first() else {
                    return ExtCallOutcome::Block;
                };
                self.voice_favorites.limit = limit;
                ExtCallOutcome::Continue
            }
            _ => ExtCallOutcome::Skip,
        }
    }

    /// 0x4154B0: pop index, field and destination. Fields: 0 id, 1 flag,
    /// and for string destinations 2 name, 3 voice, 4 text.
    fn ext_favorite_get(&mut self) -> ExtCallOutcome {
        let args = self.pop_ext_args(3);
        let [slot, field, dst] = args[..] else {
            return ExtCallOutcome::Block;
        };
        let Some(record) = usize::try_from(slot)
            .ok()
            .and_then(|slot| self.voice_favorites.records.get(slot))
            .cloned()
        else {
            return ExtCallOutcome::Value(-1);
        };
        if !is_dynamic_string_handle(dst) {
            return match field {
                0 => ExtCallOutcome::Value(record.id),
                1 => ExtCallOutcome::Value(i32::from(record.flag)),
                _ => ExtCallOutcome::Continue,
            };
        }
        let text = match field {
            0 => return ExtCallOutcome::Value(record.id),
            1 => return ExtCallOutcome::Value(i32::from(record.flag)),
            2 => record.name,
            3 => record.voice,
            4 => record.text,
            _ => return ExtCallOutcome::Value(-1),
        };
        self.replace_dynamic_string(dst, text);
        ExtCallOutcome::Value(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_round_trip_through_the_native_line_format() {
        let mut favorites = VoiceFavorites {
            limit: 500,
            ..Default::default()
        };
        assert_eq!(
            favorites.add(-1, "A", "vo01", "<r>hi</r>", Nls::ShiftJis),
            0
        );
        assert_eq!(favorites.add(-1, "B", "vo02", "yo", Nls::ShiftJis), 1);
        favorites.records[1].flag = 1;
        let text = favorites.serialize();
        assert_eq!(
            text,
            "000000,0,\"A\",\"vo01\",\"hi\"\r\n000001,1,\"B\",\"vo02\",\"yo\"\r\n"
        );
        let parsed: Vec<_> = text.lines().filter_map(parse_line).collect();
        assert_eq!(parsed, favorites.records);
    }

    #[test]
    fn adds_respect_the_configured_limit() {
        let mut favorites = VoiceFavorites::default();
        assert_eq!(favorites.add(-1, "A", "v", "t", Nls::ShiftJis), -1);
        favorites.limit = 1;
        assert_eq!(favorites.add(7, "A", "v", "t", Nls::ShiftJis), 7);
        assert_eq!(favorites.add(-1, "B", "v", "t", Nls::ShiftJis), -1);
    }
}
