use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

fn weekdays() -> Vec<u8> {
    vec![1, 2, 3, 4, 5]
}

fn large_size() -> String {
    "large".into()
}

fn glass_sound() -> String {
    "glass".into()
}

fn ping_sound() -> String {
    "ping".into()
}

fn mustache_reaction() -> String {
    "mustache".into()
}

fn widget_visible_default() -> bool {
    true
}

pub fn normalize_reaction(value: &str) -> String {
    match value {
        "none" | "shake" | "bounce" | "heartbeat" | "mustache" | "abuela" => value.to_string(),
        _ => mustache_reaction(),
    }
}

pub fn normalize_size(size: &str) -> String {
    match size {
        "medium" | "small" => size.to_string(),
        _ => large_size(),
    }
}

fn enabled() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BreakWindow {
    pub start: String,
    pub end: String,
    #[serde(default = "weekdays")]
    pub days: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Alarm {
    pub id: String,
    pub name: String,
    pub time: String,
    pub days: Vec<u8>,
    #[serde(default = "enabled")]
    pub enabled: bool,
    #[serde(default = "glass_sound")]
    pub sound: String,
    #[serde(default)]
    pub last_slot: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomSound {
    pub id: String,
    pub name: String,
    pub file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Break {
    pub id: String,
    pub name: String,
    pub message: String,
    pub every_minutes: u16,
    #[serde(default = "enabled")]
    pub enabled: bool,
    #[serde(default = "ping_sound")]
    pub sound: String,
    #[serde(default)]
    pub last_slot: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pos {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppData {
    pub version: u32,
    pub dnd: bool,
    #[serde(default = "large_size")]
    pub widget_size: String,
    #[serde(default = "mustache_reaction")]
    pub reaction: String,
    #[serde(default = "widget_visible_default")]
    pub widget_visible: bool,
    pub break_window: BreakWindow,
    pub alarms: Vec<Alarm>,
    pub breaks: Vec<Break>,
    #[serde(default)]
    pub sounds: Vec<CustomSound>,
    #[serde(default)]
    pub widget: Option<Pos>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlarmInput {
    pub id: String,
    pub name: String,
    pub time: String,
    pub days: Vec<u8>,
    #[serde(default = "enabled")]
    pub enabled: bool,
    #[serde(default = "glass_sound")]
    pub sound: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BreakInput {
    pub id: String,
    pub name: String,
    pub message: String,
    pub every_minutes: u16,
    #[serde(default = "enabled")]
    pub enabled: bool,
    #[serde(default = "ping_sound")]
    pub sound: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub dnd: bool,
    #[serde(default = "large_size")]
    pub widget_size: String,
    #[serde(default = "mustache_reaction")]
    pub reaction: String,
    #[serde(default = "widget_visible_default")]
    pub widget_visible: bool,
    pub break_window: BreakWindow,
    pub alarms: Vec<AlarmInput>,
    pub breaks: Vec<BreakInput>,
    #[serde(default)]
    pub sounds: Vec<CustomSound>,
}

pub fn default_data() -> AppData {
    AppData {
        version: 1,
        dnd: false,
        widget_size: large_size(),
        reaction: mustache_reaction(),
        break_window: BreakWindow { start: "09:00".into(), end: "18:00".into(), days: weekdays() },
        alarms: vec![
            Alarm {
                id: "almuerzo".into(),
                name: "Almuerzo".into(),
                time: "13:00".into(),
                days: weekdays(),
                enabled: true,
                sound: glass_sound(),
                last_slot: None,
            },
            Alarm {
                id: "cierre".into(),
                name: "Cierre del día".into(),
                time: "18:00".into(),
                days: weekdays(),
                enabled: true,
                sound: "hero".into(),
                last_slot: None,
            },
        ],
        breaks: vec![Break {
            id: "caminar".into(),
            name: "Caminar".into(),
            message: "Parate a caminar".into(),
            every_minutes: 120,
            enabled: true,
            sound: ping_sound(),
            last_slot: None,
        }],
        sounds: Vec::new(),
        widget: None,
        widget_visible: true,
    }
}

pub fn settings_of(data: &AppData) -> Settings {
    Settings {
        dnd: data.dnd,
        widget_size: data.widget_size.clone(),
        reaction: data.reaction.clone(),
        widget_visible: data.widget_visible,
        break_window: data.break_window.clone(),
        alarms: data
            .alarms
            .iter()
            .map(|alarm| AlarmInput {
                id: alarm.id.clone(),
                name: alarm.name.clone(),
                time: alarm.time.clone(),
                days: alarm.days.clone(),
                enabled: alarm.enabled,
                sound: alarm.sound.clone(),
            })
            .collect(),
        breaks: data
            .breaks
            .iter()
            .map(|item| BreakInput {
                id: item.id.clone(),
                name: item.name.clone(),
                message: item.message.clone(),
                every_minutes: item.every_minutes,
                enabled: item.enabled,
                sound: item.sound.clone(),
            })
            .collect(),
        sounds: data.sounds.clone(),
    }
}

pub fn load(path: &Path) -> AppData {
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|error| {
            eprintln!("gentle-break: el estado no se pudo leer ({error}), empiezo de cero");
            default_data()
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => default_data(),
        Err(error) => {
            eprintln!("gentle-break: no pude abrir el estado ({error})");
            default_data()
        }
    }
}

pub fn store(path: &Path, data: &AppData) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let text = serde_json::to_string_pretty(data).map_err(|error| error.to_string())?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, text).map_err(|error| error.to_string())?;
    fs::rename(&tmp, path).map_err(|error| error.to_string())?;
    Ok(())
}

pub fn apply_settings(data: &mut AppData, incoming: Settings) {
    data.dnd = incoming.dnd;
    data.widget_size = normalize_size(&incoming.widget_size);
    data.reaction = normalize_reaction(&incoming.reaction);
    data.widget_visible = incoming.widget_visible;
    if let Some(window) = normalize_window(&incoming.break_window) {
        data.break_window = window;
    }
    data.alarms = merge_alarms(&data.alarms, incoming.alarms);
    data.breaks = merge_breaks(&data.breaks, incoming.breaks);
    data.sounds = sanitize_sounds(incoming.sounds);
}

fn merge_alarms(previous: &[Alarm], incoming: Vec<AlarmInput>) -> Vec<Alarm> {
    incoming
        .into_iter()
        .filter_map(|alarm| {
            let id = alarm.id.trim();
            if id.is_empty() {
                return None;
            }
            let time = canonical_time(&alarm.time)?;
            let last_slot = previous.iter().find(|item| item.id == id).and_then(|item| item.last_slot.clone());
            Some(Alarm {
                id: id.to_string(),
                name: clip(&alarm.name, 80),
                time,
                days: normalize_days(&alarm.days),
                enabled: alarm.enabled,
                sound: crate::sound::normalize_choice(&alarm.sound),
                last_slot,
            })
        })
        .take(40)
        .collect()
}

fn merge_breaks(previous: &[Break], incoming: Vec<BreakInput>) -> Vec<Break> {
    incoming
        .into_iter()
        .filter_map(|item| {
            let id = item.id.trim();
            if id.is_empty() || item.every_minutes == 0 {
                return None;
            }
            let last_slot = previous.iter().find(|saved| saved.id == id).and_then(|saved| saved.last_slot.clone());
            Some(Break {
                id: id.to_string(),
                name: clip(&item.name, 80),
                message: clip(&item.message, 240),
                every_minutes: item.every_minutes.min(720),
                enabled: item.enabled,
                sound: crate::sound::normalize_choice(&item.sound),
                last_slot,
            })
        })
        .take(40)
        .collect()
}

fn sanitize_sounds(sounds: Vec<CustomSound>) -> Vec<CustomSound> {
    sounds
        .into_iter()
        .filter_map(|sound| {
            let id = sound.id.trim();
            let file = sound.file.trim();
            if id.is_empty() || file.is_empty() || file.contains('/') || file.contains('\\') || file.contains("..") {
                return None;
            }
            Some(CustomSound { id: id.to_string(), name: clip(&sound.name, 80), file: file.to_string() })
        })
        .take(40)
        .collect()
}

fn normalize_window(window: &BreakWindow) -> Option<BreakWindow> {
    let start = canonical_time(&window.start)?;
    let end = canonical_time(&window.end)?;
    let start_m = minutes_of(&start)?;
    let end_m = minutes_of(&end)?;
    if start_m >= end_m {
        return None;
    }
    Some(BreakWindow { start, end, days: normalize_days(&window.days) })
}

pub fn canonical_time(value: &str) -> Option<String> {
    let minutes = minutes_of(value)?;
    Some(format!("{:02}:{:02}", minutes / 60, minutes % 60))
}

pub fn minutes_of(value: &str) -> Option<u16> {
    let (hour, minute) = value.trim().split_once(':')?;
    let hour: u16 = hour.parse().ok()?;
    let minute: u16 = minute.parse().ok()?;
    if hour > 23 || minute > 59 {
        return None;
    }
    Some(hour * 60 + minute)
}

fn normalize_days(days: &[u8]) -> Vec<u8> {
    let mut days: Vec<u8> = days.iter().copied().filter(|day| (1..=7).contains(day)).collect();
    days.sort_unstable();
    days.dedup();
    days
}

fn clip(value: &str, max: usize) -> String {
    value.trim().chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editing_an_alarm_keeps_the_slot_already_rung() {
        let mut data = default_data();
        data.alarms[0].last_slot = Some("2026-10-27-780".into());
        let window = data.break_window.clone();
        apply_settings(
            &mut data,
            Settings {
                dnd: false,
                widget_size: "large".into(),
                reaction: "mustache".into(),
                widget_visible: true,
                break_window: window,
                alarms: vec![AlarmInput {
                    id: "almuerzo".into(),
                    name: "Almuerzo tarde".into(),
                    time: "13:30".into(),
                    days: vec![1, 2, 3, 4, 5],
                    enabled: true,
                    sound: "ping".into(),
                }],
                breaks: vec![],
                sounds: vec![],
            },
        );
        assert_eq!(data.alarms[0].name, "Almuerzo tarde");
        assert_eq!(data.alarms[0].last_slot.as_deref(), Some("2026-10-27-780"));
        assert!(data.breaks.is_empty());
    }

    #[test]
    fn an_invalid_window_leaves_the_previous_one() {
        let mut data = default_data();
        let previous = data.break_window.clone();
        apply_settings(
            &mut data,
            Settings {
                dnd: true,
                widget_size: "tiny".into(),
                reaction: "nope".into(),
                widget_visible: true,
                break_window: BreakWindow { start: "18:00".into(), end: "09:00".into(), days: vec![1] },
                alarms: vec![],
                breaks: vec![],
                sounds: vec![],
            },
        );
        assert!(data.dnd);
        assert_eq!(data.widget_size, "large");
        assert_eq!(data.reaction, "mustache");
        assert_eq!(data.break_window, previous);
    }
}
