//! Identify audited native binaries and read capabilities without executing them.
use std::path::Path;

use pal_asset::vfs;
use sha2::{Digest, Sha256};

/// Exact audited PAL/Game pairs, independent of script and export capabilities.
///
/// These variants name the reviewed releases, not every release of a title or
/// every engine revision with a similar export table.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum NativeBuild {
    #[default]
    Unknown,
    Koikake,
    Reference,
    Totsulover,
    /// `trenka.exe` with its Reference-like PAL.dll that lacks the registered
    /// DLL markers (reverse/evidence/iroha-trenka-interface-inventory.json).
    Trenka,
}

/// Extcall coordinates whose native wrapper matches the registered contract
/// even though the PAL.dll does not carry the registered markers. Each range
/// was compared handler by handler (pop count, debug strings and PAL calls);
/// see reverse/iroha-native-profile.md.
const TRENKA_REGISTERED_EXTCALLS: &[(u16, u16, u16)] = &[
    (2, 47, 47),
    (3, 59, 59),
    (5, 0, 14),
    (8, 42, 42),
    (9, 57, 57),
    (10, 37, 39),
    (14, 13, 19),
    (14, 21, 24),
    (18, 54, 54),
    (18, 88, 89),
    (18, 91, 91),
    (18, 96, 99),
];

/// Extcall coordinates where trenka.exe has its own audited wrapper: a new
/// coordinate or a different operand contract from the registered builds.
/// - 2:18 `text_n` (0x43F620) and 2:19 `text_cat` (0x43F3C0): four-operand
///   message lines without a backlog record; text_cat appends.
/// - 2:20 `set_history` (0x43E760): voice, name, text; records a backlog line.
/// - 3:13 `sp_cls_transition` (0x4293D0): slot, transition kind, duration.
/// - 3:32 `sp_create` (0x4267D0): slot, File.dat name, x, y, width, height.
/// - 3:38 `sp_bitblt` (0x426FD0): nine operands, sprite-to-sprite copy.
/// - 3:58 `drawtext` (0x42A3F0): wrapped text into an existing sprite.
/// - 10:29 (0x4317C0): swap two save slots through `save_tmp.dat`.
/// - Small wrappers in `runtime/trenka_wrappers.rs`: 2:35, 3:42, 3:43, 4:5,
///   4:16, 4:17, 8:28, 8:30, 9:48, 15:1, 15:9, 18:17, 18:43..48, 18:76,
///   18:90, 18:92, 18:109, 18:113..115.
/// - 8:27 `btn_blt` (0x40FEA0), 8:31/8:32 button tips word (0x410610/0x410720).
/// - 14:25 (0x42D1A0): per-record backlog separator sprite.
/// - 18:50, 57, 59..68, 72, 73, 79, 82..87, 106..108: dictionary table,
///   save-folder and string helpers, favourite voices with their global word
///   table, the texture-allocation hint and the scene-skip query.
const TRENKA_OWN_EXTCALLS: &[(u16, u16, u16)] = &[
    (2, 18, 20),
    (2, 35, 35),
    (3, 13, 13),
    (3, 32, 32),
    (3, 38, 38),
    (3, 42, 43),
    (3, 58, 58),
    (4, 5, 5),
    (4, 16, 17),
    (8, 27, 28),
    (8, 30, 32),
    (9, 48, 48),
    (10, 29, 29),
    (14, 25, 25),
    (15, 1, 1),
    (15, 9, 9),
    (18, 17, 17),
    (18, 43, 48),
    (18, 50, 50),
    (18, 57, 57),
    (18, 59, 68),
    (18, 72, 73),
    (18, 76, 76),
    (18, 79, 79),
    (18, 82, 87),
    (18, 90, 90),
    (18, 92, 92),
    (18, 106, 109),
    (18, 113, 115),
];

fn table_contains(table: &[(u16, u16, u16)], category: u16, index: u16) -> bool {
    table
        .iter()
        .any(|&(cat, first, last)| cat == category && (first..=last).contains(&index))
}

impl NativeBuild {
    /// Sprite category 3 index 14 writes the wrapper base position in
    /// trenka.exe (0x429060, "sp_set_pos_ex", +0x10..0x18 and +0x6C..0x74);
    /// the audited Game.exe handler writes the rotation layer instead.
    pub fn sprite_move_ex_sets_position(self) -> bool {
        self == NativeBuild::Trenka
    }

    /// Whether this build's own audited wrapper at `category:index` should be
    /// dispatched (see `TRENKA_OWN_EXTCALLS`).
    pub fn defines_extcall(self, category: u16, index: u16) -> bool {
        match self {
            NativeBuild::Trenka => table_contains(TRENKA_OWN_EXTCALLS, category, index),
            _ => false,
        }
    }

    /// Whether this build's wrapper at `category:index` follows the
    /// registered contract independently of the DLL capability markers.
    pub fn shares_registered_extcall(self, category: u16, index: u16) -> bool {
        match self {
            NativeBuild::Trenka => table_contains(TRENKA_REGISTERED_EXTCALLS, category, index),
            _ => false,
        }
    }
}

struct BinaryIdentity {
    size: usize,
    sha256: &'static str,
}

struct NativeIdentity {
    build: NativeBuild,
    pal: BinaryIdentity,
    game: BinaryIdentity,
}

// Complete-file hashes from reverse/evidence/native-interface-inventory.json,
// reverse/evidence/totsulover-interface-inventory.json and
// reverse/evidence/iroha-trenka-interface-inventory.json. Both files must
// match the same record: PAL exports alone cannot identify Game VM wrappers.
const NATIVE_IDENTITIES: &[NativeIdentity] = &[
    NativeIdentity {
        build: NativeBuild::Koikake,
        pal: BinaryIdentity {
            size: 1_250_304,
            sha256: "2a43bc4408aff086c18dc73ccd2d35be4f8a56cf8b3e82da7b75f80ffe61625e",
        },
        game: BinaryIdentity {
            size: 773_120,
            sha256: "93f228aa413aece0f047b27ddb1ea26d4e50316304e6c4029eae4a1a844f437b",
        },
    },
    NativeIdentity {
        build: NativeBuild::Reference,
        pal: BinaryIdentity {
            size: 1_233_408,
            sha256: "e676972ddaab2d45fc82210bb89143923f10d1053492a89e50571dbcec54840a",
        },
        game: BinaryIdentity {
            size: 1_053_696,
            sha256: "219395e636f8fb8eb1bc7790345b0de5294671b33f8542edf074b333b0722e8e",
        },
    },
    NativeIdentity {
        build: NativeBuild::Totsulover,
        pal: BinaryIdentity {
            size: 1_282_048,
            sha256: "744219b7c2b2ba80c5bb0440ac556201652ffbcc01d2de1211c4b576ad21db7e",
        },
        game: BinaryIdentity {
            size: 1_152_512,
            sha256: "4778fe70629a7c0ee6cb3caa30ce572cc45440ac392ea998b406ca45ec304483",
        },
    },
    NativeIdentity {
        build: NativeBuild::Trenka,
        pal: BinaryIdentity {
            size: 1_228_288,
            sha256: "2ef61fc3d6ba354dbb9e3a4da50fe82bc20a4a2e7e0cce401ec51120dc7b8463",
        },
        game: BinaryIdentity {
            size: 2_776_576,
            sha256: "1038558f878444ad6bf8964f32651396eefb843a2f138558684b4f718dd109fd",
        },
    },
];

const PAL_PATHS: &[&str] = &[
    "dll/Pal.dll",
    "dll/PAL.dll",
    "dll/pal.dll",
    "DLL/PAL.DLL",
    "Pal.dll",
    "PAL.dll",
    "pal.dll",
];

const GAME_PATHS: &[&str] = &[
    "koikake.exe",
    "Koikake.exe",
    "KOIKAKE.EXE",
    "Game.exe",
    "game.exe",
    "GAME.EXE",
    "totsulover_try.exe",
    "Totsulover_try.exe",
    "TOTSULOVER_TRY.EXE",
    // A patched sibling executable (same size, different digest) must not be
    // listed: any unmatched candidate makes the installation Unknown.
    "trenka.exe",
    "Trenka.exe",
    "TRENKA.EXE",
];

pub(crate) fn detect_native_build(root: &Path) -> NativeBuild {
    let pal = identify_candidates(root, PAL_PATHS, |identity| &identity.pal);
    let game = identify_candidates(root, GAME_PATHS, |identity| &identity.game);
    match (pal, game) {
        (Some(pal), Some(game)) if pal == game => pal,
        _ => NativeBuild::Unknown,
    }
}

fn identify_candidates(
    root: &Path,
    names: &[&str],
    binary: fn(&NativeIdentity) -> &BinaryIdentity,
) -> Option<NativeBuild> {
    let mut selected = None;
    for name in names {
        let path = root.join(name);
        if !vfs::is_file(&path) {
            continue;
        }
        let size = vfs::file_len(&path).ok()?;
        // Only read bounded, known candidate sizes. Neither a filename nor a
        // matching size enables a profile without the complete-file digest.
        if !NATIVE_IDENTITIES
            .iter()
            .any(|identity| binary(identity).size == size)
        {
            return None;
        }
        let bytes = vfs::read_range(&path, 0, size).ok()?;
        if bytes.len() != size || vfs::file_len(&path).ok()? != size {
            return None;
        }
        let digest = format!("{:x}", Sha256::digest(&bytes));
        let build = NATIVE_IDENTITIES.iter().find_map(|identity| {
            let expected = binary(identity);
            (expected.size == size && expected.sha256 == digest).then_some(identity.build)
        })?;
        // Ambiguous installations remain unknown, even if one candidate pair
        // matches. Case aliases of the same audited binary are harmless.
        if selected.is_some_and(|previous| previous != build) {
            return None;
        }
        selected = Some(build);
    }
    selected
}

pub(crate) fn has_registered_dispatch_exports(root: &Path) -> bool {
    [
        "dll/Pal.dll",
        "dll/PAL.dll",
        "dll/pal.dll",
        "DLL/PAL.DLL",
        "Pal.dll",
        "PAL.dll",
    ]
    .into_iter()
    .filter_map(|name| vfs::read_all(&root.join(name)).ok())
    .any(|bytes| {
        has_export(&bytes, b"PalSystemPathInit")
            && has_export(&bytes, b"PalGetChangeWindowModeTime")
    })
}

fn word(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(offset..offset.checked_add(2)?)?.try_into().ok()?,
    ))
}

fn dword(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn export_match(bytes: &[u8], requested: &[u8]) -> Option<bool> {
    if bytes.get(..2)? != b"MZ" {
        return Some(false);
    }
    let pe = dword(bytes, 0x3c)? as usize;
    if bytes.get(pe..pe.checked_add(4)?)? != b"PE\0\0" {
        return Some(false);
    }
    let optional = pe.checked_add(24)?;
    if word(bytes, optional)? != 0x10b {
        return Some(false);
    }
    let section_count = word(bytes, pe.checked_add(6)?)? as usize;
    let section_start = optional.checked_add(word(bytes, pe.checked_add(20)?)? as usize)?;
    if section_count > 96 {
        return Some(false);
    }
    let to_file = |rva: u32| -> Option<usize> {
        for index in 0..section_count {
            let section = section_start.checked_add(index.checked_mul(40)?)?;
            let start = dword(bytes, section.checked_add(12)?)?;
            let raw_size = dword(bytes, section.checked_add(16)?)?;
            if let Some(delta) = rva.checked_sub(start) {
                if delta < raw_size {
                    let raw = dword(bytes, section.checked_add(20)?)?;
                    return Some(raw.checked_add(delta)? as usize);
                }
            }
        }
        None
    };
    let directory = to_file(dword(bytes, optional.checked_add(96)?)?)?;
    let count = dword(bytes, directory.checked_add(24)?)? as usize;
    if count > 65536 {
        return Some(false);
    }
    let names = to_file(dword(bytes, directory.checked_add(32)?)?)?;
    for index in 0..count {
        let offset = to_file(dword(bytes, names.checked_add(index.checked_mul(4)?)?)?)?;
        if bytes.get(offset..offset.checked_add(requested.len())?) == Some(requested)
            && bytes.get(offset.checked_add(requested.len())?) == Some(&0)
        {
            return Some(true);
        }
    }
    Some(false)
}

fn has_export(bytes: &[u8], requested: &[u8]) -> bool {
    export_match(bytes, requested).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_names_reached_through_the_pe_export_directory_match() {
        let mut bytes = vec![0u8; 0x400];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
        bytes[0x86..0x88].copy_from_slice(&1u16.to_le_bytes());
        bytes[0x94..0x96].copy_from_slice(&0xe0u16.to_le_bytes());
        bytes[0x98..0x9a].copy_from_slice(&0x10bu16.to_le_bytes());
        bytes[0xf8..0xfc].copy_from_slice(&0x1000u32.to_le_bytes());
        for (offset, value) in [
            (0x184, 0x1000u32),
            (0x188, 0x200),
            (0x18c, 0x200),
            (0x218, 2),
            (0x220, 0x1040),
            (0x240, 0x1080),
            (0x244, 0x10a0),
        ] {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes[0x280..0x292].copy_from_slice(b"PalSystemPathInit\0");
        bytes[0x2a0..0x2bb].copy_from_slice(b"PalGetChangeWindowModeTime\0");
        assert!(has_export(&bytes, b"PalSystemPathInit"));
        assert!(has_export(&bytes, b"PalGetChangeWindowModeTime"));
        assert!(!has_export(&bytes, b"PalSystemPath"));
        bytes[0x240..0x244].copy_from_slice(&0xffff_ffffu32.to_le_bytes());
        assert!(!has_export(&bytes, b"PalSystemPathInit"));
    }

    #[test]
    fn registered_extcall_sharing_is_limited_to_audited_coordinates() {
        assert!(NativeBuild::Trenka.shares_registered_extcall(5, 1));
        assert!(NativeBuild::Trenka.shares_registered_extcall(18, 97));
        assert!(!NativeBuild::Trenka.shares_registered_extcall(18, 90));
        assert!(!NativeBuild::Trenka.shares_registered_extcall(5, 15));
        for build in [
            NativeBuild::Unknown,
            NativeBuild::Koikake,
            NativeBuild::Reference,
            NativeBuild::Totsulover,
        ] {
            assert!(!build.shares_registered_extcall(5, 1));
        }
    }

    #[test]
    fn truncated_pe_does_not_select_a_dispatch_profile() {
        for size in 0..256 {
            let mut bytes = vec![0xff; size];
            if size >= 2 {
                bytes[..2].copy_from_slice(b"MZ");
            }
            assert!(!has_export(&bytes, b"PalSystemPathInit"));
        }
        assert!(!has_export(b"MZPalSystemPathInit\0", b"PalSystemPathInit"));
    }
}
