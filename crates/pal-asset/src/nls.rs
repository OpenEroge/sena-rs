use std::borrow::Cow;
use std::str::FromStr;

use encoding_rs::{Encoding, GBK, SHIFT_JIS, UTF_8};

use crate::error::{AssetError, Result};
use crate::mai::MaiCharset;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Nls {
    #[default]
    ShiftJis,
    Gbk,
    Utf8,
    /// Shift-JIS bytes remapped by a `mai` translation patch. Only
    /// [`crate::MaiPatch`] produces it, so it has no command-line spelling.
    Mai(&'static MaiCharset),
}

impl Nls {
    pub fn name(self) -> &'static str {
        match self {
            Self::ShiftJis => "sjis",
            Self::Gbk => "gbk",
            Self::Utf8 => "utf-8",
            Self::Mai(_) => "mai",
        }
    }

    /// The byte-level encoding. A `mai` charset keeps the Shift-JIS byte
    /// structure, so it reports Shift-JIS here.
    pub fn encoding(self) -> &'static Encoding {
        match self {
            Self::ShiftJis | Self::Mai(_) => SHIFT_JIS,
            Self::Gbk => GBK,
            Self::Utf8 => UTF_8,
        }
    }

    pub fn decode(self, bytes: &[u8]) -> Result<String> {
        if let Self::Mai(charset) = self {
            return Ok(charset.decode(bytes));
        }
        let (decoded, _encoding_used, had_errors) = self.encoding().decode(bytes);
        if had_errors {
            return Err(AssetError::EncodingDecode { nls: self.name() });
        }
        Ok(match decoded {
            Cow::Borrowed(s) => s.to_owned(),
            Cow::Owned(s) => s,
        })
    }

    pub fn encode(self, text: &str) -> Result<Vec<u8>> {
        if let Self::Mai(charset) = self {
            return charset.encode(text).ok_or_else(|| AssetError::EncodingEncode {
                nls: self.name(),
                text: text.to_owned(),
            });
        }
        let (encoded, _encoding_used, had_errors) = self.encoding().encode(text);
        if had_errors {
            return Err(AssetError::EncodingEncode {
                nls: self.name(),
                text: text.to_owned(),
            });
        }
        Ok(match encoded {
            Cow::Borrowed(bytes) => bytes.to_vec(),
            Cow::Owned(bytes) => bytes,
        })
    }
}

impl FromStr for Nls {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "sjis" | "shift-jis" | "shift_jis" | "cp932" | "932" => Ok(Self::ShiftJis),
            "gbk" | "cp936" | "936" => Ok(Self::Gbk),
            "utf8" | "utf-8" => Ok(Self::Utf8),
            other => Err(format!(
                "unsupported nls {:?}; expected sjis, gbk, or utf-8",
                other
            )),
        }
    }
}
