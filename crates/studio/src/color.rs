use std::fmt::{Debug, Display};
use std::str::FromStr;

#[derive(Clone, Hash)]
pub struct Color(pub [u8; 3]);
impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let [r, g, b] = self.0;
        f.write_str(&format!("#{r:02x}{g:02x}{b:02x}"))
    }
}
impl Debug for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}
impl FromStr for Color {
    type Err = String;
    fn from_str(code: &str) -> Result<Self, Self::Err> {
        // parse hex string into rgb
        let mut hex = (*code).trim_start_matches('#').to_string();
        match hex.len() {
            3 => {
                // Extend 3 character hex colors
                hex = hex.chars().flat_map(|a| [a, a]).collect();
            },
            6 => {},
            l => return Err(format!("Invalid hex length for {code}: {l}")),
        }
        if let Ok(channel_bytes) = u32::from_str_radix(&hex, 16) {
            let r = ((channel_bytes >> 16) & 0xFF) as u8;
            let g = ((channel_bytes >> 8) & 0xFF) as u8;
            let b = (channel_bytes & 0xFF) as u8;
            Ok(Self([r, g, b]))
        } else {
            Err(format!("Invalid hex color: {code}"))
        }
    }
}
impl AsRef<[u8; 3]> for Color {
    fn as_ref(&self) -> &[u8; 3] {
        &self.0
    }
}
