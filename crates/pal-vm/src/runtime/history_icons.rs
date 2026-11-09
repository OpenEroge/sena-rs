//! Registered history icon wrappers and their layout-time button consumer.
//! Evidence: totsulover-history-icon-{producers,consumers}.json and
//! totsulover-twelve-reviewed.json. Native pointers never enter saved state.
use super::*;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct IconConfig {
    pub voice_icon_enabled: bool,
    pub voice_icon_name: String,
    pub face_enabled: bool,
    pub face_capture: bool,
    pub face_position: [i32; 2],
    pub face_group: i32,
    pub face_click: i32,
    pub secondary_enabled: bool,
    pub secondary_position: [i32; 2],
    pub secondary_group: i32,
    pub secondary_name: String,
    pub secondary_click: i32,
    pub secondary_hover: i32,
    pub secondary_update: i32,
    /// trenka 14:25 (ctx +0x49AB8/+0x15D94/+0x15D8C): a plain separator
    /// sprite drawn on every visible record (the script passes its
    /// `[history] HISTORY_LINE_*` settings).
    pub separator_enabled: bool,
    pub separator_name: String,
    pub separator_position: [i32; 2],
}

#[derive(Clone, Copy, Debug)]
pub(super) struct RecordIcons {
    pub face: bool,
    pub checkpoint: i32,
}
impl Default for RecordIcons {
    fn default() -> Self {
        Self {
            face: false,
            checkpoint: -1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct VisibleIcon {
    button: bool,
    group: i32,
    index: i32,
    name: String,
    position: [i32; 2],
    click: i32,
    hover: i32,
    priority: i32,
    /// Centre the sprite in a fixed record row once its height is known.
    row_fit: Option<RowFit>,
}

/// trenka 0x42B7A0: `top = anchor - (row_height - height) / 2`; rows whose
/// centred top rises above the view are dropped, then the offset applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RowFit {
    anchor: i32,
    row_height: i32,
    clip_top: i32,
    offset_y: i32,
}

impl RowFit {
    fn top(self, height: i32) -> Option<i32> {
        let top = self
            .anchor
            .wrapping_sub(self.row_height.wrapping_sub(height) / 2);
        (top >= self.clip_top).then(|| top.wrapping_add(self.offset_y))
    }
}

#[derive(Clone, Debug)]
pub(super) struct HistoryIcons {
    pub config: IconConfig,
    pub records: Vec<RecordIcons>,
    pub current_checkpoint: i32,
    sound: Option<AudioHandle>,
    init_sounds: [Option<AudioHandle>; 3],
    retired_sounds: Vec<AudioHandle>,
    owned_buttons: BTreeMap<(i32, i32), SpriteHandle>,
    owned_sprites: Vec<SpriteHandle>,
    visible: Vec<VisibleIcon>,
    valid: bool,
}
impl Default for HistoryIcons {
    fn default() -> Self {
        Self {
            config: IconConfig::default(),
            records: Vec::new(),
            current_checkpoint: -1,
            sound: None,
            init_sounds: [None; 3],
            retired_sounds: Vec::new(),
            owned_buttons: BTreeMap::new(),
            owned_sprites: Vec::new(),
            visible: Vec::new(),
            valid: false,
        }
    }
}

impl HistoryIcons {
    pub(super) fn invalidate(&mut self) {
        self.valid = false;
    }
}

impl ScriptRuntime {
    pub(super) fn dispatch_history_icons(
        &mut self,
        index: u16,
        assets: &CoreAssets,
        nls: Nls,
        mut resources: Option<&mut ResourceManager>,
        mut audio: Option<&mut AudioSystem>,
    ) -> ExtCallOutcome {
        let count = match index {
            13 | 15 | 24 => 3,
            25 => 4,
            14 | 17 | 18 | 21 => 1,
            16 | 19 | 23 => 0,
            22 => 5,
            _ => return ExtCallOutcome::Skip,
        };
        if self.stack.len() < count {
            return ExtCallOutcome::Block;
        }
        let args = self.pop_ext_args(count);
        let config = &mut self.history_state.icons.config;
        match index {
            13 => {
                config.face_enabled = args[0] != 0;
                config.face_capture = config.face_enabled;
                config.face_position = [args[1], args[2]];
            }
            14 => config.face_capture = args[0] != 0 && config.face_enabled,
            15 => {
                config.face_group = args[0];
                config.face_click = 0;
                if config.face_enabled {
                    config.face_click = args[2];
                    let group = self.button_groups.get(&args[0]);
                    let ids = [
                        group.map_or(0, |g| g.normal_image),
                        group.map_or(0, |g| g.hover_image),
                        args[1],
                    ];
                    for (slot, id) in ids.into_iter().enumerate() {
                        let handle = self.load_history_icon_sound(
                            id,
                            assets,
                            nls,
                            resources.as_deref_mut(),
                            audio.as_deref_mut(),
                        );
                        let old = std::mem::replace(
                            &mut self.history_state.icons.init_sounds[slot],
                            handle,
                        );
                        self.history_state.icons.retired_sounds.extend(old);
                    }
                }
            }
            16 => {
                if config.face_enabled {
                    let handles =
                        std::mem::replace(&mut self.history_state.icons.init_sounds, [None; 3]);
                    self.history_state
                        .icons
                        .retired_sounds
                        .extend(handles.into_iter().flatten());
                }
            }
            17 => config.face_click = args[0],
            18 => {
                if args[0] != 0 {
                    let handle = self.load_history_icon_sound(
                        args[0],
                        assets,
                        nls,
                        resources.as_deref_mut(),
                        audio.as_deref_mut(),
                    );
                    let old = std::mem::replace(&mut self.history_state.icons.sound, handle);
                    // The native replacement leaks its previous handle. Retiring it
                    // has no playback effect: field +89948 has no playback reader.
                    self.history_state.icons.retired_sounds.extend(old);
                }
            }
            19 => {
                let handle = self.history_state.icons.sound.take();
                self.history_state.icons.retired_sounds.extend(handle);
            }
            21 => {
                // Native record zero is newest; portable records are oldest first.
                let value = usize::try_from(args[0])
                    .ok()
                    .and_then(|offset| self.history_state.icons.records.iter().rev().nth(offset))
                    .map_or(-1, |record| record.checkpoint);
                return ExtCallOutcome::Value(value);
            }
            22 => {
                config.secondary_group = 0;
                config.secondary_click = 0;
                config.secondary_update = 0;
                if config.secondary_enabled {
                    let name = self
                        .resolve_resource_string(args[1], assets, nls)
                        .unwrap_or_default();
                    let config = &mut self.history_state.icons.config;
                    config.secondary_group = args[0];
                    config.secondary_name = name;
                    config.secondary_click = args[2];
                    config.secondary_hover = args[3];
                    config.secondary_update = args[4];
                }
            }
            23 => {
                if config.secondary_enabled {
                    config.secondary_group = 0;
                    config.secondary_click = 0;
                    config.secondary_hover = 0;
                }
            }
            24 => {
                config.secondary_enabled = args[0] != 0;
                config.secondary_position = [args[1], args[2]];
            }
            25 => {
                // 0x42D1A0 pops enable, File.dat name, x, y.
                config.separator_enabled = args[0] != 0;
                config.separator_position = [args[2], args[3]];
                let name = self
                    .resolve_resource_string(args[1], assets, nls)
                    .unwrap_or_default();
                log::debug!(
                    "[trace-history] separator line enabled={} name={name:?} at={:?}",
                    args[0],
                    [args[2], args[3]]
                );
                self.history_state.icons.config.separator_name = name;
            }
            _ => unreachable!(),
        }
        if let Some(audio) = audio {
            self.flush_history_icon_sounds(audio);
        }
        ExtCallOutcome::Continue
    }

    fn load_history_icon_sound(
        &self,
        id: i32,
        assets: &CoreAssets,
        nls: Nls,
        resources: Option<&mut ResourceManager>,
        audio: Option<&mut AudioSystem>,
    ) -> Option<AudioHandle> {
        if id == 0 || id == 0x0fff_ffff {
            return None;
        }
        let (Some(resources), Some(audio)) = (resources, audio) else {
            return None;
        };
        let name = self.resolve_resource_string(id, assets, nls)?;
        let result = (|| -> anyhow::Result<AudioHandle> {
            let asset = open_resource_variant(resources, &name, AUDIO_EXTENSIONS)?;
            let data = decode_game_audio(&asset.bytes, &mut |member| {
                Ok(open_resource_variant(resources, member, AUDIO_EXTENSIONS)?.bytes)
            })?;
            audio.load_static_data(asset.name, data, PalSoundGroup::GROUP5)
        })();
        match result {
            Ok(handle) => Some(handle),
            Err(error) => {
                log::debug!("history icon sound {name:?} could not load: {error}");
                None
            }
        }
    }

    pub(crate) fn flush_history_icon_sounds(&mut self, audio: &mut AudioSystem) {
        for handle in self.history_state.icons.retired_sounds.drain(..) {
            let _ = audio.stop(handle);
            let _ = audio.release(handle);
        }
    }

    /// Build actual PAL-style buttons, so hover/click callbacks use the same
    /// group ordering and reaction-completion rules as script-created buttons.
    pub(crate) fn sync_history_icons(
        &mut self,
        assets: &CoreAssets,
        nls: Nls,
        mut resources: Option<&mut ResourceManager>,
        sprites: &mut SpriteSystem,
    ) {
        let config = self.history_state.icons.config.clone();
        let mut visible = Vec::new();
        // Icons exist only where the registered history icon producers do.
        if self.history_state.active && assets.registered_extcall(14, 22) {
            let (start, end, tops) = self.history_visible_window(assets, nls);
            let cache = self.history_state.layout_cache.as_ref().unwrap();
            let [left, top, _, _] = self.history_state.rect;
            for (offset, &record_top) in tops.iter().enumerate().rev() {
                let record_index = start + offset;
                let ordinal = (end - 1 - record_index) as i32;
                let metadata = self
                    .history_state
                    .icons
                    .records
                    .get(record_index)
                    .copied()
                    .unwrap_or_default();
                let record = &cache.records[record_index];
                let body_top = top
                    + record_top
                    + if record.name.is_some() {
                        cache.name_block
                    } else {
                        0
                    };
                let priority = ordinal.wrapping_sub(self.sprite_priority_cursor.wrapping_add(4866));
                if config.voice_icon_enabled
                    && record.has_voice
                    && record.name.is_some()
                    && !config.voice_icon_name.is_empty()
                {
                    visible.push(VisibleIcon {
                        button: false,
                        group: -1,
                        index: ordinal,
                        name: config.voice_icon_name.clone(),
                        position: [
                            left + self.history_state.layout[4],
                            top + record_top + self.history_state.layout[5],
                        ],
                        click: 0,
                        hover: 0,
                        priority: 0i32.wrapping_sub(self.sprite_priority_cursor.wrapping_add(4871)),
                        row_fit: None,
                    });
                }
                if config.face_enabled
                    && metadata.face
                    && record.has_voice
                    && record.name.is_some()
                    && config.voice_icon_enabled
                {
                    let voice = self.history_record_text(record_index, assets, nls).voice;
                    if voice
                        .get(..2)
                        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("VO"))
                    {
                        let suffix = if voice.as_bytes().get(2).is_some_and(u8::is_ascii_digit) {
                            voice.get(2..4).unwrap_or("99")
                        } else {
                            "99"
                        };
                        visible.push(VisibleIcon {
                            button: true,
                            group: config.face_group,
                            index: ordinal,
                            name: format!("IC{suffix}"),
                            position: [
                                left + config.face_position[0],
                                body_top + config.face_position[1],
                            ],
                            click: config.face_click,
                            hover: 0,
                            priority,
                            row_fit: None,
                        });
                    }
                }
                if config.secondary_enabled
                    && metadata.checkpoint != -1
                    && !config.secondary_name.is_empty()
                {
                    visible.push(VisibleIcon {
                        button: true,
                        group: config.secondary_group,
                        index: ordinal,
                        name: config.secondary_name.clone(),
                        position: [
                            left + config.secondary_position[0],
                            top + record_top + config.secondary_position[1],
                        ],
                        click: config.secondary_click,
                        hover: config.secondary_hover,
                        priority,
                        row_fit: None,
                    });
                }
                if config.separator_enabled && !config.separator_name.is_empty() {
                    visible.push(VisibleIcon {
                        button: false,
                        group: -1,
                        index: ordinal,
                        name: config.separator_name.clone(),
                        position: [left + config.separator_position[0], top + record_top],
                        click: 0,
                        hover: 0,
                        priority: 0i32.wrapping_sub(self.sprite_priority_cursor.wrapping_add(4871)),
                        row_fit: Some(RowFit {
                            anchor: top + record_top,
                            row_height: self.history_state.layout[6],
                            clip_top: top,
                            offset_y: config.separator_position[1],
                        }),
                    });
                }
            }
        }
        visible.retain(|icon| {
            !icon.button
                || ((icon.group as u32) < 16 && self.button_groups.contains_key(&icon.group))
        });
        let handles_valid = self
            .history_state
            .icons
            .owned_buttons
            .iter()
            .all(|(key, handle)| {
                self.game_buttons
                    .get(key)
                    .is_some_and(|entry| entry.handle == *handle)
            });
        if self.history_state.icons.valid
            && handles_valid
            && visible == self.history_state.icons.visible
        {
            return;
        }
        self.clear_history_icon_scene(sprites);
        if resources.is_none() {
            return;
        }
        for icon in &visible {
            if !icon.button {
                let resources = resources.as_deref_mut().unwrap();
                let result = open_resource_variant(resources, &icon.name, IMAGE_EXTENSIONS)
                    .map_err(anyhow::Error::from)
                    .and_then(|asset| decode_asset_image(resources, &asset));
                if let Err(error) = &result {
                    log::debug!(
                        "[trace-history] icon {:?} could not load: {error}",
                        icon.name
                    );
                }
                if let Ok(image) = result {
                    let y = match icon.row_fit {
                        Some(fit) => match fit.top(image.height as i32) {
                            Some(y) => y,
                            None => continue,
                        },
                        None => icon.position[1],
                    };
                    if let Some(handle) = sprites.create_rgba_sprite(
                        image.width,
                        image.height,
                        image.rgba,
                        PalVec3::new(icon.position[0], y, 0),
                        icon.priority,
                        icon.name.clone(),
                    ) {
                        self.history_state.icons.owned_sprites.push(handle);
                    }
                }
                continue;
            }
            let record = assets
                .graphic_index
                .as_ref()
                .and_then(|index| index.lookup(&icon.name))
                .cloned();
            let name = record
                .as_ref()
                .and_then(|r| r.replacement_resource())
                .unwrap_or_else(|| icon.name.clone());
            self.create_button_from_name(
                &[icon.group, icon.index, 0, icon.click, icon.hover, 0, 1],
                name,
                resources.as_deref_mut(),
                Some(sprites),
            );
            let Some(entry) = self.game_buttons.get_mut(&(icon.group, icon.index)) else {
                continue;
            };
            let offset = record.as_ref().map_or([0, 0], |r| [r.offset_x, r.offset_y]);
            entry.base_position = icon.position;
            entry.slider_position = offset;
            sprites.set_pos(
                entry.handle,
                icon.position[0].wrapping_add(offset[0]),
                icon.position[1].wrapping_add(offset[1]),
                0,
            );
            sprites.set_priority(
                entry.handle,
                icon.priority.wrapping_sub(
                    record
                        .as_ref()
                        .map_or(0, |r| r.priority_lane.wrapping_mul(134)),
                ),
            );
            if let Some(record) = record {
                if record.flags & 0x100000 != 0 {
                    sprites.set_scale(entry.handle, record.scale_percent as f32 / 100.0);
                }
                if record.flags & 0x80000 != 0 {
                    sprites.set_color(entry.handle, PalColor::from_argb(record.alpha as u32));
                }
                if let (Some(name), Some(resources)) =
                    (record.animation_resource(), resources.as_deref_mut())
                {
                    if let Ok(asset) = open_resource_variant(resources, &name, ANIMATION_EXTENSIONS)
                    {
                        let handle = entry.handle;
                        self.apply_named_sprite_animation(sprites, handle, &asset.bytes);
                    }
                }
            }
            if let Some(entry) = self.game_buttons.get(&(icon.group, icon.index)) {
                self.history_state
                    .icons
                    .owned_buttons
                    .insert((icon.group, icon.index), entry.handle);
            }
        }
        self.history_state.icons.visible = visible;
        self.history_state.icons.valid = true;
    }

    pub(super) fn clear_history_icon_scene(&mut self, sprites: &mut SpriteSystem) {
        for (key, handle) in std::mem::take(&mut self.history_state.icons.owned_buttons) {
            if self
                .game_buttons
                .get(&key)
                .is_some_and(|entry| entry.handle == handle)
            {
                self.game_buttons.remove(&key);
                self.clear_button_capture(key.0, key.1);
                self.pending_button_reactions
                    .retain(|&group, reaction| group != key.0 || reaction.index != key.1);
                sprites.release(handle);
            }
        }
        for handle in self.history_state.icons.owned_sprites.drain(..) {
            sprites.release(handle);
        }
        self.history_state.icons.visible.clear();
        self.history_state.icons.valid = false;
    }
}

#[cfg(test)]
mod tests {
    use super::RowFit;

    #[test]
    fn separator_line_centres_in_the_row_and_clips_above_the_view() {
        let fit = RowFit {
            anchor: 100,
            row_height: 60,
            clip_top: 0,
            offset_y: 5,
        };
        assert_eq!(fit.top(20), Some(85));
        // Odd differences truncate toward zero like the native `sar` after `cdq`.
        assert_eq!(fit.top(21), Some(86));
        let fit = RowFit { anchor: 10, ..fit };
        assert_eq!(fit.top(20), None);
    }
}
