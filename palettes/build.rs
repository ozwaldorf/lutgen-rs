use std::{
    collections::HashMap,
    error::Error,
    fs::{read_to_string, write},
    path::Path,
};

use serde_json::{from_reader, json, to_value};
use tera::{try_get_value, Context, Tera};

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=palettes.json");
    println!("cargo:rerun-if-changed=src/palettes.tera");

    let out_dir = std::env::var("OUT_DIR")?;
    let mut tera = Tera::default();
    tera.register_filter("hex_to_rgb", hex_to_rgb_filter);
    tera.register_filter("pascal_case", pascal_case);

    let mut data: serde_json::Value = from_reader(read_to_string("palettes.json")?.as_bytes())?;

    // for each distro
    for palette in data.as_array_mut().unwrap() {
        let palette = palette.as_object_mut().unwrap();
        let name = palette.get("name").unwrap().as_str().unwrap();

        // add rust type name
        let reg = regex::Regex::new(r"[_\-./!@)(+]").unwrap();
        let type_name = reg.replace_all(name, "").replace(' ', "_");
        palette.insert("type_name".to_string(), json!(type_name));
    }

    let rust_code = tera.render_str(
        &read_to_string("src/palettes.tera")?,
        &Context::from_value(json!({ "palettes": data }))?,
    )?;

    write(Path::new(&out_dir).join("palettes.rs"), rust_code)?;

    Ok(())
}

pub fn pascal_case(
    value: &tera::Value,
    _: &HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    let s = try_get_value!("pascal_case", "value", String, value);
    let sections: Vec<_> = s.split('_').collect();
    let mut buf = String::new();
    for str in sections {
        let mut chars = str.chars();
        if let Some(f) = chars.next() {
            buf.push_str(&(f.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()))
        }
    }
    Ok(to_value(&buf).unwrap())
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
    let r = (channel_bytes >> 16) & 0xFF;
    let g = (channel_bytes >> 8) & 0xFF;
    let b = channel_bytes & 0xFF;

    Ok(json!({
        "r": r,
        "g": g,
        "b": b,
    }))
}
