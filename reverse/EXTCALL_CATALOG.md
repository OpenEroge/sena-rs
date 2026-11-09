# Effective extcall signature catalog

Generated from the shared Rust registry by `reverse/evidence/extcall_catalog.rs`.

293 explicit coordinates and 246 automatic coordinates overlap at 170; the effective union contains **369 unique coordinates**.

This is a registry inventory, not a complete native interface table. `Blocked` means insufficient verification, not an absent handler. `Verified` does not certify every runtime boundary. Runtime corrections and remaining gaps are recorded in [port-corrections.md](port-corrections.md).

| Recorded status | Count |
|---|---:|
| Blocked | 145 |
| Partial | 77 |
| StackDisciplineOnly | 2 |
| Verified | 145 |

| Coordinate | Name | Pop | Display parameters (pop index) | Status | Source | Purpose |
|---|---|---:|---|---|---|---|
| 2:0 | `text_init` | 8 | x (0, CoordinateX), y (1, CoordinateY), width (2, Integer), height (3, Integer), font_size (4, CoordinateZ), line_spacing (5, Integer), mode (6, Mode), flags (7, Flag) | Blocked | automatic | Reachable Game.exe extcall 0002:0000 (text_init) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 2:1 | `text_set_icon` | 4 | icon_slot (0, SpriteSlot), resource_name (1, ResourceStringFromFileDat), x (2, CoordinateX), y (3, CoordinateY) | Blocked | automatic | Reachable Game.exe extcall 0002:0001 (text_set_icon) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 2:2 | `text` | 4 | mode (3, Mode), text_id (2, TextId), name_or_aux_id (1, TextId), voice_or_aux_id (0, TextId) | Blocked | explicit | Set current ADV text without script blocking. |
| 2:3 | `text_hide` | 1 | redraw_flag (0, Flag) | Blocked | explicit | Hide the ADV text window/state. |
| 2:4 | `text_show` | 1 | visible (0, Flag) | Blocked | explicit | Show the ADV text window/state. |
| 2:5 | `text_set_btn` | 1 | button_id (0, Integer) | Verified | explicit | Select active ADV text-window button id. |
| 2:8 | `text_clear` | 0 |  | Blocked | explicit | Clear current text and release body/name sprites on the next text sync. |
| 2:9 | `text_set_speed` | 1 | speed (0, Integer) | Verified | explicit | Store the message speed (task data +0x20). |
| 2:10 | `text_get_time` | 0 |  | Verified | explicit | Return the configured message speed (task data +0x20). |
| 2:11 | `text_window_set_alpha` | 2 | window_id (0, Integer), alpha (1, Alpha) | Blocked | automatic | Reachable Game.exe extcall 0002:000B (text_window_set_alpha) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 2:12 | `text_voice_play` | 1 | voice_id (0, Integer) | Blocked | automatic | Reachable Game.exe extcall 0002:000C (text_voice_play) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 2:13 | `text_task_redraw_flag` | 0 |  | Verified | explicit | Set the native ADV text redraw flag at text context +4192. |
| 2:14 | `text_set_icon_animation_time` | 2 | icon_slot (0, Integer), duration_ms (1, DurationMs) | Verified | explicit | Set ADV text icon animation time. |
| 2:15 | `text_w` | 4 | duration_ms (3, DurationMs), text_id (2, TextId), name_or_aux_id (1, TextId), voice_or_aux_id (0, TextId) | Blocked | explicit | Set current ADV line and start visual reveal; does not block VM. |
| 2:16 | `text_a` | 4 | duration_ms (3, DurationMs), text_id (2, TextId), name_or_aux_id (1, TextId), voice_or_aux_id (0, TextId) | Blocked | explicit | Set current ADV line fully visible; does not block VM. |
| 2:17 | `text_wa` | 4 | duration_ms (3, DurationMs), text_id (2, TextId), name_or_aux_id (1, TextId), voice_or_aux_id (0, TextId) | Blocked | explicit | Set current ADV line and start visual reveal; does not block VM. |
| 2:18 | `text_n` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 0002:0012 (text_n) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 2:19 | `text_cat` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 0002:0013 (text_cat) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 2:20 | `set_history` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 0002:0014 (set_history) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 2:21 | `is_text_visible` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 0002:0015 (is_text_visible) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 2:22 | `text_set_base` | 4 | mode (3, Mode), x (2, CoordinateX), y (1, CoordinateY), base_resource (0, ResourceStringFromFileDat) | Blocked | explicit | Select ADV message-window base image/resource. |
| 2:23 | `enable_voice_cut` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 0002:0017 (enable_voice_cut) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 2:24 | `is_voice_cut` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 0002:0018 (is_voice_cut) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 2:25 | `texttimecheckset` | 5 | text_id (0, TextId), x (1, CoordinateX), y (2, CoordinateY), width (3, CoordinateX), height (4, CoordinateY) | Blocked | explicit | Create native ADV text timing-check task/sprite. |
| 2:26 | `text_task_free` | 0 |  | Verified | explicit | Free native text reveal/task state. |
| 2:27 | `text_sprite_lock` | 0 |  | Verified | explicit | Lock native text sprite and clear alpha bytes. |
| 2:28 | `text_set_color` | 4 | color_slot (0, Integer), label_string (1, TextId), text_color (2, Color), effect_color (3, Color) | Blocked | explicit | Configure ADV text color-table entry. |
| 2:29 | `textredraw` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 0002:001D (textredraw) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 2:30 | `set_text_mode` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 0002:001E (set_text_mode) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 2:31 | `text_init_visualnovelmode` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 0002:001F (text_init_visualnovelmode) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 2:32 | `text_set_icon_mode` | 1 | mode (0, Mode) | Blocked | automatic | Reachable Game.exe extcall 0002:0020 (text_set_icon_mode) for text/message-window state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 2:36 | `tips_set_process` | 2 | interaction (0, PointId), completion (1, PointId) | Partial | explicit | Register Tips callback points and clear records, preserving the other state. |
| 2:37 | `tips_get_last_index` | 0 |  | Partial | explicit | Return max(completed record count minus one, zero). |
| 2:38 | `tips_get_str` | 3 | record (0, Integer), destination (1, DynamicString), selector (2, Integer) | Partial | explicit | Copy one of three record strings into the destination and return the record id. |
| 2:39 | `tips_get_param` | 2 | record (0, Integer), selector (1, Integer) | Partial | explicit | Read record fields or original text_init values (selectors 12 through 17), without render scaling. |
| 2:40 | `tips_reset` | 0 |  | Partial | explicit | Clear current and first record and restart collection, preserving pause and colors. |
| 2:41 | `tips_search` | 1 | needle (0, DynamicString) | Partial | explicit | Return the last exact matching record index or minus one. |
| 2:42 | `tips_set_color` | 2 | normal (0, Color), active (1, Color) | Partial | explicit | Set custom colors; normal minus one disables without overwriting stored colors. |
| 2:43 | `tips_stop` | 0 |  | Partial | explicit | Set Tips collection mode to 256. |
| 2:44 | `tips_get_flag` | 0 |  | Partial | explicit | Read the Tips pause flag. |
| 2:45 | `tips_init` | 0 |  | Partial | explicit | Clear the complete Tips subsystem state. |
| 2:46 | `tips_pause` | 0 |  | Partial | explicit | Toggle pause bit and return its previous value. |
| 2:56 | `bgv_play` | 4 | slot (0, SoundSlot), resource (1, ResourceStringFromFileDat), flags (2, Integer), fade_ms (3, DurationMs) | Partial | explicit | Play a BGV lane independently of ADV voice; -1 allocates one of eight lanes. |
| 2:57 | `bgv_stop` | 2 | slot (0, SoundSlot), fade_ms (1, DurationMs) | Partial | explicit | Stop one BGV lane or all eight for -1, optionally fading out. |
| 2:67 | `bgv_pause` | 3 | slot (0, SoundSlot), paused (1, Integer), fade_ms (2, DurationMs) | Partial | explicit | Pause or resume a BGV lane or all eight for -1. |
| 2:68 | `bgv_mute` | 1 | muted (0, Integer) | Verified | explicit | Mute BGV group 2 while retaining its configured percentage. |
| 2:69 | `set_bgv_volume` | 1 | percent (0, Integer) | Verified | explicit | Cache BGV percentage and update group 2 through its mute latch. |
| 2:70 | `get_bgv_volume` | 0 |  | Verified | explicit | Return configured BGV percentage regardless of mute. |
| 3:2 | `sp_set` | 4 | slot (1, SpriteSlot), name (0, ResourceStringFromFileDat), x (2, CoordinateX), y (3, CoordinateY) | Blocked | explicit | Load/set a sprite slot from a File.dat resource name. |
| 3:3 | `sp_set_ex` | 5 | slot (1, SpriteSlot), name (0, ResourceStringFromFileDat), x (2, CoordinateX), y (3, CoordinateY), z (4, CoordinateZ) | Blocked | explicit | Load/set a sprite slot with explicit z/priority. |
| 3:4 | `sp_set_pos` | 4 | slot (0, SpriteSlot), x (1, CoordinateX), y (2, CoordinateY), z (3, CoordinateZ) | Blocked | explicit | Move a sprite slot immediately. |
| 3:5 | `sp_cls` | 1 | slot (0, SpriteSlot) | Blocked | explicit | Release/clear a sprite slot. |
| 3:6 | `sp_set_alpha` | 2 | slot (0, SpriteSlot), alpha (1, Alpha) | Verified | explicit | Set the sprite/button alpha lane by replacing the high byte of the native color field. |
| 3:7 | `set_priority` | 1 | lane (0, Integer) | Verified | explicit | Advance the Game sprite priority cursor. |
| 3:8 | `sp_get_filename` | 2 | slot (0, SpriteSlot), dst_slot (1, BufferPointer) | Verified | explicit | Copy sprite resource filename/name to dynamic string storage. |
| 3:9 | `sp_set_center` | 3 | slot (0, SpriteSlot), center_x (1, CoordinateX), center_y (2, CoordinateY) | Verified | explicit | Set PalSprite center offset for a sprite slot. |
| 3:11 | `sp_cls_ex` | 2 | first_slot (0, SpriteSlot), count (1, Integer) | Blocked | explicit | Clear count consecutive sprite slots from first_slot. |
| 3:12 | `set_filter` | 2 | slot (0, SpriteSlot), filter_mode (1, Mode) | Partial | explicit | Transform sprite pixels cumulatively, or restore the backed-up original for mode zero. |
| 3:13 | `sp_cls_transition` | 2 | slot (0, SpriteSlot), transition_id (1, Integer) | Blocked | automatic | Reachable Game.exe extcall 0003:000D (sp_cls_transition) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 3:14 | `sp_move_ex` | 4 | slot (0, SpriteSlot), x (1, CoordinateX), y (2, CoordinateY), z (3, CoordinateZ) | Blocked | automatic | Reachable Game.exe extcall 0003:000E (sp_move_ex) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 3:15 | `sp_set_rect_pos` | 3 | slot (0, SpriteSlot), cell_x (1, Integer), cell_y (2, Integer) | Verified | explicit | Set the active sprite cell/rect position. |
| 3:16 | `sp_set_render_mode` | 2 | slot (0, SpriteSlot), mode (1, Mode) | Verified | explicit | Set sprite render/blend mode. |
| 3:17 | `sp_set_scale` | 2 | slot (0, SpriteSlot), scale (1, Integer) | Verified | explicit | Set the sprite scale lane; native handler does not recompute x/y placement. |
| 3:18 | `sp_set_rotate` | 4 | slot (0, SpriteSlot), x (1, Integer), y (2, Integer), z (3, Integer) | Verified | explicit | Set sprite x/y/z rotation lanes. |
| 3:19 | `face_init` | 5 | face_id (0, Integer), sprite_slot (1, SpriteSlot), center_x (2, Coordinate), center_y (3, Coordinate), priority_lane (4, Integer) | Verified | explicit | Initialize a face/part slot mapping and anchor table entry. |
| 3:20 | `face_set` | 4 | resource (0, ResourceStringFromFileDat), face_id (1, Integer), dx (2, Coordinate), dy (3, Coordinate) | Verified | explicit | Load or clear a face/standing-picture part through the face table. |
| 3:21 | `sp_get_color` | 1 | slot (0, SpriteSlot) | Verified | explicit | Return a sprite's RGB color, or -1 when no image exists. |
| 3:22 | `sptext` | 5 | slot (0, SpriteSlot), text_id (1, TextId), x (2, CoordinateX), y (3, CoordinateY), mode (4, Mode) | Blocked | explicit | Create a transparent text sprite through the PAL font/text surface path. |
| 3:23 | `face_cls` | 1 | face_id (0, Integer) | Verified | explicit | Release the sprite currently mapped to a face table entry. |
| 3:24 | `sp_set_rect` | 5 | slot (0, SpriteSlot), left (1, CoordinateX), top (2, CoordinateY), right (3, CoordinateX), bottom (4, CoordinateY) | Verified | explicit | Set a sprite source rectangle. |
| 3:25 | `sp_set_pos_move` | 4 | slot (0, SpriteSlot), dx (1, CoordinateX), dy (2, CoordinateY), dz (3, CoordinateZ) | Blocked | explicit | Move or tween a sprite by a relative delta. |
| 3:26 | `sp_get_alpha` | 1 | slot (0, SpriteSlot) | Verified | explicit | Return a sprite alpha value, or -1 when no image exists. |
| 3:27 | `sp_get_rotate` | 2 | slot (0, SpriteSlot), axis (1, Mode) | Verified | explicit | Return one sprite rotation axis. |
| 3:28 | `sp_get_pos_to_mem` | 4 | slot (0, SpriteSlot), dst_x (1, BufferPointer), dst_y (2, BufferPointer), dst_z (3, BufferPointer) | Verified | explicit | Write sprite x/y/z position into Mem.dat destinations. |
| 3:29 | `sp_get_width` | 1 | slot (0, SpriteSlot) | Verified | explicit | Return PAL logical/cell sprite width. |
| 3:30 | `sp_get_height` | 1 | slot (0, SpriteSlot) | Verified | explicit | Return PAL logical/cell sprite height. |
| 3:31 | `sp_set_anim2` | 9 | slot (0, SpriteSlot), arg1 (1, Integer), arg2 (2, Integer), arg3 (3, Integer), arg4 (4, Integer), resource (5, ResourceStringFromFileDat), arg6 (6, Integer), arg7 (7, Integer), lane (8, Integer) | Blocked | explicit | Configure native sprite mix/animation lane with nine parameters. |
| 3:32 | `sp_create` | 1 | slot (0, SpriteSlot) | Blocked | automatic | Reachable Game.exe extcall 0003:0020 (sp_create) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 3:34 | `sp_set_anim_param` | 2 | slot (0, SpriteSlot), value (1, Integer) | Verified | explicit | Store native sprite parameter at ctx+666396. |
| 3:35 | `sp_get_anim_param` | 1 | slot (0, SpriteSlot) | Verified | explicit | Return native sprite parameter at ctx+666396. |
| 3:36 | `sp_get_scale` | 1 | slot (0, SpriteSlot) | Verified | explicit | Return a sprite scale as a PAL percent value, or -1 when no image exists. |
| 3:37 | `sp_set_color` | 2 | slot (0, SpriteSlot), rgb (1, Color) | Verified | explicit | Set a sprite RGB color while preserving alpha. |
| 3:38 | `sp_bitblt` | 2 | dst_slot (0, SpriteSlot), src_slot (1, SpriteSlot) | Blocked | automatic | Reachable Game.exe extcall 0003:0026 (sp_bitblt) for sprite/render-tree state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 3:39 | `sp_set_shake` | 4 | slot (0, SpriteSlot), amplitude (1, Integer), count (2, Integer), axis (3, Mode) | Blocked | explicit | Apply PAL-style shake animation to a sprite slot; VM currently models it as alternating offset motion. |
| 3:40 | `sp_paint` | 6 | slot (0, SpriteSlot), color_resource (1, ResourceStringFromFileDat), left (2, CoordinateX), top (3, CoordinateY), right (4, CoordinateX), bottom (5, CoordinateY) | Verified | explicit | Replace pixels inside a surface rectangle; an all-zero rectangle paints the entire surface. |
| 3:41 | `sp_set_anim` | 3 | slot (0, SpriteSlot), name (1, ResourceStringFromFileDat), flag (2, Flag) | Blocked | explicit | Attach sheet or named ANI animation to a sprite slot. |
| 3:44 | `sp_set_vis_clip` | 1 | slot (0, SpriteSlot) | Verified | explicit | Enable the native sprite vis-clip bit. |
| 3:46 | `sp_show` | 1 | slot (0, SpriteSlot) | Blocked | explicit | Show sprite; VM calls PalSpriteViewCtrl(slot, true) after popping only the slot. |
| 3:47 | `sp_hide` | 1 | slot (0, SpriteSlot) | Blocked | explicit | Hide sprite; VM calls PalSpriteViewCtrl(slot, false) after popping only the slot. |
| 3:48 | `is_sp` | 1 | slot (0, SpriteSlot) | Verified | explicit | Test whether a sprite slot currently exists. |
| 3:49 | `sp_set_child` | 5 | parent_slot (0, SpriteSlot), child_slot (1, SpriteSlot), offset_x (2, CoordinateX), offset_y (3, CoordinateY), child_index (4, Integer) | Verified | explicit | Attach a child sprite lane to a parent sprite. |
| 3:50 | `sp_set_transition` | 4 | slot (1, SpriteSlot), name (0, ResourceStringFromFileDat), transition_id (2, Integer), duration_ms (3, DurationMs) | Blocked | explicit | Replace a sprite image and queue a PAL transition from the old image to the new image. |
| 3:51 | `sp_copy_image` | 1 | slot (0, SpriteSlot) | Blocked | explicit | Move the current sprite image into the wrapper's transition source/copy lane. |
| 3:52 | `sp_transition` | 3 | slot (0, SpriteSlot), transition_id (1, Integer), duration_ms (2, DurationMs) | Blocked | explicit | Start a PAL transition for the current sprite, using any saved source image lane. |
| 3:53 | `set_aspect_position_type` | 2 | slot (0, SpriteSlot), type_index (1, Mode) | Partial | explicit | Store PAL aspect-position conversion type for a sprite slot. |
| 3:54 | `get_backbuffer` | 1 | slot (0, SpriteSlot) | Blocked | explicit | Copy a sprite image into PAL's active backbuffer. |
| 3:55 | `sp_set_mask` | 9 | slot (0, SpriteSlot), dst_x (1, CoordinateX), dst_y (2, CoordinateY), width (3, Integer), height (4, Integer), mask_resource (5, ResourceStringFromFileDat), mask_x (6, CoordinateX), mask_y (7, CoordinateY), lane (8, Integer) | Verified | explicit | Apply an alpha mask resource to a live sprite surface. |
| 3:56 | `sp_set_motion_pos` | 9 | slot (0, SpriteSlot), arg1 (1, Integer), arg2 (2, Integer), arg3 (3, Integer), arg4 (4, Integer), resource (5, ResourceStringFromFileDat), arg6 (6, Integer), arg7 (7, Integer), lane (8, Integer) | Blocked | explicit | Configure native sprite motion/bitblt lane with nine parameters. |
| 3:57 | `sp_set_anim` | 3 | slot (0, SpriteSlot), name (1, ResourceStringFromFileDat), flag (2, Flag) | Blocked | explicit | Alias/wrapper for sprite animation setup. |
| 4:0 | `bgm_play` | 7 | slot (0, SoundSlot), name (1, ResourceStringFromFileDat), flags (2, Flag), fade_units (3, Integer), initial_bytes (4, Integer), loop_start_bytes (5, Integer), loop_end_bytes (6, Integer) | Partial | explicit | Load and play BGM using the native byte-range descriptor and fade flags. |
| 4:1 | `bgm_stop` | 2 | slot (0, SoundSlot), fade_ms (1, DurationMs) | Blocked | explicit | Stop BGM slot, optionally with fade. |
| 4:2 | `bgm_set_volume` | 1 | volume (0, Volume) | Blocked | explicit | Set global BGM volume; runtime pops 1 arg (volume only, no per-slot). |
| 4:3 | `bgm_get_volume` | 0 |  | Partial | explicit | Return the configured BGM percent. |
| 4:4 | `bgm_get_auto_volume` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 0004:0004 (bgm_get_auto_volume) for BGM audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 4:6 | `bgm_set_auto_volume` | 2 | enabled (0, Flag), volume (1, Volume) | Blocked | explicit | Configure BGM automatic volume/ducking state. |
| 4:8 | `get_bgm_filename` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 0004:0008 (get_bgm_filename) for BGM audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 4:9 | `bgm_load` | 5 | slot (0, SoundSlot), name (1, ResourceStringFromFileDat), initial_bytes (2, Integer), loop_start_bytes (3, Integer), loop_end_bytes (4, Integer) | Partial | explicit | Preload BGM; any nonzero range word enables the native PCM byte range. |
| 4:11 | `set_master_volume` | 1 | volume (0, Volume) | Blocked | explicit | Set primary/master volume percent. |
| 4:12 | `get_master_volume` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 0004:000C (get_master_volume) for BGM audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 4:13 | `mute_master_volume` | 1 | muted (0, Flag) | Blocked | explicit | Mute/unmute primary volume while preserving configured percent. |
| 4:14 | `bgm_mute` | 1 | muted (0, Flag) | Blocked | explicit | Mute/unmute BGM group while preserving configured percent. |
| 4:15 | `mute_bgm_auto_volume` | 1 | muted (0, Flag) | Blocked | explicit | Toggle BGM automatic volume mute latch. |
| 5:0 | `se_load` | 2 | slot (0, SoundSlot), name (1, ResourceStringFromFileDat) | Blocked | explicit | Preload SE resource. |
| 5:1 | `se_play` | 5 | slot (0, SoundSlot), name (1, ResourceStringFromFileDat), unknown (2, Unknown), flags (3, Flag), volume (4, Volume) | Blocked | explicit | Play SE resource. |
| 5:2 | `se_play_ex` | 5 | slot (0, SoundSlot), name (1, ResourceStringFromFileDat), flags (2, Flag), fade_ms (3, DurationMs), group (4, Mode) | Verified | explicit | Load and play an SE resource; flags select loop/fade, the last operand selects the PAL sound group. |
| 5:3 | `se_stop` | 2 | slot (0, SoundSlot), fade_ms (1, DurationMs) | Blocked | explicit | Stop SE slot. |
| 5:4 | `se_set_volume` | 2 | volume (0, Volume), slot (1, SoundSlot) | Verified | explicit | Set SE lane volume. |
| 5:5 | `se_get_volume` | 1 | slot (0, SoundSlot) | Blocked | explicit | Read SE volume. |
| 5:6 | `se_unload` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 0005:0006 (se_unload) for sound-effect audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 5:7 | `se_wait` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 0005:0007 (se_wait) for sound-effect audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 5:8 | `channel_error_set_se_info` | 4 | arg0 (0, Unknown), arg1 (1, Unknown), arg2 (2, Unknown), arg3 (3, Unknown) | StackDisciplineOnly | explicit | Preserve the SE channel metadata call's stack contract. |
| 5:14 | `se_mute` | 2 | muted (0, Flag), slot (1, SoundSlot) | Verified | explicit | Mute/unmute SE lane while preserving cached SE volume. |
| 6:0 | `select_init` | 4 | background (0, ResourceStringFromFileDat), normal_color (1, Color), hover_color (2, Color), color3 (3, Color) | Partial | explicit | Configure the select background image and three native colors. |
| 6:1 | `select` | 1 | text_visible (0, Flag) | Partial | explicit | Begin a choice block and set ADV window visibility. |
| 6:2 | `select_set` | 6 | text_id (0, TextId), target (1, PointId), x (2, CoordinateX), y (3, CoordinateY), flags (4, Flag), frame_offset (5, Integer) | Partial | explicit | Declare an option: flags bit 0 shows it, bit 1 enables it; last argument selects the background row. |
| 6:3 | `select_commit` | 0 |  | Partial | explicit | Poll the choice, rewinding until selected, then jump to its point and run reaction callbacks. |
| 6:4 | `select_clear` | 0 |  | Verified | explicit | Release choice backgrounds and labels, retaining initialization and callbacks. |
| 6:5 | `select_set_offset` | 2 | index (0, Integer), offset (1, Integer) | Verified | explicit | Set an option background's base row to offset times three. |
| 6:6 | `select_set_process` | 3 | enter (0, PointId), leave (1, PointId), choose (2, PointId) | Verified | explicit | Register enter, leave and selection callbacks; zero disables a callback. |
| 6:7 | `select_lock` | 1 | locked (0, Flag) | Verified | explicit | Force background base row zero while locked; selection remains enabled. |
| 6:8 | `get_select_on_key` | 0 |  | Verified | explicit | Current hovered option index or -1. |
| 6:9 | `get_select_pull_key` | 0 |  | Verified | explicit | Previous polling pass's option index or -1. |
| 6:10 | `get_select_push_key` | 0 |  | Verified | explicit | Chosen option index when reaction bit 0x200 is set; otherwise -1. |
| 7:0 | `wait` | 2 | duration_ms (0, DurationMs), skip_cancel (1, Flag) | Verified | explicit | Timed script wait with native cancel/skip flag. |
| 7:1 | `wait_click` | 1 | duration_ms (0, DurationMs) | Verified | explicit | -1 completes the active text task; non-negative values are click-or-time waits. |
| 7:2 | `wait_sync_begin` | 0 |  | Verified | explicit | Begin PAL wait-sync timing scope. |
| 7:3 | `wait_sync_release` | 1 | duration_ms (0, DurationMs) | Verified | explicit | Release wait-sync with timed block. |
| 7:4 | `wait_sync_end` | 0 |  | Verified | explicit | End wait-sync timing scope. |
| 7:5 | `wait_sync_step` | 0 |  | Verified | explicit | One-frame wait/work fence used by script animation loops. |
| 7:6 | `wait_clear` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 0007:0006 (wait_clear) for wait/task scheduler state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 7:7 | `wait_click_no_anim` | 1 | duration_ms (0, DurationMs) | Verified | explicit | Input wait without wait-icon animation side effects; non-positive durations are click-only. |
| 7:8 | `wait_sync_get_time` | 0 |  | Verified | explicit | Read elapsed PAL wait-sync time. |
| 7:9 | `wait_time_push` | 0 |  | Blocked | explicit | Push PAL wait time state. |
| 7:10 | `wait_time_pop` | 0 |  | Blocked | explicit | Pop PAL wait time state. |
| 8:0 | `btn_init` | 3 | group (0, ButtonSlot), normal_image (1, ResourceStringFromFileDat), hover_image (2, ResourceStringFromFileDat) | Verified | explicit | Initialize a native PAL button group with normal/hover image resources. |
| 8:1 | `btn_uninit` | 1 | group (0, ButtonSlot) | Verified | explicit | Release a button group; group=-1 is treated as all groups by the compatibility runtime. |
| 8:3 | `btn_set` | 7 | group (0, ButtonSlot), index (1, ButtonSlot), normal (2, ResourceStringFromFileDat), hover (3, ResourceStringFromFileDat), push (4, ResourceStringFromFileDat), disabled (5, ResourceStringFromFileDat), entry_flag (6, Flag) | Blocked | explicit | Create/register a button entry and its sprite states. |
| 8:4 | `btn_hide` | 2 | group (0, ButtonSlot), index (1, ButtonSlot) | Verified | explicit | Hide a registered button entry. |
| 8:5 | `btn_show` | 2 | group (0, ButtonSlot), index (1, ButtonSlot) | Verified | explicit | Show a registered button entry. |
| 8:6 | `btn_set_pos` | 4 | group (0, ButtonSlot), index (1, ButtonSlot), x (2, CoordinateX), y (3, CoordinateY) | Blocked | explicit | Position a button entry. |
| 8:8 | `btn_release` | 2 | group (0, ButtonSlot), index (1, ButtonSlot) | Blocked | explicit | Release one or more button entries. |
| 8:9 | `btn_slider_get` | 5 | group (0, ButtonSlot), index (1, ButtonSlot), max_offset (2, Integer), axis (3, Mode), snap_to_100 (4, Flag) | Verified | explicit | Query/update slider button offset from mouse position for settings sliders. |
| 8:10 | `btn_slider_set` | 5 | group (0, ButtonSlot), index (1, ButtonSlot), offset (2, Integer), axis (3, Mode), enabled (4, Flag) | Verified | explicit | Initialize/update compatible slider offset and button state. |
| 8:11 | `btn_slider_begin` | 2 | group (0, ButtonSlot), index (1, ButtonSlot) | Verified | explicit | Arm native slider polling for a button entry. |
| 8:12 | `btn_on_check` | 2 | group (0, ButtonSlot), index (1, ButtonSlot) | Verified | explicit | Query current native reaction state for one button. |
| 8:13 | `btn_set_toggle` | 3 | group (0, ButtonSlot), index (1, ButtonSlot), toggle (2, Flag) | Verified | explicit | Select the button visual/toggle state for a group/index. |
| 8:14 | `btn_get_pos` | 4 | group (0, ButtonSlot), index (1, ButtonSlot), x_slot (2, Slot), y_slot (3, Slot) | Partial | explicit | Write the button position to two temporary-memory slots. |
| 8:15 | `btn_enable` | 3 | group (0, ButtonSlot), index (1, ButtonSlot), enabled (2, Flag) | Verified | explicit | Enable or disable button input/visual state for one entry or a group wildcard. |
| 8:16 | `btn_set_alpha_0x` | 3 | group (0, ButtonSlot), index (1, ButtonSlot), alpha (2, Alpha) | Verified | explicit | Set button sprite alpha for one entry or a group wildcard. |
| 8:17 | `btn_get_push` | 1 | group (0, ButtonSlot) | Verified | explicit | Poll/consume the native button reaction for a group; compatible no-push sentinel is 0. |
| 8:18 | `error_btn_expansion` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 0008:0012 (error_btn_expansion) for button/input sprite state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 8:19 | `btn_lock` | 2 | group (0, ButtonSlot), duration_ms (1, DurationMs) | Verified | explicit | Lock input for a button group; second argument is the native duration/timer value. |
| 8:20 | `btn_unlock` | 1 | group (0, ButtonSlot) | Verified | explicit | Unlock button input for a group. |
| 8:21 | `btn_set_anim` | 4 | group (0, ButtonSlot), index (1, ButtonSlot), anim_resource (2, ResourceStringFromFileDat), play_flag (3, Flag) | Blocked | explicit | Bind/play button animation resource for an already registered button. |
| 8:22 | `btn_set_hit` | 2 | group (0, ButtonSlot), index (1, ButtonSlot) | Verified | explicit | Bind/update the PAL reaction target for an existing button entry. |
| 8:23 | `btn_get_onmouse` | 1 | group (0, ButtonSlot) | Verified | explicit | Return current hover/onmouse button index for a group. |
| 8:24 | `btn_anim_clear` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 0008:0018 (btn_anim_clear) for button/input sprite state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 8:26 | `btn_onmouse_clear` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 0008:001A (btn_onmouse_clear) for button/input sprite state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 8:29 | `btn_set_state` | 4 | group (0, ButtonSlot), index (1, ButtonSlot), ctrl (2, Mode), state (3, Mode) | Partial | explicit | Set button control mode and visual cell/state for menus. |
| 8:36 | `btn_mode` | 1 | release_mode (0, Flag) | Verified | explicit | Choose press-edge activation (zero) or native release-edge activation (nonzero). |
| 8:37 | `btn_get_alpha_0x` | 2 | group (0, ButtonSlot), index (1, ButtonSlot) | Partial | explicit | Read the current button sprite alpha. |
| 9:0 | `skip_set` | 1 | enabled (0, Flag) | Verified | explicit | Set the ADV text skip latch. |
| 9:1 | `skip_is` | 0 |  | Verified | explicit | Return the ADV text skip latch. |
| 9:2 | `auto_set` | 1 | enabled (0, Flag) | Verified | explicit | Set the ADV text auto latch. |
| 9:3 | `auto_is` | 0 |  | Verified | explicit | Return the ADV text auto latch. |
| 9:4 | `auto_set_speed` | 1 | speed_percent (0, Integer) | Verified | explicit | Set PAL text auto/typewriter speed percent. |
| 9:5 | `auto_get_time` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 0009:0005 (auto_get_time) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 9:6 | `window_change_mode` | 1 | mode (0, Mode) | Verified | explicit | Request a PAL window mode change. |
| 9:7 | `window_set_mode_cache` | 1 | mode (0, Mode) | Verified | explicit | Cache logical window mode without posting a window-change message. |
| 9:8 | `effect_enable` | 1 | enabled (0, Flag) | Verified | explicit | Enable or disable PAL visual effects. |
| 9:9 | `effect_enable_is` | 0 |  | Verified | explicit | Query whether PAL visual effects are enabled. |
| 9:10 | `window_get_mode_cache` | 0 |  | Verified | explicit | Return cached logical window/layout mode. |
| 9:12 | `input_mouse_to_mem` | 2 | dst_x (0, BufferPointer), dst_y (1, BufferPointer) | Verified | explicit | Write mouse x/y into Mem.dat destinations. |
| 9:14 | `memory_bank_clear` | 0 |  | Verified | explicit | Clear native VM scratch memory bank. |
| 9:15 | `load_font` | 1 | font_resource (0, ResourceStringFromFileDat) | Verified | explicit | Load a PAL bitmap font DAT resource. |
| 9:16 | `unload_font` | 0 |  | Verified | explicit | Unload the PAL bitmap font while retaining the extended font sheet. |
| 9:17 | `set_language` | 1 | font_type (0, Mode) | Verified | explicit | Set PAL font type/language. |
| 9:18 | `input_key_cancel` | 2 | key_mask (0, Integer), extended_mask (1, Integer) | Verified | explicit | Cancel selected PAL input key/mouse masks. |
| 9:19 | `set_font_color` | 2 | text_color (0, Color), effect_color (1, Color) | Verified | explicit | Set PAL shared UI font colors. |
| 9:20 | `load_font_ex` | 1 | font_resource (0, ResourceStringFromFileDat) | Partial | explicit | Load an extended font TGA resource. |
| 9:21 | `memory_stack_push` | 0 |  | Verified | explicit | Snapshot the native script memory bank onto the memory stack. |
| 9:22 | `memory_stack_pop` | 0 |  | Verified | explicit | Restore the last native script memory-bank snapshot. |
| 9:23 | `list_stack_push_point` | 1 | point_id (0, PointId) | Verified | explicit | Push a resolved script point onto the native list stack. |
| 9:24 | `list_stack_pop_count` | 1 | count_mode (0, Integer) | Verified | explicit | Pop native list-stack continuation entries. |
| 9:25 | `system_scratch_clear` | 0 |  | Verified | explicit | Clear Game.exe ctx+804244 system/list scratch latch. |
| 9:26 | `system_scratch_get` | 0 |  | Verified | explicit | Return Game.exe ctx+804244 system/list scratch latch. |
| 9:27 | `set_font_size` | 1 | font_size (0, Integer) | Verified | explicit | Set PAL shared UI font size and return previous size. |
| 9:28 | `get_font_size` | 0 |  | Verified | explicit | Return PAL shared UI font size. |
| 9:29 | `get_font_type` | 0 |  | Verified | explicit | Return PAL shared UI font type/language. |
| 9:30 | `set_font_effect` | 1 | effect (0, Mode) | Verified | explicit | Set PAL shared UI font effect mode. |
| 9:31 | `get_font_effect` | 0 |  | Verified | explicit | Return PAL shared UI font effect mode. |
| 9:32 | `get_pull_key` | 0 |  | Partial | explicit | Use the recovered native input wrapper stack contract. |
| 9:33 | `get_on_key` | 0 |  | Partial | explicit | Use the recovered native input wrapper stack contract. |
| 9:34 | `get_push_key` | 0 |  | Partial | explicit | Use the recovered native input wrapper stack contract. |
| 9:35 | `input_clear` | 0 |  | Partial | explicit | Use the recovered native input wrapper stack contract. |
| 9:36 | `change_window_size` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 0009:0024 (change_window_size) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 9:48 | `set_cursor_null` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 0009:0030 (set_cursor_null) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 9:49 | `set_hide_cursor_time` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 0009:0031 (set_hide_cursor_time) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 9:51 | `scene_skip` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 0009:0033 (scene_skip) for font/system state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 9:52 | `cancel_scene_skip` | 0 |  | Verified | explicit | Cancel native scene-skip latch and update save-point state. |
| 9:53 | `list_stack_get_count` | 0 |  | Verified | explicit | Return native list-stack depth for tag 2. |
| 9:54 | `get_async_key` | 1 | virtual_key (0, Integer) | Partial | explicit | Use the recovered native input wrapper stack contract. |
| 10:0 | `save` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000A:0000 (save) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 10:1 | `load` | 1 | slot (0, SpriteSlot) | Blocked | automatic | Reachable Game.exe extcall 000A:0001 (load) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 10:4 | `save_set_thumbnail_size` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000A:0004 (save_set_thumbnail_size) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 10:5 | `thumbnail_set` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000A:0005 (thumbnail_set) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 10:7 | `save_set_font_size` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000A:0007 (save_set_font_size) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 10:9 | `is_save` | 1 | slot (0, SpriteSlot) | Blocked | automatic | Reachable Game.exe extcall 000A:0009 (is_save) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 10:11 | `savepoint` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000A:000B (savepoint) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 10:12 | `save_thumbnail_mosaic_set` | 1 | enabled (0, Flag) | Blocked | explicit | Set save thumbnail mosaic/processing flag used by save UI rendering. |
| 10:13 | `savetimedraw` | 5 | sprite_slot (0, SpriteSlot), save_slot (1, Integer), x (2, CoordinateX), y (3, CoordinateY), format_mode (4, Mode) | Blocked | explicit | Draw save-file modification time text into a save/load UI sprite slot. |
| 10:14 | `savedaydraw` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000A:000E (savedaydraw) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 10:15 | `save_set_text_rect` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000A:000F (save_set_text_rect) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 10:16 | `savetextdraw` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000A:0010 (savetextdraw) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 10:17 | `get_new_savefile` | 1 | slot (0, SpriteSlot) | Blocked | automatic | Reachable Game.exe extcall 000A:0011 (get_new_savefile) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 10:22 | `thumbnail_renew` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000A:0016 (thumbnail_renew) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 10:24 | `set_load_after_process` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000A:0018 (set_load_after_process) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 10:25 | `savesystemdata` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000A:0019 (savesystemdata) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 10:26 | `save_set_font_effect` | 1 | value (0, Integer) | Blocked | automatic | Reachable Game.exe extcall 000A:001A (save_set_font_effect) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 10:27 | `save_set_font_color_0x_0x` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000A:001B (save_set_font_color_0x_0x) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 10:28 | `delete_file` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000A:001C (delete_file) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 10:29 | `save_tmp_dat` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000A:001D (save_tmp_dat) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 10:30 | `copy_file` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000A:001E (copy_file) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 10:32 | `save_lock_not_open_savefileno` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000A:0020 (save_lock_not_open_savefileno) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 10:33 | `is_save_lock` | 1 | slot (0, SpriteSlot) | Blocked | automatic | Reachable Game.exe extcall 000A:0021 (is_save_lock) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 10:34 | `is_prev_data` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000A:0022 (is_prev_data) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 10:35 | `save_point_clear` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000A:0023 (save_point_clear) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 10:36 | `save_point_lock` | 1 | slot (0, SpriteSlot) | Blocked | automatic | Reachable Game.exe extcall 000A:0024 (save_point_lock) for save/load UI state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 11:0 | `movie_play` | 2 | name (0, ResourceStringFromFileDat), flags (1, Mode) | Partial | explicit | Start the global full-screen movie and gate normal game updates. |
| 11:1 | `msp_set_loop_sp_ep` | 8 | name (0, ResourceStringFromFileDat), slot (1, SpriteSlot), x (2, CoordinateX), y (3, CoordinateY), z (4, CoordinateZ), loop_mode (5, Mode), loop_start_ms (6, Integer), loop_end_ms (7, Integer) | Partial | explicit | Load a generic sprite, set movie loop points, and start its movie worker. |
| 11:2 | `msp_cls` | 1 | slot (0, SpriteSlot) | Partial | explicit | Release a script sprite slot through generic sp_cls. |
| 11:3 | `msp_wait` | 1 | slot (0, SpriteSlot) | Partial | explicit | Wait for natural movie completion or a scheduler skip state. |
| 11:4 | `msp_lock` | 1 | slot (0, SpriteSlot) | Partial | explicit | Set the movie-worker and renderer lock flags. |
| 11:5 | `msp_unlock` | 1 | slot (0, SpriteSlot) | Partial | explicit | Clear the movie-worker and renderer lock flags. |
| 12:0 | `system_btn_set` | 3 | index (0, ButtonSlot), image (1, Integer), state (2, Mode) | Verified | explicit | Bind an input shortcut slot (16 = right mouse button) to a script gosub point. |
| 12:1 | `system_btn_release` | 1 | index (0, ButtonSlot) | Blocked | explicit | Release a system/menu button entry. |
| 12:2 | `system_btn_enable` | 2 | index (0, ButtonSlot), enabled (1, Flag) | Verified | explicit | Enable/disable a system/menu button entry. |
| 13:0 | `voice_play` | 4 | slot (0, SoundSlot), name (1, ResourceStringFromFileDat), flags (2, Flag), volume (3, Volume) | Blocked | explicit | Play voice resource. |
| 13:1 | `voice_stop` | 2 | slot (0, SoundSlot), fade_ms (1, DurationMs) | Blocked | explicit | Stop voice slot. |
| 13:2 | `voice_set_volume` | 1 | volume (0, Volume) | Blocked | explicit | Set global voice volume; runtime pops 1 arg (volume only, no per-slot). |
| 13:3 | `voice_get_volume` | 0 |  | Blocked | explicit | Read global voice volume. |
| 13:4 | `set_voice_info` | 4 | slot (0, Integer), name (1, ResourceStringFromFileDat), volume (2, Integer), enabled (3, Integer) | Partial | explicit | Register voice prefix metadata; name=-1 preserves the previous name. |
| 13:5 | `voice_enable` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000D:0005 (voice_enable) for voice/BGV audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 13:6 | `is_voice_enable` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000D:0006 (is_voice_enable) for voice/BGV audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 13:7 | `voice_info_enable` | 1 | enabled (0, Flag) | Verified | explicit | Enable or bypass the native voice prefix configuration registry. |
| 13:8 | `bgv_play` | 4 | slot (0, SoundSlot), resource (1, ResourceStringFromFileDat), flags (2, Integer), fade_ms (3, DurationMs) | Partial | explicit | Play a BGV lane independently of ADV voice; -1 allocates one of eight lanes. |
| 13:9 | `bgv_stop` | 2 | slot (0, SoundSlot), fade_ms (1, DurationMs) | Partial | explicit | Stop one BGV lane or all eight for -1, optionally fading out. |
| 13:11 | `get_voice_ex_volume` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000D:000B (get_voice_ex_volume) for voice/BGV audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 13:12 | `set_voice_ex_volume` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000D:000C (set_voice_ex_volume) for voice/BGV audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 13:14 | `voice_autopan_initialize` | 1 | limit (0, Integer) | Verified | explicit | Set maximum voice pan in units of 100 DirectSound hundredths of a dB. |
| 13:15 | `voice_autopan_enable` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000D:000F (voice_autopan_enable) for voice/BGV audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 13:16 | `set_voice_autopan_size_over` | 4 | slot (0, Integer), target (1, Integer), name (2, TextStringFromTextDat), mode (3, Integer) | Verified | explicit | Configure or clear one native voice auto-pan table entry. |
| 13:17 | `is_voice_autopan_enable` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000D:0011 (is_voice_autopan_enable) for voice/BGV audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 13:18 | `voice_wait` | 1 | slot (0, SoundSlot) | Blocked | explicit | Wait for voice slot completion. |
| 13:19 | `bgv_pause` | 3 | slot (0, SoundSlot), paused (1, Integer), fade_ms (2, DurationMs) | Partial | explicit | Pause or resume a BGV lane or all eight for -1. |
| 13:20 | `bgv_mute` | 1 | muted (0, Integer) | Verified | explicit | Mute BGV group 2 while retaining its configured percentage. |
| 13:21 | `set_bgv_volume` | 1 | percent (0, Integer) | Verified | explicit | Cache BGV percentage and update group 2 through its mute latch. |
| 13:22 | `get_bgv_volume` | 0 |  | Verified | explicit | Return configured BGV percentage regardless of mute. |
| 13:24 | `voice_mute` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000D:0018 (voice_mute) for voice/BGV audio state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 14:0 | `history_init_0x_0x` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000E:0000 (history_init_0x_0x) for history/backlog state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 14:1 | `historybegin_lpbyte_ptagdata_sztext` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000E:0001 (historybegin_lpbyte_ptagdata_sztext) for history/backlog state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 14:2 | `history_end` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000E:0002 (history_end) for history/backlog state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 14:3 | `history_scroll` | 0 |  | Verified | explicit | Return native history scroll/current offset. |
| 14:4 | `ext_000E_0004` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000E:0004 (ext_000E_0004) for history/backlog state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 14:5 | `history_get_height` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000E:0005 (history_get_height) for history/backlog state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 14:6 | `history_update` | 0 |  | Blocked | explicit | Update interactive history/backlog hover/voice state. |
| 14:7 | `ext_000E_0007` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000E:0007 (ext_000E_0007) for history/backlog state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 14:8 | `ext_000E_0008` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000E:0008 (ext_000E_0008) for history/backlog state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 14:9 | `ext_000E_0009` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000E:0009 (ext_000E_0009) for history/backlog state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 14:10 | `history_set_rect` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000E:000A (history_set_rect) for history/backlog state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 14:11 | `history_clear` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 000E:000B (history_clear) for history/backlog state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 14:12 | `history_set` | 1 | resource (0, ResourceStringFromFileDat) | Partial | explicit | Set the history icon resource and enable its use; icon rendering remains incomplete. |
| 14:20 | `history_get_text` | 4 | record (0, Integer), name_destination (1, DynamicString), voice_destination (2, DynamicString), body_destination (3, DynamicString) | Partial | explicit | Copy a history record's name, voice filename and body into dynamic strings and return its saved instruction PC. |
| 15:2 | `set_window_text` | 2 | text (0, TextId), title_mode (1, Mode) | Partial | explicit | Capture an unlocked savepoint; native debug mode also changes the host title. |
| 15:4 | `debug_print` | 8 | format (0, TextId), value1 (1, Integer), value2 (2, Integer), value3 (3, Integer), value4 (4, Integer), value5 (5, Integer), value6 (6, Integer), value7 (7, Integer) | Partial | explicit | Emit formatted native debugger output when debug mode is enabled. |
| 15:5 | `debug_window_set` | 1 | state (0, Mode) | Verified | explicit | Set PAL debug-window state and return previous state. |
| 16:0 | `effect_stop_skip` | 0 |  | Blocked | explicit | Stop native transition/effect channels during skip/menu teardown. |
| 16:1 | `screen_shake` | 4 | x_amp (0, Integer), y_amp (1, Integer), duration_ms (2, DurationMs), phase (3, Mode) | Blocked | explicit | Start a native screen-shake effect. |
| 16:2 | `screen_flash` | 4 | red (0, Color), green (1, Color), blue (2, Color), duration_ms (3, DurationMs) | Verified | explicit | Create a full-screen PAL flash/fill overlay. |
| 17:0 | `action_run` | 2 | action_id (0, Integer), duration_ms (1, DurationMs) | Partial | explicit | Start an asynchronous action bank and finish all sections at its deadline. |
| 17:1 | `action_sync_run` | 2 | action_id (0, Integer), duration_ms (1, DurationMs) | Partial | explicit | Start an action bank and wait until its deadline. |
| 17:2 | `action_loop_run` | 2 | action_id (0, Integer), cycles (1, Integer) | Partial | explicit | Repeat all parallel action lines; -1 repeats indefinitely. |
| 17:3 | `action_clear` | 1 | action_id (0, Integer) | Partial | explicit | Apply pending final values and clear the resolved action bank. |
| 17:4 | `action_sync_wait` | 1 | action_id (0, Integer) | Partial | explicit | Poll action banks through the selected bank without consuming the argument twice. |
| 17:5 | `action_position` | 6 | line_id (0, Integer), sprite_slot (1, SpriteSlot), dx (2, Integer), dy (3, Integer), dz (4, Integer), duration_ms (5, DurationMs) | Partial | explicit | Append a linear position-delta section. |
| 17:6 | `action_move_to` | 5 | line_id (0, Integer), sprite_slot (1, SpriteSlot), column (2, Integer), curve (3, Integer), duration_ms (4, DurationMs) | Partial | explicit | Append a move to a native twenty-column placement position. |
| 17:7 | `action_shake` | 5 | line_id (0, Integer), sprite_slot (1, SpriteSlot), axis (2, Integer), amplitude (3, Integer), duration_ms (4, DurationMs) | Partial | explicit | Append a decaying random shake section. |
| 17:8 | `action_alpha_delta` | 4 | line_id (0, Integer), sprite_slot (1, SpriteSlot), alpha_delta (2, Integer), duration_ms (3, DurationMs) | Partial | explicit | Append a signed sixteen-bit alpha-delta section. |
| 17:9 | `action_line_wait` | 2 | line_id (0, Integer), duration_ms (1, DurationMs) | Partial | explicit | Append a wait section without changing the active bank. |
| 17:10 | `action_rotate` | 6 | line_id (0, Integer), sprite_slot (1, SpriteSlot), dx (2, Integer), dy (3, Integer), dz (4, Integer), duration_ms (5, DurationMs) | Partial | explicit | Append a rotation-delta section. |
| 17:11 | `action_scale` | 4 | line_id (0, Integer), sprite_slot (1, SpriteSlot), scale_delta (2, Integer), duration_ms (3, DurationMs) | Partial | explicit | Append a scale-delta section in percent units. |
| 17:12 | `action_push` | 0 |  | Partial | explicit | Back up the running active bank to the single native backup slot. |
| 17:13 | `action_pop` | 0 |  | Partial | explicit | Finish the destination bank and restore the native backup. |
| 17:14 | `action_position_decelerate` | 6 | line_id (0, Integer), sprite_slot (1, SpriteSlot), dx (2, Integer), dy (3, Integer), dz (4, Integer), duration_ms (5, DurationMs) | Partial | explicit | Append a position section using the native logarithmic deceleration curve. |
| 17:15 | `action_position_accelerate` | 6 | line_id (0, Integer), sprite_slot (1, SpriteSlot), dx (2, Integer), dy (3, Integer), dz (4, Integer), duration_ms (5, DurationMs) | Partial | explicit | Append a position section using the native logarithmic acceleration curve. |
| 17:16 | `action_sound` | 5 | line_id (0, Integer), sound_slot (1, Integer), loop (2, Integer), fade (3, Integer), duration_ms (4, DurationMs) | Partial | explicit | Append a loaded sound playback section; callback timing differs by native build. |
| 17:17 | `action_shake_periodic` | 6 | line_id (0, Integer), sprite_slot (1, SpriteSlot), axis (2, Integer), amplitude (3, Integer), period_ms (4, DurationMs), duration_ms (5, DurationMs) | Partial | explicit | Append periodic sine shake with optional decaying amplitude. |
| 17:18 | `action_rect_position` | 5 | line_id (0, Integer), sprite_slot (1, SpriteSlot), cell_x (2, Integer), cell_y (3, Integer), duration_ms (4, DurationMs) | Partial | explicit | Append a sprite-sheet cell selection callback. |
| 17:19 | `action_rect` | 7 | line_id (0, Integer), sprite_slot (1, SpriteSlot), x (2, Integer), y (3, Integer), width (4, Integer), height (5, Integer), duration_ms (6, DurationMs) | Partial | explicit | Append a source-rectangle callback. |
| 17:20 | `action_scale_curve` | 5 | line_id (0, Integer), sprite_slot (1, SpriteSlot), curve (2, Integer), scale_delta (3, Integer), duration_ms (4, DurationMs) | Partial | explicit | Append a scale delta with a native interpolation curve. |
| 17:21 | `action_rotate_curve` | 7 | line_id (0, Integer), sprite_slot (1, SpriteSlot), curve (2, Integer), dx (3, Integer), dy (4, Integer), dz (5, Integer), duration_ms (6, DurationMs) | Partial | explicit | Append a rotation delta with a native interpolation curve. |
| 17:23 | `set_active_action` | 1 | action_id (0, Integer) | Partial | explicit | Set the active bank and return its previous index. |
| 17:24 | `get_active_action` | 0 |  | Partial | explicit | Return the active action bank index. |
| 17:25 | `action_run_all` | 1 | duration_ms (0, DurationMs) | Partial | explicit | Start all idle action banks asynchronously. |
| 17:26 | `action_sync_run_all` | 1 | duration_ms (0, DurationMs) | Partial | explicit | Start all idle action banks synchronously. |
| 17:27 | `action_loop_run_all` | 1 | cycles (0, Integer) | Partial | explicit | Start all idle action banks in loop mode. |
| 17:28 | `action_clear_all` | 0 |  | Partial | explicit | Apply final values and clear all action banks. |
| 17:29 | `action_position_sine` | 6 | line_id (0, Integer), sprite_slot (1, SpriteSlot), dx (2, Integer), dy (3, Integer), dz (4, Integer), duration_ms (5, DurationMs) | Partial | explicit | Append a sine position section; the extended native build scales entry coordinates by 1.5. |
| 17:30 | `set_action_clear` | 1 | action_id (0, Integer) | Partial | explicit | Mark the resolved action bank for deferred clearing. |
| 17:31 | `action_rect_tween` | 8 | line_id (0, Integer), sprite_slot (1, SpriteSlot), mode (2, Mode), left (3, Integer), top (4, Integer), right (5, Integer), bottom (6, Integer), duration_ms (7, DurationMs) | Verified | explicit | Append a section that tweens the sprite source rect (type 16) with the shared easing modes. |
| 17:35 | `action_mask` | 5 | line_id (0, Integer), from_slot (1, SpriteSlot), mask_slot (2, SpriteSlot), to_slot (3, SpriteSlot), duration_ms (4, DurationMs) | Verified | explicit | Append a section that reveals the target's alpha through a mask image (type 20). |
| 18:1 | `app_exec` | 1 | path (0, TextStringFromTextDat) | Blocked | explicit | Open a resolved path through the OS shell; portable runtime records but does not launch. |
| 18:3 | `string_not_equal` | 2 | lhs (0, TextStringFromTextDat), rhs (1, TextStringFromTextDat) | Verified | explicit | Case-insensitive string inequality test used by table scan loops. |
| 18:4 | `strcatf_string` | 2 | dst (0, BufferPointer), src (1, TextStringFromTextDat) | Verified | explicit | Append resolved string to dynamic string buffer. |
| 18:5 | `strgetcf` | 3 | text (0, TextStringFromTextDat), offset (1, Integer), length (2, Integer) | Verified | explicit | Read a character byte or parse a numeric substring from a resolved string. |
| 18:6 | `string_alloc` | 1 | ignored (0, Integer) | Verified | explicit | Allocate/clear one native rotating dynamic string buffer and return its 0x10000000-tagged handle. |
| 18:7 | `strcpyf_string` | 3 | dst (0, BufferPointer), src (1, TextStringFromTextDat), len (2, Integer) | Verified | explicit | Copy resolved string to dynamic string buffer with optional length. |
| 18:8 | `file_exist` | 1 | path (0, TextStringFromTextDat) | Verified | explicit | Return whether a resolved resource/path exists. |
| 18:9 | `wsprint` | 10 | dst (0, BufferPointer), format (1, TextStringFromTextDat), value0 (2, Integer), value1 (3, Integer), value2 (4, Integer), value3 (5, Integer), value4 (6, Integer), value5 (7, Integer), value6 (8, Integer), value7 (9, Integer) | Verified | explicit | Format a script string into a dynamic string buffer. |
| 18:10 | `check_disc` | 1 | volume_label (0, TextStringFromTextDat) | Verified | explicit | Check mounted disc volume label; portable runtime accepts the check. |
| 18:12 | `strlen` | 1 | value (0, TextStringFromTextDat) | Verified | explicit | Return byte length of a script/dynamic string. |
| 18:13 | `process_checkpoint_set` | 1 | checkpoint_id (0, PointId) | Blocked | explicit | Store native script/process checkpoint id for the return/list scheduler. |
| 18:14 | `update_access` | 1 | entry (0, TextStringFromTextDat) | Blocked | explicit | Mark native access/read flag entry. |
| 18:15 | `system_task_value` | 0 |  | Blocked | explicit | Return native task-data value when the system latch is active. |
| 18:17 | `work_process_value` | 1 | arg_base_value (0, Integer) | Blocked | explicit | Query/update native background work-process value. |
| 18:18 | `strreplace_string` | 3 | dst (0, BufferPointer), needle (1, TextStringFromTextDat), replacement (2, TextStringFromTextDat) | Verified | explicit | Replace first substring occurrence in dynamic string buffer. |
| 18:21 | `strlenf` | 1 | value (0, TextStringFromTextDat) | Verified | explicit | Return native strlen byte length for a resolved script/dynamic string. |
| 18:28 | `attach_work_process` | 0 |  | Verified | explicit | Attach the Game.exe/PAL background work-process pump. |
| 18:29 | `detach_work_process` | 0 |  | Verified | explicit | Detach the Game.exe/PAL background work-process pump flag. |
| 18:30 | `openfile` | 1 | name (0, TextStringFromTextDat) | Blocked | explicit | Open resource/file by resolved script string; returns runtime file handle or 0. |
| 18:31 | `read_file` | 3 | handle (0, Handle), temp_offset (1, BufferPointer), count (2, Integer) | Blocked | explicit | Read file/table bytes into VM temp memory. |
| 18:32 | `close_file_not_handle` | 1 | handle (0, Handle) | Verified | explicit | Close and release an open_file table/resource handle. |
| 18:33 | `set_file_pointer` | 3 | handle (0, Handle), offset (1, Integer), origin (2, Mode) | Blocked | explicit | Seek an opened runtime file handle. |
| 18:34 | `file_string` | 3 | handle (0, Handle), entry (1, Integer), dst_slot (2, BufferPointer) | Verified | explicit | Copy a parsed open_file table string entry into dynamic string storage. |
| 18:35 | `set_last_process` | 1 | point_id (0, PointId) | Blocked | explicit | Store native last-process point bookkeeping. |
| 18:36 | `sz_buf` | 5 | section (0, TextStringFromTextDat), key (1, TextStringFromTextDat), default (2, TextStringFromTextDat), filename (3, IniFilename), buffer_or_size (4, BufferPointer) | Blocked | explicit | Read INI string into dynamic string storage. |
| 18:37 | `getprivateprofileint` | 4 | section (0, TextStringFromTextDat), key (1, TextStringFromTextDat), default (2, Integer), filename (3, IniFilename) | Blocked | explicit | Read an integer from SYSTEM.INI or requested INI. |
| 18:40 | `access_clear` | 1 | entry (0, Integer) | Blocked | explicit | Clear native access/read flag entry. |
| 18:90 | `ext_18_90` | 8 | arg0 (0, Unknown), arg1 (1, Unknown), arg2 (2, Unknown), arg3 (3, Unknown), arg4 (4, Unknown), arg5 (5, Unknown), arg6 (6, Unknown), arg7 (7, Unknown) | StackDisciplineOnly | explicit | Preserve the eight-argument extension's stack contract. |
| 18:121 | `pal_time_ms` | 0 |  | Partial | explicit | Read the monotonically advancing PAL millisecond clock. |
| 19:0 | `fx_effect_cls` | 1 | slot (0, SpriteSlot) | Verified | explicit | Release the sprite FX state and parameter block. |
| 19:1 | `fx_raster_stop` | 2 | slot (0, SpriteSlot), duration_ms (1, DurationMs) | Verified | explicit | Enter the raster ending phase. |
| 19:2 | `fx_effect_wait` | 1 | slot (0, SpriteSlot) | Verified | explicit | Wait until FX ends or input cancels the wait. |
| 19:3 | `fx_raster_start` | 4 | slot (0, SpriteSlot), x_percent (1, Integer), y_percent (2, Integer), duration_ms (3, DurationMs) | Verified | explicit | Start raster FX with a shared wave phase and amplitude ramp. |
| 20:0 | `random` | 2 | min (0, Integer), max (1, Integer) | Blocked | explicit | Return an inclusive random integer between the two popped bounds. |
| 21:0 | `create_thread` | 1 | point_id (0, PointId) | Verified | explicit | Start native script-thread scheduler state at a point id. |
| 21:1 | `exit_thread` | 0 |  | Verified | explicit | Clear native script-thread scheduler state. |
| 21:2 | `suspend_thread` | 1 | running (0, Flag) | Verified | explicit | Set native script-thread running flag and return the previous value. |
| 21:3 | `get_thread` | 0 |  | Verified | explicit | Return current native script-thread id. |
| 22:0 | `run` | 3 | effect_id (0, Integer), duration_ms (1, DurationMs), effect_arg (2, Integer) | Verified | explicit | Run a scene/effect transition and wait for it. |
| 22:1 | `run_no_wait` | 0 |  | Verified | explicit | Latch/flush a no-wait scene transition; Game.exe category 22 index 1 does not pop transition parameters. |
| 22:2 | `run_stack` | 1 | enabled (0, Flag) | Verified | explicit | Toggle Game.exe run-stack transition mode. |
| 22:4 | `fx_effect_cls` | 1 | slot (0, SpriteSlot) | Verified | explicit | Release the sprite FX state and parameter block. |
| 22:5 | `fx_raster_stop` | 2 | slot (0, SpriteSlot), duration_ms (1, DurationMs) | Verified | explicit | Enter the raster ending phase. |
| 22:6 | `fx_effect_wait` | 1 | slot (0, SpriteSlot) | Verified | explicit | Wait until FX ends or input cancels the wait. |
| 22:7 | `fx_raster_start` | 4 | slot (0, SpriteSlot), x_percent (1, Integer), y_percent (2, Integer), duration_ms (3, DurationMs) | Verified | explicit | Start raster FX with a shared wave phase and amplitude ramp. |
| 23:0 | `create_message` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 0017:0000 (create_message) for message queue state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 23:1 | `get_message` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 0017:0001 (get_message) for message queue state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
| 23:2 | `get_message_param` | 0 |  | Blocked | automatic | Reachable Game.exe extcall 0017:0002 (get_message_param) for message queue state; parameters are named in runtime pop order and must stay synchronized with pal-vm pop_ext_args. |
