use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use uuid::Uuid;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let info = MenuItem::with_id(
                app,
                "info",
                format!("keyclip - Version {}", app.package_info().version),
                false,
                None::<&str>,
            )?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[&info, &PredefinedMenuItem::separator(app).unwrap(), &quit_i],
            )?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(true)
                .tooltip("keyclip")
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "quit" => {
                        println!("Quit menu item clicked");
                        app.exit(0);
                    }
                    _ => {
                        println!("Unknown menu item clicked");
                    }
                })
                .build(app)?;
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let deafault_shortcut =
                Shortcut::new(Some(Modifiers::META | Modifiers::SHIFT), Code::KeyU);
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
