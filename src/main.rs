#![windows_subsystem = "windows"]

use std::sync::Arc;
use std::sync::RwLock;

mod actions;
mod config;
mod hook;
mod state;
mod tray;

use actions::{ActionExecutor, MappingTable};
use config::{AppConfig, Config, SharedConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let exe_dir = std::env::current_exe()?.parent().unwrap().to_path_buf();

    // Prevent multiple instances via named mutex
    {
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
        use windows::Win32::System::Threading::CreateMutexW;
        let mutex_name: Vec<u16> = "MyCapsLock_SingleInstance\0".encode_utf16().collect();
        unsafe {
            let _h = CreateMutexW(None, true, PCWSTR::from_raw(mutex_name.as_ptr()));
            if GetLastError() == ERROR_ALREADY_EXISTS {
                return Ok(());
            }
        }
    }

    let config_path = exe_dir.join("config.toml");

    if !config_path.exists() {
        Config::write_default(&config_path)?;
    }

    let config = Config::load(&config_path).unwrap_or_else(|_| {
        toml::from_str(Config::default_toml_content()).unwrap_or_else(|_| Config::default_config())
    });

    let shared_config: SharedConfig =
        Arc::new(RwLock::new(AppConfig::new(config, config_path.clone())));

    let (mapping, hold_threshold, tap_to_toggle) = {
        let cfg = shared_config.read().unwrap();
        let mapping = MappingTable::from_cursor_mappings(&cfg.config.mappings.cursor);
        (mapping, cfg.config.settings.hold_threshold_ms, cfg.config.settings.tap_to_toggle)
    };

    let (tx, rx) = std::sync::mpsc::sync_channel(256);

    let hook_thread = hook::run_hook_thread(tx, mapping, hold_threshold, tap_to_toggle);

    let executor = ActionExecutor::new()?;

    let mut tray_handle = tray::create_tray_window(config_path.clone())?;
    tray_handle.run_message_loop(rx, executor, shared_config.clone(), config_path);

    hook_thread.stop();

    Ok(())
}
