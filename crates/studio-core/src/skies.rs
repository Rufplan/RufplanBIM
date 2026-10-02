//! The Sky Library (ADR-101): photographed skies for renderings, as D5's and Enscape's sky
//! presets are. Each is a Poly Haven "pure sky" HDRI (CC0): sky only, the sun unclipped, so
//! it lights the scene (a dome light) as well as filling the background. They're downloaded
//! on first use and kept in this computer's cache, like the library's textures.

use serde::Serialize;
use ts_rs::TS;

/// A sky in the library.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct SkyPreset {
    /// Poly Haven's asset id.
    pub id: String,
    pub name: String,
    /// Clear, Partly Cloudy, Overcast, Sunrise & Sunset or Night.
    pub mood: String,
    /// A word on the light, for the picker.
    pub note: String,
}

/// (Poly Haven id, name, mood, note).
const SKIES: &[(&str, &str, &str, &str)] = &[
    // Clear
    (
        "kloofendal_43d_clear_puresky",
        "Clear Midday",
        "Clear",
        "high sun, deep blue, a few wisps",
    ),
    (
        "qwantani_noon_puresky",
        "Clear Noon",
        "Clear",
        "overhead sun, crisp shadows",
    ),
    (
        "qwantani_afternoon_puresky",
        "Clear Afternoon",
        "Clear",
        "warm sun, pale horizon",
    ),
    (
        "qwantani_mid_morning_puresky",
        "Clear Mid-Morning",
        "Clear",
        "bright, cool blue",
    ),
    (
        "syferfontein_18d_clear_puresky",
        "Clear, Low Sun",
        "Clear",
        "long shadows, golden light",
    ),
    (
        "autumn_field_puresky",
        "Autumn Clear",
        "Clear",
        "soft blue, light haze",
    ),
    (
        "pizzo_pernice_puresky",
        "Alpine Clear",
        "Clear",
        "intense blue, sharp sun",
    ),
    (
        "drakensberg_solitary_mountain_puresky",
        "Highland Clear",
        "Clear",
        "clean afternoon sky",
    ),
    // Partly cloudy
    (
        "kloofendal_48d_partly_cloudy_puresky",
        "Partly Cloudy",
        "Partly Cloudy",
        "the classic architectural sky",
    ),
    (
        "kloofendal_38d_partly_cloudy_puresky",
        "Partly Cloudy, Afternoon",
        "Partly Cloudy",
        "fair-weather cumulus",
    ),
    (
        "sunflowers_puresky",
        "Summer Cumulus",
        "Partly Cloudy",
        "bright sun, puffy clouds",
    ),
    (
        "kloppenheim_05_puresky",
        "Scattered Clouds",
        "Partly Cloudy",
        "midday, high contrast",
    ),
    (
        "kloppenheim_03_puresky",
        "Broken Clouds",
        "Partly Cloudy",
        "softer sun through clouds",
    ),
    (
        "aristea_wreck_puresky",
        "Coastal Clouds",
        "Partly Cloudy",
        "bright and airy",
    ),
    (
        "farm_field_puresky",
        "Soft Cumulus",
        "Partly Cloudy",
        "gentle, low contrast",
    ),
    (
        "rustig_koppie_puresky",
        "Morning Clouds",
        "Partly Cloudy",
        "fresh morning light",
    ),
    (
        "kloofendal_28d_misty_puresky",
        "Misty Clouds",
        "Partly Cloudy",
        "hazy, diffuse sun",
    ),
    // Overcast
    (
        "kloofendal_overcast_puresky",
        "Overcast",
        "Overcast",
        "even, shadowless light",
    ),
    (
        "overcast_soil_puresky",
        "Bright Overcast",
        "Overcast",
        "soft daylight",
    ),
    (
        "mud_road_puresky",
        "Grey Overcast",
        "Overcast",
        "flat and moody",
    ),
    (
        "kloofendal_misty_morning_puresky",
        "Misty Morning",
        "Overcast",
        "fog-soft light",
    ),
    // Sunrise and sunset
    (
        "kloppenheim_06_puresky",
        "Golden Hour",
        "Sunrise & Sunset",
        "warm, low sun, soft clouds",
    ),
    (
        "belfast_sunset_puresky",
        "Sunset Clouds",
        "Sunrise & Sunset",
        "pink and amber",
    ),
    (
        "citrus_orchard_road_puresky",
        "Late Afternoon",
        "Sunrise & Sunset",
        "honey light",
    ),
    (
        "industrial_sunset_puresky",
        "Sunset",
        "Sunrise & Sunset",
        "orange sky",
    ),
    (
        "evening_road_01_puresky",
        "Evening",
        "Sunrise & Sunset",
        "the sun just setting",
    ),
    (
        "qwantani_dusk_2_puresky",
        "Dusk",
        "Sunrise & Sunset",
        "blue hour glow",
    ),
    (
        "table_mountain_2_puresky",
        "Dramatic Sunset",
        "Sunrise & Sunset",
        "high contrast clouds",
    ),
    (
        "qwantani_sunrise_puresky",
        "Sunrise",
        "Sunrise & Sunset",
        "clear dawn",
    ),
    // Night
    (
        "kloppenheim_02_puresky",
        "Clear Night",
        "Night",
        "stars, for lit interiors",
    ),
    (
        "qwantani_night_puresky",
        "Starry Night",
        "Night",
        "deep blue night",
    ),
];

/// The library's skies, by mood.
pub fn catalog() -> Vec<SkyPreset> {
    SKIES
        .iter()
        .map(|(id, name, mood, note)| SkyPreset {
            id: (*id).into(),
            name: (*name).into(),
            mood: (*mood).into(),
            note: (*note).into(),
        })
        .collect()
}

/// Whether `id` is a library sky (only these can be downloaded).
pub fn is_sky(id: &str) -> bool {
    SKIES.iter().any(|s| s.0 == id)
}

/// Where each of a sky's files comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkyFile {
    /// The 2K HDR the scene is lit by.
    Light,
    /// Poly Haven's full-size tonemapped JPEG, for the background.
    Photo,
    /// A small preview for the picker.
    Thumb,
}

/// The URL of one of a library sky's files.
pub fn url(id: &str, file: SkyFile) -> Option<String> {
    is_sky(id).then(|| match file {
        SkyFile::Light => {
            format!("https://dl.polyhaven.org/file/ph-assets/HDRIs/hdr/2k/{id}_2k.hdr")
        }
        SkyFile::Photo => {
            format!("https://dl.polyhaven.org/file/ph-assets/HDRIs/extra/Tonemapped%20JPG/{id}.jpg")
        }
        SkyFile::Thumb => {
            format!("https://cdn.polyhaven.com/asset_img/thumbs/{id}.png?width=320&height=160")
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_library_has_skies_for_every_mood_and_only_those_download() {
        let c = catalog();
        assert!(c.len() >= 30);
        for mood in [
            "Clear",
            "Partly Cloudy",
            "Overcast",
            "Sunrise & Sunset",
            "Night",
        ] {
            assert!(c.iter().filter(|s| s.mood == mood).count() >= 2, "{mood}");
        }
        let mut ids: Vec<&str> = c.iter().map(|s| s.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), c.len(), "no sky twice");
        assert!(ids.iter().all(|i| i.ends_with("_puresky")));
        assert_eq!(
            url("kloofendal_48d_partly_cloudy_puresky", SkyFile::Light).as_deref(),
            Some("https://dl.polyhaven.org/file/ph-assets/HDRIs/hdr/2k/kloofendal_48d_partly_cloudy_puresky_2k.hdr")
        );
        assert_eq!(url("../../etc/passwd", SkyFile::Photo), None);
    }
}
