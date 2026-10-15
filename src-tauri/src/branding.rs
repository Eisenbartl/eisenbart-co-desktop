//! Offline firm presentation; captured separately from financial evidence.
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Data {
    pub name: String,
    pub address: String,
    pub email: String,
    pub phone: String,
    pub website: String,
    pub disclosure: String,
    pub logo_png: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Snapshot {
    pub organization_id: String,
    pub revision: i64,
    pub updated_at: String,
    pub data: Data,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub organization_id: String,
    pub revision: i64,
    pub updated_at: Option<String>,
    pub data: Data,
    pub history: Vec<Version>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub revision: i64,
    pub name: String,
    pub updated_at: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Input {
    pub expected_revision: i64,
    pub data: Data,
}
pub fn defaults(name: &str) -> Data {
    Data {
        name: name.into(),
        address: String::new(),
        email: String::new(),
        phone: String::new(),
        website: String::new(),
        disclosure: String::new(),
        logo_png: None,
    }
}
fn field(value: &str, max: usize, multi: bool) -> Result<String, String> {
    let value = value.trim().replace("\r\n", "\n");
    if value.chars().count() > max
        || value
            .chars()
            .any(|c| c.is_control() && !(multi && c == '\n'))
    {
        return Err(format!(
            "Firm details must use plain text within the displayed {max}-character limit."
        ));
    }
    Ok(value)
}
pub fn canonical(mut d: Data) -> Result<Data, String> {
    d.name = field(&d.name, 120, false)?;
    if d.name.is_empty() {
        return Err("Enter a firm display name.".into());
    }
    d.address = field(&d.address, 500, true)?;
    d.email = field(&d.email, 254, false)?;
    d.phone = field(&d.phone, 80, false)?;
    d.website = field(&d.website, 254, false)?;
    d.disclosure = field(&d.disclosure, 2000, true)?;
    if !d.email.is_empty() && (!d.email.contains('@') || d.email.chars().any(char::is_whitespace)) {
        return Err(
            "Enter an email address containing @ without spaces, or leave it blank.".into(),
        );
    }
    if !d.website.is_empty()
        && (!(d.website.starts_with("https://") || d.website.starts_with("http://"))
            || d.website.chars().any(char::is_whitespace))
    {
        return Err(
            "Enter a website starting with https:// or http:// without spaces, or leave it blank."
                .into(),
        );
    }
    if let Some(logo) = &d.logo_png {
        if logo.len() > 90000 {
            return Err("The saved logo exceeds the size limit.".into());
        }
        let raw = STANDARD
            .decode(logo)
            .map_err(|_| "The logo must be a valid PNG.".to_owned())?;
        let normalized = normalize_logo(&raw)?;
        if STANDARD.encode(normalized) != *logo {
            return Err(
                "The logo is not in the normalized local format. Inspect the PNG again.".into(),
            );
        }
    }
    crate::assessment_pdf::validate_branding_text(&d)?;
    Ok(d)
}
pub fn validate(s: &Snapshot) -> Result<(), String> {
    if uuid::Uuid::parse_str(&s.organization_id).is_err()
        || !(1..=100).contains(&s.revision)
        || chrono::DateTime::parse_from_rfc3339(&s.updated_at).is_err()
        || canonical(s.data.clone())? != s.data
    {
        return Err("Saved firm branding is invalid.".into());
    }
    Ok(())
}
pub struct Pixels {
    pub width: u32,
    pub height: u32,
    pub rgb: Vec<u8>,
}
pub fn pixels(raw: &[u8]) -> Result<Pixels, String> {
    let bad = || {
        "Use a static PNG logo up to 256 × 256 pixels and 64 KiB. Resize larger logos before selecting them.".to_owned()
    };
    if raw.len() > 65536 {
        return Err(bad());
    }
    let mut decoder =
        png::Decoder::new_with_limits(Cursor::new(raw), png::Limits { bytes: 1024 * 1024 });
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|_| bad())?;
    let info = reader.info();
    if info.width == 0
        || info.height == 0
        || info.width > 256
        || info.height > 256
        || info.animation_control.is_some()
    {
        return Err(bad());
    }
    let mut out = vec![0; reader.output_buffer_size().ok_or_else(bad)?];
    let frame = reader.next_frame(&mut out).map_err(|_| bad())?;
    reader.finish().map_err(|_| bad())?;
    let mut rgb = Vec::with_capacity(frame.width as usize * frame.height as usize * 3);
    let channels = match frame.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        _ => return Err(bad()),
    };
    for p in out[..frame.buffer_size()].chunks_exact(channels) {
        let (v, alpha) = match channels {
            4 => ([p[0], p[1], p[2]], p[3]),
            3 => ([p[0], p[1], p[2]], 255),
            2 => ([p[0]; 3], p[1]),
            _ => ([p[0]; 3], 255),
        };
        for c in v {
            rgb.push(
                ((u32::from(c) * u32::from(alpha) + 255 * (255 - u32::from(alpha)) + 127) / 255)
                    as u8,
            );
        }
    }
    Ok(Pixels {
        width: frame.width,
        height: frame.height,
        rgb,
    })
}
pub fn normalize_logo(raw: &[u8]) -> Result<Vec<u8>, String> {
    let p = pixels(raw)?;
    let mut result = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut result, p.width, p.height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .map_err(|_| "Unable to normalize logo.")?
            .write_image_data(&p.rgb)
            .map_err(|_| "Unable to normalize logo.")?;
    }
    if result.len() > 65536 {
        return Err("The normalized logo exceeds 64 KiB. Use a simpler logo.".into());
    }
    Ok(result)
}
pub fn inspect_logo(raw: &[u8]) -> Result<String, String> {
    Ok(STANDARD.encode(normalize_logo(raw)?))
}
pub fn logo_pixels(encoded: &str) -> Result<Pixels, String> {
    pixels(
        &STANDARD
            .decode(encoded)
            .map_err(|_| "Invalid saved logo.")?,
    )
}
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub fn logo() -> Vec<u8> {
        let mut raw = Vec::new();
        {
            let mut e = png::Encoder::new(&mut raw, 2, 1);
            e.set_color(png::ColorType::Rgba);
            e.set_depth(png::BitDepth::Eight);
            e.add_text_chunk("Private source metadata".into(), "Do not retain".into())
                .unwrap();
            e.write_header()
                .unwrap()
                .write_image_data(&[255, 0, 0, 255, 0, 0, 255, 0])
                .unwrap();
        }
        raw
    }
    #[test]
    fn png_normalization_bounds_alpha_metadata_and_text_validation() {
        let encoded = inspect_logo(&logo()).unwrap();
        let p = logo_pixels(&encoded).unwrap();
        assert_eq!((p.width, p.height), (2, 1));
        assert_eq!(p.rgb, vec![255, 0, 0, 255, 255, 255]);
        let bytes = STANDARD.decode(&encoded).unwrap();
        assert!(!bytes.windows(4).any(|v| v == b"tEXt"));
        assert_eq!(inspect_logo(&bytes).unwrap(), encoded);
        assert!(inspect_logo(b"<svg onload='bad'/>").is_err());
        assert!(inspect_logo(&vec![0; 65537]).is_err());
        assert!(inspect_logo(&logo()[..20]).is_err());
        let mut big = Vec::new();
        {
            let mut e = png::Encoder::new(&mut big, 257, 1);
            e.set_color(png::ColorType::Rgb);
            e.set_depth(png::BitDepth::Eight);
            e.write_header()
                .unwrap()
                .write_image_data(&vec![0; 257 * 3])
                .unwrap();
        }
        assert!(inspect_logo(&big).is_err());
        let mut d = defaults(" Fictional practice ");
        d.logo_png = Some(encoded);
        d.address = " Fictional street\r\nFictional town ".into();
        let d = canonical(d).unwrap();
        assert_eq!(d.name, "Fictional practice");
        assert_eq!(d.address, "Fictional street\nFictional town");
        let mut bad = d.clone();
        bad.email = "bad email".into();
        assert!(canonical(bad).is_err());
        let mut bad = d.clone();
        bad.website = "javascript:bad".into();
        assert!(canonical(bad).is_err());
        let mut bad = d.clone();
        bad.name = "\u{202e}hidden".into();
        assert!(canonical(bad).is_err());
        let mut bad = d;
        bad.disclosure = "x".repeat(2001);
        assert!(canonical(bad).is_err());
    }
}
