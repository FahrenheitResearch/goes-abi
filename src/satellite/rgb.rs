use crate::png::Color;
use serde::{Deserialize, Serialize};
use std::error::Error;
#[cfg(test)]
use std::io;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoesAbiRgbCompositeStyle {
    GeoColor,
    FireTemperature,
    AirMass,
    Dust,
    Sandwich,
    DayCloudPhase,
    DayNightCloudMicroCombo,
    NaturalColor,
}

impl GoesAbiRgbCompositeStyle {
    pub fn product_slug(self) -> &'static str {
        match self {
            Self::GeoColor => "goes_geocolor",
            Self::FireTemperature => "goes_fire_temperature_rgb",
            Self::AirMass => "goes_airmass_rgb",
            Self::Dust => "goes_dust_rgb",
            Self::Sandwich => "goes_sandwich_rgb",
            Self::DayCloudPhase => "goes_day_cloud_phase_rgb",
            Self::DayNightCloudMicroCombo => "goes_day_night_cloud_micro_combo_rgb",
            Self::NaturalColor => "goes_natural_color_rgb",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::GeoColor | Self::NaturalColor => "GeoColor",
            Self::FireTemperature => "Fire Temperature",
            Self::AirMass => "AirMass RGB",
            Self::Dust => "Dust RGB",
            Self::Sandwich => "Sandwich RGB",
            Self::DayCloudPhase => "GOES Day Cloud Phase RGB",
            Self::DayNightCloudMicroCombo => "Day Night Cloud Micro Combo RGB",
        }
    }

    pub fn base_channel(self) -> u8 {
        match self {
            Self::GeoColor | Self::NaturalColor => 2,
            Self::FireTemperature => 7,
            Self::AirMass => 8,
            Self::Dust | Self::Sandwich | Self::DayCloudPhase | Self::DayNightCloudMicroCombo => 13,
        }
    }

    pub fn required_channels(self) -> &'static [u8] {
        match self {
            Self::GeoColor | Self::NaturalColor => &[1, 2, 3],
            Self::FireTemperature => &[5, 6, 7],
            Self::AirMass => &[8, 10, 12, 13],
            Self::Dust => &[11, 13, 14, 15],
            Self::Sandwich => &[3, 13],
            Self::DayCloudPhase => &[2, 5, 13],
            Self::DayNightCloudMicroCombo => &[2, 5, 7, 13, 15],
        }
    }
}

pub fn compose_goes_abi_rgb_pixel<F>(
    style: GoesAbiRgbCompositeStyle,
    mut band_value: F,
) -> Result<Color, Box<dyn Error>>
where
    F: FnMut(u8) -> Result<f32, Box<dyn Error>>,
{
    Ok(match style {
        GoesAbiRgbCompositeStyle::GeoColor | GoesAbiRgbCompositeStyle::NaturalColor => {
            let c01 = reflectance_pct(band_value(1)?);
            let c02 = reflectance_pct(band_value(2)?);
            let c03 = reflectance_pct(band_value(3)?);
            let r = visible_component(c02);
            let g = visible_component(0.45 * c02 + 0.10 * c03 + 0.45 * c01);
            let b = visible_component(c01);
            color_or_transparent([r, g, b])
        }
        GoesAbiRgbCompositeStyle::FireTemperature => {
            let r = component(k_to_c(band_value(7)?), 0.0, 60.0, 0.4);
            let g = component(reflectance_pct(band_value(6)?), 0.0, 100.0, 1.0);
            let b = component(reflectance_pct(band_value(5)?), 0.0, 75.0, 1.0);
            color_or_transparent([r, g, b])
        }
        GoesAbiRgbCompositeStyle::AirMass => {
            let c08 = band_value(8)?;
            let c10 = band_value(10)?;
            let c12 = band_value(12)?;
            let c13 = band_value(13)?;
            let r = component((c08 - c10) as f64, -26.2, 0.6, 1.0);
            let g = component((c12 - c13) as f64, -43.2, 6.7, 1.0);
            let b = component(k_to_c(c08), -29.25, -64.65, 1.0);
            color_or_transparent([r, g, b])
        }
        GoesAbiRgbCompositeStyle::Dust => {
            let c11 = band_value(11)?;
            let c13 = band_value(13)?;
            let c14 = band_value(14)?;
            let c15 = band_value(15)?;
            let r = component((c15 - c13) as f64, -6.7, 2.6, 1.0);
            let g = component((c14 - c11) as f64, -0.5, 20.0, 2.5);
            let b = component(k_to_c(c13), -11.95, 15.55, 1.0);
            color_or_transparent([r, g, b])
        }
        GoesAbiRgbCompositeStyle::Sandwich => {
            let visible = component(reflectance_pct(band_value(3)?), 0.0, 95.0, 1.0);
            let ir_cold = normalized(k_to_c(band_value(13)?), 30.0, -70.0, 1.0);
            sandwich_color(visible, ir_cold)
        }
        GoesAbiRgbCompositeStyle::DayCloudPhase => {
            let r = component(k_to_c(band_value(13)?), 7.5, -53.5, 1.0);
            let g = component(reflectance_pct(band_value(2)?), 0.0, 78.0, 1.0);
            let b = component(reflectance_pct(band_value(5)?), 1.0, 59.0, 1.0);
            color_or_transparent([r, g, b])
        }
        GoesAbiRgbCompositeStyle::DayNightCloudMicroCombo => {
            let day_green = reflectance_pct(band_value(2)?);
            let day_blue = reflectance_pct(band_value(5)?);
            let c07 = band_value(7)?;
            let c13 = band_value(13)?;
            let c15 = band_value(15)?;
            let daylight = normalized(day_green, 0.0, 18.0, 1.0).unwrap_or(0.0);
            let r = component(k_to_c(c13), 12.0, -60.0, 1.0);
            let g_day = normalized(day_green, 0.0, 80.0, 1.0).unwrap_or(0.0);
            let b_day = normalized(day_blue, 0.0, 65.0, 1.0).unwrap_or(0.0);
            let g_night = normalized((c15 - c13) as f64, -5.0, 12.0, 1.0).unwrap_or(0.0);
            let b_night = normalized(k_to_c(c07), 30.0, -45.0, 1.0).unwrap_or(0.0);
            let g = Some(((g_night * (1.0 - daylight) + g_day * daylight) * 255.0).round() as u8);
            let b = Some(((b_night * (1.0 - daylight) + b_day * daylight) * 255.0).round() as u8);
            color_or_transparent([r, g, b])
        }
    })
}

fn k_to_c(value: f32) -> f64 {
    value as f64 - 273.15
}

fn reflectance_pct(value: f32) -> f64 {
    value as f64 * 100.0
}

fn visible_component(value_pct: f64) -> Option<u8> {
    component(value_pct, 0.0, 100.0, 2.2)
}

fn component(value: f64, min: f64, max: f64, gamma: f64) -> Option<u8> {
    normalized(value, min, max, gamma).map(|value| (value * 255.0).round() as u8)
}

fn normalized(value: f64, min: f64, max: f64, gamma: f64) -> Option<f64> {
    if !value.is_finite() || !min.is_finite() || !max.is_finite() || (max - min).abs() <= 1.0e-12 {
        return None;
    }
    let raw = if max >= min {
        (value - min) / (max - min)
    } else {
        (min - value) / (min - max)
    };
    Some(raw.clamp(0.0, 1.0).powf(1.0 / gamma.max(1.0e-6)))
}

fn color_or_transparent(channels: [Option<u8>; 3]) -> Color {
    match channels {
        [Some(r), Some(g), Some(b)] => Color::rgba(r, g, b, 255),
        _ => Color::TRANSPARENT,
    }
}

fn sandwich_color(visible: Option<u8>, ir_cold: Option<f64>) -> Color {
    let Some(v) = visible else {
        return Color::TRANSPARENT;
    };
    let cold = ir_cold.unwrap_or(0.0);
    if cold < 0.18 {
        return Color::rgba(v, v, v, 255);
    }
    let tint = [
        (0.00, Color::rgba(v, v, v, 255)),
        (0.30, Color::rgba(255, 245, 160, 255)),
        (0.55, Color::rgba(255, 158, 76, 255)),
        (0.78, Color::rgba(226, 57, 55, 255)),
        (1.00, Color::rgba(172, 50, 186, 255)),
    ];
    color_at(cold, &tint)
}

fn color_at(value: f64, anchors: &[(f64, Color)]) -> Color {
    if anchors.is_empty() {
        return Color::TRANSPARENT;
    }
    if value <= anchors[0].0 {
        return anchors[0].1;
    }
    for window in anchors.windows(2) {
        let (lo_value, lo_color) = window[0];
        let (hi_value, hi_color) = window[1];
        if value <= hi_value {
            let t = ((value - lo_value) / (hi_value - lo_value)).clamp(0.0, 1.0);
            return Color::rgba(
                lerp_u8(lo_color.r, hi_color.r, t),
                lerp_u8(lo_color.g, hi_color.g, t),
                lerp_u8(lo_color.b, hi_color.b, t),
                lerp_u8(lo_color.a, hi_color.a, t),
            );
        }
    }
    anchors
        .last()
        .map(|(_, color)| *color)
        .unwrap_or(Color::TRANSPARENT)
}

fn lerp_u8(left: u8, right: u8, t: f64) -> u8 {
    (left as f64 + (right as f64 - left as f64) * t).round() as u8
}

#[cfg(test)]
fn boxed_error(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(io::Error::new(io::ErrorKind::InvalidData, message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_styles_declare_required_channels() {
        assert_eq!(
            GoesAbiRgbCompositeStyle::GeoColor.required_channels(),
            &[1, 2, 3]
        );
        assert_eq!(GoesAbiRgbCompositeStyle::AirMass.base_channel(), 8);
        assert_eq!(
            GoesAbiRgbCompositeStyle::Dust.required_channels(),
            &[11, 13, 14, 15]
        );
    }

    #[test]
    fn geocolor_composition_is_opaque_for_valid_reflectance() {
        let color = compose_goes_abi_rgb_pixel(GoesAbiRgbCompositeStyle::GeoColor, |channel| {
            Ok(match channel {
                1 => 0.35,
                2 => 0.42,
                3 => 0.28,
                _ => return Err(boxed_error("unexpected channel")),
            })
        })
        .unwrap();
        assert_eq!(color.a, 255);
        assert!(color.r > color.b);
    }
}
