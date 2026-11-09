//! EXE 0x479900 / reference0x447FE0 / Koikake0x434E30 probe suffix
//! loaders until the first successful load. Koikake has only the first four.
//! Table 0x4EB680 is initialized at 0x401300; .mpg creates a movie sprite.
use super::*;

pub(super) const SUFFIXES: &[&str] = &[".pgd", ".mpg", ".tga", ".bmp", ".png", ".jpg"];

/// Recover the name a sprite was requested with from the loaded asset name.
/// The native record keeps the caller's string (trenka.exe 0x4250B0 copies
/// record+0x254, and save restore at 0x40D0F4 feeds that same string back
/// into the suffix-appending loader 0x447FA0), so the suffix appended by
/// `load_sprite_resource` must not leak into `sp_get_filename`.
pub(super) fn requested_sprite_name(asset_name: &str) -> &str {
    for suffix in SUFFIXES {
        let Some(split) = asset_name.len().checked_sub(suffix.len()) else {
            continue;
        };
        if asset_name.is_char_boundary(split) && asset_name[split..].eq_ignore_ascii_case(suffix) {
            return &asset_name[..split];
        }
    }
    asset_name
}

pub(super) fn load_sprite_resource(
    manager: &mut ResourceManager,
    movies: &mut MSpriteSystem,
    name: &str,
    extended: bool,
) -> anyhow::Result<(LoadedAsset, DecodedImage, Option<MSpriteHandle>)> {
    let mut last_error = anyhow::anyhow!("sprite resource {name:?} not found");
    let suffixes = if extended { SUFFIXES } else { &SUFFIXES[..4] };
    for suffix in suffixes {
        // The native helper always appends; an explicit extension is not
        // stripped by its caller or by the FILE.DAT string resolver.
        let Ok(mut asset) = manager.open(&format!("{name}{suffix}")) else {
            continue;
        };
        if *suffix == ".mpg" {
            log::debug!("[trace-assets] movie sprite {:?} from {:?}", asset.name, asset.source);
            match movies.load_movie(asset.name.clone(), std::mem::take(&mut asset.bytes)) {
                Ok(movie) => {
                    let image = DecodedImage {
                        width: movie.width,
                        height: movie.height,
                        cell_width: movie.width,
                        cell_height: movie.height,
                        offset_x: 0,
                        offset_y: 0,
                        rgba: movie.rgba,
                    };
                    return Ok((asset, image, Some(movie.handle)));
                }
                Err(error) => last_error = error,
            }
        } else {
            match decode_asset_image(manager, &asset) {
                Ok(image) => return Ok((asset, image, None)),
                Err(error) => last_error = error,
            }
        }
    }
    Err(last_error)
}

#[cfg(test)]
mod tests {
    use super::requested_sprite_name;

    #[test]
    fn requested_sprite_name_strips_one_loader_suffix() {
        assert_eq!(requested_sprite_name("ST09D_C030.pgd"), "ST09D_C030");
        assert_eq!(requested_sprite_name("OPENING.mpg"), "OPENING");
        assert_eq!(requested_sprite_name("a.pgd.pgd"), "a.pgd");
        assert_eq!(requested_sprite_name("sp_create:3"), "sp_create:3");
        assert_eq!(requested_sprite_name("#FF000000"), "#FF000000");
    }
}
