use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Runtime,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tauri_plugin_store::StoreExt;
use uuid::Uuid;

const AUTOSTART_KEY: &str = "autostart_enabled";
const STORE_FILE: &str = "store.json";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
            store.close_resource();

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

            let deafault_shortcut =
                Shortcut::new(Some(Modifiers::META | Modifiers::SHIFT), Code::KeyK);
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(move |app, shortcut, event| {
                        println!("{:?}", shortcut);
                        if shortcut == &deafault_shortcut && event.state() == ShortcutState::Pressed
                        {
                            let uuid = Uuid::new_v4();
                            app.clipboard().write_text(uuid.to_string()).unwrap();
                        }
                    })
                    .build(),
            )?;

            app.global_shortcut().register(deafault_shortcut)?;

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
