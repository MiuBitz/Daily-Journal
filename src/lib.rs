mod config;
mod journal;
mod sound;
mod startup;
mod weekly;

use config::{AppConfig, APP_NAME};
use journal::{create_today_all, open_journal, open_journal_folder, today_str, JournalBlock};
use sound::play_reminder_sound;
use startup::set_startup;

use chrono::Local;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, WindowEvent};

static DATA_LOCK: Mutex<()> = Mutex::new(());

#[tauri::command]
fn hide_window(app: AppHandle) -> Result<(), String> {
    app.get_webview_window("main")
        .ok_or("Window unavailable")?
        .hide()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_default_config() -> AppConfig {
    AppConfig::default()
}

#[tauri::command]
fn get_config() -> AppConfig {
    AppConfig::load()
}

#[tauri::command]
fn save_config(app: AppHandle, mut config: AppConfig) -> Result<(), String> {
    let _guard = DATA_LOCK.lock().map_err(|e| e.to_string())?;
    config::validate_blocks(&config.blocks)?;
    config::validate_appearance(&config)?;
    let previous = AppConfig::load();
    config.last_reminded = previous.last_reminded;
    let startup_changed = config.start_with_windows != previous.start_with_windows;
    if startup_changed {
        set_startup(config.start_with_windows)?;
    }
    if let Err(error) = config.try_save() {
        if startup_changed {
            let _ = set_startup(previous.start_with_windows);
        }
        return Err(error);
    }
    if let Some(window) = app.get_webview_window("main") {
        let theme = if config.theme_mode == "light" {
            tauri::Theme::Light
        } else {
            tauri::Theme::Dark
        };
        let _ = window.set_theme(Some(theme));
    }
    Ok(())
}

#[tauri::command]
fn load_entries(date: String) -> Result<Vec<weekly::Entry>, String> {
    weekly::load(&AppConfig::load().output_folder, &date)
}

#[tauri::command]
fn save_entry(
    date: String,
    block_id: String,
    work: String,
    notes: String,
) -> Result<String, String> {
    let _guard = DATA_LOCK.lock().map_err(|e| e.to_string())?;
    let cfg = AppConfig::load();
    let block = cfg
        .blocks
        .iter()
        .find(|b| b.id == block_id)
        .ok_or("Block no longer exists. Reopen the entry.")?;
    weekly::save(&cfg.output_folder, &date, block, &work, &notes)
}

#[tauri::command]
fn open_week(date: String) -> Result<(), String> {
    let _guard = DATA_LOCK.lock().map_err(|e| e.to_string())?;
    let cfg = AppConfig::load();
    let entries = weekly::load(&cfg.output_folder, &date)?;
    if entries.is_empty() {
        return Err("Save an entry first.".into());
    }
    let path = weekly::text_path(&cfg.output_folder, &date)?;
    std::fs::write(&path, weekly::render(&date, &entries)?).map_err(|e| e.to_string())?;
    open::that(path).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_today_str() -> String {
    today_str()
}

#[tauri::command]
fn open_journal_file(block: Option<String>) {
    let cfg = AppConfig::load();
    let target_block = match block.as_deref() {
        Some("morning") => Some(JournalBlock::Morning),
        Some("afternoon") => Some(JournalBlock::Afternoon),
        Some("evening") => Some(JournalBlock::Evening),
        _ => None,
    };
    let _ = open_journal(&cfg.output_folder, target_block, Some(&cfg.reminders));
}

#[tauri::command]
fn create_today_files() -> Result<(), String> {
    let cfg = AppConfig::load();
    create_today_all(&cfg.output_folder, Some(&cfg.reminders))
}

#[tauri::command]
fn open_folder() -> Result<(), String> {
    let cfg = AppConfig::load();
    open_journal_folder(&cfg.output_folder)
}

#[tauri::command]
fn pick_folder() -> Option<String> {
    let cfg = AppConfig::load();
    let res = rfd::FileDialog::new()
        .set_directory(&cfg.output_folder)
        .pick_folder();
    res.map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
fn set_autostart(enabled: bool) -> Result<(), String> {
    set_startup(enabled)
}

fn check_boundaries_background(app_handle: AppHandle, last_check: Arc<Mutex<Option<String>>>) {
    let now = Local::now();
    let minute_key = now.format("%Y-%m-%d %H:%M").to_string();

    {
        let mut check = last_check.lock().unwrap();
        if check.as_deref() == Some(&minute_key) {
            return;
        }
        *check = Some(minute_key);
    }

    let Ok(_guard) = DATA_LOCK.lock() else {
        return;
    };
    let mut cfg = AppConfig::load();
    let current_time_str = now.format("%H:%M").to_string();

    for block in cfg.blocks.clone() {
        let today = today_str();
        if block.reminder
            && current_time_str == block.end
            && cfg.last_reminded.get(&block.id) != Some(&today)
        {
            let saved = weekly::load(&cfg.output_folder, &today).unwrap_or_default();
            if saved
                .iter()
                .any(|e| e.date == today && e.block_id == block.id)
            {
                continue;
            }
            cfg.last_reminded.insert(block.id.clone(), today);
            cfg.save();
            play_reminder_sound();
            let _ = app_handle.emit("reminder-triggered", &block.id);
            if let Some(window) = app_handle.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }
    }
}

pub fn run() {
    let start_hidden = std::env::args().any(|arg| arg == "--autostart" || arg == "--minimized");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let config = AppConfig::load();
            if config.start_with_windows {
                let _ = set_startup(true);
            }

            if let Some(window) = app.get_webview_window("main") {
                let theme = if config.theme_mode == "light" {
                    tauri::Theme::Light
                } else {
                    tauri::Theme::Dark
                };
                let _ = window.set_theme(Some(theme));
                if start_hidden {
                    let _ = window.hide();
                } else {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }

            // System Tray Setup
            let open_item = MenuItem::with_id(app, "open", "Open Journal", true, None::<&str>)?;
            let write_item =
                MenuItem::with_id(app, "write", "Write Current Block", true, None::<&str>)?;
            let folder_item =
                MenuItem::with_id(app, "folder", "Open Journal Folder", true, None::<&str>)?;
            let exit_item = MenuItem::with_id(app, "exit", "Exit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open_item, &write_item, &folder_item, &exit_item])?;

            let handle = app.handle().clone();
            let icon_bytes = include_bytes!("../icons/32x32.png");
            let img = image::load_from_memory(icon_bytes)
                .expect("Failed to load icon")
                .to_rgba8();
            let (width, height) = img.dimensions();
            let tray_icon = tauri::image::Image::new_owned(img.into_raw(), width, height);

            TrayIconBuilder::new()
                .icon(tray_icon)
                .tooltip(APP_NAME)
                .menu(&menu)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "open" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                    "write" => {
                        let cfg = AppConfig::load();
                        let time = Local::now().format("%H:%M").to_string();
                        let block = cfg
                            .blocks
                            .iter()
                            .find(|b| {
                                if b.start < b.end {
                                    time >= b.start && time < b.end
                                } else {
                                    time >= b.start || time < b.end
                                }
                            })
                            .unwrap_or(&cfg.blocks[0]);
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                        let _ = app.emit("reminder-triggered", &block.id);
                    }
                    "folder" => {
                        let cfg = AppConfig::load();
                        let _ = open_journal_folder(&cfg.output_folder);
                    }
                    "exit" => {
                        std::process::exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Background Timer
            let app_handle_clone = handle.clone();
            let last_check = Arc::new(Mutex::new(None));
            thread::spawn(move || loop {
                thread::sleep(Duration::from_secs(5));
                check_boundaries_background(app_handle_clone.clone(), last_check.clone());
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            hide_window,
            load_entries,
            save_entry,
            open_week,
            get_default_config,
            get_config,
            save_config,
            get_today_str,
            open_journal_file,
            create_today_files,
            open_folder,
            pick_folder,
            set_autostart
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
