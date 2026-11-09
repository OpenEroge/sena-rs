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

- Wrappers matching the registered contract (pop count, debug strings,
  PAL calls): 2:47, 3:59, 5:0..14, 9:57, 10:37..39, 14:13..19, 14:21..24,
  18:54, 18:88/89/91, 18:96..99. 18:90 differs (no operands).
- `.ani` programs: loader 0x4473E0, interpreter 0x4466A0, driver
  0x446FE0/0x447060, clear 0x447180, tasks 0x402FF0..0x404440.
- Action section types 16 (rect tween, 17:31) and 20 (mask reveal,
  17:35); per-type callback tables at 0x625EE8/0x625F40/0x625FA0/0x626000.
- 3:14 writes the wrapper base position (+0x10..0x18, floats +0x6C..0x74),
  the fields sp_create sets, not rotation: an absolute move.
- Save labels (10:13/10:16/10:6) apply the save font block via 0x42F420 and
  wrap to the 10:15 rect (0x431C30).
- Category 18 extras: dictionary 82..87, save folder 50, append 59, global
  words 72/73/79, scene-skip query 108. INI wrappers resolve the file name
  through File.dat (0x44B500).
- Favourite voices 18:60..68 (0x414CA0..0x4154B0): up to 0x400 records
  (id, flag, name, voice, tag-stripped text) saved as
  `%06d,%1d,"name","voice","text"` lines; 18:79 sets the add limit. The
  backlog face button adds the line through 14:20 and 18:61.
- 18:57 `strcut` (0x41B7C0) counts characters by a lead-byte walk (CR/LF
  counts once) and cuts a longer dynamic string at byte `2 * limit` (one
  more when that byte is not a lead byte); it returns the count. 18:106
  forwards a byte to `PalSetPow`; 18:107 returns a dynamic string's byte
  length.
- 14:25 (0x42D1A0) pops enable, File.dat name, x, y. The backlog builder
  (0x42B7A0) draws that sprite on every visible record at
  `anchor - (row_height - height) / 2 + y`, dropping rows above the view.
  The script passes the `[history] HISTORY_LINE_*` separator.
- 3:32 `sp_create` (0x4267D0) pops slot, File.dat name, x, y, width,
  height and loads the named sprite (`#AARRGGBB` becomes a width x height
  surface). 3:22 `sp_text` (0x427520) draws into an existing wrapper with
  `PalSpriteCreateTextEx`, which writes glyphs over the locked surface at
  (x, y); Totsulover `sub_449B90` does the same.
- 3:13 `sp_cls_transition` (0x4293D0) pops slot, transition kind (index
  into {0, 1} at 0x625F94) and duration; with effects running it clears
  the slot through `PalSpriteCreateTransitionHandle`, otherwise at once.
  Treating it as one-operand sp_cls left two words on the VM stack per call.
- Operand counts audited against the native handlers for every wrapper
  the script uses (stack deltas from a 30000-frame trace plus a dispatch
  probe): 2:18 `text_n` / 2:19 `text_cat` pop four words (text_cat appends
  to the shown line), 2:20 `set_history` pops voice, name and text and
  records the backlog line the pair skips, 3:38 `sp_bitblt` pops nine words
  (sprite-to-sprite copy), 9:48 `set_cursor` pops name and hotspot,
  9:55 `get_font_color` pops a selector (all builds), 10:29 swaps two save
  slots through `save_tmp.dat`, 18:17 pops nothing. With the small
  wrappers in `runtime/trenka_wrappers.rs` (2:35, 3:42/43, 4:5, 4:16/17,
  8:28/30, 15:1/8/9, 18:43..48, 18:76, 18:109, 18:113..115) and 8:42 shared
  with the registered builds, every one of the 419 extcall coordinates the
  script uses pops what the native wrapper pops (18:9 wsprint pops its
  padded operands in a loop; 11:2 pops through sp_cls).
- 3:58 `drawtext` (0x42A3F0) pops slot, text, x, y, mode and typesets the
  text into an existing sprite with the message-window renderer 0x43CC30
  (text_init font size, wrap width and pitch; also used by the backlog).
- 8:27 `btn_blt` (0x40FEA0) pops group, index, sprite slot, three btn_set
  callbacks, entry flags, two unused words, cell width and height; it
  copies the slot sprite into a new button at the slot position. 8:31/8:32
  store and read a word at button record +0x4C (TIPS entry id).
- In-game TIPS: text `<dN>..</d>` spans become records; the per-tick
  callback lays a `btn_blt` button over each word (normal/hover/pressed
  drawn with `drawtext`), and clicking it opens the small TIPS window.
  Point 2233 registers the entry silently only while scene skip is active.
- Battles are script presentations, not an executable subsystem. The
  `battle.pac` names (cut-ins, `SLASH_*`, `VIEWCHANGE_*`, `SE99_BA_*`) occur
  only in `FILE.DAT`, never in `trenka.exe`. Their script users are in
  0xB9000-0xDE000 (points 6175-6672) and the chapter scripts, and call only
  audited generic extcalls (`sp_set_ex`, action lanes, `se_play_ex_ch`,
  `random`, `wait`). The region sets no buttons and waits for no input.
- Script thread (21:0..3): `create_thread` (0x42F300) only records the
  target point and sets active/running (+0x4D2324/+0x4D2328). The scheduler
  (0x42F1E0, and the wait handler at 0x445629) pushes a type-3 call frame
  to that point whenever the thread is running and no such call is in
  flight (+0x4D232C); the matching `ret` (0x42D643) only decrements the
  in-flight count. A returning body is therefore called again on the next
  visit, including while the main script waits for a click. The visit is
  skipped while an action bank is in a synchronous run (0x4455D9).
- The ADV menu is that thread. Point 2367 (created at 0x42670 when
  `mem_system[134]` is set) checks the mouse against the panel zone each
  frame and slides the `MAIN_BTN_*` panel in or out. Lane `2*i` fades
  button `i` and lane `2*i+1` moves it by the panel width. `mem_user[0]` is
  the open flag. The click handler of the compact tab (index 22, point
  2903) opens the menu only in touch mode (`mem_system[163]`, set from
  `is_touch` and the INI). The ADV setup (0x5095C) parks every panel button
  one panel width to the right with `sp_set_pos_move` on its packed slot
  (`0x02000000 | index`).
- Action tables: `set_active_action` (0x408F90) accepts banks 0..15 and the
  section writers (`action_alpha_delta` 0x409F40 and siblings) accept lines
  0..63 with up to 64 sections, like the extended builds; a rejected call
  returns without popping its tail. The menu uses lanes up to 47.
- `sp_set_pos_move` (0x428F20) resolves its slot through 0x449110, which
  maps packed button references to the button sprite, and adds the delta
  to the wrapper base position.
- 15:8 (0x411B50) writes the VM stack depth to its destination and pops
  nothing; the menu thread prints it in its debug traces.

## Implementation state

- The VM always decodes the gosub target as an operand; previously only the
  registered profile did. Plain ids keep the old behaviour. Test:
  `gosub_resolves_variable_point_operand_without_registered_profile`.
- The disassembler emits `PointOperand` for non-immediate gosub targets
  instead of failing the `Point.dat` range check.

- `sp_get_filename` strips the one loader suffix from the asset name.
- 5:2 follows the native operand order (flags, fade, group selector) with
  the default channel volume; 5:1 is unchanged.
- `NativeBuild::Trenka` (complete-file digests) selects the shared
  registered coordinates, the position layer for 3:14 and the category 18
  extras; `registered_softpal` stays off.
- `ani.rs` runs the full motion bytecode into `PalSprite::motion_lanes`.
- `sptext` draws into an existing slot sprite for every build (native
  evidence from two builds). trenka-only wrappers are listed in
  `TRENKA_OWN_EXTCALLS` and dispatched through `NativeBuild::defines_extcall`.
- The ADV renderer receives the body with its inline markup; the resolver
  used to strip `<d>` spans before the tips layout could see them, so no
  tips record was ever produced. The text block height now follows the
  per-line advances, so the last of three lines is no longer clipped.
- `--diagnostic-realtime` paces headless frames to the wall clock. Without
  it device audio lags the VM clock and the click-sound gate on button
  reactions never clears.
- A script thread whose body returns is re-armed at its point and entered
  once per frame, also during main-script click and frame waits; it runs
  with the main instruction budget (it used to stop at the first return and
  ran at most 64 instructions a frame). Test:
  `script_thread_reenters_every_frame_even_while_main_waits`.
- `NativeBuild::wide_action_tables` gives trenka the 16x64x64 action
  tables without the extended scaling; `sprite_slots_resolve_buttons` lets
  3:25 move packed button slots.
- The host-side ADV menu emulation (name-matched `MAIN_BTN_BASE0/1`, click
  interception, quick-save alpha mirroring) is gone. Group-0 buttons follow
  the text window and their show/hide state; `btn_enable` no longer hides a
  button, because the panel backdrop is a disabled button that is drawn.
  Test: `adv_toolbar_follows_text_window_and_show_state_only`.

## Runtime validation

- Headless boot reaches the title screen through the logo movie.
- GAME START enters the prologue: background, sprites, event CG, text,
  name box, face icon and voice icon render; a 30000-frame auto-advance run
  plays the OPENING movie (matches ffmpeg frames) and keeps advancing.
- With `--diagnostic-realtime` and device audio, BGM, SE, system voice and
  line voice are played; SE effective volume follows the group and master
  volumes.
- The backlog shows separator lines between records and keeps them while
  scrolling. Clicking a face icon writes `save/favoice.dat`; EXTRA → VOICE
  MODE then lists the entry with its face, count (001/500) and the
  `strcut`-shortened line.
- The first TIPS word of the common route (about frame 28440 with
  auto-advance) is highlighted; clicking it opens the TIPS window at
  (8, 334) with title and explanation. EXTRA → TIPS lists entries and
  shows their detail text.
- A 2,300,000-frame auto-advance route spends 14,137 frames in the battle
  region without an unimplemented call; captures show the view change,
  speed-line cut-ins, slash sprites and character slides.
- ADV menu (headless, mouse moved by injected clicks): with the pointer
  outside the zone only the `>>` tab shows; moving it onto the panel slides
  in all buttons in their slots (BACK/NEXT, SAVE/LOAD, Q.SAVE/Q.LOAD,
  AUTO/SKIP, LOG/SYSTEM, CLOSE/HOLD, FAVORITE, window-alpha slider) and
  hides the tab; moving away slides them out. Q.SAVE writes `save999.dat`
  and SYSTEM opens the system screen, with and without `--patch mai`.
- The `mai` translation overlay is described in
  [mai-translation-patch.md](mai-translation-patch.md).
- The dynamic gosub site was not executed within the first 2100 frames, so
  the fix is validated by native evidence and the unit test only.
