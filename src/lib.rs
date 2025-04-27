use rodio::{Decoder, OutputStream, Sink};
use std::{error::Error, fs::File, io::BufReader, str::FromStr, thread};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    path::BaseDirectory,
    tray::TrayIconBuilder,
    Manager, Runtime,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tauri_plugin_store::StoreExt;
use uuid::Uuid;

const STORE_FILE: &str = "store.json";

// Settings keys in store
const AUTOSTART_KEY: &str = "autostart_enabled";
const HOTKEY_MODIFIERS_KEY: &str = "hotkey_modifiers";
const HOTKEY_CODE_KEY: &str = "hotkey_code";

const DEFAULT_MODIFIERS: u32 = Modifiers::META.bits() | Modifiers::SHIFT.bits();
const DEFAULT_KEY: &str = "KeyK";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::default(), None))
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let store = app.store(STORE_FILE)?;
            let autostart_enabled = if let Some(val) = store.get(AUTOSTART_KEY) {
                val.as_bool().unwrap_or(false)
            } else {
                store.set(AUTOSTART_KEY, false);
                false
            };

            let info = MenuItem::with_id(
                app,
                "info",
                format!("keyclip - Version {}", app.package_info().version),
                false,
                None::<&str>,
            )?;
            let autostart_menu_item = CheckMenuItem::with_id(
                app,
                "toggle_autostart",
                "Start on Login",
                true,
                autostart_enabled,
                None::<&str>,
            )?;
            let menu = Menu::with_items(
                app,
                &[
                    &info,
                    &PredefinedMenuItem::separator(app).unwrap(),
                    &autostart_menu_item,
                    &PredefinedMenuItem::quit(app, Some("Quit")).unwrap(),
                ],
            )?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(true)
                .tooltip("keyclip")
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "toggle_autostart" => {
                        let _ = toggle_autostart(app, &autostart_menu_item);
                    }
                    _ => {
                        println!("Unknown menu item clicked");
                    }
                })
                .build(app)?;
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let modifiers = store
                .get(HOTKEY_MODIFIERS_KEY)
                .and_then(|v| v.as_i64())
                .map(|m| Modifiers::from_bits(m as u32).unwrap())
                .unwrap_or_else(|| {
                    log::info!("No stored modifier preference found, using default");
                    Modifiers::from_bits(DEFAULT_MODIFIERS).expect("Invalid default modifiers")
                });

            let code = store
                .get(HOTKEY_CODE_KEY)
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| {
                    log::info!("No stored key preference found, using default");
                    DEFAULT_KEY.to_string()
                });

            let code = Code::from_str(&code).unwrap_or_else(|_| {
                log::warn!("Tried to parse invalid key: {}. Using default", code);
                Code::KeyK
            });

            let clip_shortcut = Shortcut::new(Some(modifiers), code);

            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(move |app, shortcut, event| {
                        if shortcut == &clip_shortcut && event.state() == ShortcutState::Pressed {
                            log::info!("Clip shortcut pressed: {:?}", clip_shortcut);
                            let uuid = Uuid::new_v4();
                            app.clipboard().write_text(uuid.to_string()).unwrap();
                            let _ = play_notification(app);
                        }
                    })
                    .build(),
            )?;

            app.global_shortcut().register(clip_shortcut)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn toggle_autostart<R: Runtime>(
    app: &tauri::AppHandle,
    menu_item: &CheckMenuItem<R>,
) -> Result<(), String> {
    let store = app.store(STORE_FILE).unwrap();
    let autostart_manager = app.autolaunch();

    let autostart_enabled = if let Some(val) = store.get(AUTOSTART_KEY) {
        val.as_bool().unwrap_or(false)
    } else {
        store.set(AUTOSTART_KEY, false);
        false
    };

    if autostart_enabled {
        let _ = autostart_manager.disable();
    } else {
        let _ = autostart_manager.enable();
    }
    store.set(AUTOSTART_KEY, !autostart_enabled);
    let _ = menu_item.set_checked(!autostart_enabled);
    Ok(())
}

fn play_notification(app: &tauri::AppHandle) -> Result<(), Box<dyn Error>> {
    let path = app
        .path()
        .resolve("assets/notification.mp3", BaseDirectory::Resource)?;

    thread::spawn(move || {
        let (_stream, handle) = OutputStream::try_default().unwrap();

        let sink = Sink::try_new(&handle).unwrap();

        let file = File::open(path).unwrap();
        sink.append(Decoder::new(BufReader::new(file)).unwrap());

        sink.sleep_until_end();
    });

    Ok(())
}
