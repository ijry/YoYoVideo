use chrono::{DateTime, Datelike, Duration, LocalResult, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

/// Weekdays use Monday = bit 0. An overnight interval belongs to its start day.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrivacyTimeRule {
    pub weekdays: u8,
    pub start_minute: u16,
    pub end_minute: u16,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrivacySchedule {
    pub enabled: bool,
    pub rules: Vec<PrivacyTimeRule>,
}

/// Expiry is the next *merged start*, never the end of the current interval.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManualPrivacy {
    pub enabled: bool,
    pub until: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrivacyDecision {
    pub enabled: bool,
    pub manual: bool,
    pub next_start: Option<DateTime<Utc>>,
}

impl PrivacySchedule {
    pub fn validate(&self) -> Result<(), String> {
        if self.rules.len() > 64 {
            return Err("At most 64 privacy periods are allowed".into());
        }
        for rule in &self.rules {
            if rule.weekdays == 0
                || rule.weekdays > 127
                || rule.start_minute >= 1440
                || rule.end_minute >= 1440
                || rule.start_minute == rule.end_minute
            {
                return Err("Invalid privacy period".into());
            }
        }
        Ok(())
    }

    pub fn evaluate<Tz: TimeZone>(&self, now: DateTime<Utc>, tz: &Tz) -> PrivacyDecision {
        let mut decision = PrivacyDecision { enabled: false, manual: false, next_start: None };
        if !self.enabled {
            return decision;
        }
        if self.validate().is_err() {
            decision.enabled = true;
            return decision;
        }
        let date = now.with_timezone(tz).date_naive();
        let mut intervals = Vec::new();
        // More than a week on either side lets cross-midnight and touching rules
        // merge before we choose the next real start (not a window boundary).
        for day in -8..=15 {
            let Some(date) = date.checked_add_signed(Duration::days(day)) else {
                continue;
            };
            for rule in &self.rules {
                if rule.weekdays & (1 << date.weekday().num_days_from_monday()) == 0 {
                    continue;
                }
                let Some(start) = date.and_hms_opt(
                    u32::from(rule.start_minute / 60),
                    u32::from(rule.start_minute % 60),
                    0,
                ) else {
                    continue;
                };
                let end_date =
                    if rule.end_minute < rule.start_minute { date.succ_opt() } else { Some(date) };
                let Some(end) = end_date.and_then(|d| {
                    d.and_hms_opt(
                        u32::from(rule.end_minute / 60),
                        u32::from(rule.end_minute % 60),
                        0,
                    )
                }) else {
                    continue;
                };
                if let (Some(start), Some(end)) =
                    (resolve_local(tz, start, true), resolve_local(tz, end, false))
                {
                    if start < end {
                        intervals.push((start, end));
                    }
                }
            }
        }
        intervals.sort_unstable_by_key(|value| value.0);
        let mut merged: Vec<(DateTime<Utc>, DateTime<Utc>)> = Vec::new();
        for (start, end) in intervals {
            if let Some(last) = merged.last_mut() {
                if start <= last.1 {
                    last.1 = last.1.max(end);
                    continue;
                }
            }
            merged.push((start, end));
        }
        decision.enabled = merged.iter().any(|&(start, end)| start <= now && now < end);
        decision.next_start =
            merged.iter().find(|&&(start, _)| start > now).map(|&(start, _)| start);
        decision
    }

    pub fn with_override<Tz: TimeZone>(
        &self,
        value: Option<&ManualPrivacy>,
        now: DateTime<Utc>,
        tz: &Tz,
    ) -> PrivacyDecision {
        let mut decision = self.evaluate(now, tz);
        if let Some(value) = value.filter(|v| v.until.is_none_or(|deadline| now < deadline)) {
            decision.enabled = value.enabled;
            decision.manual = true;
        }
        decision
    }
}

fn resolve_local<Tz: TimeZone>(
    tz: &Tz,
    mut local: NaiveDateTime,
    start: bool,
) -> Option<DateTime<Utc>> {
    // Includes a skipped calendar day, not just the usual one-hour DST gap.
    for _ in 0..=2880 {
        match tz.from_local_datetime(&local) {
            LocalResult::Single(value) => return Some(value.with_timezone(&Utc)),
            LocalResult::Ambiguous(a, b) => {
                return Some(if start { a.min(b) } else { a.max(b) }.with_timezone(&Utc));
            }
            LocalResult::None => local = local.checked_add_signed(Duration::minutes(1))?,
        }
    }
    None
}
