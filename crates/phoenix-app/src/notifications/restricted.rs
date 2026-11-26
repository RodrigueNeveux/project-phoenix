use std::sync::Mutex;

use chrono::{Datelike, Timelike};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    pub start: Option<u32>,
    pub end: Option<u32>,
    pub days: Vec<u8>,
    pub enabled: bool,
}

static RULES: Mutex<Vec<Rule>> = Mutex::new(Vec::new());

const MONDAY: u8 = 1;
const SUNDAY: u8 = 7;
const DAYS: [&str; 7] = ["MONDAY", "TUESDAY", "WEDNESDAY", "THURSDAY", "FRIDAY", "SATURDAY", "SUNDAY"];

fn minutes(v: Option<&Value>) -> Option<u32> {
    let v = v?;
    Some(v.get("hours")?.as_u64()? as u32 * 60 + v.get("minutes")?.as_u64()? as u32)
}

fn day(v: &Value) -> Option<u8> {
    match v {
        Value::Number(n) => n.as_u64().filter(|d| (1..=7).contains(d)).map(|d| d as u8),
        Value::String(s) => DAYS.iter().position(|d| d == s).map(|i| i as u8 + 1),
        _ => None,
    }
}

pub fn parse(schedule: Option<&Value>) -> Vec<Rule> {
    let Some(rules) = schedule.and_then(|s| s.get("rules")).and_then(Value::as_array) else { return Vec::new() };
    rules
        .iter()
        .map(|r| Rule {
            start: minutes(r.get("start_time")),
            end: minutes(r.get("end_time")),
            days: r.get("days").and_then(Value::as_array).map(|d| d.iter().filter_map(day).collect()).unwrap_or_default(),
            enabled: r.get("enabled").and_then(Value::as_bool).unwrap_or(false),
        })
        .collect()
}

impl Rule {
    pub fn active_at(&self, today: u8, now: u32) -> bool {
        let (Some(start), Some(end)) = (self.start, self.end) else { return false };
        if self.days.is_empty() || !self.enabled {
            return false;
        }
        let overnight = start > end;
        if self.days.contains(&today) {
            if overnight {
                if now >= start {
                    return true;
                }
            } else if now >= start && now < end {
                return true;
            }
        }
        if overnight {
            let yesterday = if today == MONDAY { SUNDAY } else { today - 1 };
            if self.days.contains(&yesterday) && now < end {
                return true;
            }
        }
        false
    }
}

pub fn in_restricted_hours(rules: &[Rule], today: u8, now: u32) -> bool {
    rules.iter().any(|r| r.active_at(today, now))
}

pub fn set(schedule: Option<&Value>) {
    *RULES.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = parse(schedule);
}

pub fn active_now() -> bool {
    let rules = RULES.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if rules.is_empty() {
        return false;
    }
    let t = chrono::Local::now();
    in_restricted_hours(&rules, t.weekday().number_from_monday() as u8, t.hour() * 60 + t.minute())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn schedules_follow_the_family_center_rules() {
        let s = json!({"rules": [
            {"rule_id": "1", "label": "school", "start_time": {"hours": 9, "minutes": 0}, "end_time": {"hours": 15, "minutes": 30}, "days": [1, 2, 3], "enabled": true},
            {"rule_id": "2", "label": "night", "start_time": {"hours": 22, "minutes": 0}, "end_time": {"hours": 7, "minutes": 0}, "days": ["FRIDAY"], "enabled": true},
            {"rule_id": "3", "label": "off", "start_time": {"hours": 0, "minutes": 0}, "end_time": {"hours": 23, "minutes": 59}, "days": [6], "enabled": false},
            {"rule_id": "4", "label": "partial", "start_time": {"hours": 1}, "end_time": {"hours": 23, "minutes": 0}, "days": [7], "enabled": true}
        ]});
        let r = parse(Some(&s));
        assert_eq!(r.len(), 4);
        assert!(in_restricted_hours(&r, 1, 9 * 60));
        assert!(!in_restricted_hours(&r, 1, 15 * 60 + 30), "end is exclusive");
        assert!(!in_restricted_hours(&r, 4, 10 * 60), "thursday is not listed");
        assert!(in_restricted_hours(&r, 5, 23 * 60), "overnight rule from its start");
        assert!(in_restricted_hours(&r, 6, 6 * 60), "overnight rule spills into the next morning");
        assert!(!in_restricted_hours(&r, 6, 7 * 60));
        assert!(!in_restricted_hours(&r, 6, 12 * 60), "disabled rule");
        assert!(!in_restricted_hours(&r, 7, 12 * 60), "a missing minutes field never matches, like NaN in the client");
        let sunday_night = parse(Some(&json!({"rules": [{"start_time": {"hours": 23, "minutes": 0}, "end_time": {"hours": 2, "minutes": 0}, "days": [7], "enabled": true}]})));
        assert!(in_restricted_hours(&sunday_night, 1, 60), "monday looks back to sunday");
        assert!(parse(None).is_empty() && !in_restricted_hours(&[], 1, 0));
    }
}
