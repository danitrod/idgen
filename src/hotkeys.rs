use std::str::FromStr;
use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
use tauri_plugin_store::StoreExt;

pub fn format_hotkey(modifiers: Modifiers, code: &Code) -> String {
    let mut parts = Vec::new();

    if modifiers.contains(Modifiers::META) {
        parts.push("Cmd");
    }
    if modifiers.contains(Modifiers::SHIFT) {
        parts.push("Shift");
    }
    if modifiers.contains(Modifiers::ALT) {
        parts.push("Opt");
    }
    if modifiers.contains(Modifiers::CONTROL) {
        parts.push("Ctrl");
    }

    let key = code.to_string().replace("Key", "").replace("Digit", "");

    parts.push(&key);
    parts.join("+")
}

pub fn change_hotkey(app: &tauri::AppHandle, modifiers: Modifiers, code: Code) {
    let state = app.state::<crate::AppState>();
    let mut state_hotkey = state.current_hotkey_item.lock().unwrap();
    let shortcut = Shortcut::new(
        Modifiers::from_bits(state_hotkey.0),
        Code::from_str(&state_hotkey.1).expect("Invalid key"),
    );
    app.global_shortcut().unregister(shortcut).unwrap();

    let store = app.get_store(crate::STORE_FILE).unwrap();
    store.set(crate::HOTKEY_MODIFIERS_KEY, modifiers.bits());
    store.set(crate::HOTKEY_CODE_KEY, code.to_string());

    *state_hotkey = (modifiers.bits(), code.to_string());

    let mut state_hotkey_info = state.hotkey_info_menu_item.lock().unwrap();
    let menu_item = state_hotkey_info.as_mut().unwrap();
    let _ = menu_item.set_text(format!("Hotkey: {}", format_hotkey(modifiers, &code)));

    app.global_shortcut()
        .register(Shortcut::new(Some(modifiers), code))
        .unwrap();
    *state.is_recording_hotkey.lock().unwrap() = false;
    log::info!("Hotkey set to: {}", format_hotkey(modifiers, &code));
}
