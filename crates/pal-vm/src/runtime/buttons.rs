//! Native button sound ownership and the delayed reaction-completion latch.
use super::*;

#[derive(Clone, Debug, Default)]
pub(super) struct ButtonSounds {
    resource_ids: [i32; 2],
    hover: Vec<AudioHandle>,
    click: Option<AudioHandle>,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct PendingButtonReaction {
    pub index: i32,
    pub synthetic: bool,
}

impl ScriptRuntime {
    pub(super) fn effective_button_flags(&self, flags: u32) -> u32 {
        if self.button_release_mode && self.input_release_shortcuts {
            if self.input_registered_profile { flags | 2 } else { 2 }
        } else {
            flags
        }
    }

    fn button_sound_suppressed(&self, group: i32, index: i32) -> bool {
        self.input_registered_profile && self.game_buttons.get(&(group, index))
            .is_some_and(|entry| self.effective_button_flags(entry.native_flags) & 8 != 0)
    }

    pub(super) fn ext_btn_set_rect(&mut self, sprites: Option<&mut SpriteSystem>) -> ExtCallOutcome {
        let args = self.pop_ext_args(6);
        if args.len() != 6 { return ExtCallOutcome::Block; }
        if let (Some(entry), Some(sprites)) = (self.game_buttons.get(&(args[0], args[1])), sprites) {
            sprites.set_rect(entry.handle, Some(PalRect::new(0, 0, args[2], args[3])));
            sprites.set_offset_rect(entry.handle, args[4] as u32, args[5] as u32);
        }
        ExtCallOutcome::Continue
    }

    pub(super) fn ext_btn_anim_clear(&mut self, sprites: Option<&mut SpriteSystem>) -> ExtCallOutcome {
        let args = self.pop_ext_args(2);
        if args.len() != 2 { return ExtCallOutcome::Block; }
        if let Some(entry) = self.game_buttons.get_mut(&(args[0], args[1])) {
            entry.anim_resource = None;
            entry.anim_play_flag = 0;
            if let Some(sprites) = sprites { sprites.stop_motion(entry.handle); }
        }
        ExtCallOutcome::Continue
    }

    pub(super) fn ext_btn_get_offmouse(&mut self) -> ExtCallOutcome {
        let args = self.pop_ext_args(1);
        let Some(&group) = args.first() else { return ExtCallOutcome::Block; };
        ExtCallOutcome::Value(self.button_groups.get(&group).map_or(0, |g| g.offmouse_index))
    }

    pub(super) fn ext_btn_onmouse_clear(&mut self) -> ExtCallOutcome {
        let args = self.pop_ext_args(1);
        let Some(&group) = args.first() else { return ExtCallOutcome::Block; };
        self.leave_button_hover(group);
        ExtCallOutcome::Continue
    }

    pub(super) fn leave_button_hover(&mut self, group: i32) -> bool {
        let Some(index) = self.button_callback_hover.remove(&group) else { return false; };
        self.button_groups.entry(group).or_default().offmouse_index = index;
        if let Some(point) = self.game_buttons.get(&(group, index)).and_then(|e| e.hover_callbacks[1]) {
            self.pending_gosub_points.push_back(point);
            return true;
        }
        false
    }

    pub(super) fn expire_button_locks(&mut self) {
        let expired: Vec<_> = self.button_groups.iter().filter_map(|(&id, group)| {
            (group.locked && group.lock_duration_ms != 0
                && self.pal_time_ms.wrapping_sub(group.lock_started_ms) >= group.lock_duration_ms)
                .then_some(id)
        }).collect();
        for group in expired {
            self.button_groups.get_mut(&group).unwrap().locked = false;
            for entry in self.matching_button_entries_mut(group, -1) { entry.locked = false; }
        }
    }

    pub(super) fn inject_pending_button_callbacks(&mut self, points: &PointTable) -> bool {
        let mut targets = Vec::new();
        while let Some(point) = self.pending_gosub_points.pop_front() {
            match points.resolve_target_pc(point) {
                Ok(Some(pc)) => targets.push(pc),
                Ok(None) => log::debug!("button callback point[{point}] is empty"),
                Err(error) => log::warn!("button callback point[{point}] is invalid: {error}"),
            }
        }
        let Some(&first) = targets.first() else { return false; };
        self.call_stack.push(self.pc);
        // Return from a hover-leave handler enters the queued click handler
        // before executing any instruction of the interrupted script.
        for &target in targets.iter().skip(1).rev() { self.call_stack.push(target); }
        self.pc = first;
        true
    }

    pub(super) fn retire_button_sounds(&mut self, group: i32) {
        let keys: Vec<_> = self.button_sounds.keys().copied().filter(|&key| group < 0 || key == group).collect();
        for key in keys {
            if let Some(sounds) = self.button_sounds.remove(&key) {
                self.retired_button_sounds.extend(sounds.hover.into_iter().chain(sounds.click));
            }
        }
    }

    pub(crate) fn flush_button_audio_releases(&mut self, audio: &mut AudioSystem) {
        for handle in self.retired_button_sounds.drain(..) { let _ = audio.release(handle); }
    }

    /// Game.exe creates groups with PalButtonCreateEx: the group owns both
    /// original sound handles and four copies of the hover sound.
    pub(crate) fn prepare_button_audio(
        &mut self,
        assets: &CoreAssets,
        nls: Nls,
        resources: Option<&mut ResourceManager>,
        audio: Option<&mut AudioSystem>,
    ) {
        let Some(audio) = audio else { return; };
        self.flush_button_audio_releases(audio);
        let stale: Vec<_> = self.button_sounds.iter().filter_map(|(&group, sounds)| {
            let ids = self.button_groups.get(&group).map(|g| [g.normal_image, g.hover_image]);
            (ids != Some(sounds.resource_ids)).then_some(group)
        }).collect();
        for group in stale {
            if let Some(sounds) = self.button_sounds.remove(&group) {
                for handle in sounds.hover.into_iter().chain(sounds.click) {
                    let _ = audio.release(handle);
                }
            }
        }
        let Some(resources) = resources else { return; };
        let missing: Vec<_> = self.button_groups.iter()
            .filter(|(group, _)| !self.button_sounds.contains_key(group))
            .map(|(&group, g)| (group, [g.normal_image, g.hover_image])).collect();
        for (group, resource_ids) in missing {
            let mut handles = [None; 2];
            for (lane, &id) in resource_ids.iter().enumerate() {
                if id == 0x0fff_ffff { continue; }
                let Some(name) = self.resolve_resource_string(id, assets, nls) else { continue; };
                let result = (|| -> anyhow::Result<AudioHandle> {
                    let asset = open_resource_variant(resources, &name, AUDIO_EXTENSIONS)?;
                    let data = decode_game_audio(&asset.bytes, &mut |member| {
                        Ok(open_resource_variant(resources, member, AUDIO_EXTENSIONS)?.bytes)
                    })?;
                    audio.load_static_data(asset.name, data, PalSoundGroup::GROUP5)
                })();
                match result {
                    Ok(handle) => {
                        log::debug!("[trace-button] group={group} sound lane={lane} id={id} name={name:?} handle={handle:?}");
                        handles[lane] = Some(handle);
                    }
                    Err(error) => log::debug!("button sound {name:?} could not load: {error}"),
                }
            }
            let mut hover: Vec<_> = handles[0].into_iter().collect();
            if let Some(source) = handles[0] {
                for _ in 1..5 {
                    if let Ok(handle) = audio.copy_sound(source, PalSoundGroup::GROUP5) {
                        hover.push(handle);
                    }
                }
            }
            self.button_sounds.insert(group, ButtonSounds {
                resource_ids, hover, click: handles[1],
            });
        }
    }

    pub(super) fn begin_button_reaction(
        &mut self, group: i32, index: i32, synthetic: bool, audio: Option<&mut AudioSystem>,
    ) {
        if self.pending_button_reactions.contains_key(&group) { return; }
        self.pending_button_reactions.insert(group, PendingButtonReaction { index, synthetic });
        if !synthetic && !self.button_sound_suppressed(group, index) {
            if let (Some(handle), Some(audio)) =
                (self.button_sounds.get(&group).and_then(|s| s.click), audio) {
                let _ = audio.play(handle, false);
            }
        }
    }

    /// PAL polling consumes a completed latch independently of Game callbacks.
    fn take_button_reaction(&mut self, group: i32, audio: Option<&mut AudioSystem>) -> Option<i32> {
        let pending = *self.pending_button_reactions.get(&group)?;
        let handle = if self.button_sound_suppressed(group, pending.index) { None }
            else { self.button_sounds.get(&group).and_then(|sounds| sounds.click) };
        if pending.synthetic {
            if let (Some(handle), Some(audio)) = (handle, audio) { let _ = audio.play(handle, false); }
            self.pending_button_reactions.get_mut(&group)?.synthetic = false;
            return None;
        }
        if handle.is_some_and(|handle| audio.as_ref().is_some_and(|a| a.is_playing(handle).unwrap_or(false))) {
            return None;
        }
        self.pending_button_reactions.remove(&group);
        self.game_buttons.contains_key(&(group, pending.index)).then_some(pending.index)
    }

    pub(super) fn ext_btn_get_reaction(
        &mut self, sprites: Option<&mut SpriteSystem>, input: Option<&PalInputState>,
        audio: Option<&mut AudioSystem>,
    ) -> ExtCallOutcome {
        let args = self.pop_ext_args(2);
        if args.len() != 2 { return ExtCallOutcome::Block; }
        let (group, index) = (args[0], args[1]);
        if group as u32 >= 16 || index as u32 >= 128 { return ExtCallOutcome::Yield { value: None }; }
        if !self.game_buttons.contains_key(&(group, index)) { return ExtCallOutcome::Value(0); }
        if self.pending_button_reactions.contains_key(&group) {
            return ExtCallOutcome::Value(i32::from(self.take_button_reaction(group, audio).is_some()));
        }
        let (Some(sprites), Some(input)) = (sprites, input) else { return ExtCallOutcome::Value(0); };
        let (x, y) = input.mouse_position();
        let hovered = self.button_hit_at(sprites, x, y, group).map(|(_, index)| index);
        let reaction = if self.button_release_mode {
            if input.mouse_on(PalMouseButton::Left) {
                if let Some(index) = hovered { self.button_group_release_capture.insert(group, index); }
                else { self.button_group_release_capture.remove(&group); }
                None
            } else {
                let captured = self.button_group_release_capture.remove(&group).is_some();
                hovered.filter(|_| captured)
            }
        } else {
            hovered.filter(|&index| self.game_buttons.get(&(group, index)).is_some_and(|entry| {
                let flags = self.effective_button_flags(entry.native_flags);
                if flags & 1 != 0 { input.mouse_push(PalMouseButton::Left) }
                else { flags & 2 != 0 && (input.raw_key_push() | input.raw_key_on_ex()) & (1 << 23) != 0 }
            }))
        };
        for (&(entry_group, entry_index), entry) in &self.game_buttons {
            if entry_group != group || !entry.enabled || entry.native_flags & 4 != 0 { continue; }
            let row = if reaction == Some(entry_index) { 2 } else if hovered == Some(entry_index) { 1 } else { 0 };
            sprites.rect_set_pos(entry.handle, 0, entry.toggle.wrapping_add(row));
        }
        if let Some(index) = reaction { self.begin_button_reaction(group, index, false, audio); }
        else { self.update_button_hover_sound(group, hovered, audio); }
        ExtCallOutcome::Value(0)
    }

    /// Koikake 0x40EC60 stores the wrapping product without clamping.
    pub(super) fn ext_btn_slider_set_native(&mut self, sprites: Option<&mut SpriteSystem>) -> ExtCallOutcome {
        let args = self.pop_ext_args(5);
        if args.len() != 5 { return ExtCallOutcome::Block; }
        let (group, index, travel, axis, value) = (args[0], args[1], args[2], args[3], args[4]);
        let Some(entry) = self.game_buttons.get_mut(&(group, index)) else { return ExtCallOutcome::Continue; };
        entry.slider_position[usize::from(axis != 0)] = value.wrapping_mul(travel) / 100;
        if let Some(sprites) = sprites {
            sprites.set_pos(entry.handle, entry.base_position[0].wrapping_add(entry.slider_position[0]),
                entry.base_position[1].wrapping_add(entry.slider_position[1]), 0);
        }
        ExtCallOutcome::Continue
    }

    /// The query returns the clamped value before applying the native Y snap.
    pub(super) fn ext_btn_slider_get_native(
        &mut self, input: Option<&PalInputState>, sprites: Option<&mut SpriteSystem>,
    ) -> ExtCallOutcome {
        let args = self.pop_ext_args(5);
        if args.len() != 5 { return ExtCallOutcome::Block; }
        let (group, index, limit, axis, snap) = (args[0], args[1], args[2], args[3], args[4]);
        if !(0..=1).contains(&axis) { return ExtCallOutcome::Continue; }
        let Some(entry) = self.game_buttons.get_mut(&(group, index)) else { return ExtCallOutcome::Continue; };
        let axis = axis as usize;
        let mouse = input.map_or(-1, |input| { let (x, y) = input.mouse_position(); if axis == 0 { x } else { y } });
        if mouse == -1 { return ExtCallOutcome::Value(entry.slider_position[axis]); }
        let Some(sprites) = sprites else { return ExtCallOutcome::Continue; };
        let Some(sprite) = sprites.get(entry.handle) else { return ExtCallOutcome::Continue; };
        let cell = if axis == 0 { sprite.cell_size.width } else { sprite.cell_size.height } as i32;
        let raw = mouse.wrapping_sub(entry.base_position[axis]).wrapping_sub(cell >> 1);
        let result = if raw > limit { limit } else { raw.max(0) };
        let mut stored = result;
        if snap != 0 && axis == 1 {
            let limit = limit.max(100);
            let remainder = stored % (limit / 100);
            if stored != limit && remainder != 0 { stored = stored.wrapping_add(remainder); }
        }
        entry.slider_position[axis] = stored;
        sprites.set_pos(entry.handle, entry.base_position[0].wrapping_add(entry.slider_position[0]),
            entry.base_position[1].wrapping_add(entry.slider_position[1]), 0);
        ExtCallOutcome::Value(result)
    }

    pub(super) fn ext_btn_get_toggle(&mut self) -> ExtCallOutcome {
        let args = self.pop_ext_args(2);
        if args.len() != 2 { return ExtCallOutcome::Block; }
        let (group, index) = (args[0], args[1]);
        if group as u32 >= 16 || index as u32 >= 128 { return ExtCallOutcome::Yield { value: Some(0) }; }
        let value = if (group as u32) < 16 && (index as u32) < 128
            && self.game_buttons.contains_key(&(group, index)) {
            self.button_groups.get(&group).map_or(0, |state| state.raw_toggle)
        } else { 0 };
        ExtCallOutcome::Value(value)
    }

    pub(super) fn ext_btn_get_info(&mut self, sprites: Option<&SpriteSystem>) -> ExtCallOutcome {
        let args = self.pop_ext_args(3);
        if args.len() != 3 { return ExtCallOutcome::Block; }
        let (group, index, base) = (args[0], args[1], args[2]);
        if group as u32 >= 16 || index as u32 >= 128 { return ExtCallOutcome::Value(0); }
        let Some(entry) = self.game_buttons.get(&(group, index)) else { return ExtCallOutcome::Continue; };
        let Some(sprite) = sprites.and_then(|sprites| sprites.get(entry.handle)) else { return ExtCallOutcome::Continue; };
        if base as u32 > 120 { return ExtCallOutcome::Value(0); }
        let words = [entry.base_position[0], entry.base_position[1],
            sprite.cell_size.width as i32, sprite.cell_size.height as i32,
            sprite.texture_size.width as i32, sprite.texture_size.height as i32];
        for (offset, value) in words.into_iter().enumerate() {
            self.write_temp_mem_absolute(base + offset as i32, value);
        }
        ExtCallOutcome::Value(1)
    }

    pub(super) fn poll_button_reactions(
        &mut self, mut audio: Option<&mut AudioSystem>, input: &PalInputState,
    ) -> bool {
        let groups: Vec<_> = self.pending_button_reactions.keys().copied().collect();
        let mut completed = false;
        for group in groups {
            if self.button_groups.get(&group).is_some_and(|state| state.locked) { continue; }
            let Some(index) = self.take_button_reaction(group, audio.as_deref_mut()) else { continue; };
            // Keep a bounded compatibility latch; native queries read push_index.
            let queue = self.button_push_queue.entry(group).or_default();
            queue.clear();
            queue.push_back(index);
            // Native click installs its continuation first, then calls the
            // hover leave handler on top of it, so leave executes first.
            if self.game_buttons.get(&(group, index)).is_some_and(|entry| entry.gosub_point.is_some()) {
                self.leave_button_hover(group);
                self.button_groups.entry(group).or_default().push_index = index;
            }
            self.dispatch_button_push_compat(group, index);
            if self.skip_state(input) == 0 {
                self.system_button_blocked_at = Some(self.pal_time_ms);
                self.button_input_block_duration = 100;
            }
            completed = true;
        }
        completed
    }

    pub(super) fn update_button_hover_sound(
        &mut self, group: i32, index: Option<i32>, audio: Option<&mut AudioSystem>,
    ) {
        if self.button_sound_hover.get(&group).copied() == index { return; }
        let Some(index) = index else { self.button_sound_hover.remove(&group); return; };
        self.button_sound_hover.insert(group, index);
        if self.button_sound_suppressed(group, index) { return; }
        let (Some(sounds), Some(audio)) = (self.button_sounds.get_mut(&group), audio) else { return; };
        let Some(&first) = sounds.hover.first() else { return; };
        let handle = sounds.hover.iter().copied().find(|&h| !audio.is_playing(h).unwrap_or(false)).unwrap_or(first);
        let _ = audio.play(handle, false);
    }

    /// Koikake Game.exe 0x410180 enters one hover handler per group. Moving
    /// directly to another button first leaves the previous handler; entering
    /// the new handler happens on a subsequent poll. Both calls use mode 0.
    pub(super) fn update_button_hover_callbacks(
        &mut self, hovered: &[(i32, Option<i32>)], input: &PalInputState,
    ) -> bool {
        let mut queued = false;
        for &(group, index) in hovered {
            if let Some(&previous) = self.button_callback_hover.get(&group) {
                if index != Some(previous) { queued |= self.leave_button_hover(group); }
                continue;
            }
            if let Some(index) = index {
                if let Some(point) = self.game_buttons.get(&(group, index))
                    .and_then(|entry| entry.hover_callbacks[0]) {
                    self.button_callback_hover.insert(group, index);
                    self.pending_gosub_points.push_back(point);
                    self.button_groups.entry(group).or_default().onmouse_index = index;
                    queued = true;
                }
            }
        }
        if queued && self.skip_state(input) == 0 {
            self.system_button_blocked_at = Some(self.pal_time_ms);
            self.button_input_block_duration = 50;
        }
        queued
    }

    pub(crate) fn button_reaction_pending(&self) -> bool {
        !self.pending_button_reactions.is_empty()
    }
}

#[cfg(all(test, feature = "host-audio"))]
mod tests {
    use super::*;
    use crate::audio::AudioConfig;
    use kira::{Frame, sound::static_sound::{StaticSoundData, StaticSoundSettings}};

    #[test]
    fn physical_click_waits_for_sound_completion_before_dispatch() {
        let (mut runtime, mut sprites, mut input) = super::super::button_mode_tests::button_scene();
        let mut audio = AudioSystem::new(AudioConfig { enabled: true }).unwrap();
        let sink = audio.host_sink();
        let click = audio.load_static_data("click", StaticSoundData {
            sample_rate: 48_000,
            frames: vec![Frame::from_mono(0.25); 2400].into(),
            settings: StaticSoundSettings::default(), slice: None,
        }, PalSoundGroup::GROUP5).unwrap();
        runtime.button_sounds.insert(2, ButtonSounds {
            resource_ids: [0, 0], hover: Vec::new(), click: Some(click),
        });
        input.handle_mouse_button_event(crate::event::MouseButton::Left, true);
        assert!(runtime.update_button_input_state_with_audio(&mut sprites, &input, Some(&mut audio)).consumed_mouse_capture);
        assert!(runtime.pending_gosub_points.is_empty());
        let mut output = [0; 1024];
        sink.render_i16(&mut output);
        assert!(output.iter().any(|&sample| sample > 100));
        input.begin_frame();
        assert!(runtime.update_button_input_state_with_audio(&mut sprites, &input, Some(&mut audio)).consumed_mouse_capture);
        assert!(runtime.pending_gosub_points.is_empty());
        for _ in 0..8 { sink.render_i16(&mut output); }
        input.begin_frame();
        assert!(runtime.update_button_input_state_with_audio(&mut sprites, &input, Some(&mut audio)).consumed_push_edge);
        assert_eq!(runtime.pending_gosub_points.pop_front(), Some(123));
        assert!(!runtime.button_reaction_pending());
    }
}
