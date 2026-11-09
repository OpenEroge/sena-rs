use super::PalFontProfile;

/// Native effect kernels are L1 balls around the effect origin: the outline
/// (effect 1) has radius 1, the thick outline (effect 3) radius 2. A dense
/// raster scales the radius with the density so the outline keeps its PAL
/// pixel width. Stamps OR together, so their order does not matter.
fn l1_ring(radius: i32) -> impl Iterator<Item = (i32, i32)> {
    (-radius..=radius).flat_map(move |dy| {
        let span = radius - dy.abs();
        (-span..=span)
            .filter(move |dx| (*dx, dy) != (0, 0))
            .map(move |dx| (dx, dy))
    })
}

#[derive(Clone, Copy)]
enum Stamp {
    Overwrite,
    Or,
    Blend,
}

/// Recovered six-bit PAL glyph kernels, with checked destination clipping.
/// Native kernel loops can write outside a caller surface for negative/empty
/// rectangles; those memory faults are not reproduced by the portable adapter.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_glyph(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    coverage: &[u8],
    coverage_stride: u32,
    coverage_height: u32,
    x: i32,
    y: i32,
    color: u32,
    effect_color: u32,
    effect: u16,
    profile: PalFontProfile,
) {
    draw_glyph_dense(
        pixels,
        width,
        height,
        coverage,
        coverage_stride,
        coverage_height,
        x,
        y,
        color,
        effect_color,
        effect,
        profile,
        1,
    );
}

/// `draw_glyph` for coverage rasterized at `density` texels per PAL pixel:
/// every native offset and kernel radius is scaled by the density.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_glyph_dense(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    coverage: &[u8],
    coverage_stride: u32,
    coverage_height: u32,
    x: i32,
    y: i32,
    color: u32,
    effect_color: u32,
    effect: u16,
    profile: PalFontProfile,
    density: i32,
) {
    if effect > 3 || (effect == 3 && profile == PalFontProfile::Koikake) {
        return;
    }
    let density = density.max(1);
    let foreground_offset = if effect == 3 { 2 } else { 1 } * density;
    let mut stamp = |offset_x: i32, offset_y: i32, color: u32, mode: Stamp| {
        for source_y in 0..coverage_height {
            for source_x in 0..coverage_stride {
                let Some(&value) = coverage.get((source_y * coverage_stride + source_x) as usize)
                else {
                    return;
                };
                if value == 0 {
                    continue;
                }
                let px = i64::from(x) + i64::from(source_x) + i64::from(offset_x);
                let py = i64::from(y) + i64::from(source_y) + i64::from(offset_y);
                if px < 0 || py < 0 || px >= i64::from(width) || py >= i64::from(height) {
                    continue;
                }
                let index = (py as usize * width as usize + px as usize) * 4;
                let Some(destination) = pixels.get_mut(index..index + 4) else {
                    continue;
                };
                let alpha = value.wrapping_mul(4).wrapping_sub(1);
                let rgb = super::argb_to_rgba(color);
                let source = [rgb[0], rgb[1], rgb[2], rgb[3] | alpha];
                match mode {
                    Stamp::Overwrite => destination.copy_from_slice(&source),
                    Stamp::Or => {
                        for (dst, src) in destination.iter_mut().zip(source) {
                            *dst |= src;
                        }
                    }
                    Stamp::Blend if alpha == 255 || destination[3] == 0 => {
                        destination.copy_from_slice(&source)
                    }
                    Stamp::Blend => {
                        for channel in 0..4 {
                            let value = if channel == 3 {
                                255
                            } else {
                                u32::from(rgb[channel])
                            };
                            destination[channel] = ((value * u32::from(alpha)
                                + u32::from(destination[channel]) * (255 - u32::from(alpha)))
                                >> 8) as u8;
                        }
                    }
                }
            }
        }
    };
    match effect {
        1 => {
            for (dx, dy) in l1_ring(density) {
                stamp(density + dx, density + dy, effect_color, Stamp::Or);
            }
        }
        2 => stamp(2 * density, 2 * density, effect_color, Stamp::Overwrite),
        3 => {
            for (dx, dy) in l1_ring(2 * density) {
                stamp(2 * density + dx, 2 * density + dy, effect_color, Stamp::Or);
            }
        }
        _ => {}
    }
    stamp(foreground_offset, foreground_offset, color, Stamp::Blend);
}

#[cfg(test)]
mod tests {
    use super::l1_ring;

    #[test]
    fn unit_kernels_match_the_native_cross_and_diamond() {
        let mut cross: Vec<_> = l1_ring(1).collect();
        cross.sort();
        assert_eq!(cross, vec![(-1, 0), (0, -1), (0, 1), (1, 0)]);
        let diamond: Vec<_> = l1_ring(2).collect();
        assert_eq!(diamond.len(), 12);
        assert!(diamond.iter().all(|(x, y)| x.abs() + y.abs() <= 2 && (*x, *y) != (0, 0)));
    }
}
