use anyhow::{bail, Context, Result};
use iced::Color;
use serde::{de, Deserialize, Deserializer};
use tracing::instrument;

#[derive(Debug, Clone, Copy)]
pub struct WColor(pub Color);

#[instrument]
fn hex_to_rgb(hex_color: &str) -> Result<WColor> {
    let raw_hex = String::from(hex_color);
    let mut hex = raw_hex.as_str();
    if hex_color.starts_with('#') {
        hex = raw_hex.strip_prefix('#').expect("nope");
    }
    if hex.len() < 6 || hex.len() > 8 {
        bail!("Failed to parse color value {}", hex_color);
    }
    let r = u8::from_str_radix(&hex[..2], 16)
        .with_context(|| format!("Failed to parse color value {}", hex_color))?;
    let g = u8::from_str_radix(&hex[2..4], 16)
        .with_context(|| format!("Failed to parse color value {}", hex_color))?;
    let b = u8::from_str_radix(&hex[4..6], 16)
        .with_context(|| format!("Failed to parse color value {}", hex_color))?;
    let mut a = None;
    if hex.len() == 8 {
        a = Some(u32::from_str_radix(&hex[6..8], 16).unwrap_or(1) as f32 / 255.0);
    }
    if let Some(a) = a {
        Ok(WColor(Color::from_rgba8(r, g, b, a)))
    } else {
        Ok(WColor(Color::from_rgb8(r, g, b)))
    }
}

impl TryFrom<&str> for WColor {
    type Error = anyhow::Error;

    fn try_from(hex: &str) -> Result<Self, Self::Error> {
        hex_to_rgb(hex)
    }
}

impl From<[u8; 3]> for WColor {
    fn from(rgb: [u8; 3]) -> Self {
        WColor(Color::from_rgb8(rgb[0], rgb[1], rgb[2]))
    }
}

impl<'de> Deserialize<'de> for WColor {
    fn deserialize<D>(deserializer: D) -> Result<WColor, D::Error>
    where
        D: Deserializer<'de>,
    {
        let hex_color = String::deserialize(deserializer)?;
        WColor::try_from(hex_color.as_str()).map_err(de::Error::custom)
    }
}

impl From<WColor> for Color {
    fn from(color: WColor) -> Self {
        color.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_parse_wrong_values() {
        let result = hex_to_rgb("");
        assert!(result.is_err());
        let result = hex_to_rgb("fffff");
        assert!(result.is_err());
        let result = hex_to_rgb("fffffffff");
        assert!(result.is_err());
    }

    /*     #[test]
    fn test_color_parse_good_values() {
        let white = RgbaColor::new(255, 255, 255, None);
        let black = RgbaColor::new(0, 0, 0, None);
        let dark_grey = RgbaColor::new(33, 33, 33, None);
        let result = hex_to_rgb("ffffff").unwrap();
        assert_eq!(result, white);
        let result = hex_to_rgb("#ffffff").unwrap();
        assert_eq!(result, white);
        let result = hex_to_rgb("#000000").unwrap();
        assert_eq!(result, black);
        let result = hex_to_rgb("#212121").unwrap();
        assert_eq!(result, dark_grey);
    }

    #[test]
    fn test_color_parse_good_values_with_alpha() {
        let white = RgbaColor::new(255, 255, 255, Some(0.0));
        let result = hex_to_rgb("#ffffff00").unwrap();
        assert_eq!(result, white);
    } */
}
