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
            (18, 3) => self.ext_string_contains_ignore_case(assets, nls),
            // 0x425D70 sp_load_wait_time and 0x425C90 sp_draw pop a slot or
            // duration; portable sprites load synchronously and draw every
            // frame.
            (3, 42 | 43) => self.pop_operands(1, ExtCallOutcome::Continue),
            (4, 5) => self.ext_bgm_fade_channel_volume(),
            (4, 16 | 17) => self.ext_bgm_query(index == 17, assets, audio),
            (8, 28) => self.ext_btn_link(),
            (8, 30) => self.ext_btn_get_link(),
            // 0x438970 set_cursor pops name and hotspot; the portable cursor
            // stays the default arrow.
            (9, 48) => {
                self.system_state.set_cursor_null(true);
                self.pop_operands(3, ExtCallOutcome::Value(1))
            }
            // 0x41DD50 and 0x41DD60 jump into AdvDebugOutputCore (0x41D990)
            // like 15:4: a format and seven values, debug output only.
            (18, 90 | 92) => self.pop_operands(8, ExtCallOutcome::Continue),
            // 0x411F10 shows a message box; 0x411B00 only logs its operand.
            (15, 1 | 9) => self.pop_operands(1, ExtCallOutcome::Continue),
            // 0x411B50 returns the VM stack depth for debug traces.
            (15, 8) => ExtCallOutcome::Value(self.stack.len() as i32),
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

    /// 0x40BB20 bgm_set_volume_users pops channel, volume percent and
    /// duration, then fades the channel volume linearly from its current
    /// value (0x40B280 applies `PalSoundSetVolumeChannel(volume * 100)`); a
    /// skip makes the fade immediate.
    fn ext_bgm_fade_channel_volume(&mut self) -> ExtCallOutcome {
        let args = self.pop_ext_args(3);
        let [channel, volume, duration] = args[..] else {
            return ExtCallOutcome::Block;
        };
        if let Some(handle) = self.bgm_channel(channel) {
            let duration_ms = if self.dispatch_skip_state() != 0 {
                0
            } else {
                duration.max(0) as u32
            };
            self.channel_fades.retain(|fade| fade.handle != handle);
            self.channel_fades.push(ChannelFade {
                handle,
                from: None,
                to: percent_to_volume(volume).raw(),
                start_ms: self.pal_time_ms,
                duration_ms,
            });
        }
        ExtCallOutcome::Continue
    }

    /// Step the 4:5 channel fades; called once per frame.
    pub(crate) fn advance_channel_fades(&mut self, audio: &mut AudioSystem) {
        let now = self.pal_time_ms;
        self.channel_fades.retain_mut(|fade| {
            let Ok(current) = audio.channel_volume(fade.handle) else {
                return false;
            };
            let from = *fade.from.get_or_insert(current.raw());
            let elapsed = now.wrapping_sub(fade.start_ms).min(fade.duration_ms);
            let raw = if fade.duration_ms == 0 {
                fade.to
            } else {
                from + (fade.to - from) * elapsed as i32 / fade.duration_ms as i32
            };
            let _ = audio.set_channel_volume(fade.handle, PalVolume::from_raw(raw));
            elapsed < fade.duration_ms
        });
    }

    /// The BGM slot a native channel operand names; -1 is the current one.
    fn bgm_channel(&self, channel: i32) -> Option<AudioHandle> {
        let slot = if channel == -1 { 0 } else { channel };
        self.game_audio.get(&(4, slot)).copied()
    }

    /// 0x40C560 get_bgm_status pops a channel; 0x40C610 get_bgm_pos also
    /// pops a PalSoundGetParam selector. Channel -1 is PalSoundGetNowChannel;
    /// no channel answers -1. Status is PalSoundGetStatus.
    fn ext_bgm_query(
        &mut self,
        param: bool,
        assets: &CoreAssets,
        audio: Option<&mut AudioSystem>,
    ) -> ExtCallOutcome {
        let args = self.pop_ext_args(if param { 2 } else { 1 });
        let Some(&channel) = args.first() else {
            return ExtCallOutcome::Block;
        };
        let Some(audio) = audio else {
            return ExtCallOutcome::Value(-1);
        };
        let channel = if channel == -1 {
            audio.now_channel()
        } else {
            channel
        };
        if channel < 0 {
            return ExtCallOutcome::Value(-1);
        }
        let value = match args.get(1) {
            Some(&selector) => audio.sound_param(channel, selector, assets.extended_softpal),
            None => audio.native_group3_status(channel),
        };
        ExtCallOutcome::Value(value.unwrap_or(-1))
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
    /// 0x41B050 pops a haystack and then a needle, resolves both through
    /// 0x44B500 (dynamic string, else a File.dat entry), upper-cases them and
    /// returns whether `strstr` finds the needle. The stand-sprite helpers use
    /// it to map a sprite file id to its character (point 311). Game.exe's
    /// 18:3 is a plain inequality test instead.
    fn ext_string_contains_ignore_case(&mut self, assets: &CoreAssets, nls: Nls) -> ExtCallOutcome {
        let args = self.pop_ext_args(2);
        let [haystack, needle] = args[..] else {
            return ExtCallOutcome::Block;
        };
        let text = |value| {
            self.resolve_resource_string(value, assets, nls)
                .unwrap_or_default()
                .to_ascii_uppercase()
        };
        ExtCallOutcome::Value(i32::from(text(haystack).contains(&text(needle))))
    }

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

/// A running 4:5 channel volume fade (raw PAL volume units).
#[derive(Clone, Debug)]
pub(super) struct ChannelFade {
    handle: AudioHandle,
    from: Option<i32>,
    to: i32,
    start_ms: u32,
    duration_ms: u32,
}

#[cfg(test)]
mod tests {
    use super::super::tests::empty_core_assets;
    use super::*;
    use crate::assets::NativeBuild;

    #[test]
    fn string_test_is_a_case_insensitive_substring_search() {
        let (mut assets, _) = empty_core_assets(false);
        assets.native_build = NativeBuild::Trenka;
        let mut vm = ScriptRuntime::boot(12, ScriptRuntimeConfig::default());
        vm.dynamic_strings = vec!["st02a_a080.pgd".into(), "ST02".into(), "ST07".into()];
        let handle = |index: i32| 0x1000_0000 | index;
        // Needle pushed first, haystack on top.
        for (needle, expected) in [(1, 1), (2, 0)] {
            vm.stack = vec![handle(needle), handle(0)];
            let outcome = vm.dispatch_trenka_wrapper(18, 3, &assets, Nls::ShiftJis, None, None);
            assert!(matches!(outcome, Some(ExtCallOutcome::Value(v)) if v == expected));
            assert!(vm.stack.is_empty());
        }
    }
}
