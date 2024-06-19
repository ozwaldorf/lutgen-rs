use std::{
    collections::HashMap,
    error::Error,
    fs::{read_to_string, write},
    path::Path,
};

use tera::{try_get_value, Context, Map, Tera};

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=palettes.toml");
    println!("cargo:rerun-if-changed=src/lib.tera");

    let out_dir = std::env::var("OUT_DIR")?;
    let mut tera = Tera::default();
    tera.register_filter("hex_to_rgb", hex_to_rgb_filter);
    tera.register_filter("pascal_case", pascal_case);

    let mut context = Context::new();
    let data: tera::Value = toml::from_str(&read_to_string("palettes.toml")?)?;
    context.insert("palettes", &data);

    let rust_code = tera.render_str(&read_to_string("src/lib.tera")?, &context)?;
    write(Path::new(&out_dir).join("lib.rs"), rust_code)?;

    Ok(())
}

pub fn pascal_case(
    value: &tera::Value,
    _: &HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    let s = try_get_value!("pascal_case", "value", String, value);

    // cleanup special chars
    let reg = regex::Regex::new(r"[\-./!@)(+]").unwrap();
    let s = reg.replace_all(&s, "").replace(' ', "_");

    let sections: Vec<_> = s.split('_').collect();
    let mut buf = String::new();
    for str in sections {
        let mut chars = str.chars();
        if let Some(f) = chars.next() {
            buf.push_str(&(f.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()))
        }
    }
    Ok(buf.into())
}

fn hex_to_rgb_filter(
    value: &tera::Value,
    _args: &HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    let hex_string = try_get_value!("hex_to_rgb", "value", String, value);
    let hex_string = match hex_string.strip_prefix('#') {
        Some(s) => s,
        None => return Err(tera::Error::msg("expected hex string starting with `#`")),
    };
    if hex_string.len() != 6 {
        return Err(tera::Error::msg("expected a 6 digit hex string"));
    }
    let channel_bytes = match u32::from_str_radix(hex_string, 16) {
        Ok(n) => n,
        Err(_) => return Err(tera::Error::msg("expected a valid hex string")),
    };

    let mut map = Map::new();
    map.insert("r".to_string(), ((channel_bytes >> 16) & 0xFF).into());
    map.insert("g".to_string(), ((channel_bytes >> 8) & 0xFF).into());
    map.insert("b".to_string(), (channel_bytes & 0xFF).into());

    Ok(map.into())
}
