//! Preset chimes and the rules for a sound the user adds.

pub const MAX_SOUND_BYTES: u64 = 2 * 1024 * 1024;
pub const MAX_SOUND_SECONDS: f64 = 15.0;

pub struct Preset {
    pub id: &'static str,
    pub file: &'static str,
}

/// Bundled with the app, not a macOS system sound.
pub const MUSTACHE_ID: &str = "mustache";
pub const ABUELA_ID: &str = "abuela";

pub const MUSTACHE_MP3: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../src/assets/mustache.mp3"));
pub const ABUELA_MP3: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../src/assets/abuela.mp3"));

pub fn bundled_bytes(id: &str) -> Option<&'static [u8]> {
    match id {
        MUSTACHE_ID => Some(MUSTACHE_MP3),
        ABUELA_ID => Some(ABUELA_MP3),
        _ => None,
    }
}

pub fn is_bundled_id(id: &str) -> bool {
    bundled_bytes(id).is_some()
}

/// Sonido embebido que va con la reacción del widget, si tiene uno propio.
pub fn reaction_sound_id(reaction: &str) -> Option<&'static str> {
    match reaction {
        MUSTACHE_ID => Some(MUSTACHE_ID),
        ABUELA_ID => Some(ABUELA_ID),
        _ => None,
    }
}

pub const PRESETS: &[Preset] = &[
    Preset { id: "glass", file: "Glass" },
    Preset { id: "ping", file: "Ping" },
    Preset { id: "pop", file: "Pop" },
    Preset { id: "blow", file: "Blow" },
    Preset { id: "funk", file: "Funk" },
    Preset { id: "hero", file: "Hero" },
    Preset { id: "tink", file: "Tink" },
    Preset { id: "submarine", file: "Submarine" },
];

pub fn preset_file(id: &str) -> Option<&'static str> {
    PRESETS.iter().find(|preset| preset.id == id).map(|preset| preset.file)
}

pub fn is_known_choice(sound: &str) -> bool {
    sound == "random" || is_bundled_id(sound) || preset_file(sound).is_some() || is_custom_key(sound)
}

/// Keeps a known preset, `random`, or `custom:<id>`. Anything else becomes Cristal.
pub fn normalize_choice(sound: &str) -> String {
    let sound = sound.trim();
    if is_known_choice(sound) {
        sound.to_string()
    } else {
        "glass".into()
    }
}

pub fn is_custom_key(sound: &str) -> bool {
    sound.strip_prefix("custom:").is_some_and(|id| !id.is_empty() && !id.contains('/') && !id.contains('\\'))
}

pub fn allowed_extension(name: &str) -> Option<&'static str> {
    let ext = name.rsplit('.').next()?.to_ascii_lowercase();
    match ext.as_str() {
        "wav" => Some("wav"),
        "mp3" => Some("mp3"),
        "m4a" => Some("m4a"),
        "aiff" => Some("aiff"),
        "aif" => Some("aif"),
        "caf" => Some("caf"),
        _ => None,
    }
}

/// `afinfo` prints `estimated duration: 1.650083 sec`.
pub fn duration_from_afinfo(text: &str) -> Option<f64> {
    for line in text.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("estimated duration:").or_else(|| line.strip_prefix("duration:")) else {
            continue;
        };
        let Some(number) = rest.trim().split_whitespace().next() else {
            continue;
        };
        return number.parse().ok();
    }
    None
}

pub fn random_preset(seed: u64) -> &'static str {
    PRESETS[(seed as usize) % PRESETS.len()].id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn afinfo_duration_is_read_in_seconds() {
        let sample = "File: Glass.aiff\nestimated duration: 1.650083 sec\n";
        assert!((duration_from_afinfo(sample).unwrap() - 1.650083).abs() < 0.0001);
        assert!(duration_from_afinfo("no duration here").is_none());
    }

    #[test]
    fn only_standard_audio_extensions_pass() {
        assert_eq!(allowed_extension("pausa.MP3"), Some("mp3"));
        assert_eq!(allowed_extension("tono.m4a"), Some("m4a"));
        assert_eq!(allowed_extension("clip.wav"), Some("wav"));
        assert!(allowed_extension("virus.exe").is_none());
        assert!(allowed_extension("sin-extension").is_none());
    }

    #[test]
    fn unknown_choices_fall_back_to_glass() {
        assert_eq!(normalize_choice("ping"), "ping");
        assert_eq!(normalize_choice("mustache"), "mustache");
        assert_eq!(normalize_choice("abuela"), "abuela");
        assert_eq!(normalize_choice("random"), "random");
        assert_eq!(normalize_choice("custom:abc"), "custom:abc");
        assert_eq!(normalize_choice("custom:../x"), "glass");
        assert_eq!(normalize_choice("nope"), "glass");
    }
}
