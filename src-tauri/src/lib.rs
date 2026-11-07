pub mod analysis;
pub mod aramhex;
pub mod augments;
pub mod builds;
pub mod capture;
pub mod cards;
mod commands;
pub mod core;
#[cfg(debug_assertions)]
mod dev;
pub mod dd;
pub mod game;
pub mod hotkeys;
pub mod lcu;
pub mod live;
pub mod model;
pub mod ocr;
pub mod opgg;
pub mod paths;
pub mod players;
pub mod positions;
pub mod riot;
pub mod score;
pub mod settings;
pub mod store;
pub mod update;
pub mod win;

use std::io::Write;
use tauri::menu::MenuBuilder;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, RunEvent};

/// Minimal file logger: %APPDATA%/Mattofessor/app.log, rotated at 5 MB.
struct FileLogger {
    file: parking_lot::Mutex<Option<std::fs::File>>,
}

impl log::Log for FileLogger {
    fn enabled(&self, m: &log::Metadata) -> bool {
        m.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = format!("{} {:5} {}: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), record.level(), record.target(), record.args());
        if cfg!(debug_assertions) {
            eprintln!("{line}");
        }
        if let Some(f) = self.file.lock().as_mut() {
            let _ = writeln!(f, "{line}");
        }
    }

    fn flush(&self) {}
}

fn init_logging() {
    let path = paths::config_dir().join("app.log");
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > 5 * 1024 * 1024) {
        let _ = std::fs::rename(&path, path.with_extension("log.old"));
    }
    let file = std::fs::OpenOptions::new().create(true).append(true).open(path).ok();
    let logger = Box::leak(Box::new(FileLogger { file: parking_lot::Mutex::new(file) }));
    let _ = log::set_logger(logger);
    log::set_max_level(log::LevelFilter::Info);
}

pub fn run() {
    init_logging();
    // After an update the installer restarts us with the old arguments; show the window anyway.
    let just_updated = update::take_marker();
    let started_minimized = std::env::args().any(|a| a == "--minimized") && !just_updated;

    let builder = tauri::Builder::default();
    // One copy at a time; a dev build may run beside the installed app.
    #[cfg(not(debug_assertions))]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {
        if let Some(core) = core::get() {
            core.show_main(None);
        }
    }));
    let app = builder
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--minimized"])))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(move |app| {
            riot::load();
            let core = core::Core::init(app.handle().clone());
            app.manage(core.clone());

            // Keep the Windows startup entry pointing at this exe. Dev builds never touch it.
            #[cfg(not(debug_assertions))]
            {
                use tauri_plugin_autostart::ManagerExt;
                let al = app.autolaunch();
                let result = if core.settings().start_with_windows { al.enable() } else { al.disable() };
                if let Err(e) = result {
                    log::warn!("could not update start-with-Windows entry: {e}");
                }
            }
            if !settings::Settings::path().exists() {
                let _ = core.settings().save();
            }

            let menu = MenuBuilder::new(app).text("open", "Open Mattofessor").separator().text("quit", "Quit").build()?;
            TrayIconBuilder::with_id("tray")
                .icon(app.default_window_icon().cloned().expect("icon"))
                .tooltip("Mattofessor")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|_app, event| {
                    let Some(core) = core::get() else { return };
                    match event.id().as_ref() {
                        "open" => core.show_main(None),
                        "quit" => core.quit(),
                        _ => {}
                    }
                })
                .on_tray_icon_event(|_tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        if let Some(core) = core::get() {
                            core.show_main(None);
                        }
                    }
                })
                .build(app)?;

            #[cfg(debug_assertions)]
            dev::start(core.clone());
            game::spawn(core.clone());
            aramhex::spawn();
            cards::spawn(core.clone());
            update::spawn(core.clone());
            if !started_minimized {
                core.show_main(None);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap,
            commands::static_data,
            commands::profile,
            commands::profile_saved,
            commands::profile_quick,
            commands::search,
            commands::game,
            commands::game_scores,
            commands::player_scores,
            commands::live_state,
            commands::build,
            commands::rift_tiers,
            commands::aram_overview,
            commands::aram_champion,
            commands::aram_picks,
            commands::card_offer,
            commands::augment_names,
            commands::import_build,
            commands::save_settings,
            commands::toggle_overlay,
            commands::hide_overlay,
            commands::fit_overlay,
            commands::toggle_spells,
            commands::hide_spells,
            commands::fit_spells,
            commands::open_profile,
            commands::set_riot_key,
            commands::check_update,
            commands::install_update,
            commands::capture_info,
            commands::open_captures,
            commands::delete_captures,
            commands::quit_app,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Mattofessor");

    app.run(|_app, event| {
        // Closing every window keeps the app in the tray; only Quit exits.
        if let RunEvent::ExitRequested { api, code, .. } = event {
            let quitting = core::get().is_some_and(|c| c.is_quitting());
            if code.is_none() && !quitting {
                api.prevent_exit();
            }
        }
    });
}
