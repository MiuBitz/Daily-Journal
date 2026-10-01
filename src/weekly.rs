use crate::config::WorkBlock;
use chrono::{Datelike, Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub date: String,
    pub block_id: String,
    pub name: String,
    pub start: String,
    pub end: String,
    pub work: String,
    pub notes: String,
}

pub fn week_start(date: &str) -> Result<NaiveDate, String> {
    let day = NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|e| e.to_string())?;
    Ok(day - Duration::days(day.weekday().num_days_from_monday() as i64))
}

pub fn text_path(folder: &str, date: &str) -> Result<PathBuf, String> {
    Ok(Path::new(folder).join(format!("week-{}.txt", week_start(date)?)))
}

pub fn load(folder: &str, date: &str) -> Result<Vec<Entry>, String> {
    let path = text_path(folder, date)?.with_extension("json");
    match fs::read_to_string(path) {
        Ok(data) => {
            serde_json::from_str(&data).map_err(|e| format!("Cannot read saved entries: {e}"))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(vec![]),
        Err(e) => Err(e.to_string()),
    }
}

pub fn render(date: &str, entries: &[Entry]) -> Result<String, String> {
    let start = week_start(date)?;
    let mut text = format!(
        "Weekly work journal: {} to {}\n",
        start,
        start + Duration::days(6)
    );
    let mut sorted = entries.to_vec();
    sorted.sort_by(|a, b| (&a.date, &a.start, &a.block_id).cmp(&(&b.date, &b.start, &b.block_id)));
    let mut previous = String::new();
    for entry in sorted {
        if previous != entry.date {
            text.push_str(&format!("\nDate - {}\n", entry.date));
            previous = entry.date.clone();
        }
        text.push_str(&format!(
            "\n{} ({} - {})\nWork - {}\n",
            entry.name,
            entry.start,
            entry.end,
            entry.work.trim()
        ));
        if !entry.notes.trim().is_empty() {
            text.push_str(&format!("Notes - {}\n", entry.notes.trim()));
        }
    }
    Ok(text)
}

fn atomic_write(path: &Path, contents: &str) -> Result<(), String> {
    let temp = path.with_extension(format!(
        "{}.tmp",
        path.extension().unwrap_or_default().to_string_lossy()
    ));
    fs::write(&temp, contents).map_err(|e| e.to_string())?;
    fs::rename(temp, path).map_err(|e| e.to_string())
}

pub fn save(
    folder: &str,
    date: &str,
    block: &WorkBlock,
    work: &str,
    notes: &str,
) -> Result<String, String> {
    if work.trim().is_empty() {
        return Err("Write what you did before saving.".into());
    }
    let mut entries = load(folder, date)?;
    let entry = Entry {
        date: date.into(),
        block_id: block.id.clone(),
        name: block.name.clone(),
        start: block.start.clone(),
        end: block.end.clone(),
        work: work.trim().into(),
        notes: notes.trim().into(),
    };
    if let Some(old) = entries
        .iter_mut()
        .find(|e| e.date == date && e.block_id == block.id)
    {
        *old = entry;
    } else {
        entries.push(entry);
    }
    fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    let path = text_path(folder, date)?;
    atomic_write(
        &path.with_extension("json"),
        &serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())?,
    )?;
    atomic_write(&path, &render(date, &entries)?)?;
    Ok(path.to_string_lossy().into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn week_crosses_year_and_optional_notes() {
        assert_eq!(week_start("2027-01-01").unwrap().to_string(), "2026-12-28");
        let entry = Entry {
            date: "2027-01-01".into(),
            block_id: "a".into(),
            name: "Block 1".into(),
            start: "09:00".into(),
            end: "12:00".into(),
            work: "Built a feature".into(),
            notes: "".into(),
        };
        let text = render(&entry.date, &[entry.clone()]).unwrap();
        assert!(text.contains("Date - 2027-01-01"));
        assert!(!text.contains("Notes -"));
    }
    #[test]
    fn edits_replace_and_other_days_survive() {
        let folder =
            std::env::temp_dir().join(format!("daily-journal-test-{}", std::process::id()));
        let folder_str = folder.to_str().unwrap();
        let block = WorkBlock {
            id: "a".into(),
            name: "Block 1".into(),
            start: "09:00".into(),
            end: "12:00".into(),
            reminder: false,
        };
        save(folder_str, "2026-10-01", &block, "First", "").unwrap();
        save(folder_str, "2026-10-02", &block, "Other day", "A note").unwrap();
        save(folder_str, "2026-10-01", &block, "Updated", "").unwrap();
        let entries = load(folder_str, "2026-10-01").unwrap();
        assert_eq!(entries.len(), 2);
        let text = fs::read_to_string(text_path(folder_str, "2026-10-01").unwrap()).unwrap();
        assert!(text.contains("Updated") && text.contains("Other day") && !text.contains("First"));
        fs::remove_dir_all(folder).unwrap();
    }
}
