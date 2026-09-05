use super::*;

#[tauri::command]
pub(crate) fn get_core_status(
    process_state: tauri::State<'_, CoreProcessState>,
    gui_config_state: tauri::State<'_, GuiConfigState>,
) -> Result<CoreStatus, String> {
    let config = gui_config_state.snapshot()?;
    current_core_status(Some(process_state.inner()), Some(config.port))
}

pub(crate) fn emit_core_status(app: &tauri::AppHandle, status: &CoreStatus) {
    #[cfg(target_os = "windows")]
    update_windows_tray_status(app, status);
    let _ = app.emit(CORE_STATUS_EVENT, status);
}

#[tauri::command]
pub(crate) fn open_external_url(app: tauri::AppHandle, url: String) -> Result<(), String> {
    open_external_url_inner(&app, &url)
}

// Follows redirects only between GitHub hosts over plain HTTPS; used by the
// Codex model catalog refresh.
pub(crate) fn github_https_redirect_policy() -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(|attempt| {
        let url = attempt.url();
        let trusted_host = matches!(
            url.host_str(),
            Some(
                "github.com"
                    | "raw.githubusercontent.com"
                    | "objects.githubusercontent.com"
                    | "release-assets.githubusercontent.com"
            )
        );
        if url.scheme() == "https"
            && url.port().is_none()
            && url.username().is_empty()
            && url.password().is_none()
            && trusted_host
        {
            attempt.follow()
        } else {
            attempt.stop()
        }
    })
}

#[tauri::command]
pub(crate) fn get_gui_settings(
    gui_config_state: tauri::State<'_, GuiConfigState>,
) -> Result<GuiSettings, String> {
    let config = gui_config_state.snapshot()?;
    Ok(GuiSettings::from(&config))
}

#[tauri::command]
pub(crate) fn set_app_locale(
    app: tauri::AppHandle,
    process_state: tauri::State<'_, CoreProcessState>,
    gui_config_state: tauri::State<'_, GuiConfigState>,
    locale: String,
) -> Result<String, String> {
    let config = gui_config_state.set_locale(locale)?;
    #[cfg(target_os = "windows")]
    if let Ok(status) = current_core_status(Some(process_state.inner()), Some(config.port)) {
        update_windows_tray_locale(&app, &config.locale, &status);
    }
    #[cfg(not(target_os = "windows"))]
    let _ = (app, process_state);
    Ok(config.locale)
}

#[tauri::command]
pub(crate) fn resolve_windows_close_request(
    app: tauri::AppHandle,
    gui_config_state: tauri::State<'_, GuiConfigState>,
    action: WindowsCloseAction,
    remember: Option<bool>,
) -> Result<(), String> {
    if remember.unwrap_or(false) {
        let close_behavior = match action {
            WindowsCloseAction::Exit => WindowsCloseBehavior::Exit,
            WindowsCloseAction::MinimizeToTray => WindowsCloseBehavior::MinimizeToTray,
        };
        gui_config_state.set_close_behavior(close_behavior)?;
    }

    match action {
        WindowsCloseAction::Exit => app.exit(0),
        WindowsCloseAction::MinimizeToTray => {
            let window = app
                .get_webview_window("main")
                .ok_or_else(|| "主窗口不存在".to_string())?;
            window
                .hide()
                .map_err(|error| format!("隐藏主窗口失败: {error}"))?;
        }
    }

    Ok(())
}

pub(crate) fn app_autostart_enabled(app: &tauri::AppHandle) -> Result<bool, String> {
    app.autolaunch()
        .is_enabled()
        .map_err(|error| format!("读取系统开机自启状态失败: {error}"))
}

pub(crate) fn set_app_autostart_enabled(
    app: &tauri::AppHandle,
    enabled: bool,
) -> Result<(), String> {
    let manager = app.autolaunch();
    if enabled {
        manager
            .enable()
            .map_err(|error| format!("启用开机自启失败: {error}"))
    } else {
        manager
            .disable()
            .map_err(|error| format!("关闭开机自启失败: {error}"))
    }
}

pub(crate) fn software_settings(
    app: &tauri::AppHandle,
    config: &GuiConfigFile,
) -> Result<SoftwareSettings, String> {
    Ok(SoftwareSettings {
        close_behavior: config.close_behavior,
        autostart_enabled: app_autostart_enabled(app)?,
        start_core_on_launch: config.start_core_on_launch,
        silent_start_enabled: config.silent_start,
        default_terminal: normalize_agent_terminal(&config.default_terminal),
        available_terminals: available_agent_terminals(),
    })
}

#[tauri::command]
pub(crate) fn get_software_settings(
    app: tauri::AppHandle,
    gui_config_state: tauri::State<'_, GuiConfigState>,
) -> Result<SoftwareSettings, String> {
    let config = gui_config_state.snapshot()?;
    software_settings(&app, &config)
}

#[tauri::command]
pub(crate) fn save_software_settings(
    app: tauri::AppHandle,
    gui_config_state: tauri::State<'_, GuiConfigState>,
    settings: SoftwareSettingsInput,
) -> Result<SoftwareSettings, String> {
    let previous_config = gui_config_state.snapshot()?;
    let previous_autostart_enabled = app_autostart_enabled(&app)?;
    let autostart_changed = previous_autostart_enabled != settings.autostart_enabled;
    let default_terminal = normalize_agent_terminal(&settings.default_terminal);

    if autostart_changed {
        set_app_autostart_enabled(&app, settings.autostart_enabled)?;
    }

    let config = if previous_config.close_behavior == settings.close_behavior
        && previous_config.start_core_on_launch == settings.start_core_on_launch
        && previous_config.silent_start == settings.silent_start_enabled
        && previous_config.default_terminal == default_terminal
    {
        previous_config
    } else {
        match gui_config_state.set_software_preferences(
            settings.close_behavior,
            settings.start_core_on_launch,
            settings.silent_start_enabled,
            default_terminal.clone(),
        ) {
            Ok(config) => config,
            Err(error) => {
                let rollback_error = autostart_changed
                    .then(|| set_app_autostart_enabled(&app, previous_autostart_enabled).err())
                    .flatten();
                return Err(match rollback_error {
                    Some(rollback_error) => {
                        format!("{error}; 回滚开机自启设置也失败: {rollback_error}")
                    }
                    None => error,
                });
            }
        }
    };

    software_settings(&app, &config)
}
