use yoyo_core::{MediaChapter, MediaMarker};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressTickKind {
    Chapter,
    Marker,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProgressTick {
    pub percent: f32,
    pub label: String,
    pub kind: ProgressTickKind,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NavigationRow {
    pub title: String,
    pub subtitle: String,
    pub seconds: f64,
    pub is_marker: bool,
    pub id: String,
}

fn fmt_clock(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let secs = total % 60;
    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{secs:02}")
    } else {
        format!("{minutes:02}:{secs:02}")
    }
}

pub fn parse_jump_time(input: &str) -> Result<f64, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Enter a time".into());
    }

    let parts = trimmed.split(':').collect::<Vec<_>>();
    if parts.len() > 3 {
        return Err("Use ss, mm:ss, or hh:mm:ss".into());
    }

    let mut total = 0.0;
    for part in parts {
        if part.trim().is_empty() {
            return Err("Invalid time".into());
        }
        let value = part.parse::<f64>().map_err(|_| "Invalid time".to_string())?;
        if !value.is_finite() || value < 0.0 {
            return Err("Invalid time".into());
        }
        total = total * 60.0 + value;
    }

    Ok(total)
}

pub fn build_progress_ticks(
    chapters: &[MediaChapter],
    markers: &[MediaMarker],
    duration: Option<f64>,
) -> Vec<ProgressTick> {
    let Some(duration) = duration.filter(|duration| duration.is_finite() && *duration > 0.0) else {
        return Vec::new();
    };

    let mut ticks = Vec::new();
    for chapter in chapters {
        if chapter.time_seconds.is_finite() && chapter.time_seconds >= 0.0 {
            ticks.push(ProgressTick {
                percent: (chapter.time_seconds / duration).clamp(0.0, 1.0) as f32,
                label: chapter.title.clone().unwrap_or_else(|| "Chapter".into()),
                kind: ProgressTickKind::Chapter,
            });
        }
    }
    for marker in markers {
        if marker.time_seconds.is_finite() && marker.time_seconds >= 0.0 {
            ticks.push(ProgressTick {
                percent: (marker.time_seconds / duration).clamp(0.0, 1.0) as f32,
                label: marker.title.clone(),
                kind: ProgressTickKind::Marker,
            });
        }
    }

    ticks.sort_by(|left, right| left.percent.total_cmp(&right.percent));
    ticks
}

pub fn build_navigation_rows(
    chapters: &[MediaChapter],
    markers: &[MediaMarker],
) -> Vec<NavigationRow> {
    let mut rows = Vec::new();
    for (index, chapter) in chapters.iter().enumerate() {
        if chapter.time_seconds.is_finite() && chapter.time_seconds >= 0.0 {
            rows.push(NavigationRow {
                title: chapter.title.clone().unwrap_or_else(|| format!("Chapter {}", index + 1)),
                subtitle: fmt_clock(chapter.time_seconds),
                seconds: chapter.time_seconds,
                is_marker: false,
                id: index.to_string(),
            });
        }
    }
    for marker in markers {
        if marker.time_seconds.is_finite() && marker.time_seconds >= 0.0 {
            rows.push(NavigationRow {
                title: marker.title.clone(),
                subtitle: fmt_clock(marker.time_seconds),
                seconds: marker.time_seconds,
                is_marker: true,
                id: marker.id.clone(),
            });
        }
    }

    rows.sort_by(|left, right| left.seconds.total_cmp(&right.seconds));
    rows
}

pub fn format_preview_label(seconds: f64, nearest_label: Option<&str>) -> String {
    match nearest_label {
        Some(label) if !label.trim().is_empty() => format!("{} - {}", fmt_clock(seconds), label),
        _ => fmt_clock(seconds),
    }
}
