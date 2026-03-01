//! Low-level keyboard hook (Windows only).
//!
//! Installs a WH_KEYBOARD_LL hook that intercepts and blocks key combinations
//! listed in the policy config (e.g. Alt+Tab, Win+D, Ctrl+Alt+Del).

use once_cell::sync::OnceCell;
use std::collections::HashSet;
use std::sync::Mutex;

use windows::Win32::Foundation::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

/// Global storage for the set of blocked key-combo hashes.
static BLOCKED_COMBOS: OnceCell<Mutex<HashSet<String>>> = OnceCell::new();

/// Normalise a set of key names into a canonical, sorted, lower-case string.
fn normalize(keys: &[&str]) -> String {
    let mut v: Vec<String> = keys.iter().map(|k| k.to_lowercase()).collect();
    v.sort();
    v.join("+")
}

/// Determine which modifier keys are currently held down.
fn active_modifiers() -> Vec<&'static str> {
    let mut mods = Vec::new();
    unsafe {
        if GetAsyncKeyState(VK_CONTROL.0 as i32) < 0 {
            mods.push("ctrl");
        }
        if GetAsyncKeyState(VK_MENU.0 as i32) < 0 {
            mods.push("alt");
        }
        if GetAsyncKeyState(VK_SHIFT.0 as i32) < 0 {
            mods.push("shift");
        }
        if GetAsyncKeyState(VK_LWIN.0 as i32) < 0 || GetAsyncKeyState(VK_RWIN.0 as i32) < 0 {
            mods.push("win");
        }
    }
    mods
}

/// Map a virtual-key code to a human-friendly name.
fn vk_name(vk: u32) -> Option<&'static str> {
    match VIRTUAL_KEY(vk as u16) {
        VK_TAB => Some("tab"),
        VK_ESCAPE => Some("escape"),
        VK_DELETE => Some("delete"),
        VK_F1 => Some("f1"),
        VK_F2 => Some("f2"),
        VK_F3 => Some("f3"),
        VK_F4 => Some("f4"),
        VK_F5 => Some("f5"),
        VK_F6 => Some("f6"),
        VK_F7 => Some("f7"),
        VK_F8 => Some("f8"),
        VK_F9 => Some("f9"),
        VK_F10 => Some("f10"),
        VK_F11 => Some("f11"),
        VK_F12 => Some("f12"),
        VK_LWIN | VK_RWIN => Some("win"),
        _ => {
            // A–Z
            if (0x41..=0x5A).contains(&vk) {
                // We'll return a static str for common letters
                match vk {
                    0x41 => Some("a"), 0x42 => Some("b"), 0x43 => Some("c"),
                    0x44 => Some("d"), 0x45 => Some("e"), 0x46 => Some("f"),
                    0x47 => Some("g"), 0x48 => Some("h"), 0x49 => Some("i"),
                    0x4A => Some("j"), 0x4B => Some("k"), 0x4C => Some("l"),
                    0x4D => Some("m"), 0x4E => Some("n"), 0x4F => Some("o"),
                    0x50 => Some("p"), 0x51 => Some("q"), 0x52 => Some("r"),
                    0x53 => Some("s"), 0x54 => Some("t"), 0x55 => Some("u"),
                    0x56 => Some("v"), 0x57 => Some("w"), 0x58 => Some("x"),
                    0x59 => Some("y"), 0x5A => Some("z"),
                    _ => None,
                }
            } else {
                None
            }
        }
    }
}

/// The LL keyboard hook callback.
unsafe extern "system" fn keyboard_proc(
    code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if code as u32 == HC_ACTION {
        let kb = &*(l_param.0 as *const KBDLLHOOKSTRUCT);
        let vk = kb.vkCode;

        // Build current combo
        let mut keys: Vec<&str> = active_modifiers();

        // Don't duplicate modifiers
        let is_modifier = matches!(
            VIRTUAL_KEY(vk as u16),
            VK_CONTROL | VK_LCONTROL | VK_RCONTROL |
            VK_MENU | VK_LMENU | VK_RMENU |
            VK_SHIFT | VK_LSHIFT | VK_RSHIFT |
            VK_LWIN | VK_RWIN
        );

        if !is_modifier {
            if let Some(name) = vk_name(vk) {
                if !keys.contains(&name) {
                    keys.push(name);
                }
            }
        }

        if !keys.is_empty() {
            let combo = normalize(&keys);
            if let Some(set) = BLOCKED_COMBOS.get() {
                if let Ok(lock) = set.lock() {
                    if lock.contains(&combo) {
                        log::warn!("[Security] Blocked keyboard combo: {}", combo);
                        return LRESULT(1); // swallow the key event
                    }
                }
            }
        }
    }

    CallNextHookEx(None, code, w_param, l_param)
}

/// Install the keyboard hook on a dedicated thread with a message loop.
pub fn start_keyboard_hook(combos: Vec<Vec<String>>) {
    // Build the blocked set
    let mut set = HashSet::new();
    for combo in &combos {
        let refs: Vec<&str> = combo.iter().map(|s| s.as_str()).collect();
        set.insert(normalize(&refs));
    }
    let _ = BLOCKED_COMBOS.set(Mutex::new(set));

    std::thread::spawn(|| {
        unsafe {
            let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), None, 0);
            match hook {
                Ok(_h) => {
                    log::info!("[Security] Low-level keyboard hook installed");
                    // Message loop – required for LL hooks
                    let mut msg = MSG::default();
                    while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
                Err(e) => {
                    log::error!("[Security] Failed to install keyboard hook: {:?}", e);
                }
            }
        }
    });
}
