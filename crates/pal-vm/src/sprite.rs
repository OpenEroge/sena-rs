use crate::fx::RasterSource;
pub use crate::fx::SpriteFxEffect;
use std::collections::BTreeMap;
use std::sync::Arc;

mod channel_copy;
mod filters;
mod target_render;

use crate::msprite::MSpriteHandle;
use crate::scene::{
    DrawCommand, HiresPixels, RectF, SceneTexture, SceneTextureFormat, SceneTextureId, SpriteDraw,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SpriteHandle(pub u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SpriteSurfaceId(pub u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RenderNodeId(pub u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SpriteTransitionHandle(pub u32);

#[derive(Debug, Default)]
pub struct SpriteSystem {
    next_handle: u32,
    next_render_node: u32,
    next_surface_id: u64,
    next_transition_handle: u32,
    next_fx_handle: u32,
    sprites: BTreeMap<SpriteHandle, PalSprite>,
    surfaces: BTreeMap<SpriteSurfaceId, SpriteSurface>,
    render_nodes: BTreeMap<RenderNodeId, RenderNode>,
    transitions: BTreeMap<SpriteTransitionHandle, SpriteTransition>,
    fx_effects: BTreeMap<SpriteFxHandle, SpriteFxEffect>,
    fx_time_ms: u32,
    fx_phase_start_ms: Option<u32>,
    global_fx: Option<(i32, SpriteFxEffect)>,
    global_fx_render_attached: bool,
    motion_entries: BTreeMap<SpriteHandle, SpriteMotionEntry>,
    /// Running `.ani` programs; they own the sprites' motion lanes.
    ani_motions: BTreeMap<SpriteHandle, crate::ani::AniPlayer>,
    /// Millisecond clock for motion programs (native PaltimeGetTime).
    motion_clock_ms: u32,
    motion_random: crate::system::PalRandomState,
    motion_skipping: bool,
    option_backups: BTreeMap<SpriteHandle, SpriteSurface>,
    registered_profile: bool,
    native_draw_state: crate::scene::blend::NativeDrawState,
    target_render: target_render::TargetRenderList,
}

impl SpriteSystem {
    pub fn new() -> Self {
        Self {
            next_handle: 1,
            next_render_node: 1,
            next_surface_id: 1,
            next_transition_handle: 1,
            next_fx_handle: 1,
            sprites: BTreeMap::new(),
            surfaces: BTreeMap::new(),
            render_nodes: BTreeMap::new(),
            transitions: BTreeMap::new(),
            fx_effects: BTreeMap::new(),
            fx_time_ms: 0,
            fx_phase_start_ms: None,
            global_fx: None,
            global_fx_render_attached: false,
            motion_entries: BTreeMap::new(),
            ani_motions: BTreeMap::new(),
            motion_clock_ms: 0,
            motion_random: crate::system::PalRandomState::default(),
            motion_skipping: false,
            option_backups: BTreeMap::new(),
            registered_profile: false,
            native_draw_state: crate::scene::blend::NativeDrawState::default(),
            target_render: target_render::TargetRenderList::default(),
        }
    }

    pub fn allocate_surface_id(&mut self) -> SpriteSurfaceId {
        let id = SpriteSurfaceId(self.next_surface_id);
        self.next_surface_id = self
            .next_surface_id
            .checked_add(1)
            .expect("sprite surface id space exhausted");
        id
    }

    /// The newer PAL keeps signed cell indices and excludes a one-pixel
    /// texture padding remainder. This is a build-level semantic difference.
    pub fn set_registered_profile(&mut self, enabled: bool) {
        if self.registered_profile != enabled {
            self.registered_profile = enabled;
            for sprite in self.sprites.values_mut() {
                sprite.registered_cell_indices = enabled;
            }
        }
    }

    pub fn insert_texture(&mut self, texture: SceneTexture) -> SpriteSurfaceId {
        self.insert_surface(SpriteSurface::from_scene_texture(texture))
    }

    pub fn insert_surface(&mut self, surface: SpriteSurface) -> SpriteSurfaceId {
        let id = surface.id;
        self.surfaces.insert(id, surface);
        id
    }

    pub fn surface(&self, id: SpriteSurfaceId) -> Option<&SpriteSurface> {
        self.surfaces.get(&id)
    }

    pub fn surface_mut(&mut self, id: SpriteSurfaceId) -> Option<&mut SpriteSurface> {
        self.surfaces.get_mut(&id)
    }

    pub fn textures(&self) -> impl Iterator<Item = SceneTexture> + '_ {
        self.surfaces.values().map(SpriteSurface::to_scene_texture)
    }

    pub fn create(&mut self, desc: SpriteDesc) -> SpriteHandle {
        let handle = self.allocate_handle();
        let render_node = self.allocate_render_node();
        let mut sprite = PalSprite::new(handle, render_node, desc);
        sprite.registered_cell_indices = self.registered_profile;
        let priority = sprite.effective_priority();
        self.render_nodes
            .insert(render_node, RenderNode::new(render_node, handle, priority));
        self.sprites.insert(handle, sprite);
        handle
    }

    pub fn create_rgba_sprite(
        &mut self,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
        position: PalVec3,
        priority: i32,
        source_name: impl Into<String>,
    ) -> Option<SpriteHandle> {
        let width = width.max(1);
        let height = height.max(1);
        let surface_id = self.allocate_surface_id();
        let surface = SpriteSurface::rgba8(surface_id, 1, width, height, rgba).ok()?;
        self.insert_surface(surface);
        let mut desc = SpriteDesc::new(SceneTextureId(surface_id.0), width, height);
        desc.position = position;
        desc.base_priority = priority;
        desc.visible = true;
        desc.smooth_upscale = true;
        desc.source_name = source_name.into();
        Some(self.create(desc))
    }

    /// Restore immutable checkpoint pixels without copying a full image again.
    pub fn create_shared_rgba_sprite(&mut self, width: u32, height: u32,
        pixels: Arc<Vec<u8>>, position: PalVec3, priority: i32,
        source_name: impl Into<String>) -> Option<SpriteHandle> {
        let expected = (width as usize).checked_mul(height as usize)?.checked_mul(4)?;
        if expected == 0 || pixels.len() != expected { return None; }
        let id = self.allocate_surface_id();
        self.insert_texture(SceneTexture { id: SceneTextureId(id.0), generation: 1,
            width, height, format: SceneTextureFormat::Rgba8, pixels, hires: None });
        let mut desc = SpriteDesc::new(SceneTextureId(id.0), width, height);
        desc.position = position;
        desc.base_priority = priority;
        desc.visible = true;
        desc.smooth_upscale = true;
        desc.source_name = source_name.into();
        Some(self.create(desc))
    }

    /// Copy a sprite's current surface pixels into a new sprite at the same
    /// position. Used by `get_backbuffer` / `PalSpriteBackBafferCopy`.
    pub fn copy_sprite_pixels(
        &mut self,
        source: SpriteHandle,
        priority: i32,
        source_name: impl Into<String>,
    ) -> Option<SpriteHandle> {
        let sprite = self.get(source)?;
        let position = sprite.position;
        let surface_id = sprite.surface;
        let texture = self.surface(surface_id)?.to_scene_texture();
        self.create_rgba_sprite(
            texture.width,
            texture.height,
            texture.pixels.to_vec(),
            position,
            priority,
            source_name,
        )
    }

    /// Attach a dense copy of a host-rendered sprite's current surface (see
    /// `SpriteSurface::set_hires`). Returns false and keeps the plain
    /// surface when the copy does not fit.
    pub fn set_sprite_hires(&mut self, handle: SpriteHandle, density: u32, pixels: Vec<u8>) -> bool {
        let Some(surface_id) = self.get(handle).map(|sprite| sprite.surface) else {
            return false;
        };
        self.surface_mut(surface_id)
            .is_some_and(|surface| surface.set_hires(density, pixels))
    }

    pub fn create_msprite(
        &mut self,
        decoder: MSpriteHandle,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
        position: PalVec3,
        priority: i32,
        source_name: impl Into<String>,
    ) -> Option<SpriteHandle> {
        let width = width.max(1);
        let height = height.max(1);
        let surface_id = self.allocate_surface_id();
        let surface = SpriteSurface::rgba8(surface_id, 1, width, height, rgba).ok()?;
        self.insert_surface(surface);
        let mut desc = SpriteDesc::new(SceneTextureId(surface_id.0), width, height);
        desc.kind = SpriteKind::MSprite { decoder };
        desc.position = position;
        desc.base_priority = priority;
        desc.visible = true;
        desc.source_name = source_name.into();
        Some(self.create(desc))
    }

    pub fn replace_sprite_surface(
        &mut self,
        handle: SpriteHandle,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
        source_name: impl Into<String>,
    ) -> bool {
        let width = width.max(1);
        let height = height.max(1);
        let Some(surface_id) = self.get(handle).map(|sprite| sprite.surface) else {
            return false;
        };
        // GUI rendering caches textures by id/generation. Native PAL frequently
        // replaces a sprite surface in-place with the same id and dimensions
        // for ADV typewriter text, face parts, and animation frames, so a
        // replacement must advance the generation even when the size is stable.
        let generation = self
            .surfaces
            .get(&surface_id)
            .map(|surface| surface.generation.saturating_add(1).max(1))
            .unwrap_or(1);
        let Ok(surface) = SpriteSurface::rgba8(surface_id, generation, width, height, rgba) else {
            return false;
        };
        self.surfaces.insert(surface_id, surface);
        if let Some(sprite) = self.get_mut(handle) {
            sprite.texture_size = PalSize::new(width, height);
            sprite.cell_size = PalSize::new(width, height);
            sprite.source_rect = PalRect::new(0, 0, width as i32, height as i32);
            sprite.source_name = source_name.into();
            return true;
        }
        false
    }

    pub fn replace_msprite_frame(
        &mut self,
        handle: SpriteHandle,
        decoder: MSpriteHandle,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
        source_name: impl Into<String>,
    ) -> bool {
        if !self.replace_sprite_surface(handle, width, height, rgba, source_name) {
            return false;
        }
        if let Some(sprite) = self.get_mut(handle) {
            sprite.kind = SpriteKind::MSprite { decoder };
            return true;
        }
        false
    }

    pub fn release(&mut self, handle: SpriteHandle) -> bool {
        let Some(sprite) = self.sprites.remove(&handle) else {
            return false;
        };
        let surface = sprite.surface;
        if let Some(effect) = sprite.fx_effect {
            self.fx_effects.remove(&effect);
        }
        self.motion_entries.remove(&handle);
        self.ani_motions.remove(&handle);
        self.option_backups.remove(&handle);
        if let Some(render_node) = sprite.render_node {
            self.render_nodes.remove(&render_node);
        }
        if !self.sprites.values().any(|other| other.surface == surface) {
            self.surfaces.remove(&surface);
        }
        true
    }

    pub fn create_transition_handle(&mut self) -> SpriteTransitionHandle {
        let handle = SpriteTransitionHandle(self.next_transition_handle);
        self.next_transition_handle = self
            .next_transition_handle
            .checked_add(1)
            .expect("sprite transition handle space exhausted");
        self.transitions.insert(handle, SpriteTransition::default());
        handle
    }

    pub fn release_transition_handle(&mut self, handle: SpriteTransitionHandle) -> bool {
        if self
            .transitions
            .get(&handle)
            .is_some_and(|transition| transition.state != 0)
        {
            self.cancel_transition(handle);
        }
        self.transitions.remove(&handle).is_some()
    }

    pub fn set_transition(
        &mut self,
        handle: SpriteTransitionHandle,
        render_id: u32,
        first: Option<SpriteHandle>,
        second: Option<SpriteHandle>,
        transition_id: i32,
        start_time_ms: u32,
        duration_ms: u32,
    ) -> bool {
        if !self.transitions.contains_key(&handle) {
            return false;
        }
        if self.transition_state(handle) != 0 {
            self.cancel_transition(handle);
        }
        if let Some(sprite) = first.and_then(|sprite| self.get_mut(sprite)) {
            sprite.transition_block = transition_id as u32;
        }
        if let Some(sprite) = second.and_then(|sprite| self.get_mut(sprite)) {
            sprite.transition_block = transition_id as u32;
        }
        let transition = self.transitions.get_mut(&handle).expect("checked above");
        *transition = SpriteTransition {
            render_id,
            state: 1,
            first,
            second,
            transition_id,
            start_time_ms,
            duration_ms,
            elapsed_ms: 0,
        };
        // Native writes the handle even when no worker can be inserted.
        first.is_some() || second.is_some()
    }

    pub fn cancel_transition(&mut self, handle: SpriteTransitionHandle) -> bool {
        let Some(transition) = self.transitions.get(&handle) else {
            return false;
        };
        if transition.render_id == 0 {
            return false;
        }
        let first = transition.first;
        let second = transition.second;
        if let Some(sprite) = first.and_then(|sprite| self.get_mut(sprite)) {
            sprite.transition_block = 0;
        }
        if let Some(second) = second {
            self.release(second);
        }
        if let Some(transition) = self.transitions.get_mut(&handle) {
            *transition = SpriteTransition::default();
        }
        true
    }

    pub fn transition_state(&self, handle: SpriteTransitionHandle) -> i32 {
        self.transitions
            .get(&handle)
            .map(|transition| transition.state)
            .unwrap_or(3)
    }

    /// Runs the native type-1 worker at the cached absolute PAL clock.
    /// Completion leaves the worker and owned second sprite intact. The Game
    /// wrapper separately cancels completed transitions when it retires images.
    pub fn advance_transitions(&mut self, now_ms: u32, force_complete: bool) {
        for transition in self.transitions.values_mut() {
            if transition.state == 0
                || transition.transition_id != 1
                || (transition.first.is_none() && transition.second.is_none())
            {
                continue;
            }
            let elapsed = now_ms.wrapping_sub(transition.start_time_ms);
            transition.elapsed_ms = if force_complete {
                transition.duration_ms
            } else {
                elapsed.min(transition.duration_ms)
            };
            if transition.elapsed_ms >= transition.duration_ms {
                transition.state = 2;
            }
        }
    }

    pub fn tween_pos_by(
        &mut self,
        handle: SpriteHandle,
        dx: f32,
        dy: f32,
        dz: f32,
        duration_ms: u32,
    ) -> bool {
        let Some(sprite) = self.get(handle) else {
            return false;
        };
        let initial_entry = SpriteMotionEntry::from_sprite(handle, sprite);
        if duration_ms == 0 {
            return self.move_pos(handle, dx, dy, dz);
        }
        let from = sprite.position;
        let to = PalVec3::from_f32(from.x + dx, from.y + dy, from.z + dz);
        let entry = self.motion_entries.entry(handle).or_insert(initial_entry);
        entry.pos_b = to;
        entry.anim_delta_pos = Some(SpritePositionTween {
            from,
            to,
            elapsed_ms: 0,
            duration_ms,
        });
        true
    }

    /// Native skip mode makes motion waits finish at once.
    pub fn set_motion_skipping(&mut self, skipping: bool) {
        self.motion_skipping = skipping;
    }

    /// Bind a motion program to a sprite and run it until its first wait.
    /// A program already on the sprite is cleared first (native 0x447180).
    /// Returns false when the sprite does not exist.
    pub fn start_motion(&mut self, handle: SpriteHandle, program: crate::ani::AniProgram) -> bool {
        if !self.sprites.contains_key(&handle) {
            return false;
        }
        self.clear_motion(handle);
        let base_position = self.motion_base_position(handle);
        let mut effects = Vec::new();
        let random = &mut self.motion_random;
        let mut next_random = || random.random(false) as u32;
        let mut ctx = crate::ani::AniContext {
            now_ms: self.motion_clock_ms,
            skipping: self.motion_skipping,
            base_position,
            random: &mut next_random,
            effects: &mut effects,
        };
        let player = crate::ani::AniPlayer::start(program, &mut ctx);
        self.apply_motion_effects(handle, &effects);
        match player {
            Some(player) => {
                self.set_motion_lanes(handle, &player.delta());
                self.ani_motions.insert(handle, player);
            }
            None => self.set_motion_lanes(handle, &crate::ani::MotionDelta::default()),
        }
        true
    }

    /// Native 0x447180: stop the program, fold its committed delta into the
    /// sprite base when it ran opcode 0x13, and drop the motion lanes.
    pub fn clear_motion(&mut self, handle: SpriteHandle) -> bool {
        let Some(player) = self.ani_motions.remove(&handle) else {
            return false;
        };
        if let Some(sprite) = self.sprites.get_mut(&handle) {
            if let Some(delta) = player.persisted_delta() {
                sprite.commit_action_lanes(SpriteActionLanes::from_motion(&delta));
            }
            sprite.motion_lanes = SpriteActionLanes::default();
        }
        true
    }

    /// Game action type 20 (trenka.exe 0x401DF0): reveal `to` through the
    /// mask. With `threshold` p, mask level m = 255 - mask blue selects
    /// from's alpha below p, a 32-step ramp up to p + 31, and zero above.
    /// `None` is the completion pass (0x402010), which copies `from` whole.
    pub fn apply_action_mask(
        &mut self,
        from: SpriteHandle,
        mask: SpriteHandle,
        to: SpriteHandle,
        threshold: Option<u8>,
    ) -> bool {
        let pixels = |sprites: &Self, handle: SpriteHandle| {
            let surface = sprites.surface(sprites.get(handle)?.surface)?;
            Some((surface.width() as usize, surface.height() as usize, surface.to_scene_texture().pixels.to_vec()))
        };
        let (Some((width, height, source)), Some((mask_width, mask_height, mask_pixels))) =
            (pixels(self, from), pixels(self, mask))
        else {
            return false;
        };
        let Some(surface_id) = self.get(to).map(|sprite| sprite.surface) else { return false; };
        let Some(surface) = self.surfaces.get_mut(&surface_id) else { return false; };
        let (to_width, to_height) = (surface.width() as usize, surface.height() as usize);
        let rows = height.min(mask_height).min(to_height);
        let columns = width.min(mask_width).min(to_width);
        let target = surface.pixels_mut();
        let high = threshold.map(|low| u16::from(low).saturating_add(31).min(255));
        for y in 0..rows {
            for x in 0..columns {
                let src = (y * width + x) * 4;
                let dst = (y * to_width + x) * 4;
                let (Some(low), Some(high)) = (threshold, high) else {
                    target[dst..dst + 4].copy_from_slice(&source[src..src + 4]);
                    continue;
                };
                let level = 255 - u16::from(mask_pixels[(y * mask_width + x) * 4 + 2]);
                let alpha = u16::from(source[src + 3]);
                target[dst + 3] = if level < u16::from(low) {
                    alpha as u8
                } else if level < high {
                    (((high - level) * alpha) >> 5) as u8
                } else {
                    0
                };
            }
        }
        surface.mark_dirty();
        true
    }

    pub fn motion_active(&self, handle: SpriteHandle) -> bool {
        self.ani_motions.contains_key(&handle)
    }

    fn motion_base_position(&self, handle: SpriteHandle) -> [i32; 3] {
        self.sprites.get(&handle).map_or([0; 3], |sprite| {
            [sprite.position.x as i32, sprite.position.y as i32, sprite.position.z as i32]
        })
    }

    fn set_motion_lanes(&mut self, handle: SpriteHandle, delta: &crate::ani::MotionDelta) {
        if let Some(sprite) = self.sprites.get_mut(&handle) {
            sprite.motion_lanes = SpriteActionLanes::from_motion(delta);
        }
    }

    fn apply_motion_effects(&mut self, handle: SpriteHandle, effects: &[crate::ani::AniEffect]) {
        for effect in effects {
            match *effect {
                crate::ani::AniEffect::RectSetPos(x, y) => {
                    self.rect_set_pos(handle, x, y);
                }
                crate::ani::AniEffect::SetRect([left, top, right, bottom]) => {
                    self.set_rect(handle, Some(PalRect::new(left, top, right, bottom)));
                }
            }
        }
    }

    fn advance_ani_motions(&mut self, delta_ms: u32) {
        self.motion_clock_ms = self.motion_clock_ms.wrapping_add(delta_ms);
        let handles = self.ani_motions.keys().copied().collect::<Vec<_>>();
        for handle in handles {
            if !self.sprites.contains_key(&handle) {
                self.ani_motions.remove(&handle);
                continue;
            }
            let base_position = self.motion_base_position(handle);
            let mut effects = Vec::new();
            let Some(player) = self.ani_motions.get_mut(&handle) else { continue; };
            let random = &mut self.motion_random;
            let mut next_random = || random.random(false) as u32;
            let mut ctx = crate::ani::AniContext {
                now_ms: self.motion_clock_ms,
                skipping: self.motion_skipping,
                base_position,
                random: &mut next_random,
                effects: &mut effects,
            };
            let running = player.advance(&mut ctx);
            let delta = player.delta();
            self.apply_motion_effects(handle, &effects);
            if running {
                self.set_motion_lanes(handle, &delta);
            } else {
                self.clear_motion(handle);
            }
        }
    }

    /// Advances the Game.exe `SpriteMotionEntry` analogue recovered in the
    /// latest IDB.  Native `VmTask_ApplyMotion` walks `gSpriteMotionList` and
    /// commits layered position/scale/color deltas into the wrapped PalSprite;
    /// the Rust port keeps only the currently implemented named-ANI lanes here.
    pub fn advance_motion_entries(&mut self, delta_ms: u32) {
        self.advance_ani_motions(delta_ms);
        let handles = self.motion_entries.keys().copied().collect::<Vec<_>>();
        for handle in handles {
            let Some(mut entry) = self.motion_entries.remove(&handle) else {
                continue;
            };

            if let Some(mut tween) = entry.anim_delta_pos {
                tween.elapsed_ms = tween.elapsed_ms.saturating_add(delta_ms);
                let t = (tween.elapsed_ms as f32 / tween.duration_ms.max(1) as f32).clamp(0.0, 1.0);
                let x = tween.from.x + (tween.to.x - tween.from.x) * t;
                let y = tween.from.y + (tween.to.y - tween.from.y) * t;
                let z = tween.from.z + (tween.to.z - tween.from.z) * t;
                let _ = self.set_pos_float(handle, x, y, z);
                if tween.elapsed_ms < tween.duration_ms {
                    entry.anim_delta_pos = Some(tween);
                } else {
                    entry.pos_b = tween.to;
                    entry.anim_delta_pos = None;
                }
            }

            if let Some(mut tween) = entry.anim_delta_scale {
                tween.elapsed_ms = tween.elapsed_ms.saturating_add(delta_ms);
                let t = (tween.elapsed_ms as f32 / tween.duration_ms.max(1) as f32).clamp(0.0, 1.0);
                let scale = tween.from + (tween.to - tween.from) * t;
                let _ = self.set_scale_direct(handle, scale);
                if tween.elapsed_ms < tween.duration_ms {
                    entry.anim_delta_scale = Some(tween);
                } else {
                    entry.scale_b = tween.to;
                    entry.anim_delta_scale = None;
                }
            }

            if let Some(mut tween) = entry.anim_delta_color_a {
                tween.elapsed_ms = tween.elapsed_ms.saturating_add(delta_ms);
                let t = (tween.elapsed_ms as f32 / tween.duration_ms.max(1) as f32).clamp(0.0, 1.0);
                let alpha = tween.from as f32 + (tween.to as f32 - tween.from as f32) * t;
                let alpha = alpha.round().clamp(0.0, 255.0) as u8;
                let _ = self.set_alpha_direct(handle, alpha);
                if tween.elapsed_ms < tween.duration_ms {
                    entry.anim_delta_color_a = Some(tween);
                } else {
                    entry.color_a = tween.to;
                    entry.anim_delta_color_a = None;
                }
            }

            if entry.has_active_animation() && self.sprites.contains_key(&handle) {
                self.motion_entries.insert(handle, entry);
            }
        }
    }

    pub fn transition_commands(&self) -> Vec<DrawCommand> {
        let mut commands = Vec::new();
        for transition in self.transitions.values() {
            if transition.state == 0 || transition.transition_id != 1 {
                continue;
            }
            let first = transition.first.and_then(|handle| self.get(handle));
            let second = transition.second.and_then(|handle| self.get(handle));
            // The native two-sprite worker rejects the whole draw if either
            // participating sprite is hidden.
            if first.is_some_and(|sprite| !sprite.visible)
                || second.is_some_and(|sprite| !sprite.visible)
            {
                continue;
            }
            let progress = if transition.duration_ms == 0 {
                1.0
            } else {
                transition.elapsed_ms as f32 / transition.duration_ms as f32
            };
            let priority = first.or(second).map_or(0, |sprite| sprite.base_priority);
            let first_alpha = first.map_or(0, |sprite| sprite.color.alpha());
            let incoming_alpha = (f32::from(first_alpha) * progress) as u8;
            let outgoing_alpha = if first.is_some() {
                first_alpha - incoming_alpha
            } else {
                (f32::from(second.map_or(0, |sprite| sprite.color.alpha())) * (1.0 - progress))
                    as u8
            };
            if let Some(second) = second {
                if transition.duration_ms != 0 {
                    if let Some(mut command) = second.draw_command_transition(self) {
                        if let DrawCommand::Sprite(ref mut sprite) = command {
                            sprite.priority = priority;
                            sprite.color[3] = f32::from(outgoing_alpha) / 255.0;
                        }
                        commands.push(command);
                    }
                }
            }
            if let Some(first) = first {
                if let Some(mut command) = first.draw_command_transition(self) {
                    if let DrawCommand::Sprite(ref mut sprite) = command {
                        sprite.priority = priority;
                        sprite.color[3] = f32::from(incoming_alpha) / 255.0;
                    }
                    commands.push(command);
                }
            }
        }
        commands
    }

    pub fn is_screen_copy(&self, handle: SpriteHandle) -> bool {
        self.get(handle)
            .is_some_and(|sprite| sprite.source_name.starts_with("screen-copy:"))
    }

    pub fn set_sprite_fx_effect(
        &mut self,
        handle: SpriteHandle,
        effect_id: u32,
        action: u32,
        value: i32,
    ) -> bool {
        self.set_sprite_fx_effect_with_source(
            handle,
            effect_id,
            action,
            value,
            RasterSource::default(),
        )
    }

    /// Adapter for `abi::win32::PalSpriteSetFxEffect` and the sprite branch
    /// of `PalSetFxEffect`. Start supports the verified type-0 source layout.
    /// Other actions ignore effect_id/source, as the native functions do.
    pub fn set_sprite_fx_effect_with_source(
        &mut self,
        handle: SpriteHandle,
        effect_id: u32,
        action: u32,
        duration_ms: i32,
        source: RasterSource,
    ) -> bool {
        let Some(sprite) = self.sprites.get_mut(&handle) else {
            return false;
        };
        // Native allocates the owner before inspecting the action, including
        // clear, stop and successful no-op actions. Clear does not release it.
        let effect_handle = *sprite.fx_effect.get_or_insert_with(|| {
            let effect = SpriteFxHandle(self.next_fx_handle);
            self.next_fx_handle = self
                .next_fx_handle
                .checked_add(1)
                .expect("sprite fx handle space exhausted");
            effect
        });
        let effect = self.fx_effects.entry(effect_handle).or_default();
        match action {
            0 => effect.clear(),
            1 => {
                if effect.state != 0 || effect_id != 0 {
                    return false;
                }
                *effect = SpriteFxEffect::start(source, duration_ms as u32, self.fx_time_ms);
            }
            2 => effect.stop(duration_ms as u32, self.fx_time_ms),
            _ => {}
        }
        true
    }

    /// Type-0 global start branch of `abi::win32::PalSetFxEffect`.
    /// The render attachment survives clear and stop until PalReleaseFxEffect.
    pub fn start_global_raster(
        &mut self,
        priority: i32,
        source: RasterSource,
        duration_ms: u32,
    ) -> bool {
        if self.global_fx_render_attached {
            return false;
        }
        self.global_fx_render_attached = true;
        let (_, effect) = self
            .global_fx
            .get_or_insert((priority, SpriteFxEffect::default()));
        if effect.state != 0 {
            return false;
        }
        self.global_fx = Some((
            priority,
            SpriteFxEffect::start(source, duration_ms, self.fx_time_ms),
        ));
        true
    }

    /// Global action 0 of `abi::win32::PalSetFxEffect` retains the attachment.
    pub fn clear_global_raster(&mut self) {
        if let Some((_, effect)) = &mut self.global_fx {
            effect.clear();
        }
    }

    /// Global action 2 does not require a previous successful start.
    pub fn stop_global_raster(&mut self, duration_ms: u32) {
        let (_, effect) = self.global_fx.get_or_insert((0, SpriteFxEffect::default()));
        effect.stop(duration_ms, self.fx_time_ms);
    }

    /// Global branch of PalReleaseFxEffect: release buffers and the render node.
    pub fn release_global_raster(&mut self) {
        self.global_fx = None;
        self.global_fx_render_attached = false;
    }

    /// Global branch of PalGetFxEffectState, including source-free stop state.
    pub fn global_raster_state(&self) -> i32 {
        self.global_fx
            .as_ref()
            .map_or(0, |(_, effect)| effect.state)
    }

    pub fn global_raster_stage(&self) -> Option<crate::scene::RasterStage> {
        self.global_fx
            .filter(|(_, effect)| {
                self.global_fx_render_attached && effect.active && effect.has_source
            })
            .map(|(priority, effect)| crate::scene::RasterStage {
                priority,
                uniforms: effect.uniforms,
            })
    }

    pub fn set_fx_time(&mut self, now_ms: u32) {
        self.fx_time_ms = now_ms;
    }

    /// Advance FX independently of script waits. All raster instances share
    /// the phase origin, matching fixture PAL.dll sub_100B4180.
    pub fn tick_fx_effects(&mut self, now_ms: u32) {
        self.fx_time_ms = now_ms;
        for effect in self.fx_effects.values_mut() {
            effect.tick(now_ms, &mut self.fx_phase_start_ms);
        }
        if self.global_fx_render_attached {
            if let Some((_, effect)) = &mut self.global_fx {
                effect.tick(now_ms, &mut self.fx_phase_start_ms);
            }
        }
    }

    pub fn release_sprite_fx_effect(&mut self, handle: SpriteHandle) -> bool {
        let Some(effect) = self.get(handle).and_then(|sprite| sprite.fx_effect) else {
            return false;
        };
        self.fx_effects.remove(&effect);
        if let Some(sprite) = self.get_mut(handle) {
            sprite.fx_effect = None;
        }
        true
    }

    pub fn sprite_fx_effect_state(&self, handle: SpriteHandle) -> i32 {
        self.get(handle)
            .and_then(|sprite| sprite.fx_effect)
            .and_then(|effect| self.fx_effects.get(&effect))
            .map(|effect| effect.state)
            .unwrap_or(0)
    }

    pub fn get(&self, handle: SpriteHandle) -> Option<&PalSprite> {
        self.sprites.get(&handle)
    }

    pub fn get_mut(&mut self, handle: SpriteHandle) -> Option<&mut PalSprite> {
        self.sprites.get_mut(&handle)
    }

    pub fn view_ctrl(&mut self, handle: SpriteHandle, visible: bool) -> bool {
        match self.get_mut(handle) {
            Some(sprite) => {
                sprite.visible = visible;
                true
            }
            None => false,
        }
    }

    pub fn view_is(&self, handle: SpriteHandle) -> bool {
        self.get(handle).is_some_and(|sprite| sprite.visible)
    }

    /// Portable adapter for `abi::win32::PalSpriteSetColor`; PalColor carries
    /// the complete native ARGB DWORD. Invalid arena keys return false.
    pub fn set_color(&mut self, handle: SpriteHandle, color: PalColor) -> bool {
        match self.get_mut(handle) {
            Some(sprite) => {
                sprite.color = color;
                true
            }
            None => false,
        }
    }

    pub fn get_width(&self, handle: SpriteHandle) -> Option<u32> {
        self.get(handle).map(|sprite| sprite.cell_size.width)
    }

    pub fn get_height(&self, handle: SpriteHandle) -> Option<u32> {
        self.get(handle).map(|sprite| sprite.cell_size.height)
    }

    pub fn set_pos(&mut self, handle: SpriteHandle, x: i32, y: i32, z: i32) -> bool {
        if let Some(entry) = self.motion_entries.get_mut(&handle) {
            entry.anim_delta_pos = None;
            if !entry.has_active_animation() {
                self.motion_entries.remove(&handle);
            }
        }
        self.set_pos_float(handle, x as f32, y as f32, z as f32)
    }

    pub fn set_pos_float(&mut self, handle: SpriteHandle, x: f32, y: f32, z: f32) -> bool {
        let priority = match self.get_mut(handle) {
            Some(sprite) => {
                trace_sprite_position_invariant("set_pos", handle, sprite, x, y, z);
                sprite.position = PalVec3::from_f32(x, y, z);
                sprite.effective_priority()
            }
            None => return false,
        };
        self.update_render_node_priority(handle, priority);
        true
    }

    pub fn set_native_projection(
        &mut self,
        handle: SpriteHandle,
        projection: Option<(f32, f32)>,
    ) -> bool {
        match self.get_mut(handle) {
            Some(sprite) => {
                sprite.native_projection = projection;
                true
            }
            None => false,
        }
    }

    pub fn position(&self, handle: SpriteHandle) -> Option<PalVec3> {
        self.get(handle).map(|sprite| sprite.position)
    }

    pub fn move_pos(&mut self, handle: SpriteHandle, dx: f32, dy: f32, dz: f32) -> bool {
        let priority = match self.get_mut(handle) {
            Some(sprite) => {
                let new_x = sprite.position.x + dx;
                let new_y = sprite.position.y + dy;
                let new_z = sprite.position.z + dz;
                trace_sprite_position_invariant("move_pos", handle, sprite, new_x, new_y, new_z);
                sprite.position.x += dx;
                sprite.position.y += dy;
                sprite.position.z += dz;
                sprite.effective_priority()
            }
            None => return false,
        };
        self.update_render_node_priority(handle, priority);
        true
    }

    pub fn set_offset_pos(&mut self, handle: SpriteHandle, x: i32, y: i32) -> bool {
        match self.get_mut(handle) {
            Some(sprite) => {
                sprite.offset = PalPoint2 { x, y };
                true
            }
            None => false,
        }
    }

    pub fn move_offset_pos(&mut self, handle: SpriteHandle, dx: i32, dy: i32) -> bool {
        match self.get_mut(handle) {
            Some(sprite) => {
                sprite.offset.x = sprite.offset.x.wrapping_add(dx);
                sprite.offset.y = sprite.offset.y.wrapping_add(dy);
                true
            }
            None => false,
        }
    }

    pub fn set_center_offset(&mut self, handle: SpriteHandle, x: i32, y: i32) -> bool {
        match self.get_mut(handle) {
            Some(sprite) => {
                sprite.set_center_offset(x, y);
                true
            }
            None => false,
        }
    }

    /// Portable adapter for `abi::win32::PalSpriteSetRect`. None represents
    /// a null RECT pointer; the shared PalRect32 layout is copied by value.
    pub fn set_rect(&mut self, handle: SpriteHandle, rect: Option<PalRect>) -> bool {
        match self.get_mut(handle) {
            Some(sprite) => {
                sprite.set_rect(rect);
                true
            }
            None => false,
        }
    }

    /// Portable adapter for `abi::win32::PalSpriteRectSetPos`; native indices
    /// are i32. The adapter uses an arena key instead of a native pointer.
    pub fn rect_set_pos(&mut self, handle: SpriteHandle, cell_x: i32, cell_y: i32) -> bool {
        match self.get_mut(handle) {
            Some(sprite) => {
                sprite.rect_set_pos(cell_x, cell_y);
                true
            }
            None => false,
        }
    }

    pub fn set_priority(&mut self, handle: SpriteHandle, priority: i32) -> bool {
        let effective_priority = match self.get_mut(handle) {
            Some(sprite) => {
                sprite.base_priority = priority;
                sprite.effective_priority()
            }
            None => return false,
        };
        self.update_render_node_priority(handle, effective_priority);
        true
    }

    pub fn change_priority(&mut self, handle: SpriteHandle, priority: i32) -> bool {
        let node_priority = match self.get_mut(handle) {
            Some(sprite) => {
                sprite.base_priority = priority;
                priority
            }
            None => return false,
        };
        self.update_render_node_priority(handle, node_priority);
        true
    }

    pub fn set_offset_rect(&mut self, handle: SpriteHandle, width: u32, height: u32) -> bool {
        match self.get_mut(handle) {
            Some(sprite) => {
                sprite.cell_size = PalSize::new(width, height);
                true
            }
            None => false,
        }
    }

    pub fn set_option(&mut self, handle: SpriteHandle, option_type: u32) -> bool {
        if !self.apply_surface_option(handle, option_type) {
            return false;
        }
        match self.get_mut(handle) {
            Some(sprite) => {
                sprite.option_type = option_type;
                if option_type == 0 {
                    sprite.option_aux = None;
                } else {
                    sprite.option_aux = Some(SpriteOptionAux {
                        raw_type: option_type,
                        raw_value: 0,
                    });
                }
                true
            }
            None => false,
        }
    }

    pub fn set_scale(&mut self, handle: SpriteHandle, scale: f32) -> bool {
        if let Some(entry) = self.motion_entries.get_mut(&handle) {
            entry.anim_delta_scale = None;
            if !entry.has_active_animation() {
                self.motion_entries.remove(&handle);
            }
        }
        self.set_scale_direct(handle, scale)
    }

    fn set_scale_direct(&mut self, handle: SpriteHandle, scale: f32) -> bool {
        match self.get_mut(handle) {
            Some(sprite) => {
                sprite.scale = scale;
                sprite.scale_y = None;
                true
            }
            None => false,
        }
    }

    /// Adapter for the newer three-word PalSpriteSetScale ABI. The legacy
    /// two-word export remains a separate uniform-scale operation.
    pub fn set_scale_xy(&mut self, handle: SpriteHandle, scale_x: f32, scale_y: f32) -> bool {
        if !self.set_scale(handle, scale_x) {
            return false;
        }
        self.get_mut(handle)
            .expect("set_scale checked the handle")
            .scale_y = Some(scale_y);
        true
    }

    pub fn set_alpha(&mut self, handle: SpriteHandle, alpha: u8) -> bool {
        if let Some(entry) = self.motion_entries.get_mut(&handle) {
            entry.anim_delta_color_a = None;
            if !entry.has_active_animation() {
                self.motion_entries.remove(&handle);
            }
        }
        self.set_alpha_direct(handle, alpha)
    }

    fn set_alpha_direct(&mut self, handle: SpriteHandle, alpha: u8) -> bool {
        match self.get_mut(handle) {
            Some(sprite) => {
                let raw = sprite.color.0 & 0x00FF_FFFF;
                sprite.color = PalColor::from_argb(raw | ((alpha as u32) << 24));
                true
            }
            None => false,
        }
    }

    pub fn tween_scale_to(&mut self, handle: SpriteHandle, scale: f32, duration_ms: u32) -> bool {
        let Some(sprite) = self.get(handle) else {
            return false;
        };
        let initial_entry = SpriteMotionEntry::from_sprite(handle, sprite);
        let from = sprite.scale;
        if duration_ms == 0 {
            return self.set_scale(handle, scale);
        }
        let entry = self.motion_entries.entry(handle).or_insert(initial_entry);
        entry.scale_b = scale;
        entry.anim_delta_scale = Some(SpriteScaleTween {
            from,
            to: scale,
            elapsed_ms: 0,
            duration_ms,
        });
        true
    }

    pub fn tween_alpha_to(&mut self, handle: SpriteHandle, alpha: u8, duration_ms: u32) -> bool {
        let Some(sprite) = self.get(handle) else {
            return false;
        };
        let initial_entry = SpriteMotionEntry::from_sprite(handle, sprite);
        if duration_ms == 0 {
            return self.set_alpha(handle, alpha);
        }
        let from = sprite.color.alpha();
        let entry = self.motion_entries.entry(handle).or_insert(initial_entry);
        entry.color_a = alpha;
        entry.anim_delta_color_a = Some(SpriteAlphaTween {
            from,
            to: alpha,
            elapsed_ms: 0,
            duration_ms,
        });
        true
    }

    pub fn set_rotate(&mut self, handle: SpriteHandle, angle_z: f32) -> bool {
        match self.get_mut(handle) {
            Some(sprite) => {
                sprite.rotation.z = angle_z;
                true
            }
            None => false,
        }
    }

    pub fn set_rotate_ex(&mut self, handle: SpriteHandle, x: f32, y: f32, z: f32) -> bool {
        match self.get_mut(handle) {
            Some(sprite) => {
                sprite.rotation = PalVec3::from_f32(x, y, z);
                true
            }
            None => false,
        }
    }

    pub fn set_render_mode(&mut self, handle: SpriteHandle, render_mode: PalRenderMode) -> bool {
        match self.get_mut(handle) {
            Some(sprite) => {
                sprite.render_mode = render_mode;
                true
            }
            None => false,
        }
    }

    /// PalSpriteHit uses the unscaled selected rectangle. Failed native pixel
    /// reads return 0xffffffff, so an unavailable texel accepts an alpha hit.
    pub fn hit(&self, handle: SpriteHandle, x: i32, y: i32, alpha_test: u32) -> i32 {
        let Some(sprite) = self.get(handle) else {
            return 0;
        };
        if !sprite.visible {
            return 0;
        }
        let sprite = sprite.with_action_lanes_applied();
        let local_x = x.wrapping_sub(
            sprite
                .offset
                .x
                .wrapping_add(native_truncate_i32(sprite.position.x)),
        );
        let local_y = y.wrapping_sub(
            sprite
                .offset
                .y
                .wrapping_add(native_truncate_i32(sprite.position.y)),
        );
        if local_x < 0
            || local_y < 0
            || local_x
                >= sprite
                    .source_rect
                    .right
                    .wrapping_sub(sprite.source_rect.left)
            || local_y
                >= sprite
                    .source_rect
                    .bottom
                    .wrapping_sub(sprite.source_rect.top)
        {
            return 0;
        }
        if alpha_test == 0 {
            return 1;
        }
        let source_x = sprite.source_rect.left.wrapping_add(local_x);
        let source_y = sprite.source_rect.top.wrapping_add(local_y);
        let Some(surface) = self.surface(sprite.surface) else {
            return 1;
        };
        if source_x < 0
            || source_y < 0
            || source_x as u32 >= surface.width
            || source_y as u32 >= surface.height
        {
            return 1;
        }
        let index = (source_y as usize * surface.width as usize + source_x as usize) * 4 + 3;
        i32::from(surface.pixels[index] != 0)
    }

    pub fn lock(&mut self, handle: SpriteHandle) -> Option<&mut SpriteSurface> {
        let surface = {
            let sprite = self.sprites.get_mut(&handle)?;
            sprite.locked = true;
            sprite.surface
        };
        self.surfaces.get_mut(&surface)
    }

    pub fn unlock(&mut self, handle: SpriteHandle) -> bool {
        match self.sprites.get_mut(&handle) {
            Some(sprite) => {
                sprite.locked = false;
                true
            }
            None => false,
        }
    }

    pub fn set_buffer(
        &mut self,
        handle: SpriteHandle,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> bool {
        let Some(surface_id) = self.get(handle).map(|sprite| sprite.surface) else {
            return false;
        };
        let Some(surface) = self.surface_mut(surface_id) else {
            return false;
        };
        let Some(expected) = (width as usize)
            .checked_mul(height as usize)
            .and_then(|pixels| pixels.checked_mul(4))
        else {
            return false;
        };
        if rgba.len() < expected {
            return false;
        }
        blit_rgba_to_surface(surface, x, y, width, height, rgba, BlendMode::CopyRgba)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn composite_rgba_to_sprite(
        &mut self,
        handle: SpriteHandle,
        dst_x: i32,
        dst_y: i32,
        src_width: u32,
        src_height: u32,
        rgba: &[u8],
        src_x: i32,
        src_y: i32,
        width: u32,
        height: u32,
    ) -> bool {
        if rgba.len() < src_width as usize * src_height as usize * 4 {
            return false;
        }
        let Some(surface_id) = self.get(handle).map(|sprite| sprite.surface) else {
            return false;
        };
        let Some(surface) = self.surface_mut(surface_id) else {
            return false;
        };
        blit_surface_to_surface(
            surface,
            dst_x,
            dst_y,
            (src_width, src_height, rgba),
            src_x,
            src_y,
            width,
            height,
            BlendMode::SourceOver,
        )
    }

    pub fn paint(&mut self, handle: SpriteHandle, x: i32, y: i32, color: PalColor) -> bool {
        let Some(surface_id) = self.get(handle).map(|sprite| sprite.surface) else {
            return false;
        };
        let Some(surface) = self.surface_mut(surface_id) else {
            return false;
        };
        if x < 0 || y < 0 || x as u32 >= surface.width || y as u32 >= surface.height {
            return false;
        }
        let rgba = color.to_rgba_u8();
        let idx = ((y as u32 * surface.width + x as u32) * 4) as usize;
        surface.pixels_mut()[idx..idx + 4].copy_from_slice(&rgba);
        true
    }

    /// Replace all pixels in a half-open surface rectangle; a missing rectangle
    /// paints the complete surface. This is PAL paint, not alpha compositing.
    pub fn paint_rect(
        &mut self,
        handle: SpriteHandle,
        rect: Option<PalRect>,
        color: PalColor,
    ) -> bool {
        let Some(surface_id) = self.get(handle).map(|sprite| sprite.surface) else {
            return false;
        };
        let Some(surface) = self.surface_mut(surface_id) else {
            return false;
        };
        let width = surface.width;
        let height = surface.height;
        let rect = rect.unwrap_or(PalRect::new(0, 0, width as i32, height as i32));
        let left = rect.left.clamp(0, width as i32) as usize;
        let right = rect.right.clamp(0, width as i32) as usize;
        let top = rect.top.clamp(0, height as i32) as usize;
        let bottom = rect.bottom.clamp(0, height as i32) as usize;
        if right <= left || bottom <= top {
            return true;
        }
        let rgba = color.to_rgba_u8();
        let pixels = surface.pixels_mut();
        for y in top..bottom {
            let start = (y * width as usize + left) * 4;
            let end = (y * width as usize + right) * 4;
            for pixel in pixels[start..end].chunks_exact_mut(4) {
                pixel.copy_from_slice(&rgba);
            }
        }
        true
    }

    pub fn mask_alpha(
        &mut self,
        dst: SpriteHandle,
        dst_x: i32,
        dst_y: i32,
        width: u32,
        height: u32,
        mask: SpriteHandle,
        mask_x: i32,
        mask_y: i32,
    ) -> bool {
        let Some(dst_surface_id) = self.get(dst).map(|sprite| sprite.surface) else {
            return false;
        };
        let Some(mask_surface_id) = self.get(mask).map(|sprite| sprite.surface) else {
            return false;
        };
        let Some(mask_pixels) = self
            .surface(mask_surface_id)
            .map(|surface| (surface.width, surface.height, surface.pixels.clone()))
        else {
            return false;
        };
        let Some(dst_surface) = self.surface_mut(dst_surface_id) else {
            return false;
        };
        mask_alpha_surface(
            dst_surface,
            dst_x,
            dst_y,
            width,
            height,
            mask_pixels,
            mask_x,
            mask_y,
        )
    }

    pub fn copy_sprite_to_sprite_rgb(
        &mut self,
        dst: SpriteHandle,
        dst_x: i32,
        dst_y: i32,
        src: SpriteHandle,
        src_x: i32,
        src_y: i32,
        width: u32,
        height: u32,
    ) -> bool {
        self.blit_sprite_to_sprite(
            dst,
            dst_x,
            dst_y,
            src,
            src_x,
            src_y,
            width,
            height,
            BlendMode::CopyRgba,
        )
    }

    pub fn mix_sprite_to_sprite(
        &mut self,
        dst: SpriteHandle,
        dst_x: i32,
        dst_y: i32,
        src: SpriteHandle,
        src_x: i32,
        src_y: i32,
        width: u32,
        height: u32,
    ) -> bool {
        self.blit_sprite_to_sprite(
            dst,
            dst_x,
            dst_y,
            src,
            src_x,
            src_y,
            width,
            height,
            BlendMode::AlphaRgb,
        )
    }

    /// Typed arena adapter for PalMixSpriteToSpriteEx's eight native words.
    /// Invalid negative extents are rejected instead of reproducing native
    /// invalid-memory access; overlapping surfaces use an immutable source copy.
    #[allow(clippy::too_many_arguments)]
    pub fn mix_sprite_to_sprite_ex(
        &mut self,
        dst: SpriteHandle,
        dst_x: i32,
        dst_y: i32,
        width: i32,
        height: i32,
        src: SpriteHandle,
        src_x: i32,
        src_y: i32,
    ) -> bool {
        if width <= 0 || height <= 0 {
            return false;
        }
        self.blit_sprite_to_sprite(
            dst,
            dst_x,
            dst_y,
            src,
            src_x,
            src_y,
            width as u32,
            height as u32,
            BlendMode::SourceOver,
        )
    }

    pub fn copy_sprite_rect_to_sprite_rgb(&mut self, dst: SpriteHandle, src: SpriteHandle) -> bool {
        let Some(src_sprite) = self.get(src) else {
            return false;
        };
        self.copy_sprite_to_sprite_rgb(
            dst,
            src_sprite.position.x as i32,
            src_sprite.position.y as i32,
            src,
            src_sprite.source_rect.left,
            src_sprite.source_rect.top,
            src_sprite.cell_size.width,
            src_sprite.cell_size.height,
        )
    }

    fn blit_sprite_to_sprite(
        &mut self,
        dst: SpriteHandle,
        dst_x: i32,
        dst_y: i32,
        src: SpriteHandle,
        src_x: i32,
        src_y: i32,
        width: u32,
        height: u32,
        mode: BlendMode,
    ) -> bool {
        let Some(dst_surface_id) = self.get(dst).map(|sprite| sprite.surface) else {
            return false;
        };
        let Some(src_surface_id) = self.get(src).map(|sprite| sprite.surface) else {
            return false;
        };
        let Some((src_width, src_height, src_pixels)) = self
            .surface(src_surface_id)
            .map(|surface| (surface.width, surface.height, surface.pixels.clone()))
        else {
            return false;
        };
        let Some(dst_surface) = self.surface_mut(dst_surface_id) else {
            return false;
        };
        blit_surface_to_surface(
            dst_surface,
            dst_x,
            dst_y,
            (src_width, src_height, &src_pixels),
            src_x,
            src_y,
            width,
            height,
            mode,
        )
    }

    pub fn commands(&self) -> Vec<DrawCommand> {
        let mut nodes = self.render_nodes.values().collect::<Vec<_>>();
        // PAL keeps same-priority sprites in creation/render-entry order; later
        // entries are drawn later.  The SYSTEM menu depends on this because it
        // creates SYS_BASE at z=0 over the older TITLE_BASE at z=0.
        nodes.sort_by_key(|node| (node.priority, node.id.0));
        nodes
            .into_iter()
            .filter_map(|node| {
                self.sprites.get(&node.sprite).and_then(|sprite| {
                    let mut command = sprite.draw_command(self)?;
                    if sprite.combined_lanes().position[2] == 0.0 {
                        if let DrawCommand::Sprite(ref mut draw) = command {
                            draw.priority = node.priority;
                        }
                    }
                    Some(command)
                })
            })
            .collect()
    }

    pub fn render_nodes(&self) -> impl Iterator<Item = &RenderNode> {
        self.render_nodes.values()
    }

    pub fn iter_sprites(&self) -> impl Iterator<Item = (&SpriteHandle, &PalSprite)> {
        self.sprites.iter()
    }

    pub fn surface_count(&self) -> usize {
        self.surfaces.len()
    }

    pub fn render_node_count(&self) -> usize {
        self.render_nodes.len()
    }

    pub fn len(&self) -> usize {
        self.sprites.len()
    }

    pub fn motion_entry_count(&self) -> usize {
        self.motion_entries.len()
    }

    /// Stop the named-ANI motion without committing or resetting its current
    /// properties. The Game wrapper owns its separate animation bookkeeping.
    pub fn stop_motion(&mut self, handle: SpriteHandle) -> bool {
        self.motion_entries.remove(&handle).is_some()
    }

    pub fn is_empty(&self) -> bool {
        self.sprites.is_empty()
    }

    fn allocate_handle(&mut self) -> SpriteHandle {
        let handle = SpriteHandle(self.next_handle);
        self.next_handle = self
            .next_handle
            .checked_add(1)
            .expect("sprite handle space exhausted");
        handle
    }

    fn allocate_render_node(&mut self) -> RenderNodeId {
        let id = RenderNodeId(self.next_render_node);
        self.next_render_node = self
            .next_render_node
            .checked_add(1)
            .expect("render node id space exhausted");
        id
    }

    fn update_render_node_priority(&mut self, handle: SpriteHandle, priority: i32) {
        let Some(sprite) = self.sprites.get(&handle) else {
            return;
        };
        if let Some(node_id) = sprite.render_node {
            if let Some(node) = self.render_nodes.get_mut(&node_id) {
                node.priority = priority;
            }
        }
    }
}

/// CVTTSS2SI returns the integer-indefinite value on NaN or overflow.
fn native_truncate_i32(value: f32) -> i32 {
    if (-2_147_483_648.0..2_147_483_648.0).contains(&value) {
        value as i32
    } else {
        i32::MIN
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BlendMode {
    CopyRgba,
    AlphaRgb,
    SourceOver,
}

fn blit_rgba_to_surface(
    dst: &mut SpriteSurface,
    dst_x: i32,
    dst_y: i32,
    src_width: u32,
    src_height: u32,
    src_pixels: &[u8],
    mode: BlendMode,
) -> bool {
    blit_surface_to_surface(
        dst,
        dst_x,
        dst_y,
        (src_width, src_height, src_pixels),
        0,
        0,
        src_width,
        src_height,
        mode,
    )
}

fn mask_alpha_surface(
    dst: &mut SpriteSurface,
    dst_x: i32,
    dst_y: i32,
    width: u32,
    height: u32,
    mask: (u32, u32, Arc<Vec<u8>>),
    mask_x: i32,
    mask_y: i32,
) -> bool {
    let (mask_width, mask_height, mask_pixels) = mask;
    let Some(region) = ClipRegion::new(
        dst_x,
        dst_y,
        mask_x,
        mask_y,
        width,
        height,
        dst.width,
        dst.height,
        mask_width,
        mask_height,
    ) else {
        return false;
    };

    let dst_width = dst.width;
    let dst_pixels = dst.pixels_mut();
    for row in 0..region.height {
        let dst_row = region.dst_y + row;
        let mask_row = region.src_y + row;
        for col in 0..region.width {
            let dst_col = region.dst_x + col;
            let mask_col = region.src_x + col;
            let dst_idx = ((dst_row * dst_width + dst_col) * 4 + 3) as usize;
            // PalSpriteMaskAlpha consumes native BGRA byte 0 (blue).
            // Portable surfaces are RGBA, so the matching channel is byte 2.
            let mask_idx = ((mask_row * mask_width + mask_col) * 4 + 2) as usize;
            dst_pixels[dst_idx] =
                (((u16::from(dst_pixels[dst_idx])) * u16::from(mask_pixels[mask_idx])) >> 8) as u8;
        }
    }
    true
}

fn blit_surface_to_surface(
    dst: &mut SpriteSurface,
    dst_x: i32,
    dst_y: i32,
    src: (u32, u32, &[u8]),
    src_x: i32,
    src_y: i32,
    width: u32,
    height: u32,
    mode: BlendMode,
) -> bool {
    let (src_width, src_height, src_pixels) = src;
    let Some(region) = ClipRegion::new(
        dst_x, dst_y, src_x, src_y, width, height, dst.width, dst.height, src_width, src_height,
    ) else {
        return false;
    };

    let dst_width = dst.width;
    let dst_pixels = dst.pixels_mut();
    for row in 0..region.height {
        let dst_row = region.dst_y + row;
        let src_row = region.src_y + row;
        for col in 0..region.width {
            let dst_col = region.dst_x + col;
            let src_col = region.src_x + col;
            let dst_idx = ((dst_row * dst_width + dst_col) * 4) as usize;
            let src_idx = ((src_row * src_width + src_col) * 4) as usize;
            blend_pixel(
                &mut dst_pixels[dst_idx..dst_idx + 4],
                &src_pixels[src_idx..src_idx + 4],
                mode,
            );
        }
    }
    true
}

fn blend_pixel(dst: &mut [u8], src: &[u8], mode: BlendMode) {
    match mode {
        BlendMode::CopyRgba => dst.copy_from_slice(src),
        BlendMode::AlphaRgb => {
            let alpha = u16::from(src[3]);
            if alpha == 0 {
                return;
            }
            for channel in 0..3 {
                let src_value = u16::from(src[channel]);
                let dst_value = u16::from(dst[channel]);
                dst[channel] = (((dst_value * (255 - alpha)) + (src_value * alpha)) >> 8) as u8;
            }
        }
        BlendMode::SourceOver => {
            let src_alpha = u32::from(src[3]);
            let dst_alpha = u32::from(dst[3]);
            let retained_alpha = dst_alpha * (255 - src_alpha) / 255;
            let out_alpha = src_alpha + retained_alpha;
            if out_alpha == 0 {
                dst.fill(0);
                return;
            }
            for channel in 0..3 {
                let numerator =
                    u32::from(src[channel]) * src_alpha + u32::from(dst[channel]) * retained_alpha;
                dst[channel] = (numerator / out_alpha) as u8;
            }
            dst[3] = out_alpha as u8;
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ClipRegion {
    dst_x: u32,
    dst_y: u32,
    src_x: u32,
    src_y: u32,
    width: u32,
    height: u32,
}

impl ClipRegion {
    #[allow(clippy::too_many_arguments)]
    fn new(
        dst_x: i32,
        dst_y: i32,
        src_x: i32,
        src_y: i32,
        width: u32,
        height: u32,
        dst_width: u32,
        dst_height: u32,
        src_width: u32,
        src_height: u32,
    ) -> Option<Self> {
        let mut dst_x = i64::from(dst_x);
        let mut dst_y = i64::from(dst_y);
        let mut src_x = i64::from(src_x);
        let mut src_y = i64::from(src_y);
        let mut width = i64::from(width);
        let mut height = i64::from(height);

        if width <= 0 || height <= 0 {
            return None;
        }
        if src_x < 0 {
            dst_x -= src_x;
            width += src_x;
            src_x = 0;
        }
        if src_y < 0 {
            dst_y -= src_y;
            height += src_y;
            src_y = 0;
        }
        if dst_x < 0 {
            src_x -= dst_x;
            width += dst_x;
            dst_x = 0;
        }
        if dst_y < 0 {
            src_y -= dst_y;
            height += dst_y;
            dst_y = 0;
        }

        width = width
            .min(i64::from(dst_width).saturating_sub(dst_x))
            .min(i64::from(src_width).saturating_sub(src_x));
        height = height
            .min(i64::from(dst_height).saturating_sub(dst_y))
            .min(i64::from(src_height).saturating_sub(src_y));
        if width <= 0 || height <= 0 {
            return None;
        }

        Some(Self {
            dst_x: dst_x as u32,
            dst_y: dst_y as u32,
            src_x: src_x as u32,
            src_y: src_y as u32,
            width: width as u32,
            height: height as u32,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SpriteDesc {
    pub kind: SpriteKind,
    pub texture_id: SceneTextureId,
    pub texture_width: u32,
    pub texture_height: u32,
    pub cell_width: u32,
    pub cell_height: u32,
    pub source_rect: Option<PalRect>,
    pub position: PalVec3,
    pub offset: PalPoint2,
    pub center_offset: PalPoint2,
    pub visible: bool,
    pub color: PalColor,
    pub base_priority: i32,
    pub scale: f32,
    /// Newer PAL's independent Y scale. None retains legacy uniform scaling.
    pub scale_y: Option<f32>,
    pub center_scale: bool,
    pub rotation: PalVec3,
    pub render_mode: PalRenderMode,
    pub option_type: u32,
    pub info_extra: u32,
    pub source_name: String,
    pub native_projection: Option<(f32, f32)>,
    pub smooth_upscale: bool,
}

impl SpriteDesc {
    pub fn new(texture_id: SceneTextureId, texture_width: u32, texture_height: u32) -> Self {
        Self {
            kind: SpriteKind::Static,
            texture_id,
            texture_width,
            texture_height,
            cell_width: texture_width,
            cell_height: texture_height,
            source_rect: None,
            position: PalVec3::default(),
            offset: PalPoint2::default(),
            center_offset: PalPoint2::default(),
            visible: false,
            color: PalColor::WHITE,
            base_priority: 0,
            scale: 1.0,
            scale_y: None,
            center_scale: true,
            rotation: PalVec3::default(),
            render_mode: PalRenderMode::DEFAULT,
            option_type: 0,
            info_extra: 0,
            source_name: String::new(),
            native_projection: None,
            smooth_upscale: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PalSprite {
    registered_cell_indices: bool,
    pub handle: SpriteHandle,
    pub kind: SpriteKind,
    pub surface: SpriteSurfaceId,
    pub render_node: Option<RenderNodeId>,
    pub option_aux: Option<SpriteOptionAux>,
    pub transition_block: u32,
    pub fx_effect: Option<SpriteFxHandle>,
    pub locked: bool,
    pub visible: bool,
    /// Value copied by native `abi::win32::PalSpriteGetRect` to its output.
    pub source_rect: PalRect,
    pub color: PalColor,
    pub position: PalVec3,
    pub offset: PalPoint2,
    pub texture_size: PalSize,
    pub cell_size: PalSize,
    pub center_offset: PalPoint2,
    pub base_priority: i32,
    pub scale: f32,
    /// Newer PAL's independent Y scale. None retains legacy uniform scaling.
    pub scale_y: Option<f32>,
    pub center_scale: bool,
    pub rotation: PalVec3,
    pub render_mode: PalRenderMode,
    pub option_type: u32,
    pub info_extra: u32,
    pub source_name: String,
    pub native_projection: Option<(f32, f32)>,
    pub smooth_upscale: bool,
    /// Game action lanes compose with the base sprite properties before projection.
    pub action_lanes: SpriteActionLanes,
    /// Motion program (`.ani`) lanes: committed plus in-flight deltas.
    pub motion_lanes: SpriteActionLanes,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SpriteActionLanes {
    pub position: [f32; 3],
    pub shake: [f32; 2],
    pub rotation: [f32; 3],
    pub scale: f32,
    pub alpha: i16,
    /// Red, green and blue words; only motion programs animate them.
    pub rgb: [i16; 3],
    /// Source rect left/top/right/bottom deltas (Game action type 16).
    /// Action snapshots do not carry this lane.
    pub rect: [i32; 4],
}

impl SpriteActionLanes {
    pub fn add(&mut self, other: Self) {
        for axis in 0..3 {
            self.position[axis] += other.position[axis];
            self.rotation[axis] += other.rotation[axis];
        }
        for axis in 0..2 {
            self.shake[axis] += other.shake[axis];
        }
        self.scale += other.scale;
        self.alpha = self.alpha.wrapping_add(other.alpha);
        for channel in 0..3 {
            self.rgb[channel] = self.rgb[channel].wrapping_add(other.rgb[channel]);
        }
        for edge in 0..4 {
            self.rect[edge] = self.rect[edge].wrapping_add(other.rect[edge]);
        }
    }

    pub fn from_motion(delta: &crate::ani::MotionDelta) -> Self {
        Self {
            position: delta.position.map(|value| value as f32),
            shake: [0.0; 2],
            rotation: delta.rotation.map(|value| value as f32),
            scale: delta.scale,
            alpha: delta.color[0],
            rgb: [delta.color[1], delta.color[2], delta.color[3]],
            rect: [0; 4],
        }
    }
}

impl PalSprite {
    /// Fold `lanes` into the base properties; the live lanes are kept.
    pub(crate) fn commit_action_lanes(&mut self, lanes: SpriteActionLanes) {
        let composed = self.with_lanes(lanes);
        self.position = composed.position;
        self.rotation = composed.rotation;
        self.scale = composed.scale;
        self.scale_y = composed.scale_y;
        self.color = composed.color;
        self.source_rect = composed.source_rect;
    }

    /// Action and motion lanes that currently compose with the base state.
    pub fn combined_lanes(&self) -> SpriteActionLanes {
        let mut lanes = self.action_lanes;
        lanes.add(self.motion_lanes);
        lanes
    }

    fn has_lanes(&self) -> bool {
        self.action_lanes != SpriteActionLanes::default()
            || self.motion_lanes != SpriteActionLanes::default()
    }

    fn with_action_lanes_applied(&self) -> Self {
        self.with_lanes(self.combined_lanes())
    }

    fn with_lanes(&self, lanes: SpriteActionLanes) -> Self {
        let mut sprite = self.clone();
        sprite.action_lanes = SpriteActionLanes::default();
        sprite.motion_lanes = SpriteActionLanes::default();
        sprite.position.x += lanes.position[0] + lanes.shake[0];
        sprite.position.y += lanes.position[1] + lanes.shake[1];
        sprite.position.z += lanes.position[2];
        sprite.rotation.x += lanes.rotation[0];
        sprite.rotation.y += lanes.rotation[1];
        sprite.rotation.z += lanes.rotation[2];
        sprite.scale += lanes.scale;
        if let Some(scale_y) = &mut sprite.scale_y {
            *scale_y += lanes.scale;
        }
        let channel = |shift: u32, delta: i16| {
            (i16::from((sprite.color.0 >> shift) as u8).wrapping_add(delta).clamp(0, 255) as u32) << shift
        };
        if lanes.rect != [0; 4] {
            let rect = sprite.source_rect;
            sprite.source_rect = PalRect::new(
                rect.left.wrapping_add(lanes.rect[0]),
                rect.top.wrapping_add(lanes.rect[1]),
                rect.right.wrapping_add(lanes.rect[2]),
                rect.bottom.wrapping_add(lanes.rect[3]),
            );
        }
        sprite.color = PalColor::from_argb(
            channel(24, lanes.alpha)
                | channel(16, lanes.rgb[0])
                | channel(8, lanes.rgb[1])
                | channel(0, lanes.rgb[2]),
        );
        sprite
    }

    pub fn new(handle: SpriteHandle, render_node: RenderNodeId, desc: SpriteDesc) -> Self {
        let texture_size = PalSize::new(desc.texture_width.max(1), desc.texture_height.max(1));
        let cell_size = PalSize::new(desc.cell_width.max(1), desc.cell_height.max(1));
        let source_rect = desc
            .source_rect
            .unwrap_or_else(|| PalRect::new(0, 0, cell_size.width as i32, cell_size.height as i32));
        Self {
            handle,
            registered_cell_indices: false,
            kind: desc.kind,
            surface: SpriteSurfaceId(desc.texture_id.0),
            render_node: Some(render_node),
            option_aux: None,
            transition_block: 0,
            fx_effect: None,
            locked: false,
            visible: desc.visible,
            source_rect,
            color: desc.color,
            position: desc.position,
            offset: desc.offset,
            texture_size,
            cell_size,
            center_offset: desc.center_offset,
            base_priority: desc.base_priority,
            scale: desc.scale,
            scale_y: desc.scale_y,
            center_scale: desc.center_scale,
            rotation: desc.rotation,
            render_mode: desc.render_mode,
            option_type: desc.option_type,
            info_extra: desc.info_extra,
            source_name: desc.source_name,
            native_projection: desc.native_projection,
            smooth_upscale: desc.smooth_upscale,
            action_lanes: SpriteActionLanes::default(),
            motion_lanes: SpriteActionLanes::default(),
        }
    }

    pub fn info(&self) -> PalSpriteInfo {
        if self.has_lanes() {
            return self.with_action_lanes_applied().info();
        }
        PalSpriteInfo {
            visible: self.visible,
            color: self.color,
            position: self.position,
            offset: self.offset,
            texture_size: self.texture_size,
            cell_size: self.cell_size,
            center_offset: self.center_offset,
            priority: self.base_priority,
            scale: self.scale,
            scale_y: self.scale_y,
            center_scale: self.center_scale,
            rotation: self.rotation,
            render_mode: self.render_mode,
            extra: self.info_extra,
            source_name: self.source_name.clone(),
            native_projection: self.native_projection,
            smooth_upscale: self.smooth_upscale,
        }
    }

    pub fn set_info(&mut self, info: PalSpriteInfo) {
        self.visible = info.visible;
        self.color = info.color;
        self.position = info.position;
        self.offset = info.offset;
        self.texture_size = info.texture_size.clamped_nonzero();
        self.cell_size = info.cell_size.clamped_nonzero();
        self.center_offset = info.center_offset;
        self.base_priority = info.priority;
        self.scale = info.scale;
        self.scale_y = info.scale_y;
        self.center_scale = info.center_scale;
        self.rotation = info.rotation;
        self.render_mode = info.render_mode;
        self.info_extra = info.extra;
        self.source_name = info.source_name;
        self.native_projection = info.native_projection;
        self.smooth_upscale = info.smooth_upscale;
    }

    pub fn frame_count(&self, axis: PalAnimationAxis) -> u16 {
        let count = match axis {
            PalAnimationAxis::Horizontal => self.texture_size.width / self.cell_size.width.max(1),
            PalAnimationAxis::Vertical => self.texture_size.height / self.cell_size.height.max(1),
        };
        count.clamp(1, u16::MAX as u32) as u16
    }

    pub fn set_rect(&mut self, rect: Option<PalRect>) {
        self.source_rect = rect.unwrap_or_else(|| {
            PalRect::new(
                0,
                0,
                self.texture_size.width as i32,
                self.texture_size.height as i32,
            )
        });
    }

    pub fn rect_set_pos(&mut self, cell_x: i32, cell_y: i32) {
        if self.registered_cell_indices {
            let origin = |index: i32, size: u32, cell: u32| {
                let cell = cell.max(1);
                let count = size / cell;
                if index >= count as i32 {
                    if size % cell == 1 {
                        cell.wrapping_mul(count.wrapping_sub(1))
                    } else {
                        size.wrapping_sub(cell)
                    }
                } else {
                    (index as u32).wrapping_mul(cell)
                }
            };
            let left = origin(cell_x, self.texture_size.width, self.cell_size.width);
            let top = origin(cell_y, self.texture_size.height, self.cell_size.height);
            self.source_rect = PalRect::new(
                left as i32,
                top as i32,
                left.wrapping_add(self.cell_size.width) as i32,
                top.wrapping_add(self.cell_size.height) as i32,
            );
            return;
        }
        let mut left = (cell_x as u32).wrapping_mul(self.cell_size.width);
        let mut top = (cell_y as u32).wrapping_mul(self.cell_size.height);
        if left >= self.texture_size.width {
            left = self.texture_size.width.wrapping_sub(self.cell_size.width);
        }
        if top >= self.texture_size.height {
            top = self.texture_size.height.wrapping_sub(self.cell_size.height);
        }
        self.source_rect = PalRect::new(
            left as i32,
            top as i32,
            left.wrapping_add(self.cell_size.width) as i32,
            top.wrapping_add(self.cell_size.height) as i32,
        );
    }

    pub fn set_center_offset(&mut self, x: i32, y: i32) {
        self.center_offset = PalPoint2 { x: -x, y: -y };
    }

    pub fn effective_position(&self) -> PalPoint2 {
        let lanes = self.combined_lanes();
        PalPoint2 {
            x: self.offset.x.saturating_add(
                (self.position.x + lanes.position[0] + lanes.shake[0]) as i32,
            ),
            y: self.offset.y.saturating_add(
                (self.position.y + lanes.position[1] + lanes.shake[1]) as i32,
            ),
        }
    }

    pub fn effective_priority(&self) -> i32 {
        self.base_priority.wrapping_add(native_truncate_i32(
            self.position.z + self.combined_lanes().position[2],
        ))
    }

    pub fn draw_command(&self, sprites: &SpriteSystem) -> Option<DrawCommand> {
        if self.transition_block != 0 || !self.visible || self.locked {
            return None;
        }
        self.draw_command_inner(sprites)
    }

    fn draw_command_transition(&self, sprites: &SpriteSystem) -> Option<DrawCommand> {
        if !self.visible || self.locked {
            return None;
        }
        self.draw_command_inner(sprites)
    }

    fn draw_command_inner(&self, sprites: &SpriteSystem) -> Option<DrawCommand> {
        if self.has_lanes() {
            return self.with_action_lanes_applied().draw_command_inner(sprites);
        }
        let render_mode = if self.registered_cell_indices || self.render_mode.raw() <= 4 {
            self.render_mode.raw()
        } else {
            1
        };
        let surface = sprites.surface(self.surface)?;
        let dst_pos = self.effective_position();
        let width = self.source_rect.width() as f32;
        let height = self.source_rect.height() as f32;
        let scale = self.scale.max(0.0);
        let rotation_z = self.rotation.z.rem_euclid(512.0);
        let (draw_scale, draw_scale_y) = if let Some(scale_y) = self.scale_y {
            // Newer D3D sprites carry two independent scale lanes, without the
            // legacy software renderer's uniform scratch-buffer restriction.
            (scale, scale_y.max(0.0))
        } else {
            let uniform = pal_scratch_clamped_scale(width, height, scale);
            (uniform, uniform)
        };
        let mut dst = if self.kind == SpriteKind::SolidLayer {
            RectF::new(0.0, 0.0, surface.width as f32, surface.height as f32)
        } else if rotation_z != 0.0 {
            // PAL.dll `sub_1028D740` routes rotated sprites through a temporary
            // square work buffer. The work-buffer side is based on the source
            // diagonal times sprite.scale and is clamped to 4096 before the PAL
            // blit. `sub_1028EB30` samples the source into that square around
            // the source center; when rendering the visible source rectangle
            // directly, the equivalent content top-left is still the ordinary
            // centered rectangular scale offset.
            RectF::new(
                dst_pos.x as f32 - ((width * draw_scale) - width) * 0.5,
                dst_pos.y as f32 - ((height * draw_scale_y) - height) * 0.5,
                width * draw_scale,
                height * draw_scale_y,
            )
        } else if self.center_scale
            && ((draw_scale - 1.0).abs() > f32::EPSILON
                || (draw_scale_y - 1.0).abs() > f32::EPSILON)
        {
            // PAL.dll `sub_1028D740` does not scale from the top-left corner.
            // For scale-only sprites it creates a centered square scratch, but
            // the source pixels are centered within that scratch by
            // `sub_1028EB30`; drawing the visible source rect directly must use
            // the visible scaled width/height offset.
            RectF::new(
                dst_pos.x as f32 - ((width * draw_scale) - width) * 0.5,
                dst_pos.y as f32 - ((height * draw_scale_y) - height) * 0.5,
                width * draw_scale,
                height * draw_scale_y,
            )
        } else {
            RectF::new(
                dst_pos.x as f32,
                dst_pos.y as f32,
                width * draw_scale,
                height * draw_scale_y,
            )
        };
        if let Some((project_x, project_y)) = self.native_projection {
            dst = RectF::new(
                dst.x * project_x,
                dst.y * project_y,
                dst.w * project_x,
                dst.h * project_y,
            );
        }
        if !dst.is_drawable() {
            return None;
        }
        trace_sprite_draw_invariant(self, dst, surface.width(), surface.height());
        if self.source_name.starts_with("ST")
            || self.source_name.starts_with("BK")
            || self.source_name.starts_with("EV")
            || self.source_name.starts_with("FA")
        {
            log::debug!(
                "[trace-sprite-draw] handle={} name={} pos=({},{},{}) src={}x{} scale={:.3} native_projection={:?} dst=({:.1},{:.1},{:.1},{:.1}) alpha={} priority={}",
                self.handle.0,
                self.source_name,
                dst_pos.x,
                dst_pos.y,
                self.position.z,
                width,
                height,
                scale,
                self.native_projection,
                dst.x,
                dst.y,
                dst.w,
                dst.h,
                self.color.alpha(),
                self.effective_priority()
            );
        }
        let tex_w = surface.width().max(1) as f32;
        let tex_h = surface.height().max(1) as f32;
        let src = RectF::new(
            self.source_rect.left as f32 / tex_w,
            self.source_rect.top as f32 / tex_h,
            self.source_rect.width() as f32 / tex_w,
            self.source_rect.height() as f32 / tex_h,
        );
        Some(DrawCommand::Sprite(SpriteDraw {
            texture_id: surface.texture_id,
            smooth_upscale: self.smooth_upscale,
            priority: self.effective_priority(),
            dst,
            src,
            source_rect: [
                self.source_rect.left,
                self.source_rect.top,
                self.source_rect.right,
                self.source_rect.bottom,
            ],
            texture_size: [self.texture_size.width, self.texture_size.height],
            cell_size: [self.cell_size.width, self.cell_size.height],
            position: [self.position.x, self.position.y, self.position.z],
            offset: [self.offset.x, self.offset.y],
            color: self.color.to_rgba_f32(),
            scale: self.scale,
            rotation: [self.rotation.x, self.rotation.y, self.rotation.z],
            center_offset: [self.center_offset.x as f32, self.center_offset.y as f32],
            render_mode,
            blend_state: crate::scene::blend::NativeDrawState::default(),
            raster_fx: self
                .fx_effect
                .and_then(|handle| sprites.fx_effects.get(&handle))
                .filter(|effect| effect.active && effect.has_source)
                .map(|effect| effect.uniforms),
        }))
    }
}

fn pal_scratch_clamped_scale(width: f32, height: f32, scale: f32) -> f32 {
    if scale <= 0.0 || !scale.is_finite() {
        return 0.0;
    }
    let diagonal = (width.mul_add(width, height * height)).sqrt();
    if diagonal <= f32::EPSILON || !diagonal.is_finite() {
        return scale;
    }
    let scaled_side = diagonal * scale;
    if scaled_side <= 4096.0 {
        scale
    } else {
        4096.0 / diagonal
    }
}

fn trace_sprite_position_invariant(
    op: &str,
    handle: SpriteHandle,
    sprite: &PalSprite,
    x: f32,
    y: f32,
    z: f32,
) {
    if x.abs() <= 4096.0 && y.abs() <= 4096.0 && sprite.scale > 0.0 && sprite.scale <= 8.0 {
        return;
    }
    log::warn!(
        "[trace-sprite-invariant] op={op} handle={} source={:?} old_pos=({:.0},{:.0},{:.0}) new_pos=({x:.0},{y:.0},{z:.0}) scale={:.3} src_rect=({},{},{},{}) tex={}x{} cell={}x{}",
        handle.0,
        sprite.source_name,
        sprite.position.x,
        sprite.position.y,
        sprite.position.z,
        sprite.scale,
        sprite.source_rect.left,
        sprite.source_rect.top,
        sprite.source_rect.right,
        sprite.source_rect.bottom,
        sprite.texture_size.width,
        sprite.texture_size.height,
        sprite.cell_size.width,
        sprite.cell_size.height,
    );
}

fn trace_sprite_draw_invariant(
    sprite: &PalSprite,
    dst: RectF,
    surface_width: u32,
    surface_height: u32,
) {
    let rect_outside = sprite.source_rect.left < 0
        || sprite.source_rect.top < 0
        || sprite.source_rect.right > surface_width as i32
        || sprite.source_rect.bottom > surface_height as i32;
    let bad_source =
        sprite.source_rect.width() <= 0 || sprite.source_rect.height() <= 0 || rect_outside;
    let bad_dst = dst.x.abs() > 4096.0 || dst.y.abs() > 4096.0 || dst.w <= 0.0 || dst.h <= 0.0;
    let bad_scale =
        sprite.kind != SpriteKind::SolidLayer && (sprite.scale <= 0.0 || sprite.scale > 8.0);
    if !(bad_source || bad_dst || bad_scale) {
        return;
    }
    log::warn!(
        "[trace-sprite-invariant] op=draw source={:?} dst=({:.0},{:.0},{:.0}x{:.0}) pos=({:.0},{:.0},{:.0}) offset=({},{}) scale={:.3} src_rect=({},{},{},{}) surface={}x{} tex={}x{} cell={}x{} bad_source={} bad_dst={} bad_scale={}",
        sprite.source_name,
        dst.x,
        dst.y,
        dst.w,
        dst.h,
        sprite.position.x,
        sprite.position.y,
        sprite.position.z,
        sprite.offset.x,
        sprite.offset.y,
        sprite.scale,
        sprite.source_rect.left,
        sprite.source_rect.top,
        sprite.source_rect.right,
        sprite.source_rect.bottom,
        surface_width,
        surface_height,
        sprite.texture_size.width,
        sprite.texture_size.height,
        sprite.cell_size.width,
        sprite.cell_size.height,
        bad_source,
        bad_dst,
        bad_scale,
    );
}

#[derive(Clone, Debug)]
pub struct PalSpriteInfo {
    pub visible: bool,
    pub color: PalColor,
    pub position: PalVec3,
    pub offset: PalPoint2,
    pub texture_size: PalSize,
    pub cell_size: PalSize,
    pub center_offset: PalPoint2,
    pub priority: i32,
    pub scale: f32,
    pub scale_y: Option<f32>,
    pub center_scale: bool,
    pub rotation: PalVec3,
    pub render_mode: PalRenderMode,
    pub extra: u32,
    pub source_name: String,
    pub native_projection: Option<(f32, f32)>,
    pub smooth_upscale: bool,
}

#[derive(Clone, Debug)]
pub struct SpriteSurface {
    pub id: SpriteSurfaceId,
    pub texture_id: SceneTextureId,
    pub generation: u64,
    pub width: u32,
    pub height: u32,
    pub format: SceneTextureFormat,
    pub pixels: Arc<Vec<u8>>,
    pub dirty: bool,
    /// Dense copy of `pixels`, valid only while `generation` is unchanged:
    /// any later pixel edit bumps the generation and drops back to `pixels`.
    hires: Option<(u64, HiresPixels)>,
}

impl SpriteSurface {
    pub fn from_scene_texture(texture: SceneTexture) -> Self {
        Self {
            id: SpriteSurfaceId(texture.id.0),
            texture_id: texture.id,
            generation: texture.generation,
            width: texture.width,
            height: texture.height,
            format: texture.format,
            pixels: texture.pixels,
            dirty: false,
            hires: None,
        }
    }

    pub fn rgba8(
        id: SpriteSurfaceId,
        generation: u64,
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    ) -> anyhow::Result<Self> {
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| anyhow::anyhow!("RGBA sprite dimensions overflow: {width}x{height}"))?;
        if expected == 0 {
            anyhow::bail!("RGBA sprite dimensions must be nonzero: {width}x{height}");
        }
        if pixels.len() != expected {
            anyhow::bail!(
                "RGBA sprite surface {:?} has {} bytes, expected {} for {}x{}",
                id,
                pixels.len(),
                expected,
                width,
                height
            );
        }
        Ok(Self {
            id,
            texture_id: SceneTextureId(id.0),
            generation,
            width,
            height,
            format: SceneTextureFormat::Rgba8,
            pixels: Arc::new(pixels),
            dirty: false,
            hires: None,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
        self.generation = self.generation.saturating_add(1).max(1);
    }

    pub fn pixels_mut(&mut self) -> &mut [u8] {
        self.mark_dirty();
        Arc::make_mut(&mut self.pixels).as_mut_slice()
    }

    pub fn to_scene_texture(&self) -> SceneTexture {
        SceneTexture {
            id: self.texture_id,
            generation: self.generation,
            width: self.width,
            height: self.height,
            format: self.format,
            pixels: self.pixels.clone(),
            hires: self.hires().cloned(),
        }
    }

    /// The dense copy, if it still matches the current pixels.
    pub fn hires(&self) -> Option<&HiresPixels> {
        self.hires
            .as_ref()
            .filter(|(generation, _)| *generation == self.generation)
            .map(|(_, hires)| hires)
    }

    /// Attach a dense copy of the current pixels. It must be exactly
    /// `density` times this surface's size; anything else is rejected.
    pub fn set_hires(&mut self, density: u32, pixels: Vec<u8>) -> bool {
        let expected = (self.width as usize * density as usize)
            .checked_mul(self.height as usize * density as usize)
            .and_then(|texels| texels.checked_mul(4));
        if !(2..=crate::scene::MAX_TEXT_DENSITY).contains(&density)
            || expected != Some(pixels.len())
        {
            self.hires = None;
            return false;
        }
        self.hires = Some((
            self.generation,
            HiresPixels {
                density,
                pixels: Arc::new(pixels),
            },
        ));
        true
    }
}

#[derive(Clone, Debug)]
pub struct RenderNode {
    pub id: RenderNodeId,
    pub sprite: SpriteHandle,
    pub priority: i32,
}

impl RenderNode {
    pub const fn new(id: RenderNodeId, sprite: SpriteHandle, priority: i32) -> Self {
        Self {
            id,
            sprite,
            priority,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpriteKind {
    Static,
    SolidLayer,
    MSprite { decoder: MSpriteHandle },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MSpriteDecoderHandle(pub u32);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpriteOptionAux {
    pub raw_type: u32,
    pub raw_value: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SpriteFxHandle(pub u32);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SpriteTransition {
    pub render_id: u32,
    pub state: i32,
    /// First native sprite is retained and fades in.
    pub first: Option<SpriteHandle>,
    /// Second native sprite is owned until cancellation and fades out.
    pub second: Option<SpriteHandle>,
    pub transition_id: i32,
    pub start_time_ms: u32,
    pub duration_ms: u32,
    pub elapsed_ms: u32,
}

/// Portable subset of Game.exe `SpriteMotionEntry`.
///
/// Latest IDB evidence declares the native entry as 688 bytes / 172 DWORDs:
/// `[0] PalSprite *sprite`, base/final position and rotation lanes, scale,
/// color/alpha lanes, an animation-delta block at byte offsets 376..451, and
/// `[171] next` for `gSpriteMotionList`.  This Rust struct intentionally
/// stores only the lanes currently implemented by named ANI playback.  Keeping
/// them together prevents the old port mistake of treating move/scale/alpha as
/// unrelated sprite-local tweens.
#[derive(Clone, Copy, Debug)]
pub struct SpriteMotionEntry {
    pub sprite: SpriteHandle,
    pub pos_b: PalVec3,
    pub scale_b: f32,
    pub color_a: u8,
    anim_delta_pos: Option<SpritePositionTween>,
    anim_delta_scale: Option<SpriteScaleTween>,
    anim_delta_color_a: Option<SpriteAlphaTween>,
}

impl SpriteMotionEntry {
    fn from_sprite(sprite: SpriteHandle, state: &PalSprite) -> Self {
        Self {
            sprite,
            pos_b: state.position,
            scale_b: state.scale,
            color_a: state.color.alpha(),
            anim_delta_pos: None,
            anim_delta_scale: None,
            anim_delta_color_a: None,
        }
    }

    fn has_active_animation(&self) -> bool {
        self.anim_delta_pos.is_some()
            || self.anim_delta_scale.is_some()
            || self.anim_delta_color_a.is_some()
    }
}

#[derive(Clone, Copy, Debug)]
struct SpritePositionTween {
    from: PalVec3,
    to: PalVec3,
    elapsed_ms: u32,
    duration_ms: u32,
}

#[derive(Clone, Copy, Debug)]
struct SpriteScaleTween {
    from: f32,
    to: f32,
    elapsed_ms: u32,
    duration_ms: u32,
}

#[derive(Clone, Copy, Debug)]
struct SpriteAlphaTween {
    from: u8,
    to: u8,
    elapsed_ms: u32,
    duration_ms: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PalAnimationAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PalAnimationFlags(u8);

impl PalAnimationFlags {
    pub const HORIZONTAL: Self = Self(0x01);
    pub const VERTICAL: Self = Self(0x02);

    pub fn from_original(flags: u8) -> Self {
        let masked = flags & 0x03;
        if masked == 0 {
            Self::HORIZONTAL
        } else {
            Self(masked)
        }
    }

    pub fn axis(self) -> PalAnimationAxis {
        if (self.0 & Self::HORIZONTAL.0) != 0 {
            PalAnimationAxis::Horizontal
        } else if (self.0 & Self::VERTICAL.0) != 0 {
            PalAnimationAxis::Vertical
        } else {
            PalAnimationAxis::Horizontal
        }
    }

    pub fn raw(self) -> u8 {
        self.0
    }
}

impl From<PalAnimationAxis> for PalAnimationFlags {
    fn from(axis: PalAnimationAxis) -> Self {
        match axis {
            PalAnimationAxis::Horizontal => Self::HORIZONTAL,
            PalAnimationAxis::Vertical => Self::VERTICAL,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct PalPoint2 {
    pub x: i32,
    pub y: i32,
}

impl PalPoint2 {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct PalVec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl PalVec3 {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self {
            x: x as f32,
            y: y as f32,
            z: z as f32,
        }
    }

    pub const fn from_f32(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

pub type PalPoint3 = PalVec3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PalSize {
    pub width: u32,
    pub height: u32,
}

impl PalSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn clamped_nonzero(self) -> Self {
        Self {
            width: self.width.max(1),
            height: self.height.max(1),
        }
    }
}

pub use crate::abi::PalRect32 as PalRect;

impl PalRect {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn width(self) -> i32 {
        self.right.saturating_sub(self.left)
    }

    pub fn height(self) -> i32 {
        self.bottom.saturating_sub(self.top)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PalColor(pub u32);

impl PalColor {
    pub const WHITE: Self = Self(0xFFFF_FFFF);

    pub const fn from_argb(raw: u32) -> Self {
        Self(raw)
    }

    pub fn to_rgba_f32(self) -> [f32; 4] {
        let a = ((self.0 >> 24) & 0xFF) as f32 / 255.0;
        let r = ((self.0 >> 16) & 0xFF) as f32 / 255.0;
        let g = ((self.0 >> 8) & 0xFF) as f32 / 255.0;
        let b = (self.0 & 0xFF) as f32 / 255.0;
        [r, g, b, a]
    }

    pub fn to_rgba_u8(self) -> [u8; 4] {
        [
            ((self.0 >> 16) & 0xFF) as u8,
            ((self.0 >> 8) & 0xFF) as u8,
            (self.0 & 0xFF) as u8,
            ((self.0 >> 24) & 0xFF) as u8,
        ]
    }

    pub const fn alpha(self) -> u8 {
        ((self.0 >> 24) & 0xFF) as u8
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PalRenderMode(u32);

impl PalRenderMode {
    pub const DEFAULT: Self = Self(1);

    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgba_surface_rejects_overflow_and_empty_dimensions() {
        for (width, height) in [(0x8000_0000, 0x8000_0000), (0, 1), (1, 0)] {
            assert!(
                SpriteSurface::rgba8(SpriteSurfaceId(1), 1, width, height, Vec::new()).is_err()
            );
        }
    }

    #[test]
    fn dense_copy_is_dropped_by_any_pixel_edit() {
        let mut sprites = SpriteSystem::new();
        let handle = sprites
            .create_rgba_sprite(2, 1, vec![1; 8], PalVec3::new(0, 0, 0), 0, "text")
            .unwrap();
        // Wrong sizes are rejected; an exact 2x copy is used for rendering.
        assert!(!sprites.set_sprite_hires(handle, 2, vec![0; 8]));
        assert!(sprites.set_sprite_hires(handle, 2, vec![7; 4 * 2 * 4]));
        let surface_id = sprites.get(handle).unwrap().surface;
        let texture = sprites.surface(surface_id).unwrap().to_scene_texture();
        let (width, height, density, pixels) = texture.device_texels();
        assert_eq!((width, height, density, pixels[0]), (4, 2, 2, 7));
        assert_eq!((texture.width, texture.height), (2, 1));
        // Editing the plain pixels invalidates the copy.
        sprites.surface_mut(surface_id).unwrap().pixels_mut()[0] = 9;
        let texture = sprites.surface(surface_id).unwrap().to_scene_texture();
        assert!(texture.hires.is_none());
        assert_eq!(texture.device_texels().0, 2);
    }

    #[test]
    fn copy_sprite_pixels_duplicates_the_source_surface() {
        let mut sprites = SpriteSystem::new();
        let pixels = vec![9, 8, 7, 255, 1, 2, 3, 128];
        let source = sprites
            .create_rgba_sprite(
                2,
                1,
                pixels.clone(),
                PalVec3::from_f32(12.0, 34.0, 1.0),
                4,
                "source",
            )
            .expect("source sprite");
        let copied = sprites
            .copy_sprite_pixels(source, i32::MIN / 2, "backbuffer")
            .expect("copied sprite");
        let copied_sprite = sprites.get(copied).expect("copied handle");
        assert_eq!(copied_sprite.position.x, 12.0);
        assert_eq!(copied_sprite.position.y, 34.0);
        let texture = sprites
            .surface(copied_sprite.surface)
            .expect("copied surface")
            .to_scene_texture();
        assert_eq!(texture.pixels.as_ref(), pixels.as_slice());
        assert_ne!(copied, source);
    }
}
