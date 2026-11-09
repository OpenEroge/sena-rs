//! A `mai` translation patch as the runtime sees it: replaced text records,
//! replaced sprite pixels and the `chg` file overlay.
use super::tests::empty_core_assets;
use super::*;
use pal_asset::{MaiPatch, MaiText};

fn png(width: u32, height: u32, pixel: [u8; 4]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&pixel.repeat((width * height) as usize)).unwrap();
    writer.finish().unwrap();
    bytes
}

fn temp_root(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("sena_mai_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("data")).unwrap();
    std::fs::create_dir_all(root.join("mai").join("img")).unwrap();
    std::fs::create_dir_all(root.join("mai").join("chg")).unwrap();
    std::fs::write(root.join("mai").join("word.mai"), vec![0u8; 0x1_0000]).unwrap();
    root
}

#[test]
fn script_strings_take_the_patched_record_of_the_same_id() {
    let (mut assets, _) = empty_core_assets(false);
    let mut text_dat = b"$TEXT_LIST__".to_vec();
    text_dat.extend_from_slice(&2u32.to_le_bytes());
    for (id, text) in [(5u32, &b"orig"[..]), (6, b"kept")] {
        text_dat.extend_from_slice(&id.to_le_bytes());
        text_dat.extend_from_slice(text);
        text_dat.push(0);
    }
    assets.text_dat.bytes = text_dat;
    let patch: MaiText = [(5, b"patched".to_vec())].into_iter().collect();
    assets.apply_text_patch(&patch);
    let runtime = ScriptRuntime::boot(12, ScriptRuntimeConfig::default());
    assert_eq!(
        runtime.resolve_script_string(0x10, &assets, Nls::ShiftJis).as_deref(),
        Some("patched")
    );
    assert_eq!(
        runtime.resolve_script_string(0x19, &assets, Nls::ShiftJis).as_deref(),
        Some("kept")
    );
}

#[test]
fn patch_pixels_replace_a_sprite_of_the_same_size_only() {
    let root = temp_root("pixels");
    let original = png(2, 1, [0, 0, 255, 255]);
    std::fs::write(root.join("data").join("SAME.PGD"), &original).unwrap();
    std::fs::write(root.join("data").join("WIDE.PGD"), &original).unwrap();
    std::fs::write(root.join("mai").join("img").join("SAME.PGD.mai"), png(2, 1, [255, 0, 0, 255]))
        .unwrap();
    std::fs::write(root.join("mai").join("img").join("WIDE.PGD.mai"), png(3, 1, [255, 0, 0, 255]))
        .unwrap();
    let mut resources = ResourceManager::new(&root, Nls::ShiftJis);
    resources.add_path("data");
    resources.set_mai_patch(MaiPatch::open(&root.join("mai")).unwrap());

    let same = open_resource_variant(&mut resources, "same", IMAGE_EXTENSIONS).unwrap();
    let decoded = decode_asset_image(&mut resources, &same).unwrap();
    assert_eq!(&decoded.rgba[..4], &[255, 0, 0, 255]);

    let wide = open_resource_variant(&mut resources, "WIDE", IMAGE_EXTENSIONS).unwrap();
    let decoded = decode_asset_image(&mut resources, &wide).unwrap();
    assert_eq!((decoded.width, decoded.height), (2, 1));
    assert_eq!(&decoded.rgba[..4], &[0, 0, 255, 255]);
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn chg_files_shadow_every_search_path() {
    let root = temp_root("chg");
    std::fs::write(root.join("data").join("TIPS.CSV"), b"original").unwrap();
    std::fs::write(root.join("mai").join("chg").join("TIPS.CSV.mai"), b"patched").unwrap();
    let mut resources = ResourceManager::new(&root, Nls::ShiftJis);
    resources.add_path("data");
    assert_eq!(resources.open("tips.csv").unwrap().bytes, b"original");
    resources.set_mai_patch(MaiPatch::open(&root.join("mai")).unwrap());
    assert_eq!(resources.open("tips.csv").unwrap().bytes, b"patched");
    std::fs::remove_dir_all(&root).unwrap();
}
