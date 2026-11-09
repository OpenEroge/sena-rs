//! Game action banks: parallel lines, sequential sections, and native curves.

use super::*;
use crate::sprite::SpriteActionLanes;

pub(super) mod snapshot;
pub(super) use snapshot::SavedActions;

#[derive(Clone, Debug, Default)]
pub(super) struct ActionSubsystemState {
    pub extended: bool,
    pub active_id: i32,
    pub last_duration_ms: u32,
    banks: [ActionBank; 16],
    backup: Option<(usize, Box<ActionBank>)>,
    clear_flags: [bool; 16],
    wait_bank: Option<usize>,
    final_lanes: BTreeMap<SpriteHandle, SpriteActionLanes>,
    submitted: BTreeSet<SpriteHandle>,
    temporary: BTreeMap<SpriteHandle, SpriteActionLanes>,
    pub flush_requested: bool,
    pending_bindings: Vec<snapshot::SavedBinding>,
}

#[derive(Clone, Debug, Default)]
struct ActionBank {
    lines: BTreeMap<usize, ActionLine>,
    mode: u8,
    argument: i32,
    started_ms: u32,
    cycle_complete: bool,
}

#[derive(Clone, Debug, Default)]
struct ActionLine {
    sections: Vec<ActionSection>,
    index: usize,
    deadline: u32,
    current: Option<RunningSection>,
}

#[derive(Clone, Debug)]
struct ActionSection {
    kind: u8,
    args: Vec<i32>,
    duration: u32,
}

#[derive(Clone, Debug)]
struct RunningSection {
    handles: Vec<SpriteHandle>,
    /// Type 20 sprites: from, mask, to.
    mask: Option<[SpriteHandle; 3]>,
    started_ms: u32,
    period_start_ms: u32,
    move_delta: f32,
}

impl ActionSubsystemState {
    fn bank_count(&self) -> usize {
        if self.extended {
            16
        } else {
            8
        }
    }
    fn line_limit(&self) -> usize {
        if self.extended {
            64
        } else {
            32
        }
    }
    fn resolve(&self, id: i32) -> Option<usize> {
        let id = if id == -1 { self.active_id } else { id };
        (id >= 0 && (id as usize) < self.bank_count()).then_some(id as usize)
    }
}

fn entry_kind(index: u16) -> Option<(u8, usize)> {
    Some(match index {
        5 => (0, 6),
        6 => (6, 5),
        7 => (1, 5),
        8 => (2, 4),
        9 => (3, 2),
        10 => (4, 6),
        11 => (5, 4),
        14 => (7, 6),
        15 => (8, 6),
        16 => (9, 5),
        17 => (10, 6),
        18 => (11, 5),
        19 => (12, 7),
        20 => (13, 5),
        21 => (14, 7),
        29 => (15, 6),
        // trenka.exe 0x40A230 (source-rect tween) and 0x409170 (mask reveal).
        31 => (16, 8),
        35 => (20, 5),
        _ => return None,
    })
}

impl ScriptRuntime {
    pub(super) fn dispatch_action(
        &mut self,
        index: u16,
        mut sprites: Option<&mut SpriteSystem>,
    ) -> ExtCallOutcome {
        if let Some((kind, arity)) = entry_kind(index) {
            // Native checks the line and capacity before consuming the tail.
            let Some(line_id) = self.pop_ext_args(1).first().copied() else {
                return ExtCallOutcome::Block;
            };
            let Some(bank_id) = self.action_state.resolve(-1) else {
                return ExtCallOutcome::Value(1);
            };
            let limit = self.action_state.line_limit();
            if line_id < 0 || line_id as usize >= limit {
                return ExtCallOutcome::Value(1);
            }
            let line = self.action_state.banks[bank_id]
                .lines
                .entry(line_id as usize)
                .or_default();
            if line.sections.len() >= limit {
                return ExtCallOutcome::Value(1);
            }
            let mut args = self.pop_ext_args(arity - 1);
            if args.len() != arity - 1 {
                return ExtCallOutcome::Block;
            }
            let duration = args.pop().unwrap() as u32;
            if kind == 15 && self.action_state.extended {
                for value in &mut args[1..4] {
                    *value = (*value as f32 * 1.5) as i32;
                }
            }
            self.action_state.banks[bank_id]
                .lines
                .get_mut(&(line_id as usize))
                .unwrap()
                .sections
                .push(ActionSection {
                    kind,
                    args,
                    duration: if duration == 0 { 1 } else { duration },
                });
            return ExtCallOutcome::Value(1);
        }
        let mut state = std::mem::take(&mut self.action_state);
        self.action_state.extended = state.extended;
        let outcome = match index {
            0..=2 | 25..=27 => {
                let all = index >= 25;
                let args = self.pop_ext_args(if all { 1 } else { 2 });
                if args.len() != if all { 1 } else { 2 } {
                    ExtCallOutcome::Block
                } else {
                    let argument = args[if all { 0 } else { 1 }];
                    let mode = if all { index - 24 } else { index + 1 } as u8;
                    let ids: Vec<_> = if all {
                        (0..state.bank_count()).collect()
                    } else {
                        state.resolve(args[0]).into_iter().collect()
                    };
                    let skip = self.dispatch_skip_state();
                    let mut started = false;
                    for id in ids {
                        let bank = &mut state.banks[id];
                        if bank.mode != 0 {
                            continue;
                        }
                        bank.mode = mode;
                        bank.argument = argument;
                        bank.started_ms = self.pal_time_ms;
                        bank.cycle_complete = false;
                        started = true;
                        let skip_mask = if state.extended { 0x30 } else { 0x10 };
                        if !self.effect_system.enabled()
                            || (mode == 1 && skip & skip_mask != 0)
                            || (mode == 2 && skip != 0)
                        {
                            self.finish_action_bank(
                                bank,
                                &mut state.final_lanes,
                                sprites.as_deref_mut(),
                            );
                            state.temporary.clear();
                        }
                    }
                    state.last_duration_ms = argument.max(0) as u32;
                    if mode == 2 && started && skip == 0 && self.effect_system.enabled() {
                        ExtCallOutcome::Wait {
                            value: 1,
                            request: WaitRequest::Time(argument.max(1) as u32),
                        }
                    } else {
                        ExtCallOutcome::Value(1)
                    }
                }
            }
            3 | 28 => {
                let ids: Vec<_> = if index == 28 {
                    (0..state.bank_count()).collect()
                } else {
                    let args = self.pop_ext_args(1);
                    args.first()
                        .and_then(|id| state.resolve(*id))
                        .into_iter()
                        .collect()
                };
                for id in ids {
                    self.finish_action_bank(
                        &mut state.banks[id],
                        &mut state.final_lanes,
                        sprites.as_deref_mut(),
                    );
                    state.temporary.clear();
                }
                ExtCallOutcome::Value(1)
            }
            4 => {
                let id = if let Some(id) = state.wait_bank {
                    Some(id)
                } else {
                    self.pop_ext_args(1)
                        .first()
                        .and_then(|id| state.resolve(*id))
                };
                if let Some(id) = id {
                    let busy = state.banks[..=id]
                        .iter()
                        .any(|bank| bank.mode != 0 && !bank.cycle_complete);
                    if busy && self.dispatch_skip_state() == 0 {
                        state.wait_bank = Some(id);
                        self.pc = self.pc.saturating_sub(self.extcall_instruction_size);
                    } else {
                        state.wait_bank = None;
                    }
                    ExtCallOutcome::Wait {
                        value: 1,
                        request: WaitRequest::Frame(1),
                    }
                } else {
                    ExtCallOutcome::Value(1)
                }
            }
            12 => {
                if let Some(id) = state.resolve(-1).filter(|id| state.banks[*id].mode != 0) {
                    if let Some((old_id, old)) = state.backup.take() {
                        self.finish_action_bank(
                            &mut state.banks[old_id],
                            &mut state.final_lanes,
                            sprites.as_deref_mut(),
                        );
                        state.temporary.clear();
                        state.banks[old_id] = *old;
                        self.finish_action_bank(
                            &mut state.banks[old_id],
                            &mut state.final_lanes,
                            sprites.as_deref_mut(),
                        );
                        state.temporary.clear();
                    }
                    state.backup = Some((id, Box::new(std::mem::take(&mut state.banks[id]))));
                }
                ExtCallOutcome::Value(1)
            }
            13 => {
                if let Some((id, bank)) = state.backup.take() {
                    self.finish_action_bank(
                        &mut state.banks[id],
                        &mut state.final_lanes,
                        sprites.as_deref_mut(),
                    );
                    state.temporary.clear();
                    state.banks[id] = *bank;
                }
                ExtCallOutcome::Value(1)
            }
            23 => {
                let old = state.active_id;
                if let Some(id) = self
                    .pop_ext_args(1)
                    .first()
                    .copied()
                    .filter(|id| *id >= 0 && (*id as usize) < state.bank_count())
                {
                    state.active_id = id;
                }
                ExtCallOutcome::Value(old)
            }
            24 => ExtCallOutcome::Value(state.active_id),
            30 => {
                if let Some(id) = self
                    .pop_ext_args(1)
                    .first()
                    .and_then(|id| state.resolve(*id))
                {
                    state.clear_flags[id] = true;
                }
                ExtCallOutcome::Value(1)
            }
            _ => ExtCallOutcome::Skip,
        };
        self.action_state = state;
        if let Some(sprites) = sprites {
            let temporary = self.action_state.temporary.clone();
            self.submit_action_lanes(sprites, temporary, true);
        }
        outcome
    }

    /// Called once per engine frame; a line advances at most one section.
    pub(crate) fn advance_actions(&mut self, sprites: &mut SpriteSystem) {
        self.restore_pending_action_bindings(sprites);
        self.flush_deferred_action_clears(sprites);
        let mut state = std::mem::take(&mut self.action_state);
        self.action_state.extended = state.extended;
        let now = self.pal_time_ms;
        let mut temporary = BTreeMap::<SpriteHandle, SpriteActionLanes>::new();
        for bank in &mut state.banks {
            if bank.mode == 0 {
                continue;
            }
            bank.cycle_complete = false;
            let elapsed = now.wrapping_sub(bank.started_ms);
            if bank.mode == 2
                && (self.dispatch_skip_state() != 0 || elapsed >= bank.argument as u32)
            {
                self.finish_action_bank(bank, &mut state.final_lanes, Some(sprites));
                temporary.clear();
                continue;
            }
            for line in bank.lines.values_mut() {
                let Some(section) = line.sections.get(line.index) else {
                    continue;
                };
                if let Some(running) = line.current.as_mut() {
                    if elapsed >= line.deadline {
                        self.finish_action_section(
                            section,
                            running,
                            &mut state.final_lanes,
                            sprites,
                        );
                        line.current = None;
                        line.index += 1;
                    } else {
                        self.evaluate_action_section(section, running, &mut temporary, sprites);
                        continue;
                    }
                }
                if let Some(section) = line.sections.get(line.index) {
                    let running = self.start_action_section(section, sprites, &state.final_lanes);
                    if !matches!(section.kind, 11 | 12) && !(section.kind == 9 && !state.extended) {
                        line.deadline = line.deadline.wrapping_add(section.duration);
                        line.current = Some(running);
                    }
                }
            }
            if bank.mode == 1 && elapsed >= bank.argument as u32 {
                self.finish_action_bank(bank, &mut state.final_lanes, Some(sprites));
                temporary.clear();
            } else if bank.mode == 3
                && bank
                    .lines
                    .values()
                    .all(|line| line.index == line.sections.len())
            {
                bank.started_ms = now;
                for line in bank.lines.values_mut() {
                    line.index = 0;
                    line.deadline = 0;
                    line.current = None;
                }
                if bank.argument == -1 {
                    bank.cycle_complete = true;
                } else {
                    let old = bank.argument;
                    bank.argument = bank.argument.wrapping_sub(1);
                    if old == 1 {
                        self.finish_action_bank(bank, &mut state.final_lanes, Some(sprites));
                    }
                }
            }
        }
        self.action_state = state;
        self.submit_action_lanes(sprites, temporary, true);
    }

    fn action_targets(&self, slot: i32) -> Vec<SpriteHandle> {
        if slot == 255 || slot == -255 {
            return [self.text_state.sprite, self.text_state.name_sprite]
                .into_iter()
                .flatten()
                .collect();
        }
        if let Some(handle) = self.packed_button_sprite_handle(slot) {
            return vec![handle];
        }
        decode_pal_sprite_slot(slot)
            .and_then(|(slot, _)| self.game_sprites.get(&slot).copied())
            .into_iter()
            .collect()
    }

    fn start_action_section(
        &mut self,
        section: &ActionSection,
        sprites: &mut SpriteSystem,
        finals: &BTreeMap<SpriteHandle, SpriteActionLanes>,
    ) -> RunningSection {
        let mut running = RunningSection {
            handles: self.action_targets(section.args.first().copied().unwrap_or(-1)),
            mask: None,
            started_ms: self.pal_time_ms,
            period_start_ms: self.pal_time_ms,
            move_delta: 0.0,
        };
        match section.kind {
            3 => running.handles.clear(),
            20 => {
                running.handles.clear();
                let target = |slot: i32| self.action_targets(slot).first().copied();
                running.mask = match (target(section.args[0]), target(section.args[1]), target(section.args[2])) {
                    (Some(from), Some(mask), Some(to)) => Some([from, mask, to]),
                    _ => {
                        log::debug!("[trace-action] mask section is missing an image {:?}", section.args);
                        None
                    }
                };
            }
            9 => {
                running.handles.clear();
                self.action_audio_requests.push((
                    section.args[0],
                    section.args[1] != 0,
                    if self.action_state.extended {
                        if section.args[2] != 0 {
                            section.duration as i32
                        } else {
                            0
                        }
                    } else {
                        section.args[2]
                    },
                ));
            }
            6 => {
                if let Some(sprite) = running
                    .handles
                    .first()
                    .and_then(|handle| sprites.get(*handle))
                {
                    let pos = section.args[1];
                    let width = sprite.cell_size.width as i32;
                    let native_width = if self.action_state.extended {
                        1920
                    } else {
                        1280
                    };
                    let target = if pos < 0 {
                        -width
                    } else if pos > 20 {
                        native_width
                    } else {
                        pos * (native_width / 20) - width / 2
                    };
                    let factor = sprite.native_projection.map(|p| p.0).unwrap_or(1.0)
                        * native_width as f32
                        / self.logical_size().0 as f32;
                    running.move_delta = target as f32
                        - sprite.position.x * factor
                        - finals
                            .get(&sprite.handle)
                            .map_or(0.0, |lanes| lanes.position[0]);
                }
            }
            11 | 12 => {
                for handle in &running.handles {
                    if let Some(sprite) = sprites.get_mut(*handle) {
                        let x = section.args[1];
                        let y = section.args[2];
                        if section.kind == 11 {
                            sprite.rect_set_pos(x, y);
                        } else {
                            sprite.source_rect = PalRect::new(
                                x,
                                y,
                                x.saturating_add(section.args[3]),
                                y.saturating_add(section.args[4]),
                            );
                        }
                    }
                }
            }
            _ => {}
        }
        running
    }

    fn finish_action_bank(
        &mut self,
        bank: &mut ActionBank,
        finals: &mut BTreeMap<SpriteHandle, SpriteActionLanes>,
        mut sprites: Option<&mut SpriteSystem>,
    ) {
        if let Some(sprites) = sprites.as_deref_mut() {
            for line in bank.lines.values_mut() {
                for index in line.index..line.sections.len() {
                    let section = &line.sections[index];
                    if matches!(section.kind, 1 | 3 | 9 | 10 | 11 | 12) {
                        continue;
                    }
                    let running = if index == line.index {
                        line.current.take()
                    } else {
                        None
                    }
                    .unwrap_or_else(|| self.start_action_section(section, sprites, finals));
                    self.finish_action_section(section, &running, finals, sprites);
                }
            }
            // Koikake 0x4353A0 folds the completed lanes into each wrapper's
            // base properties and clears them globally when a bank ends.
            for (handle, delta) in std::mem::take(finals) {
                if let Some(sprite) = sprites.get_mut(handle) {
                    let mut delta = self.project_action_lanes(delta, sprite.native_projection);
                    // Both native commit helpers omit the z lane before
                    // clearing the final block, despite accumulating it.
                    delta.position[2] = 0.0;
                    sprite.commit_action_lanes(delta);
                }
            }
        }
        *bank = ActionBank::default();
    }

    fn finish_action_section(
        &self,
        section: &ActionSection,
        running: &RunningSection,
        lanes: &mut BTreeMap<SpriteHandle, SpriteActionLanes>,
        sprites: &mut SpriteSystem,
    ) {
        let mut delta = SpriteActionLanes::default();
        let args = &section.args;
        match section.kind {
            16 => delta.rect = [args[2], args[3], args[4], args[5]],
            20 => {
                if let Some([from, mask, to]) = running.mask {
                    sprites.apply_action_mask(from, mask, to, None);
                }
                return;
            }
            0 | 7 | 8 | 15 => delta.position = [args[1] as f32, args[2] as f32, args[3] as f32],
            2 => delta.alpha = args[1] as i16,
            4 => delta.rotation = [args[1] as f32, args[2] as f32, args[3] as f32],
            5 => delta.scale = args[1] as f32 / 100.0,
            6 => delta.position[0] = running.move_delta,
            13 => delta.scale = args[2] as f32 / 100.0,
            14 => delta.rotation = [args[2] as f32, args[3] as f32, args[4] as f32],
            _ => return,
        }
        for handle in &running.handles {
            if sprites.get(*handle).is_some() {
                lanes.entry(*handle).or_default().add(delta);
            }
        }
    }

    fn evaluate_action_section(
        &mut self,
        section: &ActionSection,
        running: &mut RunningSection,
        lanes: &mut BTreeMap<SpriteHandle, SpriteActionLanes>,
        sprites: &mut SpriteSystem,
    ) {
        let elapsed = self
            .pal_time_ms
            .wrapping_sub(running.started_ms)
            .min(section.duration);
        let t = elapsed as f32 / section.duration as f32;
        let a = &section.args;
        let mut delta = SpriteActionLanes::default();
        match section.kind {
            0 | 7 | 8 | 15 => {
                let mode = match section.kind {
                    7 => 2,
                    8 => 1,
                    15 => 3,
                    _ => 0,
                };
                let ratio = curve(mode, elapsed, section.duration);
                delta.position = [a[1], a[2], a[3]].map(|value| biased_delta(value, ratio));
            }
            1 => {
                let span = ((1.0 - t) * a[2] as f32) as u32;
                let modulus = span.wrapping_add(1).max(1);
                let mut sample = || {
                    (self.random_state.random_ex() as u32 % modulus) as i32 - (modulus >> 1) as i32
                };
                if matches!(a[1], 1 | 2) {
                    delta.position[0] = sample() as f32;
                }
                if matches!(a[1], 0 | 2) {
                    delta.position[1] = sample() as f32;
                }
            }
            2 => delta.alpha = (a[1] as i16 as f32 * t) as i16,
            4 => delta.rotation = [a[1], a[2], a[3]].map(|value| value as f32 * t),
            5 => delta.scale = a[1] as f32 * t / 100.0,
            6 => {
                delta.position[0] = biased_delta(
                    running.move_delta as i32,
                    curve(a[2], elapsed, section.duration),
                )
            }
            10 => {
                let phase =
                    self.pal_time_ms.wrapping_sub(running.period_start_ms) as f32 / a[3] as f32;
                let phase = if phase > 1.0 {
                    running.period_start_ms = self.pal_time_ms;
                    1.0
                } else {
                    phase
                };
                let degrees = (phase * 360.0) as i32;
                let angle = degrees as f32 * f32::from_bits(0x40490fd8) / 180.0;
                let amplitude = if a[1] >= 2 {
                    ((1.0 - t) * a[2] as f32) as i32
                } else {
                    a[2]
                };
                let displacement = ((angle as f64).sin() as f32 * amplitude as f32) as i32 as f32;
                match a[1] {
                    0 | 2 => delta.position[1] = displacement,
                    1 | 3 => delta.position[0] = displacement,
                    _ => {}
                }
            }
            13 => delta.scale = a[2] as f32 * curve(a[1], elapsed, section.duration) / 100.0,
            14 => {
                let ratio = curve(a[1], elapsed, section.duration);
                delta.rotation = [a[2], a[3], a[4]]
                    .map(|value| (value as f32 + if value < 0 { -0.5 } else { 0.5 }) * ratio);
            }
            16 => {
                let ratio = curve(a[1], elapsed, section.duration);
                delta.rect = [a[2], a[3], a[4], a[5]].map(|value| biased_delta(value, ratio) as i32);
            }
            20 => {
                if let Some([from, mask, to]) = running.mask {
                    sprites.apply_action_mask(from, mask, to, Some((t * 255.0) as i32 as u8));
                }
                return;
            }
            _ => return,
        }
        for handle in &running.handles {
            let lane = lanes.entry(*handle).or_default();
            if matches!(section.kind, 1 | 10) {
                // Shake writes one separate x/y pair. Later shake callbacks
                // replace it without erasing the accumulated movement lanes.
                lane.shake = [delta.position[0], delta.position[1]];
            } else {
                lane.add(delta);
            }
        }
    }

    fn submit_action_lanes(
        &mut self,
        sprites: &mut SpriteSystem,
        temporary: BTreeMap<SpriteHandle, SpriteActionLanes>,
        clear_previous: bool,
    ) {
        if clear_previous {
            for handle in std::mem::take(&mut self.action_state.submitted) {
                if let Some(sprite) = sprites.get_mut(handle) {
                    sprite.action_lanes = SpriteActionLanes::default();
                }
            }
        }
        if clear_previous {
            self.action_state.temporary = temporary;
        }
        self.action_state.final_lanes.retain(|handle, _| {
            sprites.get(*handle).is_some()
                || self
                    .action_state
                    .pending_bindings
                    .iter()
                    .any(|b| b.token == *handle)
        });
        let mut combined = self.action_state.final_lanes.clone();
        for (&handle, &delta) in &self.action_state.temporary {
            combined.entry(handle).or_default().add(delta);
        }
        for (handle, delta) in combined {
            if let Some(sprite) = sprites.get_mut(handle) {
                sprite.action_lanes = self.project_action_lanes(delta, sprite.native_projection);
                self.action_state.submitted.insert(handle);
            }
        }
    }

    fn project_action_lanes(
        &self,
        mut delta: SpriteActionLanes,
        projection: Option<(f32, f32)>,
    ) -> SpriteActionLanes {
        let (w, h) = self.logical_size();
        let (nw, nh) = if self.action_state.extended {
            (1920.0, 1080.0)
        } else {
            (1280.0, 720.0)
        };
        let (px, py) = projection.unwrap_or((1.0, 1.0));
        delta.position[0] *= w as f32 / nw / px;
        delta.position[1] *= h as f32 / nh / py;
        delta.shake[0] *= w as f32 / nw / px;
        delta.shake[1] *= h as f32 / nh / py;
        delta
    }

    pub(crate) fn play_action_sounds(&mut self, audio: &mut AudioSystem) {
        for (slot, looping, fade_ms) in std::mem::take(&mut self.action_audio_requests) {
            if let Some(handle) = self.game_audio.get(&(5, slot)).copied() {
                if let Err(error) = audio.play_fade(handle, looping, fade_ms) {
                    log::warn!("action sound slot {slot}: {error}");
                }
            }
        }
    }

    /// Native clears marked banks when the ADV click-wait stage completes.
    pub(crate) fn flush_deferred_action_clears(&mut self, sprites: &mut SpriteSystem) {
        if !self.action_state.flush_requested {
            return;
        }
        let mut state = std::mem::take(&mut self.action_state);
        self.action_state.extended = state.extended;
        state.flush_requested = false;
        for id in 0..state.bank_count() {
            if state.clear_flags[id] {
                if state.banks[id].mode != 0 {
                    self.finish_action_bank(
                        &mut state.banks[id],
                        &mut state.final_lanes,
                        Some(sprites),
                    );
                } else {
                    state.banks[id] = ActionBank::default();
                }
                state.clear_flags[id] = false;
            }
        }
        state.temporary.clear();
        self.action_state = state;
        self.submit_action_lanes(sprites, BTreeMap::new(), true);
    }
}

fn biased_delta(value: i32, t: f32) -> f32 {
    ((value as f32 + if value < 0 { -0.5 } else { 0.5 }) * t) as i32 as f32
}

fn curve(mode: i32, elapsed: u32, duration: u32) -> f32 {
    let t = elapsed.min(duration) as f32 / duration.max(1) as f32;
    match mode {
        1 => {
            let remaining = duration - elapsed.saturating_add(1).min(duration);
            let ratio = remaining as f32 / duration.max(1) as f32;
            (((ratio as f64).log10() as f32 * 55.0) * -0.01).min(1.0)
        }
        2 => {
            let ratio = elapsed.saturating_add(1).min(duration) as f32 / duration.max(1) as f32;
            1.0 - (((ratio as f64).log10() as f32 * 80.0) * -0.01).min(1.0)
        }
        3 => (((t * 128.0) * 0.00390625 * std::f32::consts::PI) as f64).sin() as f32,
        _ => t,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(
        vm: &mut ScriptRuntime,
        sprites: &mut SpriteSystem,
        index: u16,
        args: &[i32],
    ) -> ExtCallOutcome {
        vm.stack.extend(args.iter().rev());
        vm.dispatch_action(index, Some(sprites))
    }

    fn fixture() -> (ScriptRuntime, SpriteSystem, SpriteHandle) {
        let mut vm = ScriptRuntime::boot(12, ScriptRuntimeConfig::default());
        let mut sprites = SpriteSystem::new();
        let handle = sprites
            .create_rgba_sprite(1, 1, vec![255; 4], PalVec3::default(), 0, "action fixture")
            .unwrap();
        sprites.set_native_projection(handle, Some((1.0, 1.0)));
        vm.game_sprites.insert(0, handle);
        (vm, sprites, handle)
    }

    #[test]
    fn entries_are_deferred_lines_are_parallel_and_sections_are_sequential() {
        let (mut vm, mut sprites, handle) = fixture();
        call(&mut vm, &mut sprites, 5, &[0, 0, 100, 0, 0, 100]);
        call(&mut vm, &mut sprites, 5, &[0, 0, 200, 0, 0, 100]);
        call(&mut vm, &mut sprites, 8, &[1, 0, -100, 200]);
        vm.set_pal_time(500);
        vm.advance_actions(&mut sprites);
        assert_eq!(sprites.get(handle).unwrap().info().position.x, 0.0);
        assert_eq!(
            vm.action_state.active_id, 0,
            "entry line ids do not change the active bank"
        );
        call(&mut vm, &mut sprites, 0, &[0, 500]);
        vm.advance_actions(&mut sprites);
        vm.set_pal_time(550);
        vm.advance_actions(&mut sprites);
        assert_eq!(sprites.get(handle).unwrap().info().position.x, 50.0);
        assert_eq!(sprites.get(handle).unwrap().info().color.alpha(), 230);
        vm.set_pal_time(600);
        vm.advance_actions(&mut sprites);
        assert_eq!(sprites.get(handle).unwrap().info().position.x, 100.0);
        vm.set_pal_time(650);
        vm.advance_actions(&mut sprites);
        assert_eq!(sprites.get(handle).unwrap().info().position.x, 200.0);
        vm.set_pal_time(700);
        vm.advance_actions(&mut sprites);
        assert_eq!(sprites.get(handle).unwrap().info().position.x, 300.0);
        assert_eq!(sprites.get(handle).unwrap().info().color.alpha(), 155);
    }

    fn alphas(sprites: &SpriteSystem, handle: SpriteHandle) -> Vec<u8> {
        let surface = sprites.surface(sprites.get(handle).unwrap().surface).unwrap();
        surface.to_scene_texture().pixels.chunks(4).map(|pixel| pixel[3]).collect()
    }

    #[test]
    fn mask_section_reveals_target_alpha_then_copies_the_source() {
        let mut vm = ScriptRuntime::boot(12, ScriptRuntimeConfig::default());
        let mut sprites = SpriteSystem::new();
        let pixel = |alpha: u8| [10, 20, 30, alpha];
        let from = sprites
            .create_rgba_sprite(3, 1, [pixel(200); 3].concat(), PalVec3::default(), 0, "from")
            .unwrap();
        // Blue 255/128/0 gives mask levels 0/127/255.
        let mask_pixels = vec![0, 0, 255, 255, 0, 0, 128, 255, 0, 0, 0, 255];
        let mask = sprites.create_rgba_sprite(3, 1, mask_pixels, PalVec3::default(), 0, "mask").unwrap();
        let to = sprites
            .create_rgba_sprite(3, 1, [pixel(50); 3].concat(), PalVec3::default(), 0, "to")
            .unwrap();
        for (slot, handle) in [(1, from), (2, mask), (3, to)] {
            vm.game_sprites.insert(slot, handle);
        }
        call(&mut vm, &mut sprites, 35, &[0, 1, 2, 3, 100]);
        call(&mut vm, &mut sprites, 0, &[0, 1000]);
        vm.advance_actions(&mut sprites);
        vm.set_pal_time(50);
        vm.advance_actions(&mut sprites);
        // p = 127, ramp end 158: below, inside ((158 - 127) * 200) >> 5, above.
        assert_eq!(alphas(&sprites, to), [200, 193, 0]);
        vm.set_pal_time(100);
        vm.advance_actions(&mut sprites);
        assert_eq!(alphas(&sprites, to), [200, 200, 200]);
    }

    #[test]
    fn rect_section_tweens_the_source_rect_and_commits_it() {
        let (mut vm, mut sprites, handle) = fixture();
        call(&mut vm, &mut sprites, 31, &[0, 0, 0, 0, 0, 10, 20, 100]);
        call(&mut vm, &mut sprites, 0, &[0, 1000]);
        vm.advance_actions(&mut sprites);
        vm.set_pal_time(50);
        vm.advance_actions(&mut sprites);
        let drawn = match sprites.get(handle).unwrap().draw_command(&sprites) {
            Some(crate::scene::DrawCommand::Sprite(draw)) => draw.source_rect,
            other => panic!("unexpected draw {other:?}"),
        };
        assert_eq!((drawn[2], drawn[3]), (1 + 5, 1 + 10));
        vm.set_pal_time(100);
        vm.advance_actions(&mut sprites);
        call(&mut vm, &mut sprites, 3, &[0]);
        let rect = sprites.get(handle).unwrap().source_rect;
        assert_eq!((rect.right, rect.bottom), (1 + 10, 1 + 20));
    }

    #[test]
    fn clear_applies_unstarted_final_values_and_only_clears_requested_bank() {
        let (mut vm, mut sprites, handle) = fixture();
        call(&mut vm, &mut sprites, 5, &[0, 0, 40, 0, 0, 100]);
        call(&mut vm, &mut sprites, 5, &[0, 0, 60, 0, 0, 100]);
        assert!(matches!(
            call(&mut vm, &mut sprites, 23, &[1]),
            ExtCallOutcome::Value(0)
        ));
        call(&mut vm, &mut sprites, 5, &[0, 0, 500, 0, 0, 100]);
        call(&mut vm, &mut sprites, 3, &[0]);
        assert_eq!(sprites.get(handle).unwrap().info().position.x, 100.0);
        assert_eq!(vm.action_state.banks[1].lines[&0].sections.len(), 1);
        assert!(matches!(
            call(&mut vm, &mut sprites, 24, &[]),
            ExtCallOutcome::Value(1)
        ));
    }

    #[test]
    fn corrected_entry_arities_and_push_pop_preserve_stack_sentinel() {
        let (mut vm, mut sprites, _) = fixture();
        vm.stack.push(0x123456);
        for (index, argc) in [(6, 5), (7, 5), (17, 6), (18, 5), (19, 7), (20, 5), (21, 7)] {
            call(&mut vm, &mut sprites, index, &vec![0; argc]);
            assert_eq!(vm.stack, [0x123456], "index {index}");
        }
        call(&mut vm, &mut sprites, 0, &[0, 100]);
        call(&mut vm, &mut sprites, 12, &[]);
        assert_eq!(vm.action_state.banks[0].mode, 0);
        assert!(vm.action_state.backup.is_some());
        call(&mut vm, &mut sprites, 13, &[]);
        assert_eq!(vm.action_state.banks[0].mode, 1);
        assert!(vm.action_state.backup.is_none());
        assert_eq!(vm.stack, [0x123456]);
        assert!(matches!(
            call(&mut vm, &mut sprites, 22, &[]),
            ExtCallOutcome::Skip
        ));
    }

    #[test]
    fn native_log_curves_and_sine_are_not_polynomial_easing() {
        assert!((curve(1, 49, 100) - 0.1655665).abs() < 0.000001);
        assert!((curve(2, 49, 100) - 0.759176).abs() < 0.000001);
        assert!((curve(3, 50, 100) - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.000001);
        assert_eq!(curve(1, 99, 100), 1.0);
        assert_eq!(curve(2, 99, 100), 1.0);
        assert_eq!(biased_delta(-11, 0.5), -5.0);
    }

    #[test]
    fn rotation_scale_and_alpha_compose_with_independent_base_properties() {
        let (mut vm, mut sprites, handle) = fixture();
        sprites.set_scale(handle, 2.0);
        call(&mut vm, &mut sprites, 10, &[0, 0, 0, 0, 128, 100]);
        call(&mut vm, &mut sprites, 11, &[1, 0, 100, 100]);
        call(&mut vm, &mut sprites, 8, &[2, 0, -101, 100]);
        call(&mut vm, &mut sprites, 0, &[0, 200]);
        vm.advance_actions(&mut sprites);
        vm.set_pal_time(50);
        vm.advance_actions(&mut sprites);
        let info = sprites.get(handle).unwrap().info();
        assert_eq!(info.rotation.z, 64.0);
        assert_eq!(info.scale, 2.5);
        assert_eq!(
            info.color.alpha(),
            205,
            "native alpha truncates rather than rounding"
        );
        sprites.set_scale(handle, 3.0);
        assert_eq!(sprites.get(handle).unwrap().info().scale, 3.5);
    }

    #[test]
    fn sync_wait_polls_without_popping_arguments_again() {
        let (mut vm, mut sprites, _) = fixture();
        call(&mut vm, &mut sprites, 0, &[0, 100]);
        vm.stack.push(42);
        vm.pc = 100;
        call(&mut vm, &mut sprites, 4, &[0]);
        assert_eq!(vm.pc, 88);
        vm.pc = 100;
        call(&mut vm, &mut sprites, 4, &[]);
        assert_eq!(vm.pc, 88);
        assert_eq!(vm.stack, [42]);
        vm.set_pal_time(100);
        vm.advance_actions(&mut sprites);
        vm.pc = 100;
        call(&mut vm, &mut sprites, 4, &[]);
        assert_eq!(vm.pc, 100);
        assert!(vm.action_state.wait_bank.is_none());
    }

    #[test]
    fn clear_mid_tween_drops_the_temporary_lane_and_commits_each_target_once() {
        let (mut vm, mut sprites, handle) = fixture();
        call(&mut vm, &mut sprites, 5, &[0, 0, 100, 0, 0, 100]);
        call(&mut vm, &mut sprites, 0, &[0, 200]);
        vm.advance_actions(&mut sprites);
        vm.set_pal_time(50);
        vm.advance_actions(&mut sprites);
        assert_eq!(sprites.get(handle).unwrap().info().position.x, 50.0);
        call(&mut vm, &mut sprites, 3, &[0]);
        assert_eq!(sprites.get(handle).unwrap().info().position.x, 100.0);
        call(&mut vm, &mut sprites, 6, &[0, 0, 5, 0, 100]);
        call(&mut vm, &mut sprites, 6, &[0, 0, 10, 0, 100]);
        call(&mut vm, &mut sprites, 3, &[0]);
        assert_eq!(sprites.get(handle).unwrap().info().position.x, 640.0);
    }

    #[test]
    fn positive_alpha_action_can_reveal_a_transparent_base_sprite() {
        let (mut vm, mut sprites, handle) = fixture();
        sprites.set_color(handle, PalColor(0x00ffffff));
        call(&mut vm, &mut sprites, 8, &[0, 0, 200, 100]);
        call(&mut vm, &mut sprites, 0, &[0, 200]);
        vm.advance_actions(&mut sprites);
        vm.set_pal_time(50);
        vm.advance_actions(&mut sprites);
        let sprite = sprites.get(handle).unwrap();
        assert_eq!(sprite.info().color.alpha(), 100);
        assert!(sprite.draw_command(&sprites).is_some());
        assert!(matches!(
            call(&mut vm, &mut sprites, 3, &[0]),
            ExtCallOutcome::Value(1)
        ));
    }

    #[test]
    fn save_round_trip_rebinds_active_actions_and_rebases_their_clocks() {
        let (mut vm, mut sprites, original) = fixture();
        assert!(sprites.set_scale_xy(original, 1.25, 0.75));
        sprites.get_mut(original).unwrap().rotation.z = 32.0;
        call(&mut vm, &mut sprites, 5, &[0, 0, 100, 0, 0, 100]);
        call(&mut vm, &mut sprites, 10, &[1, 0, 0, 0, 128, 100]);
        call(&mut vm, &mut sprites, 0, &[0, 200]);
        vm.advance_actions(&mut sprites);
        vm.set_pal_time(50);
        vm.advance_actions(&mut sprites);
        let snapshot = vm.capture_resumable_scene(Some(&sprites));
        let bytes = encode_runtime_save_snapshot(&snapshot).unwrap();
        let restored = decode_runtime_save_snapshot(&mut bytes.as_slice()).unwrap();
        let expected_random = vm.random_state.random_ex();
        call(&mut vm, &mut sprites, 3, &[0]);
        vm.random_state.random_ex();
        vm.set_pal_time(10_000);
        vm.restore_save_snapshot(restored.clone());
        vm.restore_checkpoint_scene(&restored, &mut sprites);
        let handle = vm.game_sprites[&0];
        assert_ne!(handle, original);
        assert_eq!(sprites.get(handle).unwrap().source_name, "action fixture");
        assert_eq!(sprites.get(handle).unwrap().scale, 1.25);
        assert_eq!(sprites.get(handle).unwrap().scale_y, Some(0.75));
        assert_eq!(vm.random_state.random_ex(), expected_random);
        assert_eq!(sprites.get(handle).unwrap().info().position.x, 50.0);
        assert_eq!(sprites.get(handle).unwrap().info().rotation.z, 96.0);
        vm.set_pal_time(10_025);
        vm.advance_actions(&mut sprites);
        assert_eq!(sprites.get(handle).unwrap().info().position.x, 75.0);
        assert_eq!(sprites.get(handle).unwrap().info().rotation.z, 128.0);
        vm.set_pal_time(10_050);
        vm.advance_actions(&mut sprites);
        assert_eq!(sprites.get(handle).unwrap().info().position.x, 100.0);
        assert_eq!(sprites.get(handle).unwrap().info().rotation.z, 160.0);
        let final_snapshot = vm.capture_resumable_scene(Some(&sprites));
        vm.restore_checkpoint_scene(&final_snapshot, &mut sprites);
        assert_eq!(
            sprites.get(vm.game_sprites[&0]).unwrap().info().position.x,
            100.0
        );
    }

    #[test]
    fn restored_text_actions_wait_for_the_rebuilt_text_sprite() {
        let (mut vm, mut sprites, original) = fixture();
        vm.text_state.sprite = Some(original);
        vm.game_sprites.clear();
        call(&mut vm, &mut sprites, 5, &[0, 255, 100, 0, 0, 100]);
        call(&mut vm, &mut sprites, 3, &[0]);
        let snapshot = vm.capture_resumable_scene(Some(&sprites));
        vm.restore_checkpoint_scene(&snapshot, &mut sprites);
        assert!(!vm.action_state.pending_bindings.is_empty());
        let snapshot = vm.capture_resumable_scene(Some(&sprites));
        let bytes = encode_runtime_save_snapshot(&snapshot).unwrap();
        let snapshot = decode_runtime_save_snapshot(&mut bytes.as_slice()).unwrap();
        vm.restore_checkpoint_scene(&snapshot, &mut sprites);
        let rebuilt = sprites
            .create_rgba_sprite(1, 1, vec![255; 4], PalVec3::default(), 0, "rebuilt text")
            .unwrap();
        vm.text_state.sprite = Some(rebuilt);
        vm.advance_actions(&mut sprites);
        assert_eq!(sprites.get(rebuilt).unwrap().info().position.x, 100.0);
        assert!(vm.action_state.pending_bindings.is_empty());
    }

    #[test]
    fn deferred_text_binding_does_not_leave_stale_lanes_on_other_sprites() {
        let (mut vm, mut sprites, handle) = fixture();
        let text = sprites
            .create_rgba_sprite(1, 1, vec![255; 4], PalVec3::default(), 0, "text")
            .unwrap();
        vm.text_state.sprite = Some(text);
        call(&mut vm, &mut sprites, 5, &[0, 0, 100, 0, 0, 100]);
        call(&mut vm, &mut sprites, 0, &[0, 100]);
        vm.advance_actions(&mut sprites);
        vm.set_pal_time(50);
        vm.advance_actions(&mut sprites);
        assert_eq!(sprites.get(handle).unwrap().info().position.x, 50.0);
        let snapshot = vm.capture_resumable_scene(Some(&sprites));
        vm.restore_checkpoint_scene(&snapshot, &mut sprites);
        vm.set_pal_time(100);
        vm.advance_actions(&mut sprites);
        let sprite = sprites.get(vm.game_sprites[&0]).unwrap();
        assert_eq!(sprite.info().position.x, 100.0);
        assert_eq!(sprite.action_lanes, SpriteActionLanes::default());
    }

    #[test]
    fn truncated_action_snapshots_fail_without_partial_state() {
        let (vm, sprites, _) = fixture();
        let snapshot = vm.capture_resumable_scene(Some(&sprites));
        let bytes = encode_runtime_save_snapshot(&snapshot).unwrap();
        for removed in [1, 20, 100] {
            assert!(decode_runtime_save_snapshot(&mut &bytes[..bytes.len() - removed]).is_err());
        }
    }

    #[test]
    fn pal_cell_indices_survive_deferred_action_dispatch() {
        let (mut vm, mut sprites, handle) = fixture();
        sprites.replace_sprite_surface(handle, 64, 32, vec![255; 64 * 32 * 4], "sheet");
        sprites.set_offset_rect(handle, 16, 16);
        call(&mut vm, &mut sprites, 18, &[0, 0, 65_536, -1, 100]);
        call(&mut vm, &mut sprites, 0, &[0, 200]);
        vm.advance_actions(&mut sprites);
        assert_eq!(
            sprites.get(handle).unwrap().source_rect,
            PalRect::new(48, 16, 64, 32)
        );
    }

    #[test]
    fn rectangle_position_uses_cell_indices_and_does_not_advance_the_line() {
        let (mut vm, mut sprites, handle) = fixture();
        sprites.replace_sprite_surface(handle, 8, 4, vec![255; 8 * 4 * 4], "sheet");
        sprites.set_offset_rect(handle, 2, 2);
        call(&mut vm, &mut sprites, 18, &[0, 0, 2, 1, 100]);
        call(&mut vm, &mut sprites, 0, &[0, 200]);
        vm.advance_actions(&mut sprites);
        assert_eq!(
            sprites.get(handle).unwrap().source_rect,
            PalRect::new(4, 2, 6, 4)
        );
        vm.set_pal_time(101);
        vm.advance_actions(&mut sprites);
        assert_eq!(vm.action_state.banks[0].lines[&0].index, 0);
    }

    #[test]
    fn last_shake_replaces_its_lane_without_replacing_movement() {
        let (mut vm, mut sprites, handle) = fixture();
        call(&mut vm, &mut sprites, 5, &[0, 0, 100, 0, 0, 100]);
        call(&mut vm, &mut sprites, 17, &[1, 0, 1, 10, 100, 100]);
        call(&mut vm, &mut sprites, 17, &[2, 0, 0, 20, 100, 100]);
        call(&mut vm, &mut sprites, 0, &[0, 200]);
        vm.advance_actions(&mut sprites);
        vm.set_pal_time(25);
        vm.advance_actions(&mut sprites);
        let info = sprites.get(handle).unwrap().info();
        assert_eq!(info.position.x, 25.0);
        assert_eq!(info.position.y, 20.0);
        let snapshot = vm.capture_resumable_scene(Some(&sprites));
        let bytes = encode_runtime_save_snapshot(&snapshot).unwrap();
        let restored = decode_runtime_save_snapshot(&mut bytes.as_slice()).unwrap();
        vm.restore_checkpoint_scene(&restored, &mut sprites);
        let info = sprites.get(vm.game_sprites[&0]).unwrap().info();
        assert_eq!((info.position.x, info.position.y), (25.0, 20.0));
    }

    #[test]
    fn bank_completion_commits_and_clamps_before_later_base_setters() {
        let (mut vm, mut sprites, handle) = fixture();
        call(&mut vm, &mut sprites, 5, &[0, 0, 100, 0, 7, 100]);
        call(&mut vm, &mut sprites, 8, &[1, 0, -300, 100]);
        call(&mut vm, &mut sprites, 3, &[0]);
        let sprite = sprites.get(handle).unwrap();
        assert_eq!(sprite.position.x, 100.0);
        assert_eq!(sprite.position.z, 0.0);
        assert_eq!(sprite.color.alpha(), 0);
        assert!(vm.action_state.final_lanes.is_empty());
        sprites.get_mut(handle).unwrap().position.x = 400.0;
        call(&mut vm, &mut sprites, 8, &[0, 0, 100, 100]);
        call(&mut vm, &mut sprites, 3, &[0]);
        assert_eq!(sprites.get(handle).unwrap().info().position.x, 400.0);
        assert_eq!(sprites.get(handle).unwrap().info().color.alpha(), 100);
    }

    #[test]
    fn loop_boundary_is_a_one_frame_latch_and_finite_stop_flushes_restarted_sections() {
        let (mut vm, mut sprites, handle) = fixture();
        call(&mut vm, &mut sprites, 5, &[0, 0, 10, 0, 0, 100]);
        call(&mut vm, &mut sprites, 2, &[0, -1]);
        vm.advance_actions(&mut sprites);
        vm.set_pal_time(100);
        vm.advance_actions(&mut sprites);
        assert!(vm.action_state.banks[0].cycle_complete);
        vm.set_pal_time(101);
        vm.advance_actions(&mut sprites);
        assert!(!vm.action_state.banks[0].cycle_complete);
        call(&mut vm, &mut sprites, 3, &[0]);
        assert_eq!(sprites.get(handle).unwrap().info().position.x, 20.0);
        call(&mut vm, &mut sprites, 5, &[0, 0, 10, 0, 0, 100]);
        call(&mut vm, &mut sprites, 2, &[0, 1]);
        vm.advance_actions(&mut sprites);
        vm.set_pal_time(201);
        vm.advance_actions(&mut sprites);
        // Both native schedulers reset the line cursors before uninitializing
        // the last finite cycle, so its unstarted final callback runs once.
        assert_eq!(sprites.get(handle).unwrap().info().position.x, 40.0);
        assert_eq!(vm.action_state.banks[0].mode, 0);
    }
}
