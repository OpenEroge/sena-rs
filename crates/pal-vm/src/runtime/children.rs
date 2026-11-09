//! Newer Game.exe 0x30052..0x30054 target-list wrappers. These are distinct
//! from 0x30031's high-bit parent/child attachment flag.

use super::*;

pub(super) fn truncate_native(value: f32) -> i32 {
    if (-2_147_483_648.0..2_147_483_648.0).contains(&value) {
        value as i32
    } else {
        i32::MIN
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct ChildRenderLane {
    pub flags: u32,
    pub source_slot: i32,
}

/// Attachment offsets belong to the child wrapper, not to each parent lane.
/// A later attachment of the same child overwrites this shared auxiliary bank.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct ChildVisualOffsets {
    offset: [i32; 2],
    scale_delta: [f32; 2],
    rotation_delta: [f32; 3],
    color_delta: [i16; 4],
}

#[derive(Clone, Debug, Default)]
pub(super) struct SavedChildren {
    attachments: Vec<(i32, i32, GameSpriteChildLane)>,
    offsets: Vec<(i32, ChildVisualOffsets)>,
    render_lanes: Vec<(i32, i32, ChildRenderLane)>,
    target_slots: Vec<Option<i32>>,
    capture: Option<crate::scene::SceneTexture>,
    draw_state: crate::scene::blend::NativeDrawState,
}

impl ScriptRuntime {
    /// Game.exe attachment initialization: Koikake 0x41AA50, reference
    /// 0x424A20, newer 0x447860. The native PAL rotation/scale boundaries are
    /// declared separately in graphics32.json.
    pub(super) fn ext_sp_set_child(
        &mut self,
        sprites: Option<&mut SpriteSystem>,
    ) -> ExtCallOutcome {
        let args = self.pop_ext_args(5);
        if args.len() != 5 {
            return ExtCallOutcome::Block;
        }
        let (parent, child, lane) = (args[0], args[1], args[4]);
        let upper = if self.input_registered_profile {
            127
        } else {
            128
        };
        if !(-1..=upper).contains(&parent) || !(-1..=upper).contains(&child) {
            return ExtCallOutcome::Continue;
        }
        // Native omits a lane bound check; keep malformed writes in bounds.
        if !(0..8).contains(&lane) {
            return ExtCallOutcome::Continue;
        }
        let Some(sprites) = sprites else {
            return ExtCallOutcome::Continue;
        };
        let Some(parent_sprite) = self.game_sprites.get(&parent).and_then(|h| sprites.get(*h))
        else {
            return ExtCallOutcome::Continue;
        };
        if child == -1 {
            self.game_sprite_child_lanes.remove(&(parent, lane));
            self.game_sprite_render_lanes.remove(&(parent, lane));
            return ExtCallOutcome::Continue;
        }
        let Some(child_sprite) = self.game_sprites.get(&child).and_then(|h| sprites.get(*h)) else {
            return ExtCallOutcome::Continue;
        };
        let parent_scale = [
            parent_sprite.scale,
            parent_sprite.scale_y.unwrap_or(parent_sprite.scale),
        ];
        let child_scale = [
            child_sprite.scale,
            child_sprite.scale_y.unwrap_or(child_sprite.scale),
        ];
        let offsets = ChildVisualOffsets {
            offset: [args[2], args[3]],
            scale_delta: std::array::from_fn(|axis| {
                child_scale[axis] * parent_scale[axis] - parent_scale[axis]
            }),
            rotation_delta: [
                child_sprite.rotation.x - parent_sprite.rotation.x,
                child_sprite.rotation.y - parent_sprite.rotation.y,
                child_sprite.rotation.z - parent_sprite.rotation.z,
            ],
            color_delta: child_sprite
                .color
                .to_rgba_u8()
                .map(|component| i16::from(component) - 255),
        };
        self.game_sprite_child_offsets.insert(child, offsets);
        self.game_sprite_child_lanes
            .insert((parent, lane), GameSpriteChildLane { child_slot: child });
        if let Some(render) = self.game_sprite_render_lanes.get_mut(&(parent, lane)) {
            render.flags |= 0x1000_0000;
            render.source_slot = child;
        }
        self.apply_game_sprite_child_lanes(sprites);
        ExtCallOutcome::Continue
    }

    /// Copy the parent's submitted lanes, then add the child's saved auxiliary
    /// lanes. Parent slots are walked 0..63 and child lanes 0..7 in all builds.
    pub(super) fn apply_game_sprite_child_lanes(&self, sprites: &mut SpriteSystem) {
        for (&(parent_slot, _), lane) in &self.game_sprite_child_lanes {
            if !(0..64).contains(&parent_slot) {
                continue;
            }
            let (Some(&parent_handle), Some(&child_handle)) = (
                self.game_sprites.get(&parent_slot),
                self.game_sprites.get(&lane.child_slot),
            ) else {
                continue;
            };
            let (Some(parent), Some(child)) =
                (sprites.get(parent_handle), sprites.get(child_handle))
            else {
                continue;
            };
            let parent = parent.info();
            let offsets = self
                .game_sprite_child_offsets
                .get(&lane.child_slot)
                .copied()
                .unwrap_or_default();
            let local_actions = child.combined_lanes();
            let parent_scale = [parent.scale, parent.scale_y.unwrap_or(parent.scale)];
            let child_scale = std::array::from_fn::<_, 2, _>(|axis| {
                parent_scale[axis] + offsets.scale_delta[axis]
            });
            let parent_size = [parent.cell_size.width, parent.cell_size.height];
            let child_size = [child.cell_size.width, child.cell_size.height];
            let parent_position = [parent.position.x, parent.position.y];
            let position = std::array::from_fn::<_, 2, _>(|axis| {
                let local = offsets.offset[axis]
                    .wrapping_add(truncate_native(local_actions.position[axis]));
                let centers = ((parent_size[axis] >> 1) as i32)
                    .wrapping_sub((child_size[axis] >> 1) as i32)
                    .wrapping_sub(local);
                let adjustment =
                    centers.wrapping_sub(truncate_native(centers as f32 * parent_scale[axis]));
                truncate_native(parent_position[axis])
                    .wrapping_add(offsets.offset[axis])
                    .wrapping_add(adjustment) as f32
            });
            let rotation = [parent.rotation.x, parent.rotation.y, parent.rotation.z];
            let rotation = std::array::from_fn::<_, 3, _>(|axis| {
                rotation[axis] + offsets.rotation_delta[axis]
            });
            let color = parent.color.to_rgba_u8();
            let color = std::array::from_fn::<_, 4, _>(|channel| {
                i16::from(color[channel])
                    .wrapping_add(offsets.color_delta[channel])
                    .clamp(0, 255) as u8
            });
            let argb = u32::from(color[3]) << 24
                | u32::from(color[0]) << 16
                | u32::from(color[1]) << 8
                | u32::from(color[2]);
            sprites.set_pos_float(child_handle, position[0], position[1], parent.position.z);
            if self.input_registered_profile {
                sprites.set_scale_xy(child_handle, child_scale[0], child_scale[1]);
            } else {
                sprites.set_scale(child_handle, child_scale[0]);
            }
            sprites.set_rotate_ex(child_handle, rotation[0], rotation[1], rotation[2]);
            sprites.set_color(child_handle, PalColor::from_argb(argb));
            sprites.set_native_projection(child_handle, parent.native_projection);
        }
    }

    pub(super) fn dispatch_target_render(
        &mut self,
        index: u16,
        sprites: Option<&mut SpriteSystem>,
    ) -> ExtCallOutcome {
        let count = match index {
            82 => 1,
            83 => 3,
            84 => 2,
            _ => return ExtCallOutcome::Skip,
        };
        let args = self.pop_ext_args(count);
        if args.len() != count {
            return ExtCallOutcome::Block;
        }
        let parent = args[0];
        if !(-1..128).contains(&parent) {
            return ExtCallOutcome::Value(0);
        }
        let Some(sprites) = sprites else {
            return ExtCallOutcome::Value(0);
        };
        if index == 82 {
            self.game_sprite_child_lanes
                .retain(|(slot, _), _| *slot != parent);
            self.game_sprite_render_lanes
                .retain(|(slot, _), _| *slot != parent);
            sprites.target_render_list_clear();
            return ExtCallOutcome::Value(1);
        }
        let lane = args[if index == 83 { 2 } else { 1 }];
        // Native permits -1 and then writes before its lane array. Do not
        // reproduce that malformed index's out-of-bounds memory corruption.
        if !(0..8).contains(&lane) {
            return ExtCallOutcome::Value(0);
        }
        let source = if index == 83 { args[1] } else { parent };
        if !(-1..128).contains(&source) {
            return ExtCallOutcome::Value(0);
        }
        if index == 83
            && !self
                .game_sprites
                .get(&parent)
                .is_some_and(|handle| sprites.get(*handle).is_some())
        {
            return ExtCallOutcome::Value(0);
        }
        let key = (parent, lane);
        let attachment = self.game_sprite_child_lanes.get_mut(&key);
        let attached = attachment.is_some();
        if let Some(attachment) = attachment {
            attachment.child_slot = source;
        }
        let entry = self.game_sprite_render_lanes.entry(key).or_default();
        entry.flags |= if index == 83 { 17 } else { 18 };
        if attached {
            entry.flags |= 0x1000_0000;
        }
        entry.source_slot = source;
        if index == 83 {
            let Some(handle) = self
                .game_sprites
                .get(&source)
                .copied()
                .filter(|handle| sprites.get(*handle).is_some())
            else {
                return ExtCallOutcome::Value(0);
            };
            sprites.target_render_list_add(Some(handle));
            return ExtCallOutcome::Value(1);
        }
        if self.dispatch_skip_state() & 0x30 != 0 {
            return ExtCallOutcome::Value(1);
        }
        self.prepare_target_children(parent, sprites);
        let Some(destination) = self.game_sprites.get(&parent).copied() else {
            return ExtCallOutcome::Value(0);
        };
        if sprites.target_render_list_draw() == 0 {
            return ExtCallOutcome::Value(0);
        }
        ExtCallOutcome::Value(sprites.render_copy(destination))
    }

    fn prepare_target_children(&mut self, parent: i32, sprites: &mut SpriteSystem) {
        let sources: Vec<_> = self
            .game_sprite_render_lanes
            .iter()
            .filter(|((slot, _), lane)| *slot == parent && lane.flags as u16 == 17)
            .map(|(_, lane)| lane.source_slot)
            .collect();
        for slot in sources {
            if let Some(&transition) = self.game_sprite_transitions.get(&slot) {
                if matches!(sprites.transition_state(transition), 1 | 2) {
                    sprites.cancel_transition(transition);
                    self.game_sprite_transition_sources.remove(&slot);
                }
            }
            if let Some(handle) = self.game_sprites.get(&slot).copied() {
                let _ = self.commit_game_sprite_wrapper_visual(sprites, slot, handle);
            }
        }
        self.apply_game_sprite_child_lanes(sprites);
    }

    pub(super) fn capture_children(&self, sprites: Option<&SpriteSystem>) -> SavedChildren {
        let mut saved = SavedChildren {
            attachments: self
                .game_sprite_child_lanes
                .iter()
                .map(|(&(parent, lane), &value)| (parent, lane, value))
                .collect(),
            render_lanes: self
                .game_sprite_render_lanes
                .iter()
                .map(|(&(parent, lane), &value)| (parent, lane, value))
                .collect(),
            offsets: self
                .game_sprite_child_offsets
                .iter()
                .map(|(&slot, &offsets)| (slot, offsets))
                .collect(),
            ..SavedChildren::default()
        };
        if let Some(sprites) = sprites {
            saved.target_slots = sprites
                .target_render_list_objects()
                .iter()
                .map(|handle| {
                    handle.and_then(|handle| {
                        self.game_sprites
                            .iter()
                            .find_map(|(&slot, &current)| (current == handle).then_some(slot))
                    })
                })
                .collect();
            saved.capture = sprites
                .target_render_capture()
                .filter(|capture| capture.pixels.len() <= SAVE_SPRITE_BYTES_CAP)
                .cloned();
            saved.draw_state = sprites.native_draw_state();
        }
        saved
    }

    pub(super) fn restore_children(&mut self, saved: &SavedChildren, sprites: &mut SpriteSystem) {
        self.game_sprite_child_lanes = saved
            .attachments
            .iter()
            .map(|&(parent, lane, value)| ((parent, lane), value))
            .collect();
        self.game_sprite_render_lanes = saved
            .render_lanes
            .iter()
            .map(|&(parent, lane, value)| ((parent, lane), value))
            .collect();
        self.game_sprite_child_offsets = saved.offsets.iter().copied().collect();
        sprites.target_render_list_clear();
        for slot in &saved.target_slots {
            sprites.target_render_list_add(
                slot.and_then(|slot| self.game_sprites.get(&slot).copied()),
            );
        }
        sprites.restore_target_render_capture(saved.capture.clone());
        sprites.restore_native_draw_state(saved.draw_state);
    }
}

pub(super) fn write(out: &mut Vec<u8>, saved: &SavedChildren) -> std::io::Result<()> {
    if saved.attachments.len() > 1024
        || saved.offsets.len() > 1024
        || saved.render_lanes.len() > 1024
        || saved.target_slots.len() > 4096
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "too many child records",
        ));
    }
    write_u32(out, saved.attachments.len() as u32)?;
    for &(parent, lane, value) in &saved.attachments {
        for word in [parent, lane, value.child_slot] {
            write_i32(out, word)?;
        }
    }
    write_u32(out, saved.offsets.len() as u32)?;
    for &(slot, offsets) in &saved.offsets {
        write_i32(out, slot)?;
        for word in offsets.offset {
            write_i32(out, word)?;
        }
        for value in offsets
            .scale_delta
            .into_iter()
            .chain(offsets.rotation_delta)
        {
            write_u32(out, value.to_bits())?;
        }
        for value in offsets.color_delta {
            write_i32(out, i32::from(value))?;
        }
    }
    write_u32(out, saved.render_lanes.len() as u32)?;
    for &(parent, lane, value) in &saved.render_lanes {
        write_i32(out, parent)?;
        write_i32(out, lane)?;
        write_u32(out, value.flags)?;
        write_i32(out, value.source_slot)?;
    }
    write_u32(out, saved.target_slots.len() as u32)?;
    for slot in &saved.target_slots {
        write_u32(out, u32::from(slot.is_some()))?;
        write_i32(out, slot.unwrap_or(0))?;
    }
    write_u32(out, u32::from(saved.draw_state.blend_disabled))?;
    write_u32(out, u32::from(saved.draw_state.texture_rgb))?;
    write_u32(out, u32::from(saved.capture.is_some()))?;
    if let Some(capture) = &saved.capture {
        write_u32(out, capture.width)?;
        write_u32(out, capture.height)?;
        write_bytes(out, &capture.pixels, SAVE_SPRITE_BYTES_CAP)?;
    }
    Ok(())
}

pub(super) fn read(reader: &mut impl Read) -> std::io::Result<SavedChildren> {
    fn count(reader: &mut impl Read, cap: u32) -> std::io::Result<u32> {
        let count = read_u32_from(reader)?;
        if count > cap {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "too many child records",
            ));
        }
        Ok(count)
    }
    let mut saved = SavedChildren::default();
    for _ in 0..count(reader, 1024)? {
        let parent = read_i32_from(reader)?;
        let lane = read_i32_from(reader)?;
        let value = GameSpriteChildLane {
            child_slot: read_i32_from(reader)?,
        };
        saved.attachments.push((parent, lane, value));
    }
    for _ in 0..count(reader, 1024)? {
        let slot = read_i32_from(reader)?;
        let mut offsets = ChildVisualOffsets::default();
        for value in &mut offsets.offset {
            *value = read_i32_from(reader)?;
        }
        for value in offsets
            .scale_delta
            .iter_mut()
            .chain(offsets.rotation_delta.iter_mut())
        {
            *value = f32::from_bits(read_u32_from(reader)?);
        }
        for value in &mut offsets.color_delta {
            *value = i16::try_from(read_i32_from(reader)?).map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "invalid child color offset",
                )
            })?;
        }
        saved.offsets.push((slot, offsets));
    }
    for _ in 0..count(reader, 1024)? {
        let parent = read_i32_from(reader)?;
        let lane = read_i32_from(reader)?;
        let value = ChildRenderLane {
            flags: read_u32_from(reader)?,
            source_slot: read_i32_from(reader)?,
        };
        saved.render_lanes.push((parent, lane, value));
    }
    for _ in 0..count(reader, 4096)? {
        let present = read_u32_from(reader)? != 0;
        let slot = read_i32_from(reader)?;
        saved.target_slots.push(present.then_some(slot));
    }
    saved.draw_state.blend_disabled = read_u32_from(reader)? != 0;
    saved.draw_state.texture_rgb = read_u32_from(reader)? != 0;
    if read_u32_from(reader)? != 0 {
        let width = read_u32_from(reader)?;
        let height = read_u32_from(reader)?;
        let pixels = read_bytes_capped(reader, SAVE_SPRITE_BYTES_CAP)?;
        if width == 0
            || height == 0
            || (u64::from(width) * u64::from(height)).checked_mul(4) != Some(pixels.len() as u64)
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid target capture dimensions",
            ));
        }
        saved.capture = Some(crate::scene::SceneTexture::rgba8(
            SceneTextureId(0),
            1,
            width,
            height,
            pixels,
        ));
    }
    Ok(saved)
}
