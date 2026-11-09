//! Reviewed category 11 wrappers and the independent PalVideoPlay transport.
//! See reverse/movie-audio-contract.md and reverse/contracts/audio_video.json.
use super::*;

#[derive(Debug)]
pub(super) struct FullMovieState {
    pub(super) sprite: SpriteHandle,
    pub(super) handle: MSpriteHandle,
    flags: u16,
    fade_started_ms: Option<u32>,
}

impl ScriptRuntime {
    pub(crate) fn movie_active(&self) -> bool { self.full_movie.is_some() }

    pub(crate) fn set_movie_cancel_fade_enabled(&mut self, enabled: bool) {
        self.movie_cancel_fade_enabled = enabled;
    }

    fn video_gain(&self, audio: &AudioSystem) -> i32 {
        let group0 = i64::from(audio.group_volume(PalSoundGroup::GROUP0).raw());
        let group3 = i64::from(audio.group_volume(PalSoundGroup::GROUP3).raw());
        let group = match self.movie_volume_type & 3 {
            1 => group0,
            2 => group0 * group3 / 10000,
            3 => (group3 - group0).clamp(0, 10000),
            _ => group3,
        };
        (group * i64::from(audio.primary_volume().raw()) / 10000) as i32
    }

    pub(super) fn finish_full_movie(&mut self, sprites: &mut SpriteSystem) {
        if let Some(movie) = self.full_movie.take() {
            self.msprite_system.release(movie.handle);
            sprites.release(movie.sprite);
        }
        self.msprite_system.stop_movie();
    }

    /// PAL's movie graph runs while the EXE's ordinary frame callback is gated.
    /// Generic PAL tasks are still processed by Engine during this interval.
    pub(crate) fn advance_movie_transport(&mut self, sprites: &mut SpriteSystem,
        audio: &mut AudioSystem, input: &mut PalInputState, delta_ms: u32) -> bool {
        let was_movie = self.movie_active();
        self.msprite_system.sync_audio(audio);
        self.advance_msprites(sprites, delta_ms);
        let mut clear_input = false;
        if let Some(movie) = &mut self.full_movie {
            if self.msprite_system.state(movie.handle) & MSPRITE_STATE_FINISHED != 0 {
                self.finish_full_movie(sprites);
            } else {
                let mask = if movie.flags == 0 { 0x01000001 } else { 0x01800007 };
                let alt_enter = input.async_virtual_key(18) && input.async_virtual_key(13);
                if movie.fade_started_ms.is_none() && !alt_enter && input.raw_key_push() & mask != 0 {
                    if self.movie_cancel_fade_enabled {
                        movie.fade_started_ms = Some(self.pal_time_ms);
                    } else {
                        self.finish_full_movie(sprites);
                        clear_input = true;
                    }
                }
                if let Some(movie) = &self.full_movie {
                    if let Some(start) = movie.fade_started_ms {
                        let elapsed = self.pal_time_ms.wrapping_sub(start);
                        if elapsed >= 1000 {
                            self.finish_full_movie(sprites);
                            clear_input = true;
                        } else {
                            let percent = 100 - (elapsed as f32 / 1000.0 * 100.0) as i32;
                            let handle = movie.handle;
                            sprites.set_alpha(movie.sprite, (percent * 255 / 100) as u8);
                            let gain = percent * self.video_gain(audio) / 100;
                            self.msprite_system.set_raw_audio_gain(handle, gain);
                        }
                    }
                }
            }
        }
        self.msprite_system.sync_audio(audio);
        // The abort push belongs to PAL's movie thread. Clear it before normal
        // game work resumes, including a movie which ends in this same tick.
        if clear_input { input.clear(); }
        was_movie
    }

    pub(super) fn ext_movie_play(&mut self, assets: &CoreAssets, nls: Nls,
        resource_manager: Option<&mut ResourceManager>, sprites: Option<&mut SpriteSystem>,
        audio: Option<&mut AudioSystem>) -> ExtCallOutcome {
        let args = self.pop_ext_args(2);
        if args.len() != 2 { return ExtCallOutcome::Block; }
        let skip_mask = if assets.extended_softpal { 0x30 } else { 0x10 };
        if self.dispatch_skip_state() & skip_mask != 0 { return ExtCallOutcome::Continue; }
        let Some(mut name) = self.resolve_resource_string(args[0], assets, nls) else { return ExtCallOutcome::Continue; };
        if assets.registered_softpal {
            if let Some(dot) = name.rfind('.') {
                if !name[dot..].contains(['/', '\\']) { name.truncate(dot); }
            }
        }
        let (Some(resources), Some(sprites)) = (resource_manager, sprites) else { return ExtCallOutcome::Continue; };
        // The EXE probes these three suffixes, in this order. Explicit suffixes
        // are stripped only by the newer wrapper; PAL itself takes a filename.
        let asset = [".wmv", ".mpg", ".avi"].into_iter().find_map(|ext| resources.open(&format!("{name}{ext}")).ok());
        let Some(asset) = asset else {
            log::warn!("movie_play could not find a supported movie for {name:?}");
            return ExtCallOutcome::Continue;
        };
        log::debug!("[trace-assets] movie_play {:?} from {:?}", asset.name, asset.source);
        self.finish_full_movie(sprites);
        let loaded = match self.msprite_system.load_movie(asset.name, asset.bytes) {
            Ok(loaded) => loaded,
            Err(error) => { log::warn!("movie_play {name:?} failed: {error}"); return ExtCallOutcome::Continue; }
        };
        let Some(sprite) = sprites.create_msprite(loaded.handle, loaded.width, loaded.height,
            loaded.rgba, PalVec3::new(0, 0, 0), i32::MAX, loaded.name.clone()) else {
            self.msprite_system.release(loaded.handle);
            return ExtCallOutcome::Continue;
        };
        let (width, height) = self.logical_size();
        sprites.set_native_projection(sprite, Some((width as f32 / loaded.width as f32, height as f32 / loaded.height as f32)));
        let flags = args[1] as u16;
        self.full_movie = Some(FullMovieState { sprite, handle: loaded.handle, flags, fade_started_ms: None });
        if let Some(audio) = audio {
            let gain = self.video_gain(audio);
            self.msprite_system.set_raw_audio_gain(loaded.handle, gain);
        }
        self.msprite_system.play(loaded.handle, 0);
        self.msprite_system.start_movie(loaded.name, flags, loaded.handle);
        ExtCallOutcome::Continue
    }

    pub(super) fn ext_msp_set_loop_sp_ep(&mut self, assets: &CoreAssets, nls: Nls,
        resource_manager: Option<&mut ResourceManager>, sprites: Option<&mut SpriteSystem>,
        task_system: Option<&mut TaskSystem>) -> ExtCallOutcome {
        let args = self.pop_ext_args(8);
        if args.len() != 8 { return ExtCallOutcome::Block; }
        if !(0..128).contains(&args[1]) { return self.msp_load_result(assets, false); }
        // The native wrapper delegates image selection and placement to the
        // ordinary five-argument sprite loader, including its MPEG suffix path.
        self.stack.extend(args[..5].iter().rev().copied());
        self.ext_sp_set(5, assets, nls, resource_manager, sprites, task_system, false);
        let mut played = false;
        if let Some(state) = self.game_msprites.get_mut(&args[1]) {
            if let Some(handle) = state.handle {
                self.msprite_system.set_loop_point(handle, args[6], args[7]);
                played = self.msprite_system.play(handle, args[5]);
                state.playing = played;
                state.finished = false;
                state.loop_mode = args[5];
                state.last_play = args[5];
                state.loop_start = args[6];
                state.loop_end = args[7];
            }
        }
        self.msp_load_result(assets, played)
    }

    fn msp_load_result(&self, assets: &CoreAssets, played: bool) -> ExtCallOutcome {
        if assets.registered_softpal { ExtCallOutcome::Value(i32::from(played || self.dispatch_skip_state() != 0)) }
        else { ExtCallOutcome::Continue }
    }

    pub(super) fn ext_msp_cls(&mut self, sprites: Option<&mut SpriteSystem>, tasks: Option<&mut TaskSystem>) -> ExtCallOutcome {
        if self.stack.is_empty() { return ExtCallOutcome::Block; }
        self.ext_sp_cls(sprites, tasks);
        ExtCallOutcome::Continue
    }

    pub(super) fn ext_msp_wait(&mut self, extended: bool) -> ExtCallOutcome {
        if let Some(slot) = self.pending_msp_wait_slot {
            let finished = self.game_msprites.get(&slot).and_then(|state| state.handle)
                .is_none_or(|handle| self.msprite_system.state(handle) & MSPRITE_STATE_FINISHED != 0);
            if finished || self.dispatch_skip_state() != 0 {
                self.pending_msp_wait_slot = None;
                return ExtCallOutcome::Continue;
            }
        } else {
            let args = self.pop_ext_args(1);
            let Some(&slot) = args.first() else { return ExtCallOutcome::Block; };
            let Some(handle) = self.game_msprites.get(&slot).and_then(|state| state.handle) else { return ExtCallOutcome::Continue; };
            self.msprite_system.set_loop(handle, 0);
            self.pending_msp_wait_slot = Some(slot);
        }
        if self.dispatch_skip_state() & if extended { 0x30 } else { 0x10 } != 0 { return ExtCallOutcome::Continue; }
        self.pc = self.pc.saturating_sub(self.extcall_instruction_size);
        ExtCallOutcome::Wait { value: 1, request: WaitRequest::Frame(1) }
    }

    pub(super) fn dispatch_movie_control(&mut self, index: u16, audio: Option<&mut AudioSystem>) -> ExtCallOutcome {
        let count = match index { 8 | 9 | 11 | 12 => 1, 10 => 0, 13 => 2, _ => return ExtCallOutcome::Skip };
        let args = self.pop_ext_args(count);
        if args.len() != count { return ExtCallOutcome::Block; }
        match index {
            8 => self.movie_volume_type = args[0] as u32 & 3,
            9 => {
                self.movie_volume_raw = args[0].wrapping_mul(100);
                if let Some(audio) = audio {
                    let gain = if self.movie_muted { 0 } else { self.movie_volume_raw };
                    let _ = audio.set_group_volume(PalSoundGroup::GROUP0, PalVolume::from_raw(gain));
                }
            }
            10 => return ExtCallOutcome::Value(self.movie_volume_raw.wrapping_mul(100) / 10000),
            11 => {
                self.movie_muted = args[0] != 0;
                if let Some(audio) = audio {
                    let gain = if self.movie_muted { 0 } else { self.movie_volume_raw };
                    let _ = audio.set_group_volume(PalSoundGroup::GROUP0, PalVolume::from_raw(gain));
                }
            }
            12 | 13 => {
                let slot = args[0];
                if !(0..128).contains(&slot) || !self.game_sprites.contains_key(&slot) { return ExtCallOutcome::Value(-1); }
                let Some(handle) = self.game_msprites.get(&slot).and_then(|state| state.handle) else { return ExtCallOutcome::Value(0); };
                if index == 12 { return ExtCallOutcome::Value(self.msprite_system.state(handle) as i32); }
                self.msprite_system.set_volume(handle, args[1].clamp(0, 100), true);
                return ExtCallOutcome::Value(1);
            }
            _ => unreachable!(),
        }
        ExtCallOutcome::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::AudioConfig;
    use super::super::tests::empty_core_assets;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("sena_movie_{}_{}", std::process::id(), SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
            std::fs::create_dir_all(&root).unwrap();
            std::fs::write(root.join("clip.mpg"), include_bytes!("../../tests/fixtures/timing_25fps.mpg")).unwrap();
            Self(root)
        }
        fn resources(&self) -> ResourceManager { ResourceManager::new(&self.0, Nls::ShiftJis) }
    }
    impl Drop for Fixture { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }

    fn start(vm: &mut ScriptRuntime, assets: &CoreAssets, resources: &mut ResourceManager,
        sprites: &mut SpriteSystem, audio: &mut AudioSystem, flags: i32) {
        let name = vm.store_dynamic_string("clip".into());
        vm.stack.extend([flags, name]);
        assert_eq!(vm.ext_movie_play(assets, Nls::ShiftJis, Some(resources), Some(sprites), Some(audio)), ExtCallOutcome::Continue);
        assert!(vm.movie_active());
    }

    #[test]
    fn full_movie_flags_never_replace_an_ordinary_sprite_slot() {
        let fixture = Fixture::new();
        let mut resources = fixture.resources();
        let (assets, _) = empty_core_assets(false);
        let mut vm = ScriptRuntime::boot(12, ScriptRuntimeConfig::default());
        let mut sprites = SpriteSystem::new();
        let ordinary = sprites.create(SpriteDesc::new(SceneTextureId(0), 1, 1));
        vm.game_sprites.insert(7, ordinary);
        let mut audio = AudioSystem::new(AudioConfig { enabled: false }).unwrap();
        start(&mut vm, &assets, &mut resources, &mut sprites, &mut audio, 0x10007);
        assert_eq!(vm.full_movie.as_ref().unwrap().flags, 7);
        assert_eq!(vm.game_sprites[&7], ordinary);
        assert_eq!(sprites.len(), 2);
        vm.advance_movie_transport(&mut sprites, &mut audio, &mut PalInputState::default(), 200);
        assert!(!vm.movie_active());
        assert_eq!(vm.game_sprites[&7], ordinary);
        assert_eq!(sprites.len(), 1);
    }

    #[test]
    fn movie_cancel_masks_use_push_edges_and_fade_on_the_native_clock() {
        let fixture = Fixture::new();
        let mut resources = fixture.resources();
        let (assets, _) = empty_core_assets(false);
        let mut vm = ScriptRuntime::boot(12, ScriptRuntimeConfig::default());
        let mut sprites = SpriteSystem::new();
        let mut audio = AudioSystem::new(AudioConfig { enabled: false }).unwrap();
        start(&mut vm, &assets, &mut resources, &mut sprites, &mut audio, 0);
        let mut input = PalInputState::default();
        input.set_key_push_bits(4);
        vm.advance_movie_transport(&mut sprites, &mut audio, &mut input, 0);
        assert!(vm.full_movie.as_ref().unwrap().fade_started_ms.is_none());
        input.set_key_push_bits(1);
        vm.set_pal_time(100);
        vm.advance_movie_transport(&mut sprites, &mut audio, &mut input, 0);
        assert_eq!(vm.full_movie.as_ref().unwrap().fade_started_ms, Some(100));
        vm.set_pal_time(1099);
        vm.advance_movie_transport(&mut sprites, &mut audio, &mut PalInputState::default(), 0);
        assert!(vm.movie_active());
        vm.set_pal_time(1100);
        vm.advance_movie_transport(&mut sprites, &mut audio, &mut input, 0);
        assert!(!vm.movie_active());
        vm.apply_input_control(&mut input);
        assert_eq!(input.raw_key_push(), 0);
        start(&mut vm, &assets, &mut resources, &mut sprites, &mut audio, 1);
        vm.set_movie_cancel_fade_enabled(false);
        input.set_key_push_bits(4);
        vm.advance_movie_transport(&mut sprites, &mut audio, &mut input, 0);
        assert!(!vm.movie_active());
    }

    #[test]
    fn movie_gain_uses_the_selected_groups_before_master() {
        let mut vm = ScriptRuntime::boot(12, ScriptRuntimeConfig::default());
        let mut audio = AudioSystem::new(AudioConfig { enabled: false }).unwrap();
        audio.set_primary_volume(PalVolume::from_raw(5000)).unwrap();
        audio.set_group_volume(PalSoundGroup::GROUP0, PalVolume::from_raw(4000)).unwrap();
        audio.set_group_volume(PalSoundGroup::GROUP3, PalVolume::from_raw(6000)).unwrap();
        for (mode, expected) in [(0,3000),(1,2000),(2,1200),(3,1000)] {
            vm.stack.push(mode);
            assert_eq!(vm.dispatch_movie_control(8, Some(&mut audio)), ExtCallOutcome::Continue);
            assert_eq!(vm.video_gain(&audio), expected);
        }
        vm.stack.push(73);
        vm.dispatch_movie_control(9, Some(&mut audio));
        assert_eq!(audio.group_volume(PalSoundGroup::GROUP0).raw(),7300);
        vm.stack.push(-1);
        vm.dispatch_movie_control(11, Some(&mut audio));
        assert_eq!(audio.group_volume(PalSoundGroup::GROUP0).raw(),0);
        assert_eq!(vm.dispatch_movie_control(10, None), ExtCallOutcome::Value(73));
        vm.stack.push(0);
        vm.dispatch_movie_control(11, Some(&mut audio));
        assert_eq!(audio.group_volume(PalSoundGroup::GROUP0).raw(),7300);
    }

    #[test]
    fn msp_clear_consumes_one_slot_and_wait_does_not_treat_manual_stop_as_eof() {
        let fixture = Fixture::new();
        let mut resources = fixture.resources();
        let (mut assets, _) = empty_core_assets(false);
        let mut vm = ScriptRuntime::boot(24, ScriptRuntimeConfig::default());
        let mut sprites = SpriteSystem::new();
        let name = vm.store_dynamic_string("clip".into());
        for (registered,slot) in [(false,3),(true,4)] {
            assets.registered_softpal = registered;
            vm.stack.extend([name,slot,0,0,0,1,40,80].into_iter().rev());
            let result = vm.ext_msp_set_loop_sp_ep(&assets, Nls::ShiftJis, Some(&mut resources), Some(&mut sprites), None);
            assert_eq!(result, if registered { ExtCallOutcome::Value(1) } else { ExtCallOutcome::Continue });
        }
        vm.stack.push(3);
        assert_eq!(vm.ext_msp_stop(), ExtCallOutcome::Continue);
        let handle = vm.game_msprites[&3].handle.unwrap();
        assert_eq!(vm.msprite_system.state(handle), 2);
        vm.stack.push(3);
        assert!(matches!(vm.ext_msp_wait(false), ExtCallOutcome::Wait { .. }));
        assert!(matches!(vm.ext_msp_wait(false), ExtCallOutcome::Wait { .. }));
        assert!(vm.stack.is_empty());
        vm.stack.extend([888, 3]);
        assert_eq!(vm.ext_msp_cls(Some(&mut sprites), None), ExtCallOutcome::Continue);
        assert_eq!(vm.stack, [888]);
        assert!(!vm.game_sprites.contains_key(&3));
        assert!(vm.game_sprites.contains_key(&4));
        assert!(!vm.msprite_system.check(handle));
    }

    #[test]
    fn full_movie_yields_vm_without_storing_destination_then_resumes_after_eof() {
        let fixture = Fixture::new();
        let mut resources = fixture.resources();
        let (mut assets, _) = empty_core_assets(false);
        let mut vm = ScriptRuntime::boot(12, ScriptRuntimeConfig::default());
        let name = vm.store_dynamic_string("clip".into());
        vm.vars[10] = name;
        let words: [u32; 8] = [0x1001f, 0, 0x1001f, 0x4000000a, 0x10017, 0xb0000, 11, 0x10015];
        assets.script.bytes = b"Sv20".to_vec();
        assets.script.bytes.extend(0u32.to_le_bytes());
        assets.script.bytes.extend(12u32.to_le_bytes());
        for word in words { assets.script.bytes.extend(word.to_le_bytes()); }
        vm.vars[11] = 777;
        let mut sprites = SpriteSystem::new();
        let mut audio = AudioSystem::new(AudioConfig { enabled: false }).unwrap();
        let config = ScriptRuntimeConfig::default();
        let tick = vm.run_frame_with_resources(&assets,Some(&mut resources),Some(&mut sprites),None,Some(&mut audio),None,&config).unwrap();
        assert_eq!(tick.executed,3);
        assert!(vm.movie_active());
        assert_eq!(vm.vars[11],777);
        let tick = vm.run_frame_with_resources(&assets,None,None,None,None,None,&config).unwrap();
        assert_eq!(tick.executed,0);
        vm.advance_movie_transport(&mut sprites,&mut audio,&mut PalInputState::default(),200);
        let tick = vm.run_frame_with_resources(&assets,None,None,None,None,None,&config).unwrap();
        assert!(matches!(tick.status, RuntimeStatus::Halted { .. }));
    }
}
