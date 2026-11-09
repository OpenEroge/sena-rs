# HiDPI presentation

This is host presentation, not recovered PAL behavior. The PAL screen keeps its
logical size (`DEF_CG_WIDTH/HEIGHT`). Scripts, hit tests, saves and every
logical rasterizer still see PAL pixels.

## Device mapping

- `scene::letterbox_rect(surface, logical)` is the single PAL-to-device
  mapping:
  - It uses a uniform scale and centers the PAL screen.
  - The remaining area is a black letterbox or pillarbox.
  - Edges snap to whole device pixels.
- These all use it:
  - The wgpu backend: geometry is in viewport space, and the render pass sets
    `set_viewport` and a scissor.
  - The software compositor: it composites the viewport, then blits it
    centered.
  - The cursor mapping in `PalInputState::map_physical_to_logical`. This keeps
    native truncation and the signed off-screen values.
  - Headless and window diagnostic clicks.
- The PAL `ChangeAspect` modes are still unmapped and only log a warning.

## Dense text surfaces

Host-drawn text is rasterized at a higher density on HiDPI windows:
- the ADV body (`adv:text`),
- the speaker name (`adv:name`),
- the backlog page (`history:text`).

**Density.**
- `scene::text_density` is the whole part of the letterbox scale, clamped to
  `1..=MAX_TEXT_DENSITY`.
- A dense surface is therefore only ever magnified: 1:1 with nearest sampling
  at integer scales, linear sampling otherwise.
- The engine passes the density to the runtime every frame
  (`set_text_density`); a change marks the ADV text dirty.

**Rasterization.**
- `PalFontSystem::rasterize_dense(text, d)` returns exactly `d` times the size
  `rasterize` returns, with the glyphs at the same PAL positions.
- Outline faces are rasterized again at `font_size * d`.
- The effect kernels scale by `d` (`raster::draw_glyph_dense`). The outline and
  thick outline become L1 balls of radius `d` and `2d`, and `d == 1` reproduces
  the native cross and diamond exactly.
- Bitmap fonts and native markup are magnified with `scene::upscale_rgba`.
- `draws_outline_text` gates the whole path. With a bitmap body font no dense
  surface is built, because it would add nothing.

**Composition.**
- The ADV cache (`AdvTextPanelCache::dense`) keeps a dense panel and a dense
  text block built on the logical line layout.
- `compose_adv_text_layer` scales these by `d`:
  - reveal limits,
  - line geometry,
  - the wait mark.
- The text-animation threshold map stays indexed by PAL pixels.
- TIPS recoloring copies the same spans at `d` times their size.
- The backlog draws each operation (hover plate, name, body lines) into the
  dense page alongside the plain one.

**Attachment.**
- `SpriteSystem::set_sprite_hires` stores the dense pixels on the surface
  together with its current generation.
- `SceneTexture::hires` carries them only while that generation is unchanged.
  Any pixel edit (`pixels_mut`, filters, replacement) bumps the generation, so
  the surface falls back to its plain pixels.
- Only the device renderers read the copy, through
  `SceneTexture::device_texels`. The software compositor scales
  `source_rect` bounds by the density.
- Logical consumers ignore the copy:
  - PNG snapshots,
  - save thumbnails,
  - transition captures.

**Backlog cache.**
- The backlog page is redrawn only when its inputs change
  (`HistoryPageKey`):
  - layout generation,
  - rect,
  - scroll start and record tops,
  - hover,
  - colors,
  - typeset parameters,
  - density.
- It used to be redrawn every frame. At density 2 that cost about 6 ms per
  frame.

## Diagnostics

- `--diagnostic-device-size WxH` (headless) emulates a physical surface:
  - Input maps through its letterbox.
  - PNG snapshots go through the device compositor at that size
    (`renderer::rasterize_scene_device_rgba`).
  - Without the flag, headless output is unchanged.
- `--window-dump-frame-at` with the wgpu backend now reads back the real GPU
  frame. The pass is recorded again into an offscreen copy of the surface, so
  this also works while the window is occluded.
