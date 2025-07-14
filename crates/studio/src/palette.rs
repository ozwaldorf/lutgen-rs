use std::fmt::Display;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq)]
pub enum DynamicPalette {
    Builtin(lutgen_palettes::Palette),
    Custom,
}

impl Display for DynamicPalette {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl DynamicPalette {
    pub fn get(&self) -> &[[u8; 3]] {
        match self {
            DynamicPalette::Builtin(palette) => palette.get(),
            DynamicPalette::Custom => &[],
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            DynamicPalette::Builtin(palette) => palette.into(),
            DynamicPalette::Custom => "custom",
        }
    }
}

impl<'de> Deserialize<'de> for DynamicPalette {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if &s == "custom" {
            Ok(DynamicPalette::Custom)
        } else {
            lutgen_palettes::Palette::from_str(&s)
                .map(DynamicPalette::Builtin)
                .map_err(|e| serde::de::Error::custom(e.to_string()))
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
            DynamicPalette::Custom => serializer.serialize_str("custom"),
        }
    }
}
