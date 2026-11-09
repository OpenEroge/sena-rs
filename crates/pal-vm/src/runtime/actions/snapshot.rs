//! Portable action snapshots use stable scene bindings, never live handles.

use super::*;
use crate::sprite::{PalSize, PalSpriteInfo};
use std::io::{Error, ErrorKind, Result};

const BINDING_CAP: usize = SAVE_SPRITE_CAP * 2 + 2;

#[derive(Clone, Debug, Default)]
pub(in crate::runtime) struct SavedActions {
    time_ms: u32,
    state: ActionSubsystemState,
    bindings: Vec<SavedBinding>,
    random: Option<(Vec<i32>, usize, u32)>,
}

#[derive(Clone, Debug)]
pub(super) struct SavedBinding {
    pub(super) token: SpriteHandle,
    // 0: sprite slot, 1: button group/index, 2/3: ADV body/name.
    target: [i32; 3],
    base: PalSpriteInfo,
}

fn remap(
    state: &mut ActionSubsystemState,
    map: &BTreeMap<SpriteHandle, SpriteHandle>,
    discard: bool,
) {
    let mapped = |handle| map.get(&handle).copied().or((!discard).then_some(handle));
    for bank in state
        .banks
        .iter_mut()
        .chain(state.backup.iter_mut().map(|(_, bank)| bank.as_mut()))
    {
        for line in bank.lines.values_mut() {
            if let Some(current) = &mut line.current {
                current.handles = current.handles.iter().filter_map(|&h| mapped(h)).collect();
            }
        }
    }
    for lanes in [&mut state.final_lanes, &mut state.temporary] {
        *lanes = std::mem::take(lanes)
            .into_iter()
            .filter_map(|(h, v)| mapped(h).map(|h| (h, v)))
            .collect();
    }
    state.submitted = std::mem::take(&mut state.submitted)
        .into_iter()
        .filter_map(mapped)
        .collect();
}

impl ScriptRuntime {
    pub(in crate::runtime) fn capture_actions(&self, sprites: &SpriteSystem) -> SavedActions {
        let mut bindings = Vec::new();
        let mut map = BTreeMap::new();
        let targets = self
            .game_sprites
            .iter()
            .take(SAVE_SPRITE_CAP)
            .map(|(&slot, &h)| (h, [0, slot, 0]))
            .chain(
                self.game_buttons
                    .iter()
                    .take(SAVE_SPRITE_CAP)
                    .map(|(&(g, i), e)| (e.handle, [1, g, i])),
            )
            .chain(self.text_state.sprite.map(|h| (h, [2, 0, 0])))
            .chain(self.text_state.name_sprite.map(|h| (h, [3, 0, 0])));
        for (handle, target) in targets {
            if let Some(sprite) = sprites.get(handle) {
                let token = SpriteHandle(u32::MAX - bindings.len() as u32);
                let mut base = sprite.clone();
                base.action_lanes = SpriteActionLanes::default();
                bindings.push(SavedBinding {
                    token,
                    target,
                    base: base.info(),
                });
                map.insert(handle, token);
            }
        }
        // A second save can arrive before deferred ADV surfaces are rebuilt.
        // Preserve their stable bindings and their not-yet-remapped lanes.
        for pending in &self.action_state.pending_bindings {
            let token = if let Some(existing) = bindings.iter().find(|b| b.target == pending.target)
            {
                existing.token
            } else if bindings.len() < BINDING_CAP {
                let mut binding = pending.clone();
                binding.token = SpriteHandle(u32::MAX - bindings.len() as u32);
                let token = binding.token;
                bindings.push(binding);
                token
            } else {
                continue;
            };
            map.insert(pending.token, token);
        }
        let mut state = self.action_state.clone();
        remap(&mut state, &map, true);
        state.pending_bindings.clear();
        SavedActions {
            time_ms: self.pal_time_ms,
            state,
            bindings,
            random: Some(self.random_state.snapshot()),
        }
    }

    pub(in crate::runtime) fn restore_actions(
        &mut self,
        saved: &SavedActions,
        sprites: &mut SpriteSystem,
    ) {
        if let Some((seeds, cursor, lcg)) = &saved.random {
            self.random_state
                .restore_snapshot(seeds.clone(), *cursor, *lcg);
        }
        let elapsed = self.pal_time_ms.wrapping_sub(saved.time_ms);
        let mut state = saved.state.clone();
        for bank in state
            .banks
            .iter_mut()
            .chain(state.backup.iter_mut().map(|(_, bank)| bank.as_mut()))
        {
            bank.started_ms = bank.started_ms.wrapping_add(elapsed);
            for line in bank.lines.values_mut() {
                if let Some(current) = &mut line.current {
                    current.started_ms = current.started_ms.wrapping_add(elapsed);
                    current.period_start_ms = current.period_start_ms.wrapping_add(elapsed);
                }
            }
        }
        state.pending_bindings = saved.bindings.clone();
        self.action_state = state;
        self.action_audio_requests.clear();
        self.restore_pending_action_bindings(sprites);
        let temporary = self.action_state.temporary.clone();
        self.submit_action_lanes(sprites, temporary, true);
    }

    pub(super) fn restore_pending_action_bindings(&mut self, sprites: &mut SpriteSystem) {
        if self.action_state.pending_bindings.is_empty() {
            return;
        }
        let mut map = BTreeMap::new();
        for binding in std::mem::take(&mut self.action_state.pending_bindings) {
            let [kind, a, b] = binding.target;
            let handle = match kind {
                0 => self.game_sprites.get(&a).copied(),
                1 => self.game_buttons.get(&(a, b)).map(|e| e.handle),
                2 => self.text_state.sprite,
                3 => self.text_state.name_sprite,
                _ => None,
            };
            if let Some(sprite) = handle.and_then(|h| sprites.get_mut(h)) {
                map.insert(binding.token, sprite.handle);
                // Text textures are rebuilt by the font system on load.
                let mut base = binding.base;
                base.texture_size = sprite.texture_size;
                sprite.set_info(base);
                sprite.action_lanes = SpriteActionLanes::default();
            } else if kind >= 2 {
                self.action_state.pending_bindings.push(binding);
            }
        }
        remap(&mut self.action_state, &map, false);
    }
}

fn invalid() -> Error {
    Error::new(ErrorKind::InvalidData, "invalid action snapshot")
}
fn count(reader: &mut impl Read, cap: usize) -> Result<usize> {
    let n = read_u32_from(reader)? as usize;
    if n > cap {
        Err(invalid())
    } else {
        Ok(n)
    }
}
fn flag(reader: &mut impl Read) -> Result<bool> {
    match read_u32_from(reader)? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(invalid()),
    }
}
fn float(reader: &mut impl Read) -> Result<f32> {
    let value = f32::from_bits(read_u32_from(reader)?);
    if value.is_finite() {
        Ok(value)
    } else {
        Err(invalid())
    }
}

fn write_bank(out: &mut Vec<u8>, bank: &ActionBank) -> Result<()> {
    for v in [
        bank.mode as u32,
        bank.argument as u32,
        bank.started_ms,
        bank.cycle_complete as u32,
        bank.lines.len() as u32,
    ] {
        write_u32(out, v)?;
    }
    for (&id, line) in &bank.lines {
        for v in [
            id as u32,
            line.index as u32,
            line.deadline,
            line.sections.len() as u32,
        ] {
            write_u32(out, v)?;
        }
        for section in &line.sections {
            write_u32(out, section.kind as u32)?;
            write_u32(out, section.duration)?;
            write_i32_vec(out, &section.args)?;
        }
        write_u32(out, line.current.is_some() as u32)?;
        if let Some(current) = &line.current {
            for v in [
                current.started_ms,
                current.period_start_ms,
                current.move_delta.to_bits(),
                current.handles.len() as u32,
            ] {
                write_u32(out, v)?;
            }
            for h in &current.handles {
                write_u32(out, h.0)?;
            }
        }
    }
    Ok(())
}

fn read_bank(reader: &mut impl Read, limit: usize) -> Result<ActionBank> {
    let mode = count(reader, 3)? as u8;
    let argument = read_i32_from(reader)?;
    let started_ms = read_u32_from(reader)?;
    let cycle_complete = flag(reader)?;
    let mut lines = BTreeMap::new();
    for _ in 0..count(reader, limit)? {
        let id = count(reader, limit - 1)?;
        let index = count(reader, limit)?;
        let deadline = read_u32_from(reader)?;
        let mut sections = Vec::new();
        for _ in 0..count(reader, limit)? {
            let kind = count(reader, 15)? as u8;
            let duration = read_u32_from(reader)?;
            let args = read_i32_vec_capped(reader, 5)?;
            let arity = [6, 5, 4, 2, 6, 4, 5, 6, 6, 5, 6, 5, 7, 5, 7, 6][kind as usize];
            if duration == 0 || args.len() != arity - 2 {
                return Err(invalid());
            }
            sections.push(ActionSection {
                kind,
                duration,
                args,
            });
        }
        if index > sections.len() {
            return Err(invalid());
        }
        let current = if flag(reader)? {
            if index == sections.len() {
                return Err(invalid());
            }
            let started_ms = read_u32_from(reader)?;
            let period_start_ms = read_u32_from(reader)?;
            let move_delta = float(reader)?;
            let mut handles = Vec::new();
            for _ in 0..count(reader, 2)? {
                handles.push(SpriteHandle(read_u32_from(reader)?));
            }
            // Mask sections are not serialized; a restored one ends without
            // touching pixels.
            Some(RunningSection {
                handles,
                mask: None,
                started_ms,
                period_start_ms,
                move_delta,
            })
        } else {
            None
        };
        if lines
            .insert(
                id,
                ActionLine {
                    sections,
                    index,
                    deadline,
                    current,
                },
            )
            .is_some()
        {
            return Err(invalid());
        }
    }
    Ok(ActionBank {
        lines,
        mode,
        argument,
        started_ms,
        cycle_complete,
    })
}

fn write_lanes(out: &mut Vec<u8>, lanes: &BTreeMap<SpriteHandle, SpriteActionLanes>) -> Result<()> {
    write_u32(out, lanes.len() as u32)?;
    for (h, lane) in lanes {
        write_u32(out, h.0)?;
        for v in lane
            .position
            .into_iter()
            .chain(lane.shake)
            .chain(lane.rotation)
            .chain([lane.scale])
        {
            write_u32(out, v.to_bits())?;
        }
        write_i32(out, i32::from(lane.alpha))?;
    }
    Ok(())
}

fn read_lanes(reader: &mut impl Read) -> Result<BTreeMap<SpriteHandle, SpriteActionLanes>> {
    let mut lanes = BTreeMap::new();
    for _ in 0..count(reader, BINDING_CAP)? {
        let h = SpriteHandle(read_u32_from(reader)?);
        let mut lane = SpriteActionLanes::default();
        for v in lane
            .position
            .iter_mut()
            .chain(lane.shake.iter_mut())
            .chain(lane.rotation.iter_mut())
            .chain([&mut lane.scale])
        {
            *v = float(reader)?;
        }
        lane.alpha = read_i32_from(reader)?.try_into().map_err(|_| invalid())?;
        if lanes.insert(h, lane).is_some() {
            return Err(invalid());
        }
    }
    Ok(lanes)
}

pub(in crate::runtime) fn write(out: &mut Vec<u8>, saved: &SavedActions) -> Result<()> {
    let state = &saved.state;
    write_u32(out, saved.random.is_some() as u32)?;
    if let Some((seeds, cursor, lcg)) = &saved.random {
        write_i32_vec(out, seeds)?;
        write_u32(out, *cursor as u32)?;
        write_u32(out, *lcg)?;
    }
    for v in [
        saved.time_ms,
        state.extended as u32,
        state.active_id as u32,
        state.last_duration_ms,
        state.wait_bank.map_or(u32::MAX, |v| v as u32),
        state.flush_requested as u32,
    ] {
        write_u32(out, v)?;
    }
    for bank in &state.banks {
        write_bank(out, bank)?;
    }
    write_u32(out, state.backup.is_some() as u32)?;
    if let Some((id, bank)) = &state.backup {
        write_u32(out, *id as u32)?;
        write_bank(out, bank)?;
    }
    for flag in state.clear_flags {
        write_u32(out, flag as u32)?;
    }
    write_lanes(out, &state.final_lanes)?;
    write_lanes(out, &state.temporary)?;
    write_u32(out, saved.bindings.len() as u32)?;
    for binding in &saved.bindings {
        write_u32(out, binding.token.0)?;
        for v in binding.target {
            write_i32(out, v)?;
        }
        let b = &binding.base;
        for v in [
            b.position.x,
            b.position.y,
            b.position.z,
            b.rotation.x,
            b.rotation.y,
            b.rotation.z,
            b.scale,
        ] {
            write_u32(out, v.to_bits())?;
        }
        for v in [
            b.offset.x,
            b.offset.y,
            b.center_offset.x,
            b.center_offset.y,
            b.priority,
        ] {
            write_i32(out, v)?;
        }
        for v in [
            b.color.0,
            b.cell_size.width,
            b.cell_size.height,
            b.visible as u32,
            b.center_scale as u32,
            b.render_mode.raw(),
            b.extra,
            b.smooth_upscale as u32,
        ] {
            write_u32(out, v)?;
        }
        write_u32(out, b.native_projection.is_some() as u32)?;
        if let Some((x, y)) = b.native_projection {
            write_u32(out, x.to_bits())?;
            write_u32(out, y.to_bits())?;
        }
        let source_name: String = b.source_name.chars().take(1024).collect();
        write_bytes(out, source_name.as_bytes(), 4096)?;
        write_u32(out, u32::from(b.scale_y.is_some()))?;
        write_u32(out, b.scale_y.unwrap_or(0.0).to_bits())?;
    }
    Ok(())
}

pub(in crate::runtime) fn read(reader: &mut impl Read, version: u32) -> Result<SavedActions> {
    let random = if flag(reader)? {
        let seeds = read_i32_vec_capped(reader, 4096)?;
        if seeds.is_empty() {
            return Err(invalid());
        }
        let cursor = count(reader, seeds.len() - 1)?;
        Some((seeds, cursor, read_u32_from(reader)?))
    } else {
        None
    };
    let time_ms = read_u32_from(reader)?;
    let mut state = ActionSubsystemState {
        extended: flag(reader)?,
        ..Default::default()
    };
    state.active_id = count(reader, state.bank_count() - 1)? as i32;
    state.last_duration_ms = read_u32_from(reader)?;
    state.wait_bank = match read_u32_from(reader)? {
        u32::MAX => None,
        n if (n as usize) < state.bank_count() => Some(n as usize),
        _ => return Err(invalid()),
    };
    state.flush_requested = flag(reader)?;
    let limit = state.line_limit();
    for bank in &mut state.banks {
        *bank = read_bank(reader, limit)?;
    }
    if flag(reader)? {
        state.backup = Some((
            count(reader, state.bank_count() - 1)?,
            Box::new(read_bank(reader, limit)?),
        ));
    }
    for v in &mut state.clear_flags {
        *v = flag(reader)?;
    }
    state.final_lanes = read_lanes(reader)?;
    state.temporary = read_lanes(reader)?;
    let mut bindings = Vec::new();
    for _ in 0..count(reader, BINDING_CAP)? {
        let token = SpriteHandle(read_u32_from(reader)?);
        if token.0 != u32::MAX - bindings.len() as u32 {
            return Err(invalid());
        }
        let target = [
            count(reader, 3)? as i32,
            read_i32_from(reader)?,
            read_i32_from(reader)?,
        ];
        let position = PalVec3::from_f32(float(reader)?, float(reader)?, float(reader)?);
        let rotation = PalVec3::from_f32(float(reader)?, float(reader)?, float(reader)?);
        let scale = float(reader)?;
        let offset = PalPoint2 {
            x: read_i32_from(reader)?,
            y: read_i32_from(reader)?,
        };
        let center_offset = PalPoint2 {
            x: read_i32_from(reader)?,
            y: read_i32_from(reader)?,
        };
        let priority = read_i32_from(reader)?;
        let color = PalColor(read_u32_from(reader)?);
        let cell_size = PalSize::new(read_u32_from(reader)?, read_u32_from(reader)?);
        let visible = flag(reader)?;
        let center_scale = flag(reader)?;
        let render_mode = PalRenderMode::new(read_u32_from(reader)?);
        let extra = read_u32_from(reader)?;
        let smooth_upscale = flag(reader)?;
        let native_projection = if flag(reader)? {
            let (x, y) = (float(reader)?, float(reader)?);
            if x <= 0.0 || y <= 0.0 {
                return Err(invalid());
            }
            Some((x, y))
        } else {
            None
        };
        let source_name =
            String::from_utf8(read_bytes_capped(reader, 4096)?).map_err(|_| invalid())?;
        let scale_y = if version >= 12 {
            let present = flag(reader)?;
            let value = f32::from_bits(read_u32_from(reader)?);
            present.then_some(value)
        } else {
            None
        };
        bindings.push(SavedBinding {
            token,
            target,
            base: PalSpriteInfo {
                visible,
                color,
                position,
                offset,
                texture_size: cell_size,
                cell_size,
                center_offset,
                priority,
                scale,
                scale_y,
                center_scale,
                rotation,
                render_mode,
                extra,
                source_name,
                native_projection,
                smooth_upscale,
            },
        });
    }
    let valid = |h: &SpriteHandle| bindings.iter().any(|b| b.token == *h);
    if state
        .final_lanes
        .keys()
        .chain(state.temporary.keys())
        .any(|h| !valid(h))
    {
        return Err(invalid());
    }
    for bank in state
        .banks
        .iter()
        .chain(state.backup.iter().map(|(_, b)| b.as_ref()))
    {
        if bank
            .lines
            .values()
            .filter_map(|l| l.current.as_ref())
            .flat_map(|r| &r.handles)
            .any(|h| !valid(h))
        {
            return Err(invalid());
        }
    }
    Ok(SavedActions {
        time_ms,
        state,
        bindings,
        random,
    })
}
