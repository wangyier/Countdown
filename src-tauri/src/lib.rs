mod calculator;
mod clock;
mod config;
mod desktop;
mod position;

use calculator::{CountdownStats, calculate};
use chrono::Local;
use config::WorkConfig;
use serde::Serialize;
use std::{net::IpAddr, path::PathBuf, sync::Mutex};
use tauri::{Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

struct Runtime {
    config: WorkConfig,
    warnings: Vec<String>,
}

struct AppState {
    runtime: Mutex<Runtime>,
    directory: PathBuf,
}

#[derive(Serialize)]
struct AppInfo {
    local_ip_suffix: Option<String>,
    warnings: Vec<String>,
}

#[tauri::command]
fn get_config(state: tauri::State<'_, AppState>) -> Result<WorkConfig, String> {
    Ok(state
        .runtime
        .lock()
        .map_err(|e| e.to_string())?
        .config
        .clone())
}

fn autostart(app: &tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    result.map_err(|e| format!("开机自启动设置失败：{e}"))
}

fn apply_config(
    app: &tauri::AppHandle,
    state: &AppState,
    config: WorkConfig,
) -> Result<WorkConfig, String> {
    config.times()?;
    let mut runtime = state.runtime.lock().map_err(|e| e.to_string())?;
    let window = app.get_webview_window("main").ok_or("窗口未就绪")?;
    let old_autostart = app.autolaunch().is_enabled().map_err(|e| e.to_string())?;
    let old_position = window.outer_position().map_err(|e| e.to_string())?;
    let old_size = window.outer_size().map_err(|e| e.to_string())?;
    autostart(app, config.autostart_enabled)?;
    let result = window
        .set_always_on_top(config.always_on_top)
        .map_err(|e| e.to_string())
        .and_then(|_| {
            if config.mini_mode != runtime.config.mini_mode {
                position::resize(&window, config.mini_mode).map_err(|e| e.to_string())
            } else {
                Ok(())
            }
        })
        .and_then(|_| config::write_json(&state.directory.join("config.json"), &config));
    if let Err(error) = result {
        let rollback_autostart = autostart(app, old_autostart);
        let rollback_pin = window.set_always_on_top(runtime.config.always_on_top);
        let rollback_size = window.set_size(old_size);
        let rollback_position = window.set_position(old_position);
        return Err(format!(
            "保存失败：{error}{}",
            if rollback_autostart.is_err()
                || rollback_pin.is_err()
                || rollback_size.is_err()
                || rollback_position.is_err()
            {
                "；系统设置回退失败，请重新检查设置"
            } else {
                ""
            }
        ));
    }
    runtime.config = config.clone();
    runtime
        .warnings
        .retain(|warning| !warning.starts_with("配置") && !warning.starts_with("开机自启动"));
    drop(runtime);
    desktop::update_tooltip(app, state);
    let _ = app.emit("config-changed", ());
    Ok(config)
}

#[tauri::command]
fn save_config(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    config: WorkConfig,
) -> Result<WorkConfig, String> {
    apply_config(&app, &state, config)
}

#[tauri::command]
fn get_countdown_stats(state: tauri::State<'_, AppState>) -> Result<CountdownStats, String> {
    let config = state
        .runtime
        .lock()
        .map_err(|e| e.to_string())?
        .config
        .clone();
    calculate(&config, Local::now().naive_local())
}

#[tauri::command]
fn get_app_info(state: tauri::State<'_, AppState>) -> Result<AppInfo, String> {
    Ok(AppInfo {
        local_ip_suffix: local_ip_suffix(),
        warnings: state
            .runtime
            .lock()
            .map_err(|e| e.to_string())?
            .warnings
            .clone(),
    })
}

fn local_ip_suffix() -> Option<String> {
    use std::{
        sync::OnceLock,
        time::{Duration, Instant},
    };
    type Cache = Option<(Instant, Option<String>)>;
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    let mut cache = CACHE.get_or_init(|| Mutex::new(None)).lock().ok()?;
    if let Some((updated, value)) = &*cache {
        if updated.elapsed() < Duration::from_secs(30) {
            return value.clone();
        }
    }
    // Prefer the requested LAN subnet, then other private IPv4 addresses.
    let mut candidates: Vec<_> = local_ip_address::list_afinet_netifas()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(name, ip)| {
            let IpAddr::V4(ip) = ip else {
                return None;
            };
            if !ip.is_private() || ip.is_loopback() {
                return None;
            }
            let bytes = ip.octets();
            let subnet = if bytes[..3] == [192, 168, 0] {
                0
            } else if bytes[..2] == [192, 168] {
                1
            } else {
                2
            };
            let name = name.to_lowercase();
            let virtual_adapter = ["vpn", "virtual", "wsl", "tun", "tap"]
                .iter()
                .any(|part| name.contains(part));
            Some(((subnet, virtual_adapter, bytes), bytes[3].to_string()))
        })
        .collect();
    candidates.sort_by_key(|(key, _)| *key);
    let suffix = candidates.first().map(|(_, suffix)| suffix.clone());
    *cache = Some((Instant::now(), suffix.clone()));
    suffix
}

#[tauri::command]
fn hide_window(window: tauri::WebviewWindow) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}

#[tauri::command]
fn toggle_pin(app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let mut config = state
        .runtime
        .lock()
        .map_err(|e| e.to_string())?
        .config
        .clone();
    config.always_on_top = !config.always_on_top;
    Ok(apply_config(&app, &state, config)?.always_on_top)
}

#[tauri::command]
fn set_mini_mode(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    enabled: bool,
) -> Result<WorkConfig, String> {
    let mut config = state
        .runtime
        .lock()
        .map_err(|e| e.to_string())?
        .config
        .clone();
    config.mini_mode = enabled;
    apply_config(&app, &state, config)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            desktop::show_window(app)
        }))
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let directory = app.path().app_local_data_dir()?;
            let mut warnings = Vec::new();
            let config = match config::load(&directory.join("config.json")) {
                Ok(config) => config,
                Err(error) => {
                    warnings.push(format!(
                        "配置读取失败：{error}；暂用默认设置，保存前请备份原文件"
                    ));
                    WorkConfig::default()
                }
            };
            if let Err(error) = autostart(app.handle(), config.autostart_enabled) {
                warnings.push(error);
            }
            if let Some(window) = app.get_webview_window("main") {
                window.set_always_on_top(config.always_on_top)?;
                window.set_size(position::mode_size(config.mini_mode))?;
                position::restore(&window, &directory)?;
            }
            app.manage(AppState {
                runtime: Mutex::new(Runtime { config, warnings }),
                directory,
            });
            desktop::setup_tray(app)?;
            desktop::start_clock(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Moved(point) = event {
                if let Some(state) = window.app_handle().try_state::<AppState>() {
                    let saved = position::SavedPosition {
                        x: point.x,
                        y: point.y,
                    };
                    if let Err(error) =
                        config::write_json(&state.directory.join("window-position.json"), &saved)
                    {
                        eprintln!("窗口位置保存失败：{error}");
                    }
                }
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            get_countdown_stats,
            get_app_info,
            hide_window,
            toggle_pin,
            set_mini_mode
        ])
        .run(tauri::generate_context!())
        .expect("无法启动下班倒计时");
}
