use crate::config::RemindersConfig;
use chrono::{Local, Timelike};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalBlock {
    Morning,
    Afternoon,
    Evening,
}

impl JournalBlock {
    pub fn key(&self) -> &'static str {
        match self {
            JournalBlock::Morning => "morning",
            JournalBlock::Afternoon => "afternoon",
            JournalBlock::Evening => "evening",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            JournalBlock::Morning => "Morning",
            JournalBlock::Afternoon => "Afternoon",
            JournalBlock::Evening => "Evening",
        }
    }

    pub fn times_formatted(&self, config: &RemindersConfig) -> (String, String) {
        match self {
            JournalBlock::Morning => ("05:00".to_string(), config.morning_time.clone()),
            JournalBlock::Afternoon => (config.morning_time.clone(), config.afternoon_time.clone()),
            JournalBlock::Evening => (config.afternoon_time.clone(), config.evening_time.clone()),
        }
    }

    pub fn all() -> [JournalBlock; 3] {
        [
            JournalBlock::Morning,
            JournalBlock::Afternoon,
            JournalBlock::Evening,
        ]
    }
}

pub fn today_str() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

pub fn block_for_now() -> JournalBlock {
    let now = Local::now();
    let mins = now.hour() * 60 + now.minute();
    if (5 * 60..12 * 60).contains(&mins) {
        JournalBlock::Morning
    } else if (12 * 60..18 * 60).contains(&mins) {
        JournalBlock::Afternoon
    } else {
        JournalBlock::Evening
    }
}

pub fn file_path_for(output_folder: &str, block: JournalBlock) -> PathBuf {
    let folder = PathBuf::from(output_folder);
    let filename = format!("{}-{}.md", today_str(), block.key());
    folder.join(filename)
}

pub fn template_for(block: JournalBlock, rem: Option<&RemindersConfig>) -> String {
    let default_rem = RemindersConfig::default();
    let rem_ref = rem.unwrap_or(&default_rem);
    let (start, end) = block.times_formatted(rem_ref);
    format!(
        "# {} — {}\n\n## {}–{}\n\n- Energy: /10\n- Focus: /10\n- Main:\n- Done:\n- Distraction:\n- Note:\n\n",
        today_str(),
        block.label(),
        start,
        end
    )
}

pub fn open_journal(
    output_folder: &str,
    block: Option<JournalBlock>,
    rem: Option<&RemindersConfig>,
) -> Result<PathBuf, String> {
    let target_block = block.unwrap_or_else(block_for_now);
    let folder = PathBuf::from(output_folder);
    if let Err(e) = fs::create_dir_all(&folder) {
        return Err(format!("Failed to create directory {:?}: {}", folder, e));
    }

    let path = file_path_for(output_folder, target_block);
    if !path.exists() {
        if let Err(e) = fs::write(&path, template_for(target_block, rem)) {
            return Err(format!("Failed to create journal file {:?}: {}", path, e));
        }
    }

    if let Err(e) = open::that(&path) {
        return Err(format!("Failed to open file {:?}: {}", path, e));
    }

    Ok(path)
}

pub fn create_today_all(output_folder: &str, rem: Option<&RemindersConfig>) -> Result<(), String> {
    let folder = PathBuf::from(output_folder);
    if let Err(e) = fs::create_dir_all(&folder) {
        return Err(format!("Failed to create directory {:?}: {}", folder, e));
    }

    for block in JournalBlock::all() {
        let path = file_path_for(output_folder, block);
        if !path.exists() {
            if let Err(e) = fs::write(&path, template_for(block, rem)) {
                return Err(format!("Failed to create {:?}: {}", path, e));
            }
        }
    }

    Ok(())
}

pub fn open_journal_folder(output_folder: &str) -> Result<(), String> {
    let folder = PathBuf::from(output_folder);
    if let Err(e) = fs::create_dir_all(&folder) {
        return Err(format!("Failed to create directory {:?}: {}", folder, e));
    }
    if let Err(e) = open::that(&folder) {
        return Err(format!("Failed to open folder {:?}: {}", folder, e));
    }
    Ok(())
}
