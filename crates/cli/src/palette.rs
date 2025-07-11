use std::collections::BTreeSet;
use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;

use bpaf::{long, positional, Doc, Parser};
use lutgen_palettes::Palette;

use crate::color::Color;

#[derive(Clone, Debug, Hash)]
pub enum DynamicPalette {
    Builtin(Palette),
    Custom(String, Vec<[u8; 3]>),
}
impl Display for DynamicPalette {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DynamicPalette::Builtin(palette) => std::fmt::Display::fmt(palette, f),
            DynamicPalette::Custom(name, _) => f.write_str(name),
        }
    }
}
impl FromStr for DynamicPalette {
    type Err = String;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        if let Some((name, path)) = Self::get_custom_palettes()
            .iter()
            .find(|(name, _)| input == name.to_lowercase())
        {
            let contents = std::fs::read_to_string(path)
                .map_err(|e| format!("failed to read custom palette file: {e}"))?;

            let mut palette = Vec::new();
            for color in contents.split_whitespace() {
                palette.push(Color::from_str(color)?.0)
            }

            Ok(DynamicPalette::Custom(name.clone(), palette))
        } else if let Ok(palette) = Palette::from_str(input) {
            Ok(DynamicPalette::Builtin(palette))
        } else {
            let mut doc: Doc = "unknown palette\n  \n  ".into();

            if let Some((_, name)) = Self::suggestions(input).pop_first() {
                doc.text("Did you mean ");
                doc.emphasis(&name);
                doc.text("\n  \n  ");
            }

            doc.emphasis("Hint: ");
            doc.text("view all palettes with ");
            doc.literal("`lutgen palette names`");
            Err(doc.monochrome(true))
        }
    }
}
impl DynamicPalette {
    const HELP: &'static str = "\
Builtin or custom palette to use.

Custom palettes can be added to `$LUTGEN_DIR` or `<CONFIG DIR>/lutgen`.
   - Linux: `/home/alice/.config/lutgen`
   - macOS: `/Users/Alice/Library/Application Support/lutgen`
   - Windows: `C:\\Users\\Alice\\AppData\\Roaming\\lutgen`

Names are case-insensitive and parsed from the file stem, minus any file extensions.
For example, `~/.config/lutgen/My-palette.txt` would be avalable to use as `my-palette`.";

    /// Argument parser and completion for palettes
    pub fn flag_parser() -> impl Parser<Self> {
        long("palette")
            .short('p')
            .argument::<String>("PALETTE")
            .help(Self::HELP)
            .complete(|v| {
                DynamicPalette::suggestions(v)
                    .into_iter()
                    .map(|(_, name)| (name, None))
                    .collect()
            })
            .parse(|s| DynamicPalette::from_str(&s))
    }

    /// Positional parser and completion for palettes
    pub fn arg_parser() -> impl Parser<Vec<Self>> {
        positional::<String>("PALETTE")
            .help(Self::HELP)
            .complete(|v| {
                DynamicPalette::suggestions(v)
                    .into_iter()
                    .map(|(_, name)| (name, None))
                    .collect()
            })
            .parse(|s| DynamicPalette::from_str(&s))
            .some("missing `all`, `names`, or at least one palette to preview")
    }

    pub fn get(&self) -> &[[u8; 3]] {
        match self {
            DynamicPalette::Builtin(p) => p.get(),
            DynamicPalette::Custom(_, p) => p.as_ref(),
        }
    }

    /// Compute a set of palette suggestions based on some input. Best matches are first in the set.
    pub fn suggestions(input: &str) -> BTreeSet<(u16, String)> {
        let input = input.to_lowercase();
        Palette::VARIANTS
            .iter()
            .map(ToString::to_string)
            .chain(Self::get_custom_palettes().iter().map(|v| v.0.clone()))
            .filter_map(|variant| {
                let score = strsim::jaro_winkler(&input, &variant);
                (score > 0.7).then_some((((1. - score) * 10000.) as u16, variant))
            })
            .collect::<BTreeSet<_>>()
    }

    /// Parse files in the palette directory and return all items and locations
    pub fn get_custom_palettes() -> Vec<(String, PathBuf)> {
        let path = std::env::var("LUTGEN_DIR")
            .map(Into::into)
            .unwrap_or(dirs::config_dir().unwrap().join("lutgen"));

        if path.is_dir() {
            std::fs::read_dir(path)
                .expect("failed to read lutgen dir")
                .map(|v| {
                    let path = v.expect("failed to get file info").path();
                    (
                        path.file_stem()
                            .expect("missing file stem")
                            .to_string_lossy()
                            .to_lowercase(),
                        path,
                    )
                })
                .collect()
        } else {
            vec![]
        }
    }
}
