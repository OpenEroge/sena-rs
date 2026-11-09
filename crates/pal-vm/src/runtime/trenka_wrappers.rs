//! Small trenka.exe wrappers with audited operand contracts that have no
//! wider subsystem in Sena (text queries, button links, BGM status, host-only
//! features and save-folder file helpers). Only coordinates listed in
//! `TRENKA_OWN_EXTCALLS` reach this module; each arm pops exactly what the
//! native handler pops so the script stack stays aligned even where the host
//! feature (message boxes, screenshots, tweets) has no portable equivalent.
use super::tips_dictionary::save_folder_path;
use super::*;

impl ScriptRuntime {
    /// Run the trenka wrapper at `category:index`, or `None` when the
    /// coordinate is handled by a shared subsystem instead.
    pub(super) fn dispatch_trenka_wrapper(
        &mut self,
        category: u16,
        index: u16,
        assets: &CoreAssets,
        nls: Nls,
        resource_manager: Option<&ResourceManager>,
        audio: Option<&mut AudioSystem>,
    ) -> Option<ExtCallOutcome> {
        let root = resource_manager.map(|manager| manager.root().to_path_buf());
        Some(match (category, index) {
            (2, 35) => self.ext_get_current_text(assets, nls),
            // 0x425D70 sp_load_wait_time and 0x425C90 sp_draw pop a slot or
            // duration; portable sprites load synchronously and draw every
            // frame.
            (3, 42 | 43) => self.pop_operands(1, ExtCallOutcome::Continue),
            // 0x40BB20 bgm_set_volume_users pops three words for the user
            // volume fade; the user volume stays with the system settings.
            (4, 5) => self.pop_operands(3, ExtCallOutcome::Continue),
            (4, 16) => self.ext_bgm_status(audio),
            // 0x40C610 get_bgm_pos pops channel and PalSoundGetParam selector.
            (4, 17) => {
                let args = self.pop_ext_args(2);
                let Some(&channel) = args.first() else {
                    return Some(ExtCallOutcome::Block);
                };
                ExtCallOutcome::Value(if self.bgm_channel(channel).is_some() {
                    0
                } else {
                    -1
                })
            }
            (8, 28) => self.ext_btn_link(),
            (8, 30) => self.ext_btn_get_link(),
            // 0x438970 set_cursor pops name and hotspot; the portable cursor
            // stays the default arrow.
            (9, 48) => {
                self.system_state.set_cursor_null(true);
                self.pop_operands(3, ExtCallOutcome::Value(1))
            }
            // 0x411F10 shows a message box; 0x411B00 only logs its operand.
            (15, 1 | 9) => self.pop_operands(1, ExtCallOutcome::Continue),
            // 0x4182C0 returns a stamp decoded from the executable image;
            // only an unreferenced point reads it.
            (18, 17) => ExtCallOutcome::Value(0),
            // Screenshot export (JPEG 18:43, PNG 18:45), StretchImage (18:46)
            // and the tweet key (18:76) are host features without a portable
            // target; their status is not written to the destination.
            (18, 43 | 45 | 76) => self.pop_operands(2, ExtCallOutcome::Continue),
            (18, 46) => self.pop_operands(3, ExtCallOutcome::Continue),
            // 0x418C70 opens an edit dialog; report it as cancelled.
            (18, 44) => self.pop_operands(4, ExtCallOutcome::Value(0)),
            // 0x419A10 loads an image file into a sprite; unsupported.
            (18, 48) => self.pop_operands(2, ExtCallOutcome::Value(0)),
            (18, 47) => self.ext_delete_save_folder_file(assets, nls, root.as_deref()),
            (18, 109) => self.ext_strings_differ_ignore_case(assets, nls),
            (18, 113 | 114) => self.ext_set_append_file(index == 113, assets, nls, root.as_deref()),
            (18, 115) => self.ext_append_file(assets, nls),
            _ => return None,
        })
    }

    fn pop_operands(&mut self, count: usize, outcome: ExtCallOutcome) -> ExtCallOutcome {
        if self.pop_ext_args(count).len() < count {
            return ExtCallOutcome::Block;
        }
        outcome
    }

    /// 0x4414B0 pops three dynamic strings and copies the current line's
    /// name, voice and text into them; 0 when any operand is not dynamic.
    fn ext_get_current_text(&mut self, assets: &CoreAssets, nls: Nls) -> ExtCallOutcome {
        let args = self.pop_ext_args(3);
        let [name_dst, voice_dst, text_dst] = args[..] else {
            return ExtCallOutcome::Block;
        };
        if ![name_dst, voice_dst, text_dst]
            .into_iter()
            .all(is_dynamic_string_handle)
        {
            return ExtCallOutcome::Value(0);
        }
        let (text, name) = self.adv_text_parts_for_render(assets, nls);
        let voice = self
            .resolve_script_string(self.text_voice_value(), assets, nls)
            .unwrap_or_default();
        self.replace_dynamic_string(name_dst, name);
        self.replace_dynamic_string(voice_dst, voice);
        self.replace_dynamic_string(text_dst, text);
        ExtCallOutcome::Value(1)
    }

    /// The BGM slot a native channel operand names; -1 is the current one.
    fn bgm_channel(&self, channel: i32) -> Option<AudioHandle> {
        let slot = if channel == -1 { 0 } else { channel };
        self.game_audio.get(&(4, slot)).copied()
    }

    /// 0x40C560 get_bgm_status: playing state of the channel, -1 without one.
    fn ext_bgm_status(&mut self, audio: Option<&mut AudioSystem>) -> ExtCallOutcome {
        let args = self.pop_ext_args(1);
        let Some(&channel) = args.first() else {
            return ExtCallOutcome::Block;
        };
        let Some(handle) = self.bgm_channel(channel) else {
            return ExtCallOutcome::Value(-1);
        };
        let playing = audio.is_some_and(|audio| audio.is_playing(handle).unwrap_or(false));
        ExtCallOutcome::Value(i32::from(playing))
    }

    /// 0x410410 btn_link pops group and two indexes and links the two
    /// button records to each other (+0x48).
    fn ext_btn_link(&mut self) -> ExtCallOutcome {
        let args = self.pop_ext_args(3);
        let [group, first, second] = args[..] else {
            return ExtCallOutcome::Block;
        };
        if !(0..=16).contains(&group) || !(0..=128).contains(&first) || !(0..=128).contains(&second)
        {
            return ExtCallOutcome::Value(0);
        }
        self.button_links.insert((group, first), second);
        self.button_links.insert((group, second), first);
        ExtCallOutcome::Value(1)
    }

    /// 0x410530 btn_get_link reads the linked index of a button record.
    fn ext_btn_get_link(&mut self) -> ExtCallOutcome {
        let args = self.pop_ext_args(2);
        let [group, index] = args[..] else {
            return ExtCallOutcome::Block;
        };
        if !(0..=16).contains(&group) || !(0..=128).contains(&index) {
            return ExtCallOutcome::Value(0);
        }
        ExtCallOutcome::Value(self.button_links.get(&(group, index)).copied().unwrap_or(0))
    }

    /// 0x419C00 deletes a file; the portable path only touches the save
    /// folder.
    fn ext_delete_save_folder_file(
        &mut self,
        assets: &CoreAssets,
        nls: Nls,
        root: Option<&std::path::Path>,
    ) -> ExtCallOutcome {
        let args = self.pop_ext_args(1);
        let Some(&value) = args.first() else {
            return ExtCallOutcome::Block;
        };
        let name = self
            .resolve_script_string(value, assets, nls)
            .unwrap_or_default();
        let deleted = root
            .and_then(|root| save_folder_path(root, &name))
            .is_some_and(|path| std::fs::remove_file(path).is_ok());
        ExtCallOutcome::Value(i32::from(deleted))
    }

    /// 0x41E8E0: 1 when two strings differ after ASCII upper-casing.
    fn ext_strings_differ_ignore_case(&mut self, assets: &CoreAssets, nls: Nls) -> ExtCallOutcome {
        let args = self.pop_ext_args(2);
        let [first, second] = args[..] else {
            return ExtCallOutcome::Block;
        };
        let text = |value| {
            self.resolve_script_string(value, assets, nls)
                .unwrap_or_default()
        };
        ExtCallOutcome::Value(i32::from(!text(first).eq_ignore_ascii_case(&text(second))))
    }

    /// 0x41EBC0 (create, "w") and 0x41ECC0 (keep) remember the append target
    /// from a dynamic string; the portable target must be in the save folder.
    fn ext_set_append_file(
        &mut self,
        truncate: bool,
        assets: &CoreAssets,
        nls: Nls,
        root: Option<&std::path::Path>,
    ) -> ExtCallOutcome {
        let args = self.pop_ext_args(1);
        let Some(&value) = args.first() else {
            return ExtCallOutcome::Block;
        };
        let path = is_dynamic_string_handle(value)
            .then(|| self.resolve_script_string(value, assets, nls))
            .flatten()
            .zip(root)
            .and_then(|(name, root)| save_folder_path(root, &name));
        let ready = path.as_ref().is_some_and(|path| {
            !truncate
                || path
                    .parent()
                    .map_or(Ok(()), std::fs::create_dir_all)
                    .is_ok()
                    && std::fs::write(path, b"").is_ok()
        });
        self.append_file = path.filter(|_| ready);
        if ready {
            ExtCallOutcome::Continue
        } else {
            ExtCallOutcome::Value(0)
        }
    }

    /// 0x41EDA0 appends a string to the remembered file.
    fn ext_append_file(&mut self, assets: &CoreAssets, nls: Nls) -> ExtCallOutcome {
        use std::io::Write;
        let args = self.pop_ext_args(1);
        let Some(&value) = args.first() else {
            return ExtCallOutcome::Block;
        };
        let text = self
            .resolve_script_string(value, assets, nls)
            .unwrap_or_default();
        let written = self.append_file.as_ref().is_some_and(|path| {
            let bytes = nls
                .encode(&text)
                .unwrap_or_else(|_| text.clone().into_bytes());
            std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(path)
                .and_then(|mut file| file.write_all(&bytes))
                .is_ok()
        });
        ExtCallOutcome::Value(i32::from(written))
    }
}
