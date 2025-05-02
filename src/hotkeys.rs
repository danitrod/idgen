use tauri_plugin_global_shortcut::{Code, Modifiers};

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

pub fn capture_hotkey(_app: &tauri::AppHandle) -> (Modifiers, Code) {
    // TODO: Capturing hotkeys is hard because we don't have an open window. It seems we will
    // need to open a hidden window to capture the hotkey.
    // Alternative was to temporarily open a listener for all shortcuts with global_shortcut, but
    // it doesn't have support for it (other than registering all combinations one by one).
    (Modifiers::META | Modifiers::SHIFT, Code::KeyK)
}
