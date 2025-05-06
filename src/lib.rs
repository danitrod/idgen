use rodio::{Decoder, OutputStream, Sink};
use std::{
    error::Error,
    fs::File,
    io::BufReader,
    str::FromStr,
    sync::{Arc, Mutex},
    thread,
};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    path::BaseDirectory,
    tray::TrayIconBuilder,
    AppHandle, Manager, Runtime, WebviewWindowBuilder, Wry,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tauri_plugin_store::StoreExt;
use uuid::Uuid;

mod hotkeys;

const STORE_FILE: &str = "store.json";

// Settings keys in store
const AUTOSTART_KEY: &str = "autostart_enabled";
const HOTKEY_MODIFIERS_KEY: &str = "hotkey_modifiers";
const HOTKEY_CODE_KEY: &str = "hotkey_code";
const PLAY_SOUND_KEY: &str = "play_sound";

const DEFAULT_MODIFIERS: u32 = Modifiers::META.bits() | Modifiers::SHIFT.bits();
const DEFAULT_KEY: &str = "KeyK";

struct AppState {
    is_recording_hotkey: Arc<Mutex<bool>>,
    current_hotkey_item: Arc<Mutex<(u32, String)>>,
    hotkey_info_menu_item: Arc<Mutex<Option<MenuItem<Wry>>>>,
    play_sound: Arc<Mutex<bool>>,
}

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
        .invoke_handler(tauri::generate_handler![hotkey_selected,])
        .manage(AppState {
            is_recording_hotkey: Arc::new(Mutex::new(false)),
            current_hotkey_item: Arc::new(Mutex::new((DEFAULT_MODIFIERS, DEFAULT_KEY.to_string()))),
            hotkey_info_menu_item: Arc::new(Mutex::new(None)),
            play_sound: Arc::new(Mutex::new(true)),
        })
        .setup(|app| {
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let store = app.store(STORE_FILE)?;
            let autostart_enabled = if let Some(val) = store.get(AUTOSTART_KEY) {
                val.as_bool().unwrap_or(false)
            } else {
                store.set(AUTOSTART_KEY, false);
                false
            };

            let play_sound = if let Some(val) = store.get(PLAY_SOUND_KEY) {
                val.as_bool().unwrap_or(true)
            } else {
                store.set(PLAY_SOUND_KEY, true);
                true
            };

            let app_state = app.state::<AppState>();
            let mut sound_state = app_state.play_sound.lock().unwrap();
            *sound_state = play_sound;

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

            let app_info = MenuItem::with_id(
                app,
                "info",
                format!("keyclip - Version {}", app.package_info().version),
                false,
                None::<&str>,
            )?;
            let hotkey_info = MenuItem::with_id(
                app,
                "hotkey_info",
                format!("Hotkey: {}", hotkeys::format_hotkey(modifiers, &code)),
                false,
                None::<&str>,
            )?;
            let change_hotkey =
                MenuItem::with_id(app, "change_hotkey", "Change hotkey...", true, None::<&str>)?;
            let autostart_menu_item = CheckMenuItem::with_id(
                app,
                "toggle_autostart",
                "Start on Login",
                true,
                autostart_enabled,
                None::<&str>,
            )?;
            let play_sound_menu_item = CheckMenuItem::with_id(
                app,
                "toggle_play_sound",
                "Play sound on clip",
                true,
                play_sound,
                None::<&str>,
            )?;
            let menu = Menu::with_items(
                app,
                &[
                    &app_info,
                    &hotkey_info,
                    &PredefinedMenuItem::separator(app).unwrap(),
                    &change_hotkey,
                    &autostart_menu_item,
                    &play_sound_menu_item,
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
                    "change_hotkey" => {
                        let state = app.state::<AppState>();
                        *state.is_recording_hotkey.lock().unwrap() = true;
                        WebviewWindowBuilder::from_config(
                            app,
                            &app.config().app.windows.get(1).unwrap().clone(),
                        )
                        .unwrap()
                        .build()
                        .unwrap();
                    }
                    "toggle_play_sound" => {
                        let app_state = app.state::<AppState>();
                        let mut sound_state = app_state.play_sound.lock().unwrap();
                        *sound_state = !*sound_state;
                        store.set(PLAY_SOUND_KEY, *sound_state);
                        let _ = play_sound_menu_item.set_checked(*sound_state);
                    }
                    _ => {
                        println!("Unknown menu item clicked");
                    }
                })
                .build(app)?;

            let state = app.state::<AppState>();
            let mut state_hotkey = state.current_hotkey_item.lock().unwrap();
            *state_hotkey = (modifiers.bits(), code.to_string());
            let mut state_hotkey_info = state.hotkey_info_menu_item.lock().unwrap();
            *state_hotkey_info = Some(hotkey_info);

            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(move |app, shortcut, event| {
                        let state = app.state::<AppState>();
                        if *state.is_recording_hotkey.lock().unwrap() {
                            return;
                        }

                        let hotkey = state.current_hotkey_item.lock().unwrap();
                        let hotkey = Shortcut::new(
                            Modifiers::from_bits(hotkey.0),
                            Code::from_str(&hotkey.1).expect("Invalid key"),
                        );

                        if shortcut == &hotkey && event.state() == ShortcutState::Pressed {
                            log::info!("Clip hotkey pressed: {:?}", hotkey);
                            let uuid = Uuid::new_v4();
                            app.clipboard().write_text(uuid.to_string()).unwrap();
                            if *state.play_sound.lock().unwrap() {
                                let _ = play_clip_sound(app);
                            }
                        }
                    })
                    .build(),
            )?;

            app.global_shortcut()
                .register(Shortcut::new(Some(modifiers), code))?;

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

fn play_clip_sound(app: &tauri::AppHandle) -> Result<(), Box<dyn Error>> {
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

#[tauri::command]
fn hotkey_selected(app: AppHandle, modifiers: ModifiersInput, code: String) {
    let modifiers = Modifiers::from_bits(
        (if modifiers.meta {
            Modifiers::META.bits()
        } else {
            0
        }) | (if modifiers.shift {
            Modifiers::SHIFT.bits()
        } else {
            0
        }) | (if modifiers.alt {
            Modifiers::ALT.bits()
        } else {
            0
        }) | (if modifiers.control {
            Modifiers::CONTROL.bits()
        } else {
            0
        }),
    )
    .ok_or(Modifiers::empty())
    .unwrap();

    let code = Code::from_str(&code).unwrap_or_else(|_| {
        log::warn!("Tried to parse invalid key: {}. Using default", code);
        Code::KeyK
    });

    hotkeys::change_hotkey(&app, modifiers, code);
}

#[derive(Debug, serde::Deserialize)]
struct ModifiersInput {
    meta: bool,
    shift: bool,
    alt: bool,
    control: bool,
}
