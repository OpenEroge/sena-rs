# Iroha (trenka.exe) native interface profile

Scope: the `trenka.exe` / `dll/Pal.dll` pair whose hashes are recorded in
`reverse/evidence/iroha-trenka-interface-inventory.json`. Addresses below are
only valid for that build. The inventory records interface layout only; a
present slot does not mean Sena implements the handler's semantics.

## Native evidence

- `trenka.exe` binds `PAL.dll` through the delay-load directory (213 names);
  it has no ordinary PAL import entries.
- `dll/Pal.dll` exports 285 names. Against the audited Reference DLL it adds
  only `?PalGetlpPool@@YAPAEXZ` and lacks the six language/position exports
  (`PalGetLanguage`, `PalSetLanguage`, `PalSystemCNPathSet`,
  `PalSystemTCPathSet`, `PalSpriteGetPosX`, `PalSpriteGetPosY`). It lacks the
  registered-dispatch markers `PalSystemPathInit` and
  `PalGetChangeWindowModeTime`, so the registered profile stays disabled.
- Primary opcode table: `0x64C890`, 265 slots, no bounds check. Extcall
  category table: `0x626098` (24 entries), dispatcher `0x42D920`; a null
  category returns 0 without writing the destination.
- Primary alias regions differ from Reference: starts are 23:35, 10:40, 5:82,
  6:98, 9:110, 12:171, 2:176, 13:226, 7:254 (Reference: 5:80, 6:96, 9:108,
  13:224, 7:252, length 263). `primary_aliases.rs` encodes the Reference
  layout, so high primary opcodes would be mis-routed for this build. The
  shipped script only uses primary opcodes 1..33, so this is latent.
- Category 18 slots 88/89/91/96 have the registered-build meanings
  (network status, touch availability, touch enable, `Pal3DIs`) even though
  the registered DLL markers are absent. Sena gates them on
  `registered_softpal`; a capability that is not tied to those exports is
  still needed.
- `gosub_point` (`0x42D7B0`) resolves its target through the shared operand
  resolver `0x42D2E0`, as do primary opcodes 1..19, 26..28 and 30..33. The
  resolver returns the raw word for top nibble 0, so plain point ids are
  unchanged; the script also emits `gosub var[1]` (`0x40000001`).

- `sp_get_filename` (3:8, `0x4250B0`) copies the caller's string kept at
  sprite record +0x254 (stride 0x274, table `0x655920`); id-based sprites
  resolve the id through `0x44B500`. Save restore (`0x40D0F4`) feeds the
  same string into the suffix-appending loader `0x447FA0`, so the stored
  name never carries a probed suffix.
- SE play 5:2 (`0x435A30`) pops slot, name, flags, fade_ms and a group
  selector indexing `0x625FF4` = {5, 4, 6}; flags bit 2 selects
  `PalSoundPlayFade`. There is no volume operand. Totsulover `0x462990`
  matches.
- SE 5:1 (`0x435D60`) pops three words (slot, flags, fade_ms) and replays an
  already loaded slot. Koikake scripts push four words for 5:1, so its
  arity is build-specific and Sena still pops five for non-extended builds.
- The title loop gates a first-run notice sequence on `mem_system[262]`; the
  script sets it afterwards but only four sites call `savesystemdata`
  (10:25), so persistence depends on reaching one of them.

## Implementation state

- The VM always decodes the gosub target as an operand; previously only the
  registered profile did. Plain ids keep the old behaviour. Test:
  `gosub_resolves_variable_point_operand_without_registered_profile`.
- The disassembler emits `PointOperand` for non-immediate gosub targets
  instead of failing the `Point.dat` range check.

- `sp_get_filename` strips the one loader suffix from the asset name.
- 5:2 follows the native operand order (flags, fade, group selector) with
  the default channel volume; 5:1 is unchanged.
- `--diagnostic-realtime` paces headless frames to the wall clock. Without
  it device audio lags the VM clock and the click-sound gate on button
  reactions never clears.

## Runtime validation

- Headless boot reaches the title screen through the logo movie.
- GAME START enters the prologue: background, sprites, event CG, text,
  name box, face icon and voice icon render; a 30000-frame auto-advance run
  plays the OPENING movie (matches ffmpeg frames) and keeps advancing.
- With `--diagnostic-realtime` and device audio, BGM, SE, system voice and
  line voice are played; SE effective volume follows the group and master
  volumes.
- The dynamic gosub site was not executed within the first 2100 frames, so
  the fix is validated by native evidence and the unit test only.
