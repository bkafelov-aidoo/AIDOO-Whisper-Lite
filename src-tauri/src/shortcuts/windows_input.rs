//! Physical Windows input only: no layout translation or AttachThreadInput in the hook.
//! The callback forwards events; Tauri operations run on the existing processor thread.
use super::InputEvent;
use std::{cell::RefCell, sync::mpsc::Sender};

thread_local! { static SENDER: RefCell<Option<Sender<InputEvent>>> = const { RefCell::new(None) }; }

#[repr(C)]
struct KeyboardEvent {
    vk: u32,
    scan: u32,
    flags: u32,
    time: u32,
    extra: usize,
}
#[repr(C)]
#[derive(Default)]
struct Message {
    window: isize,
    message: u32,
    wparam: usize,
    lparam: isize,
    time: u32,
    x: i32,
    y: i32,
    private: u32,
}
#[link(name = "user32")]
extern "system" {
    fn SetWindowsHookExW(
        id: i32,
        callback: unsafe extern "system" fn(i32, usize, isize) -> isize,
        module: isize,
        thread: u32,
    ) -> isize;
    fn CallNextHookEx(hook: isize, code: i32, param: usize, data: isize) -> isize;
    fn GetMessageW(message: *mut Message, window: isize, min: u32, max: u32) -> i32;
    fn TranslateMessage(message: *const Message) -> i32;
    fn DispatchMessageW(message: *const Message) -> isize;
    fn UnhookWindowsHookEx(hook: isize) -> i32;
}
#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(name: *const u16) -> isize;
}

unsafe extern "system" fn callback(code: i32, param: usize, data: isize) -> isize {
    if code == 0 && matches!(param, 0x100 | 0x101 | 0x104 | 0x105) {
        // SAFETY: WH_KEYBOARD_LL supplies KBDLLHOOKSTRUCT for the duration of this call.
        let event = unsafe { &*(data as *const KeyboardEvent) };
        if let Some(code) = key_code(event.vk, event.scan, event.flags & 1 != 0) {
            SENDER.with(|sender| {
                if let Some(sender) = sender.borrow().as_ref() {
                    let _ = sender.send(InputEvent::Key {
                        code,
                        pressed: matches!(param, 0x100 | 0x104),
                    });
                }
            });
        }
    }
    // SAFETY: pass the original callback parameters to the next hook without retaining pointers.
    unsafe { CallNextHookEx(0, code, param, data) }
}

pub(super) fn install(sender: Sender<InputEvent>) {
    std::thread::spawn(move || {
        SENDER.with(|slot| *slot.borrow_mut() = Some(sender));
        // SAFETY: callback and its thread-local sender outlive the hook. MSG has its Win32 ABI
        // layout and stays valid through the entire message pump; unhook before thread exit.
        unsafe {
            let hook = SetWindowsHookExW(13, callback, GetModuleHandleW(std::ptr::null()), 0);
            if hook == 0 {
                eprintln!("Could not install Windows keyboard hook");
                return;
            }
            let mut message = Message::default();
            while GetMessageW(&mut message, 0, 0, 0) > 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            UnhookWindowsHookEx(hook);
        }
        SENDER.with(|slot| *slot.borrow_mut() = None);
    });
}

fn key_code(vk: u32, scan: u32, extended: bool) -> Option<&'static str> {
    Some(match vk {
        0x08 => "backspace",
        0x09 => "tab",
        0x0d if extended => "kp_return",
        0x0d => "return",
        0x10 if scan == 0x36 => "shift_right",
        0x10 => "shift_left",
        0x11 if extended => "control_right",
        0x11 => "control_left",
        0x12 if extended => "alt_gr",
        0x12 => "alt",
        0xa0 => "shift_left",
        0xa1 => "shift_right",
        0xa2 => "control_left",
        0xa3 => "control_right",
        0xa4 => "alt",
        0xa5 => "alt_gr",
        0x13 => "pause",
        0x14 => "caps_lock",
        0x1b => "escape",
        0x20 => "space",
        0x21 => "page_up",
        0x22 => "page_down",
        0x23 => "end",
        0x24 => "home",
        0x25 => "left",
        0x26 => "up",
        0x27 => "right",
        0x28 => "down",
        0x2c => "print_screen",
        0x2d => "insert",
        0x2e => "delete",
        0x30..=0x39 => [
            "num_0", "num_1", "num_2", "num_3", "num_4", "num_5", "num_6", "num_7", "num_8",
            "num_9",
        ][(vk - 0x30) as usize],
        0x41..=0x5a => [
            "key_a", "key_b", "key_c", "key_d", "key_e", "key_f", "key_g", "key_h", "key_i",
            "key_j", "key_k", "key_l", "key_m", "key_n", "key_o", "key_p", "key_q", "key_r",
            "key_s", "key_t", "key_u", "key_v", "key_w", "key_x", "key_y", "key_z",
        ][(vk - 0x41) as usize],
        0x5b => "meta_left",
        0x5c => "meta_right",
        0x60..=0x69 => [
            "kp_0", "kp_1", "kp_2", "kp_3", "kp_4", "kp_5", "kp_6", "kp_7", "kp_8", "kp_9",
        ][(vk - 0x60) as usize],
        0x6a => "kp_multiply",
        0x6b => "kp_plus",
        0x6d => "kp_minus",
        0x6e => "kp_delete",
        0x6f => "kp_divide",
        0x70..=0x7b => [
            "f1", "f2", "f3", "f4", "f5", "f6", "f7", "f8", "f9", "f10", "f11", "f12",
        ][(vk - 0x70) as usize],
        0x90 => "num_lock",
        0x91 => "scroll_lock",
        0xba => "semicolon",
        0xbb => "equal",
        0xbc => "comma",
        0xbd => "minus",
        0xbe => "dot",
        0xbf => "slash",
        0xc0 => "back_quote",
        0xdb => "left_bracket",
        0xdc => "backslash",
        0xdd => "right_bracket",
        0xde => "quote",
        0xe2 => "intl_backslash",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn physical_keys_do_not_depend_on_keyboard_layout() {
        assert_eq!(key_code(0x77, 0x42, false), Some("f8"));
        assert_eq!(key_code(0x11, 0x1d, true), Some("control_right"));
        assert_eq!(key_code(0x11, 0x1d, false), Some("control_left"));
        assert_eq!(key_code(0x60, 0x52, false), Some("kp_0"));
        assert_eq!(key_code(0x2d, 0x52, false), Some("insert"));
        assert_eq!(key_code(0x0d, 0x1c, true), Some("kp_return"));
    }
}
