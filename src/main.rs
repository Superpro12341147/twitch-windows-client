#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "windows")]
fn main() {
    if let Err(error) = run() {
        show_error(&error.to_string());
        std::process::exit(1);
    }
}

#[cfg(target_os = "windows")]
fn show_error(message: &str) {
    use windows::{
        core::PCWSTR,
        Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK},
    };

    let message: Vec<u16> = message.encode_utf16().chain(std::iter::once(0)).collect();
    let title: Vec<u16> = "Twitch Windows Client"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let _ = MessageBoxW(
            None,
            PCWSTR(message.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(target_os = "windows")]
struct ComUninitialize;

#[cfg(target_os = "windows")]
impl Drop for ComUninitialize {
    fn drop(&mut self) {
        unsafe {
            windows::Win32::System::Com::CoUninitialize();
        }
    }
}

#[cfg(target_os = "windows")]
fn create_start_menu_shortcut() -> Result<(), Box<dyn std::error::Error>> {
    use std::{env, fs, io, os::windows::ffi::OsStrExt, path::PathBuf};
    use windows::{
        core::{Interface, PCWSTR},
        Win32::{
            System::Com::{
                CoCreateInstance, CoInitializeEx, IPersistFile, CLSCTX_INPROC_SERVER,
                COINIT_APARTMENTTHREADED,
            },
            UI::Shell::{IShellLinkW, ShellLink},
        },
    };

    let app_data = env::var_os("APPDATA")
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "APPDATA is not set"))?;
    let programs = PathBuf::from(app_data).join("Microsoft\\Windows\\Start Menu\\Programs");
    fs::create_dir_all(&programs)?;
    let shortcut_path = programs.join("Twitch Windows Client.lnk");
    let executable = env::current_exe()?;
    let working_directory = executable.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "Executable has no parent directory",
        )
    })?;
    let executable_wide: Vec<u16> = executable
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let working_directory_wide: Vec<u16> = working_directory
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let description: Vec<u16> = "Lightweight Twitch desktop client"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let shortcut_wide: Vec<u16> = shortcut_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        let _com = ComUninitialize;
        let shell_link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
        shell_link.SetPath(PCWSTR(executable_wide.as_ptr()))?;
        shell_link.SetWorkingDirectory(PCWSTR(working_directory_wide.as_ptr()))?;
        shell_link.SetDescription(PCWSTR(description.as_ptr()))?;
        shell_link.SetIconLocation(PCWSTR(executable_wide.as_ptr()), 0)?;
        let persist_file: IPersistFile = shell_link.cast()?;
        persist_file.Save(PCWSTR(shortcut_wide.as_ptr()), true)?;
    }

    Ok(())
}

#[cfg(target_os = "windows")]
#[derive(serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum UiMessage {
    SetPlugin {
        name: String,
        enabled: bool,
    },
    SetTheme {
        name: Option<String>,
    },
    SetResourceLimits {
        cpu_percent: Option<u32>,
        ram_mb: Option<u32>,
    },
    OpenMultiStream {
        channels: Vec<String>,
    },
    OpenDevTools,
}

#[cfg(target_os = "windows")]
#[derive(serde::Serialize, serde::Deserialize, Default, Clone)]
#[serde(default)]
struct Settings {
    plugins: std::collections::BTreeSet<String>,
    theme: Option<String>,
    resource_cpu_percent: Option<u32>,
    resource_ram_mb: Option<u32>,
}

#[cfg(target_os = "windows")]
struct Plugin {
    filename: String,
    source: String,
}

#[cfg(target_os = "windows")]
struct Theme {
    filename: String,
    source: String,
}

#[cfg(target_os = "windows")]
#[derive(Clone)]
enum AppEvent {
    ReloadSettings,
    OpenMultiStream(Vec<String>),
    OpenDevTools,
}

#[cfg(target_os = "windows")]
struct PlayerWindow {
    webviews: Vec<wry::WebView>,
    window: tao::window::Window,
}

#[cfg(target_os = "windows")]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    use std::{env, fs, io, path::PathBuf};
    use tao::{
        event::{ElementState, Event, WindowEvent},
        event_loop::{ControlFlow, EventLoopBuilder},
        keyboard::KeyCode,
        window::{Icon, WindowBuilder},
    };
    use wry::WebContext;

    let local_app_data = env::var_os("LOCALAPPDATA")
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "LOCALAPPDATA is not set"))?;
    let app_data_dir = PathBuf::from(local_app_data).join("TwitchWindowsClient");
    fs::create_dir_all(&app_data_dir)?;

    let webview_data_dir = app_data_dir.join("WebView2");
    fs::create_dir_all(&webview_data_dir)?;
    let plugins_dir = app_data_dir.join("plugins");
    let themes_dir = app_data_dir.join("themes");
    fs::create_dir_all(&plugins_dir)?;
    ensure_example(
        &plugins_dir,
        "resource-limiter.js",
        include_str!("../assets/resource-limiter.js"),
    )?;
    ensure_example(
        &themes_dir,
        "catppuccin-mocha.css",
        include_str!("../themes/catppuccin-mocha.css"),
    )?;
    create_start_menu_shortcut()?;

    let settings_path = app_data_dir.join("settings.json");
    let mut settings = load_settings(&settings_path)?;
    remove_adblocker_settings(&settings_path, &mut settings)?;
    apply_resource_limits(settings.resource_cpu_percent, settings.resource_ram_mb)
        .map_err(|error| io::Error::other(error.to_string()))?;
    let plugins = load_plugins(&plugins_dir)?;
    let themes = load_themes(&themes_dir)?;

    let mut event_loop_builder = EventLoopBuilder::with_user_event();
    let event_loop = event_loop_builder.build();
    let window_icon = Icon::from_rgba(
        include_bytes!("../assets/twitch-client.rgba").to_vec(),
        64,
        64,
    )?;
    let window = WindowBuilder::new()
        .with_title("Twitch")
        .with_window_icon(Some(window_icon))
        .with_inner_size(tao::dpi::LogicalSize::new(1280.0, 800.0))
        .with_min_inner_size(tao::dpi::LogicalSize::new(800.0, 500.0))
        .build(&event_loop)?;

    let mut web_context = WebContext::new(Some(webview_data_dir));
    let proxy = event_loop.create_proxy();
    let mut webview = Some(build_webview(
        &window,
        &mut web_context,
        &plugins,
        &themes,
        &settings,
        &settings_path,
        proxy.clone(),
    )?);
    let mut player_windows = Vec::<PlayerWindow>::new();

    event_loop.run(move |event, target, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::UserEvent(AppEvent::ReloadSettings) => {
                let settings = match load_settings(&settings_path) {
                    Ok(settings) => settings,
                    Err(error) => {
                        show_error(&format!("Could not load client settings: {error}"));
                        return;
                    }
                };
                webview.take();
                match build_webview(
                    &window,
                    &mut web_context,
                    &plugins,
                    &themes,
                    &settings,
                    &settings_path,
                    proxy.clone(),
                ) {
                    Ok(updated_webview) => webview = Some(updated_webview),
                    Err(error) => {
                        show_error(&format!("Could not reload Twitch: {error}"));
                        *control_flow = ControlFlow::Exit;
                    }
                }
            }
            Event::UserEvent(AppEvent::OpenMultiStream(channels)) => {
                match build_player_window(
                    target,
                    &mut web_context,
                    &plugins,
                    &settings,
                    &channels,
                    "Twitch Multi-stream",
                ) {
                    Ok(player_window) => player_windows.push(player_window),
                    Err(error) => {
                        show_error(&format!("Could not open Twitch player window: {error}"));
                    }
                }
            }
            Event::UserEvent(AppEvent::OpenDevTools) => {
                if let Some(webview) = webview.as_ref() {
                    webview.open_devtools();
                }
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::KeyboardInput { event, .. },
                ..
            } if event.state == ElementState::Pressed && event.physical_key == KeyCode::F12 => {
                if window_id == window.id() {
                    if let Some(webview) = webview.as_ref() {
                        webview.open_devtools();
                    }
                } else if let Some(player_window) = player_windows
                    .iter()
                    .find(|player_window| player_window.window.id() == window_id)
                {
                    if let Some(webview) = player_window.webviews.first() {
                        webview.open_devtools();
                    }
                }
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::Resized(size),
                ..
            } => {
                if let Some(player_window) = player_windows
                    .iter()
                    .find(|player_window| player_window.window.id() == window_id)
                {
                    if let Err(error) = layout_player_webviews(player_window, size) {
                        show_error(&format!("Could not resize Twitch players: {error}"));
                    }
                }
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CloseRequested,
                ..
            } => {
                if window_id == window.id() {
                    *control_flow = ControlFlow::Exit;
                } else {
                    player_windows.retain(|player_window| player_window.window.id() != window_id);
                }
            }
            _ => {}
        }
    });
}

#[cfg(target_os = "windows")]
fn build_webview(
    window: &tao::window::Window,
    web_context: &mut wry::WebContext,
    plugins: &[Plugin],
    themes: &[Theme],
    settings: &Settings,
    settings_path: &std::path::Path,
    proxy: tao::event_loop::EventLoopProxy<AppEvent>,
) -> Result<wry::WebView, Box<dyn std::error::Error>> {
    use std::collections::BTreeSet;
    use wry::WebViewBuilder;

    let plugin_names = plugins
        .iter()
        .map(|plugin| plugin.filename.clone())
        .collect::<Vec<_>>();
    let theme_names = themes
        .iter()
        .map(|theme| theme.filename.clone())
        .collect::<Vec<_>>();
    let config = serde_json::json!({
        "plugins": plugin_names,
        "themes": theme_names,
        "settings": {
            "plugins": settings.plugins.iter().collect::<Vec<_>>(),
            "theme": settings.theme,
            "resource_cpu_percent": settings.resource_cpu_percent,
            "resource_ram_mb": settings.resource_ram_mb,
        }
    });
    let settings_script = format!(
        "(() => {{\n\
            if (window.top !== window) return;\n\
            if (location.hostname !== 'twitch.tv' && !location.hostname.endsWith('.twitch.tv')) return;\n\
            console.warn('[Twitch Windows Client] Developer tools can inspect page content and your signed-in session. Keep cookies and tokens private.');\n\
            window.__twitchClientConfig = {config};\n\
            {ui}\n\
        }})();",
        config = config,
        ui = include_str!("../assets/settings.js"),
    );

    let mut builder = WebViewBuilder::new_with_web_context(web_context)
        .with_devtools(true)
        .with_initialization_script(settings_script);
    for plugin in plugins
        .iter()
        .filter(|plugin| settings.plugins.contains(&plugin.filename))
    {
        let script = format!(
            "(() => {{\n\
                if (window.top !== window) return;\n\
                if (location.hostname !== 'twitch.tv' && !location.hostname.endsWith('.twitch.tv')) return;\n\
                {source}\n\
            }})();",
            source = plugin.source,
        );
        builder = builder.with_initialization_script(script);
    }

    if let Some(theme) = settings
        .theme
        .as_ref()
        .and_then(|name| themes.iter().find(|theme| &theme.filename == name))
    {
        let css = serde_json::to_string(&theme.source)?;
        builder = builder.with_initialization_script(format!(
            "(() => {{\n\
                if (window.top !== window) return;\n\
                if (location.hostname !== 'twitch.tv' && !location.hostname.endsWith('.twitch.tv')) return;\n\
                const install = () => {{\n\
                    const parent = document.head || document.documentElement;\n\
                    if (!parent) return false;\n\
                    if (document.getElementById('twitch-client-theme')) return true;\n\
                    const style = document.createElement('style');\n\
                    style.id = 'twitch-client-theme';\n\
                    style.textContent = {css};\n\
                    parent.appendChild(style);\n\
                    return true;\n\
                }};\n\
                const observer = new MutationObserver(() => {{ if (install()) observer.disconnect(); }});\n\
                observer.observe(document, {{ childList: true, subtree: true }});\n\
                if (install()) observer.disconnect();\n\
            }})();"
        ));
    }

    let settings_path = settings_path.to_path_buf();
    let valid_plugins = plugin_names.into_iter().collect::<BTreeSet<_>>();
    let valid_themes = theme_names.into_iter().collect::<BTreeSet<_>>();
    let builder = builder.with_ipc_handler(move |request| {
        let Some(host) = request.uri().host() else {
            return;
        };
        let trusted_origin = request.uri().scheme_str() == Some("https")
            && (host == "twitch.tv" || host.ends_with(".twitch.tv"));
        if !trusted_origin {
            return;
        }

        let Ok(message) = serde_json::from_str::<UiMessage>(request.body()) else {
            return;
        };
        match &message {
            UiMessage::OpenMultiStream { channels } => {
                if !are_valid_multi_stream_channels(channels) {
                    show_error("Enter 2-4 unique Twitch channel names using letters, numbers, and underscores.");
                    return;
                }
                if let Err(error) = proxy.send_event(AppEvent::OpenMultiStream(channels.clone())) {
                    show_error(&format!("Could not open the multi-stream view: {error}"));
                }
                return;
            }
            UiMessage::OpenDevTools => {
                if let Err(error) = proxy.send_event(AppEvent::OpenDevTools) {
                    show_error(&format!("Could not open Developer Tools: {error}"));
                }
                return;
            }
            UiMessage::SetResourceLimits { .. } => {}
            _ => {}
        }
        let mut settings = match load_settings(&settings_path) {
            Ok(settings) => settings,
            Err(error) => {
                show_error(&format!("Could not load client settings: {error}"));
                return;
            }
        };

        if !apply_ui_message(&mut settings, message, &valid_plugins, &valid_themes) {
            return;
        }

        if let Err(error) = save_settings(&settings_path, &settings) {
            show_error(&format!("Could not save client settings: {error}"));
            return;
        }
        if let Err(error) = proxy.send_event(AppEvent::ReloadSettings) {
            show_error(&format!("Could not apply client settings: {error}"));
        }
    });

    let webview = builder.build(window)?;
    webview.load_url("https://www.twitch.tv/")?;
    Ok(webview)
}

#[cfg(target_os = "windows")]
fn build_player_window(
    target: &tao::event_loop::EventLoopWindowTarget<AppEvent>,
    web_context: &mut wry::WebContext,
    plugins: &[Plugin],
    settings: &Settings,
    channels: &[String],
    title: &str,
) -> Result<PlayerWindow, Box<dyn std::error::Error>> {
    use tao::{dpi::LogicalSize, window::WindowBuilder};
    use wry::WebViewBuilder;

    let icon = tao::window::Icon::from_rgba(
        include_bytes!("../assets/twitch-client.rgba").to_vec(),
        64,
        64,
    )?;
    let window = WindowBuilder::new()
        .with_title(title)
        .with_window_icon(Some(icon))
        .with_inner_size(LogicalSize::new(1280.0, 800.0))
        .build(target)?;
    let size = window.inner_size();
    let mut webviews = Vec::with_capacity(channels.len());

    for (index, channel) in channels.iter().enumerate() {
        let bounds = player_bounds(index, channels.len(), size.width, size.height);
        let settings_script = "(() => {\n\
                if (location.hostname !== 'twitch.tv' && !location.hostname.endsWith('.twitch.tv')) return;\n\
                console.warn('[Twitch Windows Client] Developer tools can inspect page content and your signed-in session. Keep cookies and tokens private.');\n\
            })();"
            .to_owned();
        let mut builder = WebViewBuilder::new_with_web_context(web_context)
            .with_devtools(true)
            .with_bounds(bounds)
            .with_initialization_script(settings_script);
        if channels.len() > 1 && index > 0 {
            builder = builder.with_initialization_script(
                "(() => {\n\
                    const mute = (video) => {\n\
                        const enforceMute = () => {\n\
                            if (!video.muted || video.volume !== 0) {\n\
                                video.muted = true;\n\
                                video.volume = 0;\n\
                            }\n\
                        };\n\
                        video.addEventListener('volumechange', enforceMute);\n\
                        enforceMute();\n\
                    };\n\
                    const scan = (node) => {\n\
                        if (node instanceof HTMLVideoElement) mute(node);\n\
                        else if (node.querySelectorAll) node.querySelectorAll('video').forEach(mute);\n\
                    };\n\
                    new MutationObserver((records) => records.forEach((record) => record.addedNodes.forEach(scan)))\n\
                        .observe(document, { childList: true, subtree: true });\n\
                    document.querySelectorAll('video').forEach(mute);\n\
                })();",
            );
        }

        for plugin in plugins
            .iter()
            .filter(|plugin| settings.plugins.contains(&plugin.filename))
        {
            builder = builder.with_initialization_script(format!(
                "(() => {{\n\
                    if (window.top !== window) return;\n\
                    if (location.hostname !== 'twitch.tv' && !location.hostname.endsWith('.twitch.tv')) return;\n\
                    {source}\n\
                }})();",
                source = plugin.source,
            ));
        }

        let webview = builder.build_as_child(&window)?;
        webview.load_url(&format!("https://www.twitch.tv/popout/{channel}/player"))?;
        webviews.push(webview);
    }

    Ok(PlayerWindow { webviews, window })
}

#[cfg(target_os = "windows")]
fn player_bounds(index: usize, count: usize, width: u32, height: u32) -> wry::Rect {
    use wry::{
        dpi::{PhysicalPosition, PhysicalSize},
        Rect,
    };

    let columns = if count == 1 { 1 } else { 2 };
    let rows = count.div_ceil(columns);
    let mut cell_width = width / columns as u32;
    let cell_height = height / rows as u32;
    let mut column = index % columns;
    let row = index / columns;
    if count == 3 && index == 2 {
        cell_width = width;
        column = 0;
    }

    Rect {
        position: PhysicalPosition::new(
            (column as u32 * (width / columns as u32)) as i32,
            (row as u32 * cell_height) as i32,
        )
        .into(),
        size: PhysicalSize::new(cell_width, cell_height).into(),
    }
}

#[cfg(target_os = "windows")]
fn layout_player_webviews(
    player_window: &PlayerWindow,
    size: tao::dpi::PhysicalSize<u32>,
) -> wry::Result<()> {
    for (index, webview) in player_window.webviews.iter().enumerate() {
        webview.set_bounds(player_bounds(
            index,
            player_window.webviews.len(),
            size.width,
            size.height,
        ))?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn apply_ui_message(
    settings: &mut Settings,
    message: UiMessage,
    valid_plugins: &std::collections::BTreeSet<String>,
    valid_themes: &std::collections::BTreeSet<String>,
) -> bool {
    match message {
        UiMessage::SetPlugin { name, enabled } if valid_plugins.contains(&name) => {
            if enabled {
                settings.plugins.insert(name);
            } else {
                settings.plugins.remove(&name);
            }
            true
        }
        UiMessage::SetTheme { name }
            if name
                .as_ref()
                .is_none_or(|theme| valid_themes.contains(theme)) =>
        {
            settings.theme = name;
            true
        }
        UiMessage::SetResourceLimits {
            cpu_percent,
            ram_mb,
        } => {
            if cpu_percent.is_some_and(|value| !(10..=100).contains(&value))
                || ram_mb.is_some_and(|value| !(256..=16384).contains(&value))
            {
                return false;
            }
            if let Err(error) = apply_resource_limits(cpu_percent, ram_mb) {
                show_error(&format!("Could not apply resource limits: {error}"));
                return false;
            }
            settings.resource_cpu_percent = cpu_percent;
            settings.resource_ram_mb = ram_mb;
            true
        }
        _ => false,
    }
}

#[cfg(target_os = "windows")]
fn is_valid_channel_name(channel: &str) -> bool {
    !channel.is_empty()
        && channel.len() <= 25
        && channel
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

#[cfg(target_os = "windows")]
fn are_valid_multi_stream_channels(channels: &[String]) -> bool {
    use std::collections::BTreeSet;

    (2..=4).contains(&channels.len())
        && channels
            .iter()
            .all(|channel| is_valid_channel_name(channel))
        && channels
            .iter()
            .map(|channel| channel.to_ascii_lowercase())
            .collect::<BTreeSet<_>>()
            .len()
            == channels.len()
}

#[cfg(target_os = "windows")]
fn apply_resource_limits(
    cpu_percent: Option<u32>,
    ram_mb: Option<u32>,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::OnceLock;
    use windows::{
        core::PCWSTR,
        Win32::System::{
            JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JobObjectCpuRateControlInformation,
                JobObjectExtendedLimitInformation, SetInformationJobObject,
                JOBOBJECT_CPU_RATE_CONTROL_INFORMATION, JOBOBJECT_CPU_RATE_CONTROL_INFORMATION_0,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_CPU_RATE_CONTROL_ENABLE,
                JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP, JOB_OBJECT_LIMIT_PROCESS_MEMORY,
            },
            Threading::GetCurrentProcess,
        },
    };

    static JOB_HANDLE: OnceLock<isize> = OnceLock::new();
    let handle = *JOB_HANDLE.get_or_init(|| unsafe {
        CreateJobObjectW(None, PCWSTR::null())
            .map(|handle| handle.0 as isize)
            .unwrap_or(0)
    });
    if handle == 0 {
        return Err("Windows could not create a resource limit job".into());
    }
    let handle = windows::Win32::Foundation::HANDLE(handle as *mut core::ffi::c_void);

    unsafe {
        AssignProcessToJobObject(handle, GetCurrentProcess())?;
    }

    if let Some(cpu) = cpu_percent {
        let cpu = cpu.clamp(10, 100);
        let mut cpu_info = JOBOBJECT_CPU_RATE_CONTROL_INFORMATION::default();
        cpu_info.ControlFlags =
            JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP;
        cpu_info.Anonymous = JOBOBJECT_CPU_RATE_CONTROL_INFORMATION_0 { CpuRate: cpu };
        unsafe {
            SetInformationJobObject(
                handle,
                JobObjectCpuRateControlInformation,
                &cpu_info as *const _ as *const core::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_CPU_RATE_CONTROL_INFORMATION>() as u32,
            )?;
        }
    } else {
        let cpu_info = JOBOBJECT_CPU_RATE_CONTROL_INFORMATION::default();
        unsafe {
            SetInformationJobObject(
                handle,
                JobObjectCpuRateControlInformation,
                &cpu_info as *const _ as *const core::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_CPU_RATE_CONTROL_INFORMATION>() as u32,
            )?;
        }
    }

    if let Some(ram) = ram_mb {
        let ram = ram.clamp(256, 16384) as usize * 1024 * 1024;
        let mut memory_info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        memory_info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_PROCESS_MEMORY;
        memory_info.ProcessMemoryLimit = ram;
        unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &memory_info as *const _ as *const core::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )?;
        }
    } else {
        let mut memory_info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        memory_info.BasicLimitInformation.LimitFlags = Default::default();
        memory_info.ProcessMemoryLimit = 0;
        unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &memory_info as *const _ as *const core::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )?;
        }
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn ensure_example(
    directory: &std::path::Path,
    filename: &str,
    source: &str,
) -> std::io::Result<()> {
    use std::{fs, io};

    fs::create_dir_all(directory)?;
    let path = directory.join(filename);
    if path.exists() {
        return Ok(());
    }
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(mut file) => {
            use std::io::Write;
            file.write_all(source.as_bytes())
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(target_os = "windows")]
fn remove_adblocker_settings(
    settings_path: &std::path::Path,
    settings: &mut Settings,
) -> std::io::Result<()> {
    let removed =
        settings.plugins.remove("ad-blocker.js") | settings.plugins.remove("ad-blocker.yaml");
    if removed {
        save_settings(settings_path, settings)?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn load_plugins(plugin_dir: &std::path::Path) -> std::io::Result<Vec<Plugin>> {
    use std::{ffi::OsStr, fs, io};

    let mut plugins = Vec::new();
    for entry in fs::read_dir(plugin_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file()
            || !entry
                .path()
                .extension()
                .and_then(OsStr::to_str)
                .is_some_and(|extension| extension.eq_ignore_ascii_case("js"))
        {
            continue;
        }
        let path = entry.path();
        let filename = entry.file_name().to_string_lossy().into_owned();
        if filename.eq_ignore_ascii_case("ad-blocker.js")
            || filename.eq_ignore_ascii_case("ad-blocker.yaml")
        {
            continue;
        }
        let source = fs::read_to_string(&path).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("Could not read plugin {}: {error}", path.display()),
            )
        })?;
        plugins.push(Plugin { filename, source });
    }
    plugins.sort_by(|left, right| left.filename.cmp(&right.filename));
    Ok(plugins)
}

#[cfg(target_os = "windows")]
fn load_themes(theme_dir: &std::path::Path) -> std::io::Result<Vec<Theme>> {
    use std::{ffi::OsStr, fs, io};

    let mut themes = Vec::new();
    for entry in fs::read_dir(theme_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file()
            || !entry
                .path()
                .extension()
                .and_then(OsStr::to_str)
                .is_some_and(|extension| extension.eq_ignore_ascii_case("css"))
        {
            continue;
        }
        let path = entry.path();
        let source = fs::read_to_string(&path).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("Could not read theme {}: {error}", path.display()),
            )
        })?;
        themes.push(Theme {
            filename: entry.file_name().to_string_lossy().into_owned(),
            source,
        });
    }
    themes.sort_by(|left, right| left.filename.cmp(&right.filename));
    Ok(themes)
}

#[cfg(target_os = "windows")]
fn load_settings(path: &std::path::Path) -> std::io::Result<Settings> {
    use std::{fs, io};

    match fs::read(path) {
        Ok(contents) => serde_json::from_slice(&contents)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Settings::default()),
        Err(error) => Err(error),
    }
}

#[cfg(target_os = "windows")]
fn save_settings(path: &std::path::Path, settings: &Settings) -> std::io::Result<()> {
    let contents = serde_json::to_vec_pretty(settings)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    std::fs::write(path, contents)
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::{
        apply_ui_message, load_settings, remove_adblocker_settings, save_settings, Settings,
        UiMessage,
    };
    use std::collections::BTreeSet;

    #[test]
    fn plugins_are_disabled_by_default() {
        assert!(Settings::default().plugins.is_empty());
    }

    #[test]
    fn only_discovered_plugins_can_be_enabled() {
        let valid_plugins = BTreeSet::from(["example.js".to_owned()]);
        let valid_themes = BTreeSet::new();
        let mut settings = Settings::default();

        assert!(!apply_ui_message(
            &mut settings,
            UiMessage::SetPlugin {
                name: "unknown.js".to_owned(),
                enabled: true,
            },
            &valid_plugins,
            &valid_themes,
        ));
        assert!(settings.plugins.is_empty());

        assert!(apply_ui_message(
            &mut settings,
            UiMessage::SetPlugin {
                name: "example.js".to_owned(),
                enabled: true,
            },
            &valid_plugins,
            &valid_themes,
        ));
        assert_eq!(settings.plugins, valid_plugins);
    }

    #[test]
    fn only_discovered_themes_can_be_selected() {
        let valid_plugins = BTreeSet::new();
        let valid_themes = BTreeSet::from(["theme.css".to_owned()]);
        let mut settings = Settings::default();

        assert!(!apply_ui_message(
            &mut settings,
            UiMessage::SetTheme {
                name: Some("unknown.css".to_owned()),
            },
            &valid_plugins,
            &valid_themes,
        ));
        assert!(settings.theme.is_none());

        assert!(apply_ui_message(
            &mut settings,
            UiMessage::SetTheme {
                name: Some("theme.css".to_owned()),
            },
            &valid_plugins,
            &valid_themes,
        ));
        assert_eq!(settings.theme.as_deref(), Some("theme.css"));
    }

    #[test]
    fn resource_limits_reject_out_of_range_values() {
        let valid_plugins = BTreeSet::new();
        let valid_themes = BTreeSet::new();
        let mut settings = Settings::default();

        assert!(!apply_ui_message(
            &mut settings,
            UiMessage::SetResourceLimits {
                cpu_percent: Some(5),
                ram_mb: None,
            },
            &valid_plugins,
            &valid_themes,
        ));
        assert!(!apply_ui_message(
            &mut settings,
            UiMessage::SetResourceLimits {
                cpu_percent: None,
                ram_mb: Some(128),
            },
            &valid_plugins,
            &valid_themes,
        ));
    }

    #[test]
    fn removed_adblocker_settings_are_not_preserved() {
        let directory =
            std::env::temp_dir().join(format!("twitch-client-adblock-yaml-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let settings_path = directory.join("settings.json");
        let mut settings = Settings {
            plugins: BTreeSet::from(["ad-blocker.yaml".to_owned(), "custom.js".to_owned()]),
            ..Settings::default()
        };
        save_settings(&settings_path, &settings).unwrap();
        remove_adblocker_settings(&settings_path, &mut settings).unwrap();
        assert!(!settings.plugins.contains("ad-blocker.js"));
        assert!(settings.plugins.contains("custom.js"));
        assert!(!settings.plugins.contains("ad-blocker.yaml"));
        assert_eq!(
            load_settings(&settings_path).unwrap().plugins,
            settings.plugins
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn multi_stream_requires_unique_valid_channel_names() {
        assert!(super::are_valid_multi_stream_channels(&[
            "first".to_owned(),
            "second_2".to_owned(),
        ]));
        assert!(!super::are_valid_multi_stream_channels(
            &["only".to_owned()]
        ));
        assert!(!super::are_valid_multi_stream_channels(&[
            "same".to_owned(),
            "SAME".to_owned(),
        ]));
        assert!(!super::are_valid_multi_stream_channels(&[
            "valid".to_owned(),
            "invalid/name".to_owned(),
        ]));
        assert!(!super::are_valid_multi_stream_channels(&[
            "one".to_owned(),
            "two".to_owned(),
            "three".to_owned(),
            "four".to_owned(),
            "five".to_owned(),
        ]));
    }
}

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("This Twitch client currently supports Windows only.");
}
