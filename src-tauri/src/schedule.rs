use crate::state::{minutes_of, AppData};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Alarm,
    Break,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub id: String,
    pub kind: Kind,
    pub slot: String,
    pub title: String,
    pub body: String,
    pub sound: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Moment {
    /// Monday is 1, Sunday is 7.
    pub weekday: u8,
    pub minutes: u16,
    pub date: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tick {
    pub notices: Vec<Notice>,
    /// Do-not-disturb still consumes the slot, it just stays quiet.
    pub audible: bool,
}

pub fn evaluate(data: &AppData, now: Moment) -> Tick {
    Tick { notices: collect(data, now), audible: !data.dnd }
}

pub fn collect(data: &AppData, now: Moment) -> Vec<Notice> {
    let mut notices = Vec::new();
    for alarm in &data.alarms {
        if let Some(notice) = alarm_due(alarm, &now) {
            notices.push(notice);
        }
    }
    for item in &data.breaks {
        if let Some(notice) = break_due(item, &data.break_window, &now) {
            notices.push(notice);
        }
    }
    notices
}

pub fn mark(data: &mut AppData, notice: &Notice) {
    match notice.kind {
        Kind::Alarm => {
            if let Some(alarm) = data.alarms.iter_mut().find(|alarm| alarm.id == notice.id) {
                alarm.last_slot = Some(notice.slot.clone());
            }
        }
        Kind::Break => {
            if let Some(item) = data.breaks.iter_mut().find(|item| item.id == notice.id) {
                item.last_slot = Some(notice.slot.clone());
            }
        }
    }
}

fn alarm_due(alarm: &crate::state::Alarm, now: &Moment) -> Option<Notice> {
    if !alarm.enabled || !alarm.days.contains(&now.weekday) {
        return None;
    }
    let minutes = minutes_of(&alarm.time)?;
    if minutes != now.minutes {
        return None;
    }
    let slot = slot_of(now);
    if alarm.last_slot.as_deref() == Some(slot.as_str()) {
        return None;
    }
    let title = title_or(&alarm.name, "Alarma");
    Some(Notice {
        id: alarm.id.clone(),
        kind: Kind::Alarm,
        slot,
        body: format!("Es la hora ({})", alarm.time),
        title,
        sound: alarm.sound.clone(),
    })
}

fn break_due(item: &crate::state::Break, window: &crate::state::BreakWindow, now: &Moment) -> Option<Notice> {
    if !item.enabled || item.every_minutes == 0 || !window.days.contains(&now.weekday) {
        return None;
    }
    let start = minutes_of(&window.start)?;
    let end = minutes_of(&window.end)?;
    if start >= end || now.minutes < start || now.minutes >= end {
        return None;
    }
    let elapsed = now.minutes - start;
    if elapsed == 0 || elapsed % item.every_minutes != 0 {
        return None;
    }
    let slot = slot_of(now);
    if item.last_slot.as_deref() == Some(slot.as_str()) {
        return None;
    }
    let message = item.message.trim();
    Some(Notice {
        id: item.id.clone(),
        kind: Kind::Break,
        slot,
        title: title_or(&item.name, "Break"),
        body: if message.is_empty() { "Es hora de un break.".into() } else { message.to_string() },
        sound: item.sound.clone(),
    })
}

fn slot_of(now: &Moment) -> String {
    format!("{}-{}", now.date, now.minutes)
}

fn title_or(name: &str, fallback: &str) -> String {
    let name = name.trim();
    if name.is_empty() { fallback.into() } else { name.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::default_data;

    fn at(weekday: u8, hour: u16, minute: u16) -> Moment {
        Moment { weekday, minutes: hour * 60 + minute, date: "2026-10-27".into() }
    }

    #[test]
    fn the_sample_day_rings_lunch_the_walk_and_the_end_of_day() {
        let data = default_data();
        let morning = collect(&data, at(2, 9, 0));
        assert!(morning.is_empty(), "the window opening is not a break");

        let walk = collect(&data, at(2, 11, 0));
        assert_eq!(walk.len(), 1);
        assert_eq!(walk[0].title, "Caminar");
        assert_eq!(walk[0].body, "Parate a caminar");

        let lunch = collect(&data, at(2, 13, 0));
        let titles: Vec<_> = lunch.iter().map(|notice| notice.title.as_str()).collect();
        assert!(titles.contains(&"Almuerzo"));
        assert!(titles.contains(&"Caminar"));

        let closing = collect(&data, at(2, 18, 0));
        assert_eq!(closing.len(), 1);
        assert_eq!(closing[0].title, "Cierre del día");
    }

    #[test]
    fn a_break_does_not_ring_outside_its_days_or_twice() {
        let mut data = default_data();
        assert!(collect(&data, at(7, 11, 0)).is_empty());
        let notice = collect(&data, at(2, 11, 0)).pop().unwrap();
        mark(&mut data, &notice);
        assert!(collect(&data, at(2, 11, 0)).is_empty());
    }

    #[test]
    fn do_not_disturb_keeps_the_notice_but_stays_quiet() {
        let mut data = default_data();
        data.dnd = true;
        let tick = evaluate(&data, at(2, 13, 0));
        assert!(!tick.audible);
        assert!(!tick.notices.is_empty());
    }

    #[test]
    fn a_paused_alarm_and_a_wrong_day_stay_silent() {
        let mut data = default_data();
        data.alarms[0].enabled = false;
        let notices = collect(&data, at(6, 13, 0));
        assert!(notices.iter().all(|notice| notice.kind != Kind::Alarm));
        assert!(collect(&data, at(2, 13, 1)).iter().all(|notice| notice.title != "Almuerzo"));
    }

    #[test]
    fn breaks_follow_the_interval_from_the_window_start() {
        let mut data = default_data();
        data.break_window.start = "09:30".into();
        data.break_window.end = "17:30".into();
        data.breaks[0].every_minutes = 90;
        assert!(collect(&data, at(2, 9, 30)).is_empty());
        assert_eq!(collect(&data, at(2, 11, 0)).len(), 1);
        assert!(collect(&data, at(2, 11, 30)).is_empty());
        assert_eq!(collect(&data, at(2, 12, 30)).len(), 1);
        assert!(collect(&data, at(2, 17, 30)).is_empty());
    }
}
