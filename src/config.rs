use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub const APP_NAME: &str = "Daily Journal";

fn default_morning_time() -> String {
    "12:00".to_string()
}
fn default_afternoon_time() -> String {
    "18:00".to_string()
}
fn default_evening_time() -> String {
    "22:00".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemindersConfig {
    pub morning: bool,
    pub afternoon: bool,
    pub evening: bool,

    #[serde(default = "default_morning_time")]
    pub morning_time: String,

    #[serde(default = "default_afternoon_time")]
    pub afternoon_time: String,

    #[serde(default = "default_evening_time")]
    pub evening_time: String,
}

impl Default for RemindersConfig {
    fn default() -> Self {
        Self {
            morning: true,
            afternoon: true,
            evening: true,
            morning_time: default_morning_time(),
            afternoon_time: default_afternoon_time(),
            evening_time: default_evening_time(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkBlock {
    pub id: String,
    pub name: String,
    pub start: String,
    pub end: String,
    pub reminder: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_theme_mode")]
    pub theme_mode: String,
    #[serde(default = "default_color_preset")]
    pub color_preset: String,
    pub output_folder: String,
    #[serde(default)]
    pub blocks: Vec<WorkBlock>,
    pub reminders: RemindersConfig,
    pub start_with_windows: bool,
    pub minimize_to_tray: bool,
    pub last_reminded: HashMap<String, String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        let default_output = dirs_next_output_folder();
        Self {
            theme_mode: default_theme_mode(),
            color_preset: default_color_preset(),
            output_folder: default_output,
            blocks: legacy_blocks(&RemindersConfig::default()),
            reminders: RemindersConfig::default(),
            start_with_windows: true,
            minimize_to_tray: true,
            last_reminded: HashMap::new(),
        }
    }
}

pub fn config_dir() -> PathBuf {
    if let Some(app_data) = std::env::var_os("APPDATA") {
        PathBuf::from(app_data).join("DailyJournal")
    } else if let Some(home) = dirs_home_dir() {
        home.join("DailyJournal")
    } else {
        PathBuf::from("DailyJournal")
    }
}

pub fn config_file() -> PathBuf {
    config_dir().join("config.json")
}

pub fn dirs_home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}

pub fn dirs_next_output_folder() -> String {
    if let Some(home) = dirs_home_dir() {
        home.join("Documents")
            .join("Daily Journal")
            .to_string_lossy()
            .to_string()
    } else {
        String::from("Daily Journal")
    }
}

impl AppConfig {
    pub fn load() -> Self {
        let path = config_file();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(mut cfg) = serde_json::from_str::<AppConfig>(&content) {
                    if cfg.blocks.is_empty() {
                        cfg.blocks = legacy_blocks(&cfg.reminders);
                    }
                    return cfg;
                }
            }
        }
        let default_cfg = AppConfig::default();
        default_cfg.save();
        default_cfg
    }

    pub fn save(&self) {
        let _ = self.try_save();
    }

    pub fn try_save(&self) -> Result<(), String> {
        fs::create_dir_all(config_dir()).map_err(|e| e.to_string())?;
        let temp = config_file().with_extension("json.tmp");
        let data = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&temp, data).map_err(|e| e.to_string())?;
        fs::rename(temp, config_file()).map_err(|e| e.to_string())
    }
}

pub fn legacy_blocks(rem: &RemindersConfig) -> Vec<WorkBlock> {
    vec![
        WorkBlock {
            id: "morning".into(),
            name: "Morning".into(),
            start: "05:00".into(),
            end: rem.morning_time.clone(),
            reminder: rem.morning,
        },
        WorkBlock {
            id: "afternoon".into(),
            name: "Afternoon".into(),
            start: rem.morning_time.clone(),
            end: rem.afternoon_time.clone(),
            reminder: rem.afternoon,
        },
        WorkBlock {
            id: "evening".into(),
            name: "Evening".into(),
            start: rem.afternoon_time.clone(),
            end: rem.evening_time.clone(),
            reminder: rem.evening,
        },
    ]
}

pub fn validate_blocks(blocks: &[WorkBlock]) -> Result<(), String> {
    if blocks.is_empty() {
        return Err("Add at least one block.".into());
    }
    let mut ids = std::collections::HashSet::new();
    for block in blocks {
        if block.id.is_empty()
            || !ids.insert(&block.id)
            || block.name.trim().is_empty()
            || block.name.contains(['\n', '\r'])
        {
            return Err("Each block needs a name and a unique ID.".into());
        }
        for time in [&block.start, &block.end] {
            if time.len() != 5 || chrono::NaiveTime::parse_from_str(time, "%H:%M").is_err() {
                return Err("Use valid 24-hour times for each block.".into());
            }
        }
        if block.start == block.end {
            return Err("Block start and end must differ.".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn appearance_migrates_and_round_trips() {
        let mut json = serde_json::to_value(AppConfig::default()).unwrap();
        json.as_object_mut().unwrap().remove("theme_mode");
        json.as_object_mut().unwrap().remove("color_preset");
        let mut config: AppConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.theme_mode, "dark");
        assert_eq!(config.color_preset, "blue");
        config.theme_mode = "light".into();
        config.color_preset = "sage".into();
        let saved: AppConfig =
            serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
        assert_eq!(saved.theme_mode, "light");
        assert_eq!(saved.color_preset, "sage");
        assert!(validate_appearance(&saved).is_ok());
        config.color_preset = "invalid".into();
        assert!(validate_appearance(&config).is_err());
    }

    #[test]
    fn existing_config_retains_reminder_times() {
        let mut config = AppConfig::default();
        config.reminders.morning_time = "11:30".into();
        config.reminders.afternoon = false;
        let mut json = serde_json::to_value(config).unwrap();
        json.as_object_mut().unwrap().remove("blocks");
        let old: AppConfig = serde_json::from_value(json).unwrap();
        let blocks = legacy_blocks(&old.reminders);
        assert_eq!(blocks[0].end, "11:30");
        assert_eq!(blocks[1].start, "11:30");
        assert!(!blocks[1].reminder);
    }
    #[test]
    fn block_validation_accepts_overnight_and_rejects_invalid_times() {
        let mut blocks = legacy_blocks(&RemindersConfig::default());
        blocks[0].start = "22:00".into();
        blocks[0].end = "02:00".into();
        assert!(validate_blocks(&blocks).is_ok());
        blocks[0].end = "25:00".into();
        assert!(validate_blocks(&blocks).is_err());
        assert!(validate_blocks(&[]).is_err());
    }
}

fn default_theme_mode() -> String {
    "dark".into()
}
fn default_color_preset() -> String {
    "blue".into()
}

pub fn validate_appearance(config: &AppConfig) -> Result<(), String> {
    if !["light", "dark"].contains(&config.theme_mode.as_str())
        || !["violet", "blue", "sage", "amber"].contains(&config.color_preset.as_str())
    {
        return Err("Choose an available theme and color preset.".into());
    }
    Ok(())
}
