//! Windows messages that winit and egui-winit get wrong.
//!
//! - A window whose DPI changes goes where Windows suggests (`follow_dpi_changes`).
//!
//! And two that winit's message hook (`hook`) sees first:
//!
//! - Ctrl+Alt types what AltGr types (`@ € { [ ] } \ ~ |` on a German keyboard), as everywhere in Windows. egui-winit
//!   drops text typed while Ctrl is held, so that Ctrl+C doesn't type a "c"; here Windows says which character the
//!   key makes, and the window that has the focus types it (`type_ctrl_alt`).
//! - A late `WM_MOUSELEAVE` while a button is held makes egui-winit forget where the mouse is, and then drop the
//!   button's release: a tab dragged out of the window would never be let go of. The window keeps getting the
//!   mouse's moves anyway, since it has the mouse captured.

use std::sync::Mutex;

use egui::{Context, Event};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, GetKeyboardLayout, GetKeyboardState, ToUnicodeEx, VK_CONTROL, VK_LBUTTON, VK_LMENU,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CWPRETSTRUCT, CWPSTRUCT, CallNextHookEx, GetClassNameW, MSG, SWP_NOACTIVATE, SWP_NOZORDER,
    SetWindowPos, SetWindowsHookExW, WH_CALLWNDPROC, WH_CALLWNDPROCRET, WM_DPICHANGED, WM_KEYDOWN,
    WM_SYSKEYDOWN,
};

/// In `Win32::UI::Controls`, which windows-sys builds only for that feature.
const WM_MOUSELEAVE: u32 = 0x02A3;

static TYPED: Mutex<String> = Mutex::new(String::new());

/// Call once, on the thread that runs the event loop.
///
/// When Windows moves a window to a monitor with another DPI itself (snapping, Win+arrow), it says so before the
/// move. winit then nudges the window back onto the monitor it is still on, and the window jumps between the two.
/// So it goes where Windows suggests, before winit handles the change (so winit finds it on the right monitor) and
/// after (winit resizes it its own way).
pub fn follow_dpi_changes() {
    // winit's windows (by its default class name), not the file dialogs, which run on the same thread.
    fn ours(window: HWND) -> bool {
        let mut class = [0u16; 16];
        let n = unsafe { GetClassNameW(window, class.as_mut_ptr(), class.len() as i32) };
        String::from_utf16_lossy(&class[..n.max(0) as usize]) == "Window Class"
    }
    fn place(window: HWND, suggested: LPARAM) {
        // Safety: WM_DPICHANGED's lParam is the suggested rect, valid while the message is handled.
        let r = unsafe { &*(suggested as *const RECT) };
        let (w, h) = (r.right - r.left, r.bottom - r.top);
        let flags = SWP_NOZORDER | SWP_NOACTIVATE;
        unsafe { SetWindowPos(window, std::ptr::null_mut(), r.left, r.top, w, h, flags) };
    }
    unsafe extern "system" fn before(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
        let m = unsafe { &*(l as *const CWPSTRUCT) };
        if code >= 0 && m.message == WM_DPICHANGED && ours(m.hwnd) {
            place(m.hwnd, m.lParam);
        }
        unsafe { CallNextHookEx(std::ptr::null_mut(), code, w, l) }
    }
    unsafe extern "system" fn after(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
        let m = unsafe { &*(l as *const CWPRETSTRUCT) };
        if code >= 0 && m.message == WM_DPICHANGED && ours(m.hwnd) {
            place(m.hwnd, m.lParam);
        }
        unsafe { CallNextHookEx(std::ptr::null_mut(), code, w, l) }
    }
    unsafe {
        let thread = GetCurrentThreadId();
        SetWindowsHookExW(WH_CALLWNDPROC, Some(before), std::ptr::null_mut(), thread);
        SetWindowsHookExW(WH_CALLWNDPROCRET, Some(after), std::ptr::null_mut(), thread);
    }
}

/// For winit's message hook: true keeps the message from winit.
pub fn hook(msg: *const std::ffi::c_void) -> bool {
    // Safety: winit hands over the message it is about to dispatch.
    let msg = unsafe { &*(msg as *const MSG) };
    let down = |key: u16| unsafe { GetKeyState(key as i32) } < 0;
    if msg.message == WM_MOUSELEAVE {
        return down(VK_LBUTTON);
    }
    // The right Alt is AltGr, which winit already types.
    if !matches!(msg.message, WM_KEYDOWN | WM_SYSKEYDOWN) || !down(VK_CONTROL) || !down(VK_LMENU) {
        return false;
    }
    let mut keys = [0u8; 256];
    let mut out = [0u16; 8];
    let n = unsafe {
        GetKeyboardState(keys.as_mut_ptr());
        // Flag 4: leaves the keyboard's state alone, so a pending dead key still works.
        ToUnicodeEx(
            msg.wParam as u32,
            ((msg.lParam >> 16) & 0xff) as u32,
            keys.as_ptr(),
            out.as_mut_ptr(),
            out.len() as i32,
            4,
            GetKeyboardLayout(0),
        )
    };
    if let Ok(text) = String::from_utf16(&out[..n.max(0) as usize])
        && !text.chars().any(char::is_control)
        && let Ok(mut typed) = TYPED.lock()
    {
        typed.push_str(&text);
    }
    false
}

/// Call at the start of each window's frame. Removes Ctrl+Alt key presses, so they don't run shortcuts
/// (egui quits on Ctrl+Q, Ctrl+Alt+Q types `@`), and types what they made in the focused window.
pub fn type_ctrl_alt(ctx: &Context) {
    let is_ctrl_alt = |e: &Event| matches!(e, Event::Key { pressed: true, modifiers, .. } if modifiers.ctrl && modifiers.alt);
    ctx.input_mut(|i| {
        let at = i.events.iter().position(is_ctrl_alt);
        i.events.retain(|e| !is_ctrl_alt(e));
        if !i.focused {
            return;
        }
        // Taken even without a key press here, so nothing is left over for the next one.
        let text = TYPED
            .lock()
            .map(|mut t| std::mem::take(&mut *t))
            .unwrap_or_default();
        if let Some(at) = at
            && !text.is_empty()
        {
            i.events.insert(at, Event::Text(text));
        }
    });
}
