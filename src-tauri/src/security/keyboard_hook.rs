//! Low-level keyboard hook (Windows only).
//!
//! Installs a WH_KEYBOARD_LL hook that intercepts and blocks key combinations
//! listed in the policy config (e.g. Alt+Tab, Win+D, Ctrl+Alt+Del).

use once_cell::sync::OnceCell;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use windows::Win32::Foundation::*;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

/// Global storage for the set of blocked key-combo hashes.
static BLOCKED_COMBOS: OnceCell<Mutex<HashSet<String>>> = OnceCell::new();
static HOOK_THREAD_ID: OnceCell<Mutex<Option<u32>>> = OnceCell::new();
static KEYBOARD_HOOK_RUNNING: AtomicBool = AtomicBool::new(false);
static LAST_BLOCKED_COMBO_MS: AtomicU64 = AtomicU64::new(0);
static LAST_BLOCKED_COMBO_NAME: OnceCell<Mutex<Option<String>>> = OnceCell::new();

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn mark_blocked_combo(combo: &str) {
    LAST_BLOCKED_COMBO_MS.store(now_ms(), Ordering::SeqCst);
    let slot = LAST_BLOCKED_COMBO_NAME.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = slot.lock() {
        *guard = Some(combo.to_string());
    }
}

/// Return the most recent blocked combo if it happened within `max_age_ms`.
pub fn take_recent_blocked_combo(max_age_ms: u64) -> Option<String> {
    let ts = LAST_BLOCKED_COMBO_MS.load(Ordering::SeqCst);
    if ts == 0 {
        return None;
    }

    let age = now_ms().saturating_sub(ts);
    if age > max_age_ms {
        return None;
    }

    let slot = LAST_BLOCKED_COMBO_NAME.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = slot.lock() {
        return guard.take();
    }

    None
}

/// Normalise a set of key names into a canonical, sorted, lower-case string.
fn normalize(keys: &[&str]) -> String {
    let mut v: Vec<String> = keys.iter().map(|k| k.to_lowercase()).collect();
    v.sort();
    v.join("+")
}

/// Determine which modifier keys are active for the current event.
fn active_modifiers_for_event(vk: u32, flags: u32) -> Vec<&'static str> {
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

    // Alt+Tab often depends on this event flag rather than async state timing.
    if (flags & 0x20) != 0 && !mods.contains(&"alt") {
        mods.push("alt");
    }

    // Include the current modifier key itself for reliable single-key combos (e.g. Win).
    match VIRTUAL_KEY(vk as u16) {
        VK_CONTROL | VK_LCONTROL | VK_RCONTROL if !mods.contains(&"ctrl") => mods.push("ctrl"),
        VK_MENU | VK_LMENU | VK_RMENU if !mods.contains(&"alt") => mods.push("alt"),
        VK_SHIFT | VK_LSHIFT | VK_RSHIFT if !mods.contains(&"shift") => mods.push("shift"),
        VK_LWIN | VK_RWIN if !mods.contains(&"win") => mods.push("win"),
        _ => {}
    }

    mods
}

/// Map a virtual-key code to a human-friendly name.
fn vk_name(vk: u32) -> Option<&'static str> {
    match VIRTUAL_KEY(vk as u16) {
        VK_TAB => Some("tab"),
        VK_ESCAPE => Some("escape"),
        VK_DELETE => Some("delete"),
        VK_INSERT => Some("insert"),
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
        let msg = w_param.0 as u32;
        let is_key_down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
        if !is_key_down {
            return CallNextHookEx(None, code, w_param, l_param);
        }

        let kb = &*(l_param.0 as *const KBDLLHOOKSTRUCT);
        let vk = kb.vkCode;
        let alt_down = (kb.flags.0 & 0x20) != 0 || GetAsyncKeyState(VK_MENU.0 as i32) < 0;
        let ctrl_down = GetAsyncKeyState(VK_CONTROL.0 as i32) < 0;
        let shift_down = GetAsyncKeyState(VK_SHIFT.0 as i32) < 0;
        let win_down = GetAsyncKeyState(VK_LWIN.0 as i32) < 0 || GetAsyncKeyState(VK_RWIN.0 as i32) < 0;

        // Fast-path for the most sensitive Windows escape combos.
        let explicit_combo = match VIRTUAL_KEY(vk as u16) {
            VK_TAB if alt_down => Some("alt+tab"),
            VK_F4 if alt_down => Some("alt+f4"),
            VK_ESCAPE if alt_down => Some("alt+escape"),
            VK_ESCAPE if ctrl_down && shift_down => Some("ctrl+shift+escape"),
            VK_ESCAPE if ctrl_down => Some("ctrl+escape"),
            VK_LWIN | VK_RWIN => Some("win"),
            _ if win_down && vk == 0x44 => Some("win+d"),
            _ if win_down && vk == 0x45 => Some("win+e"),
            _ if win_down && vk == 0x52 => Some("win+r"),
            _ if win_down && vk == 0x4C => Some("win+l"),
            VK_F11 => Some("f11"),
            VK_F12 => Some("f12"),
            _ if ctrl_down && shift_down && vk == 0x49 => Some("ctrl+shift+i"),
            _ => None,
        };

        if let Some(combo) = explicit_combo {
            mark_blocked_combo(combo);
            log::warn!("[Security] Explicitly blocked keyboard combo: {}", combo);
            return LRESULT(1);
        }

        // Build current combo
        let mut keys: Vec<&str> = active_modifiers_for_event(vk, kb.flags.0);

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
                        mark_blocked_combo(&combo);
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
    if KEYBOARD_HOOK_RUNNING.swap(true, Ordering::SeqCst) {
        log::warn!("[Security] Keyboard hook already running");
        return;
    }

    // Build the blocked set
    let mut set = HashSet::new();
    for combo in &combos {
        let refs: Vec<&str> = combo.iter().map(|s| s.as_str()).collect();
        set.insert(normalize(&refs));
    }
    if let Some(existing) = BLOCKED_COMBOS.get() {
        if let Ok(mut lock) = existing.lock() {
            *lock = set;
        }
    } else {
        let _ = BLOCKED_COMBOS.set(Mutex::new(set));
    }

    let thread_slot = HOOK_THREAD_ID.get_or_init(|| Mutex::new(None));

    std::thread::spawn(|| {
        unsafe {
            let tid = GetCurrentThreadId();
            if let Ok(mut slot) = thread_slot.lock() {
                *slot = Some(tid);
            }

            let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), None, 0);
            match hook {
                Ok(h) => {
                    log::info!("[Security] Low-level keyboard hook installed");
                    // Message loop – required for LL hooks
                    let mut msg = MSG::default();
                    while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }

                    let _ = UnhookWindowsHookEx(h);
                    KEYBOARD_HOOK_RUNNING.store(false, Ordering::SeqCst);
                    if let Ok(mut slot) = thread_slot.lock() {
                        *slot = None;
                    }
                    log::info!("[Security] Keyboard hook stopped");
                }
                Err(e) => {
                    KEYBOARD_HOOK_RUNNING.store(false, Ordering::SeqCst);
                    if let Ok(mut slot) = thread_slot.lock() {
                        *slot = None;
                    }
                    log::error!("[Security] Failed to install keyboard hook: {:?}", e);
                }
            }
        }
    });
}

pub fn stop_keyboard_hook() {
    if !KEYBOARD_HOOK_RUNNING.load(Ordering::SeqCst) {
        return;
    }

    if let Some(slot) = HOOK_THREAD_ID.get() {
        if let Ok(lock) = slot.lock() {
            if let Some(tid) = *lock {
                unsafe {
                    let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
                }
                log::info!("[Security] Keyboard hook stop requested");
            }
        }
    }
}
