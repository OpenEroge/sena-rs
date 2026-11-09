//! Registered Game.exe button wrappers 0x426AF0..0x427150.
//! Stack and destination contracts: totsulover-twelve-reviewed.json.
use super::*;

impl ScriptRuntime {
    pub(super) fn dispatch_button_extensions(
        &mut self,
        index: u16,
        sprites: Option<&mut SpriteSystem>,
        input: Option<&PalInputState>,
    ) -> ExtCallOutcome {
        let count = match index {
            45 => 5,
            48 => 2,
            42..=47 => 1,
            _ => return ExtCallOutcome::Skip,
        };
        if self.stack.len() < count {
            return ExtCallOutcome::Block;
        }
        let args = self.pop_ext_args(count);
        let group = args[0];
        match index {
            42 => {
                self.system_button_blocked_at = Some(self.pal_time_ms);
                self.button_input_block_duration = args[0] as u32;
            }
            43 => self.button_run_skip = args[0],
            44 => {
                return ExtCallOutcome::Value(if group as u32 >= 16 {
                    -1
                } else {
                    i32::from(self.button_groups.get(&group).is_some_and(|g| g.locked))
                })
            }
            45 => {
                let (parent, child) = (args[1], args[2]);
                if group as u32 >= 16
                    || child as u32 >= 128
                    || (parent != -1 && parent as u32 >= 128)
                {
                    return ExtCallOutcome::Continue;
                }
                if parent == -1 {
                    self.button_children.remove(&(group, child));
                } else if let Some(entry) = self.game_buttons.get_mut(&(group, child)) {
                    entry.slider_position = [args[3], args[4]];
                    self.button_children.insert((group, child), parent);
                }
            }
            46 => {
                if group != -1 && group as u32 >= 16 {
                    return ExtCallOutcome::Yield { value: Some(0) };
                }
                let processing = self.pending_button_reactions.keys().any(|&id| {
                    (group == -1 || group == id)
                        && (id as u32) < 16
                        && self.button_groups.contains_key(&id)
                });
                return ExtCallOutcome::Value(i32::from(processing));
            }
            47 => {
                let hit = if (group as u32) < 16 && self.button_groups.contains_key(&group) {
                    sprites.as_deref().zip(input).and_then(|(sprites, input)| {
                        let (x, y) = input.mouse_position();
                        self.button_hit_at_with_lock(sprites, x, y, group, false)
                            .map(|(_, index)| index)
                    })
                } else {
                    None
                };
                return ExtCallOutcome::Value(hit.unwrap_or(-1));
            }
            48 => {
                if group as u32 >= 16 || args[1] as u32 >= 128 {
                    return ExtCallOutcome::Value(-1);
                }
                let entry = self.game_buttons.get(&(group, args[1]));
                let status = i32::from(
                    self.system_button_blocked_at.is_some()
                        && self.button_input_block_duration != 0,
                ) | (i32::from(!self.button_groups.contains_key(&group)) << 1)
                    | (i32::from(self.button_groups.get(&group).is_some_and(|g| g.locked)) << 2)
                    | (i32::from(entry.is_some_and(|e| !e.enabled)) << 3)
                    | (i32::from(entry.is_none()) << 4);
                return ExtCallOutcome::Value(status);
            }
            _ => unreachable!(),
        }
        ExtCallOutcome::Continue
    }

    /// Game.exe 0x476F80 visits these after the ordinary sprite child lanes.
    /// Button attachments preserve the child's base color (+100) and use its auxiliary
    /// position; no scale/color deltas are initialized by btn_set_child.
    pub(super) fn apply_button_children(&mut self, sprites: &mut SpriteSystem) {
        for (&(group, index), &parent_index) in &self.button_children {
            let (Some(parent), Some(child)) = (
                self.game_buttons.get(&(group, parent_index)),
                self.game_buttons.get(&(group, index)),
            ) else {
                continue;
            };
            let child_handle = child.handle;
            let offset = child.slider_position;
            let (Some(parent), Some(child)) =
                (sprites.get(parent.handle), sprites.get(child_handle))
            else {
                continue;
            };
            let parent = parent.info();
            let scale = [parent.scale, parent.scale_y.unwrap_or(parent.scale)];
            let parent_size = [parent.cell_size.width, parent.cell_size.height];
            let child_size = [child.cell_size.width, child.cell_size.height];
            let parent_pos = [parent.position.x, parent.position.y];
            let base = std::array::from_fn::<_, 2, _>(|axis| {
                let local = offset[axis]
                    .wrapping_add(children::truncate_native(child.combined_lanes().position[axis]));
                let centers = ((parent_size[axis] >> 1) as i32)
                    .wrapping_sub((child_size[axis] >> 1) as i32)
                    .wrapping_sub(local);
                children::truncate_native(parent_pos[axis])
                    .wrapping_add(centers)
                    .wrapping_sub(children::truncate_native(centers as f32 * scale[axis]))
            });
            sprites.set_pos(
                child_handle,
                base[0].wrapping_add(offset[0]),
                base[1].wrapping_add(offset[1]),
                parent.position.z as i32,
            );
            sprites.set_scale_xy(child_handle, scale[0], scale[1]);
            sprites.set_rotate_ex(
                child_handle,
                parent.rotation.x,
                parent.rotation.y,
                parent.rotation.z,
            );
            sprites.set_native_projection(child_handle, parent.native_projection);
            self.game_buttons
                .get_mut(&(group, index))
                .unwrap()
                .base_position = base;
        }
    }
}
