use std::path::PathBuf;

use notify::{Event, EventKind, RecursiveMode, Watcher};
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_SZ,
};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::UI::Shell::*;

use crate::actions::{ActionExecutor, MappingTable};
use crate::config::SharedConfig;
use crate::state::HookEvent;

const WINDOW_CLASS: PCWSTR = w!("MyCapsLockTrayClass");
const WM_TRAYICON: u32 = WM_APP + 1;
const WM_OPEN_CONFIG: u32 = WM_APP + 2;
const WM_CHECK_CONFIG: u32 = WM_APP + 3;
const WM_TOGGLE_AUTOSTART: u32 = WM_APP + 4;
const ID_TRAY_EXIT: u16 = 1001;
const ID_TRAY_OPEN: u16 = 1002;
const ID_TRAY_CHECK: u16 = 1003;
const ID_TRAY_AUTOSTART: u16 = 1004;

const RUN_KEY: PCWSTR = w!("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run");
const VALUE_NAME: PCWSTR = w!("MyCapsLock");

fn is_autostart_enabled() -> bool {
    unsafe {
        let mut hkey = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, RUN_KEY, Some(0), KEY_READ, &mut hkey).is_ok() {
            let mut buf = [0u16; 520];
            let mut size = (buf.len() * 2) as u32;
            let result = RegQueryValueExW(
                hkey,
                VALUE_NAME,
                None,
                None,
                Some(buf.as_mut_ptr() as *mut u8),
                Some(&mut size),
            );
            let _ = RegCloseKey(hkey);
            return result.is_ok();
        }
    }
    false
}

fn set_autostart(enabled: bool) {
    unsafe {
        let exe_path = std::env::current_exe().unwrap();
        let exe_str: Vec<u16> = exe_path.to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect();
        let data = std::slice::from_raw_parts(exe_str.as_ptr() as *const u8, exe_str.len() * 2);

        let mut hkey = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, RUN_KEY, Some(0), KEY_WRITE, &mut hkey).is_ok() {
            if enabled {
                let _ = RegSetValueExW(hkey, VALUE_NAME, Some(0), REG_SZ, Some(data));
                log::info!("Auto-start enabled");
            } else {
                let _ = RegDeleteValueW(hkey, VALUE_NAME);
                log::info!("Auto-start disabled");
            }
            let _ = RegCloseKey(hkey);
        }
    }
}

pub struct TrayHandle {
    hwnd: HWND,
    config_path: PathBuf,
}

unsafe extern "system" fn tray_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_TRAYICON => {
            let lparam_u32 = lparam.0 as u32;
            if lparam_u32 == WM_RBUTTONUP || lparam_u32 == WM_CONTEXTMENU {
                unsafe { show_popup_menu(hwnd) };
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let cmd = (wparam.0 & 0xFFFF) as u16;
            if cmd == ID_TRAY_EXIT {
                unsafe {
                    destroy_tray_icon(hwnd);
                    let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
                }
            } else if cmd == ID_TRAY_OPEN {
                unsafe {
                    let _ = PostMessageW(Some(hwnd), WM_OPEN_CONFIG, WPARAM(0), LPARAM(0));
                }
            } else if cmd == ID_TRAY_CHECK {
                unsafe {
                    let _ = PostMessageW(Some(hwnd), WM_CHECK_CONFIG, WPARAM(0), LPARAM(0));
                }
            } else if cmd == ID_TRAY_AUTOSTART {
                unsafe {
                    let _ = PostMessageW(Some(hwnd), WM_TOGGLE_AUTOSTART, WPARAM(0), LPARAM(0));
                }
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

unsafe fn show_popup_menu(hwnd: HWND) {
    let menu = unsafe { CreatePopupMenu().unwrap() };
    let autostart = is_autostart_enabled();
    let check_flag = if autostart { MF_CHECKED } else { MENU_ITEM_FLAGS(0) };
    unsafe {
        let _ = AppendMenuW(menu, MENU_ITEM_FLAGS(0), ID_TRAY_OPEN as usize, w!("Open Config"));
        let _ = AppendMenuW(menu, MENU_ITEM_FLAGS(0), ID_TRAY_CHECK as usize, w!("Check Config"));
        let _ = AppendMenuW(menu, MENU_ITEM_FLAGS(0) | MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, check_flag, ID_TRAY_AUTOSTART as usize, w!("Auto Start"));
        let _ = AppendMenuW(menu, MENU_ITEM_FLAGS(0), ID_TRAY_EXIT as usize, w!("Exit"));
    }
    let mut pt = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut pt);
        let _ = SetForegroundWindow(hwnd);
        let _ = TrackPopupMenu(menu, TPM_BOTTOMALIGN | TPM_LEFTALIGN, pt.x, pt.y, Some(0), hwnd, None);
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
    }
}

unsafe fn add_tray_icon(hwnd: HWND) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let icon = unsafe { LoadIconW(None, IDI_APPLICATION)? };

    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
        uCallbackMessage: WM_TRAYICON,
        hIcon: HICON(icon.0),
        ..Default::default()
    };

    let tip: Vec<u16> = "MyCapsLock\0".encode_utf16().collect();
    let len = tip.len().min(128);
    nid.szTip[..len].copy_from_slice(&tip[..len]);

    if !unsafe { Shell_NotifyIconW(NIM_ADD, &nid).as_bool() } {
        return Err("Shell_NotifyIconW(NIM_ADD) failed".into());
    }
    Ok(())
}

unsafe fn destroy_tray_icon(hwnd: HWND) {
    let nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        ..Default::default()
    };
    unsafe {
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}

pub fn create_tray_window(config_path: PathBuf) -> std::result::Result<TrayHandle, Box<dyn std::error::Error>> {
    let hinstance = unsafe { GetModuleHandleW(None)? };
    let hinstance: HINSTANCE = hinstance.into();

    let wc = WNDCLASSW {
        lpfnWndProc: Some(tray_wnd_proc),
        hInstance: hinstance,
        lpszClassName: WINDOW_CLASS,
        ..Default::default()
    };

    if unsafe { RegisterClassW(&wc) } == 0 {
        return Err("RegisterClassW failed".into());
    }

    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            WINDOW_CLASS,
            w!("MyCapsLock"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(hinstance),
            None,
        )?
    };

    unsafe { add_tray_icon(hwnd)? };

    Ok(TrayHandle { hwnd, config_path })
}

impl TrayHandle {
    pub fn run_message_loop(
        &mut self,
        action_rx: std::sync::mpsc::Receiver<HookEvent>,
        mut action_executor: ActionExecutor,
        shared_config: SharedConfig,
        config_path: PathBuf,
    ) {
        // Set up file watcher for config hot reload
        // Watch parent directory (not the file itself) to survive editor save-as-replace
        let (watch_tx, watch_rx) = std::sync::mpsc::channel();
        let config_path_for_watch = config_path.clone();
        let _watcher = match notify::recommended_watcher(move |res: std::result::Result<Event, notify::Error>| {
            if let Ok(event) = res {
                // Only forward events that affect our config file
                if event.paths.iter().any(|p| *p == config_path_for_watch) {
                    let _ = watch_tx.send(event);
                }
            }
        }) {
            Ok(mut w) => {
                let parent = config_path.parent().unwrap_or(&config_path);
                if let Err(e) = w.watch(parent, RecursiveMode::NonRecursive) {
                    log::error!("Failed to watch config dir: {}", e);
                } else {
                    log::info!("Watching config: {}", config_path.display());
                }
                Some(w)
            }
            Err(e) => {
                log::error!("Failed to create file watcher: {}", e);
                None
            }
        };

        let mut msg = MSG::default();
        loop {
            // Process any pending window messages
            unsafe {
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    if msg.message == WM_QUIT {
                        destroy_tray_icon(self.hwnd);
                        let _ = DestroyWindow(self.hwnd);
                        return;
                    }
                    if msg.message == WM_OPEN_CONFIG {
                        let path: Vec<u16> = self.config_path
                            .to_string_lossy()
                            .encode_utf16()
                            .chain(std::iter::once(0))
                            .collect();
                        let _ = ShellExecuteW(
                            None,
                            w!("open"),
                            PCWSTR::from_raw(path.as_ptr()),
                            None,
                            None,
                            SW_SHOW,
                        );
                    }
                    if msg.message == WM_CHECK_CONFIG {
                        let result = check_config(&self.config_path);
                        let msg_text: Vec<u16> = result.encode_utf16().chain(std::iter::once(0)).collect();
                        let _ = MessageBoxW(
                            None,
                            PCWSTR::from_raw(msg_text.as_ptr()),
                            w!("Config Status"),
                            MB_OK,
                        );
                    }
                    if msg.message == WM_TOGGLE_AUTOSTART {
                        let current = is_autostart_enabled();
                        set_autostart(!current);
                    }
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }

            // Drain ALL pending actions
            loop {
                match action_rx.try_recv() {
                    Ok(HookEvent::Action(action)) => {
                        log::info!("Main => {:?}", action);
                        if let Err(e) = action_executor.execute(&action) {
                            log::error!("Failed to execute action: {}", e);
                        }
                    }
                    Ok(HookEvent::ToggleCapsLock) => {
                        log::info!("Main => ToggleCapsLock");
                        let _ = action_executor.execute(&crate::actions::Action::KeyPress(
                            enigo::Key::CapsLock,
                        ));
                    }
                    Ok(HookEvent::PassThrough | HookEvent::Block) => {}
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        log::error!("Hook thread disconnected unexpectedly");
                        return;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                }
            }

            // Check for config file changes (hot reload)
            while let Ok(event) = watch_rx.try_recv() {
                if matches!(event.kind, EventKind::Modify(_)) {
                    log::info!("Config file changed, reloading...");
                    reload_hook_from_config(&shared_config);
                }
            }

            // Wait up to 10ms for next window message
            unsafe {
                MsgWaitForMultipleObjects(None, false, 10, QS_ALLINPUT);
            }
        }
    }
}

fn check_config(path: &PathBuf) -> String {
    if !path.exists() {
        return format!("Config file not found:\n{}\n\nRun with default.toml to create one.", path.display());
    }
    match std::fs::read_to_string(path) {
        Err(e) => format!("Cannot read config:\n{}", e),
        Ok(content) => {
            match toml::from_str::<crate::config::Config>(&content) {
                Ok(cfg) => {
                    let mut lines = vec![
                        format!("Config OK: {}", path.display()),
                        String::new(),
                        format!("Settings:"),
                        format!("  hold_threshold_ms = {}ms", cfg.settings.hold_threshold_ms),
                        format!("  tap_to_toggle = {}", cfg.settings.tap_to_toggle),
                        String::new(),
                        format!("Mappings: {} key bindings loaded", cfg.mappings.cursor.len()),
                    ];
                    for (key, action) in &cfg.mappings.cursor {
                        lines.push(format!("  {} -> {}", key, action));
                    }
                    lines.join("\r\n")
                }
                Err(e) => format!("CONFIG PARSE ERROR:\n{}", e),
            }
        }
    }
}

fn reload_hook_from_config(shared_config: &SharedConfig) {
    if let Ok(mut app_cfg) = shared_config.write() {
        if let Err(e) = app_cfg.reload() {
            log::error!("Config reload failed: {}", e);
            return;
        }
        let mapping = MappingTable::from_cursor_mappings(&app_cfg.config.mappings.cursor);
        let threshold = app_cfg.config.settings.hold_threshold_ms;
        let toggle = app_cfg.config.settings.tap_to_toggle;
        crate::hook::update_hook_settings(threshold, toggle, mapping);
        log::info!("Config hot-reloaded successfully");
    }
}
