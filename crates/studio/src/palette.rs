use std::borrow::Cow;
use std::fmt::Display;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::color::Color;

#[derive(Clone, PartialEq, Eq)]
pub enum DynamicPalette {
    Builtin(lutgen_palettes::Palette),
    Custom(String),
}

impl Display for DynamicPalette {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

pub fn lutgen_dir() -> PathBuf {
    std::env::var("LUTGEN_DIR")
        .map(Into::into)
        .unwrap_or(dirs::config_dir().unwrap().join("lutgen"))
}

impl DynamicPalette {
    /// Get all palettes from disk & builtin
    pub fn get_all() -> std::io::Result<Vec<Self>> {
        let mut palettes = lutgen_palettes::Palette::VARIANTS
            .iter()
            .map(|v| DynamicPalette::Builtin(*v))
            .collect::<Vec<_>>();

        let path = std::env::var("LUTGEN_DIR")
            .map(Into::into)
            .unwrap_or(dirs::config_dir().unwrap().join("lutgen"));
        if path.is_dir() {
            for entry in std::fs::read_dir(path)? {
                let path = entry?.path();
                palettes.push(DynamicPalette::Custom(
                    path.file_stem()
                        .expect("missing file stem")
                        .display()
                        .to_string(),
                ));
            }
        }
        palettes.sort_by_cached_key(|a| a.to_string());

        Ok(palettes)
    }

    pub fn save(&self, palette: &[[u8; 3]]) -> std::io::Result<()> {
        if let Self::Custom(name) = self {
            let path = lutgen_dir().join(name);
            std::fs::write(
                path,
                palette
                    .iter()
                    .map(|c| Color(*c).to_string())
                    .collect::<Vec<_>>()
                    .join("\n"),
            )?;
        }
        Ok(())
    }

    pub fn get(&self) -> Cow<'static, [[u8; 3]]> {
        match self {
            DynamicPalette::Builtin(palette) => palette.get().into(),
            DynamicPalette::Custom(name) => {
                // read palette from disk
                let path = lutgen_dir().join(name);
                let mut contents = String::new();
                File::open(path)
                    .unwrap()
                    .read_to_string(&mut contents)
                    .unwrap();

                // split by whitespace and parse colors
                contents
                    .split_whitespace()
                    .map(Color::from_str)
                    .map(|v| v.map(|v| v.0))
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap()
                    .into()
            },
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            DynamicPalette::Builtin(palette) => palette.into(),
            DynamicPalette::Custom(name) => name,
        }
    }
}

impl<'de> Deserialize<'de> for DynamicPalette {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if let Ok(p) = lutgen_palettes::Palette::from_str(&s) {
            Ok(DynamicPalette::Builtin(p))
        } else {
            Ok(DynamicPalette::Custom(s))
        }
    }
}

impl Serialize for DynamicPalette {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            DynamicPalette::Builtin(palette) => serializer.serialize_str(palette.into()),
            DynamicPalette::Custom(name) => serializer.serialize_str(name),
        }
    }
}
