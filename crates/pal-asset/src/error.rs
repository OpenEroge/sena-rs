use std::fmt;
use std::io;
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, AssetError>;

#[derive(Debug)]
pub enum AssetError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    EncodingDecode {
        nls: &'static str,
    },
    EncodingEncode {
        nls: &'static str,
        text: String,
    },
    NameTooLong {
        encoded_len: usize,
        max_len: usize,
        name: String,
    },
    EmptyAssetName,
    InvalidArchiveDat {
        reason: String,
    },
    PacTooSmall {
        path: PathBuf,
        len: usize,
    },
    PacBucketOutOfRange {
        path: PathBuf,
        bucket: usize,
        first: u32,
        count: u32,
    },
    PacRecordOutOfRange {
        path: PathBuf,
        record_index: u32,
    },
    PacDataOutOfRange {
        path: PathBuf,
        offset: u32,
        size: u32,
    },
    PacUnsupportedBucket {
        first_byte: u8,
    },
    AssetNotFound {
        name: String,
    },
    InvalidEncryptedFile {
        name: String,
        len: usize,
    },
    InvalidMaiPatch {
        reason: String,
    },
    Vfs {
        path: PathBuf,
        reason: String,
    },
}

impl AssetError {
    pub(crate) fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}

impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "I/O error at {}: {}", path.display(), source),
            Self::EncodingDecode { nls } => write!(f, "failed to decode bytes as {}", nls),
            Self::EncodingEncode { nls, text } => {
                write!(f, "failed to encode {:?} as {}", text, nls)
            }
            Self::NameTooLong {
                encoded_len,
                max_len,
                name,
            } => write!(
                f,
                "asset name {:?} encodes to {} bytes, but PAL pac keys allow at most {} bytes",
                name, encoded_len, max_len
            ),
            Self::EmptyAssetName => write!(f, "asset name is empty"),
            Self::InvalidArchiveDat { reason } => write!(f, "invalid archive.dat: {}", reason),
            Self::PacTooSmall { path, len } => write!(
                f,
                "pac file {} is too small: {} bytes, expected at least 0x804",
                path.display(),
                len
            ),
            Self::PacBucketOutOfRange {
                path,
                bucket,
                first,
                count,
            } => write!(
                f,
                "pac file {} has out-of-range bucket {}: first={}, count={}",
                path.display(),
                bucket,
                first,
                count
            ),
            Self::PacRecordOutOfRange { path, record_index } => write!(
                f,
                "pac file {} has out-of-range record index {}",
                path.display(),
                record_index
            ),
            Self::PacDataOutOfRange { path, offset, size } => write!(
                f,
                "pac file {} has out-of-range data region offset=0x{:08X}, size=0x{:08X}",
                path.display(),
                offset,
                size
            ),
            Self::PacUnsupportedBucket { first_byte } => write!(
                f,
                "PAL pac bucket table has 255 buckets; first byte 0x{:02X} cannot be represented",
                first_byte
            ),
            Self::AssetNotFound { name } => write!(f, "asset not found: {}", name),
            Self::InvalidEncryptedFile { name, len } => write!(
                f,
                "encrypted PAL file {} is too short: {} bytes, expected at least 0x10",
                name, len
            ),
            Self::InvalidMaiPatch { reason } => write!(f, "invalid mai patch: {}", reason),
            Self::Vfs { path, reason } => {
                write!(
                    f,
                    "virtual filesystem error at {}: {}",
                    path.display(),
                    reason
                )
            }
        }
    }
}

impl std::error::Error for AssetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}
