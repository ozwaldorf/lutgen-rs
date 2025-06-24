use std::collections::BTreeMap;
use std::error::Error;
use std::fs::{read_to_string, write};
use std::path::Path;

use serde::Serialize;
use tinytemplate::TinyTemplate;

const TEMPLATE: &str = r#"
use strum::\{Display, EnumString, VariantArray};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Display, EnumString, VariantArray)]
#[strum(serialize_all = "kebab-case")]
pub enum Palette \{
  {{ for item in palettes }}
    { item.name },
  {{ endfor }}
}

impl Palette \{
    /// Collection of all palette variants
    pub const VARIANTS: &'static [Palette] = <Palette as VariantArray>::VARIANTS;

    /// Get the colors for the palette
    pub fn get(&self) -> &'static [[u8; 3]] \{
        match self \{
          {{ for item in palettes }}
            Palette::{ item.name } => &[
              {{ for color in item.palette }}
                [ { color.r }u8, { color.g }, { color.b } ],
              {{ endfor }}
            ],
          {{ endfor }}
        }
    }
}

impl std::hash::Hash for Palette \{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) \{
        self.to_string().hash(state);
        self.get().hash(state);
    }
}"#;

#[derive(Serialize)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}
#[derive(Serialize)]
struct Palette {
    name: String,
    palette: Vec<Color>,
}
#[derive(Serialize)]
struct Context {
    palettes: Vec<Palette>,
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=palettes.toml");
    println!("cargo:rerun-if-changed=src/lib.template");

    let out_dir = std::env::var("OUT_DIR")?;
    let mut tt = TinyTemplate::new();
    tt.add_template("lib.rs", TEMPLATE)?;

    let ctx = Context {
        palettes: toml::from_str::<BTreeMap<String, Vec<String>>>(&read_to_string(
            "palettes.toml",
        )?)?
        .into_iter()
        .map(|(name, palette)| Palette {
            name: pascal_case(name),
            palette: palette.into_iter().map(parse_color).collect(),
        })
        .collect(),
    };

    let rust_code = tt.render("lib.rs", &ctx)?;
    write(Path::new(&out_dir).join("lib.rs"), rust_code)?;

    Ok(())
}

pub fn pascal_case(s: String) -> String {
    let sections: Vec<_> = s.split('_').collect();
    let mut buf = String::new();
    if s.starts_with('_') {
        buf.push('_');
    }
    for str in sections {
        let mut chars = str.chars();
        if let Some(f) = chars.next() {
            buf.push_str(&(f.to_uppercase().to_string() + &chars.as_str().to_lowercase()))
        }
    }
    buf
}

fn parse_color(s: String) -> Color {
    let hex_string = match s.strip_prefix('#') {
        Some(s) => s,
        None => panic!("expected hex string starting with `#`"),
    };
    if hex_string.len() != 6 {
        panic!("expected a 6 digit hex string");
    }
    let channel_bytes = match u32::from_str_radix(hex_string, 16) {
        Ok(n) => n,
        Err(_) => panic!("expected a valid hex string"),
    };

    Color {
        r: ((channel_bytes >> 16) & 0xFF) as u8,
        g: ((channel_bytes >> 8) & 0xFF) as u8,
        b: (channel_bytes & 0xFF) as u8,
    }
}
