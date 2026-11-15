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
const NATIVE_AD_HOSTS: &[&str] = &[
    "edge.ads.twitch.tv",
    "ads.twitch.tv",
    "doubleclick.net",
    "googleadservices.com",
    "googlesyndication.com",
    "amazon-adsystem.com",
];

#[cfg(target_os = "windows")]
const NATIVE_AD_REQUEST_FILTERS: &[&str] = &[
    "https://edge.ads.twitch.tv/*",
    "https://ads.twitch.tv/*",
    "https://*.ads.twitch.tv/*",
    "https://doubleclick.net/*",
    "https://*.doubleclick.net/*",
    "https://googleadservices.com/*",
    "https://*.googleadservices.com/*",
    "https://googlesyndication.com/*",
    "https://*.googlesyndication.com/*",
    "https://amazon-adsystem.com/*",
    "https://*.amazon-adsystem.com/*",
];

#[cfg(target_os = "windows")]
fn is_native_ad_request(url: &str) -> bool {
    let Some((scheme, remainder)) = url.split_once("://") else {
        return false;
    };
    if !scheme.eq_ignore_ascii_case("https") && !scheme.eq_ignore_ascii_case("http") {
        return false;
    }

    let Some(authority) = remainder
        .split(['/', '?', '#'])
        .next()
        .and_then(|authority| authority.rsplit('@').next())
    else {
        return false;
    };
    let host = authority
        .split(':')
        .next()
        .unwrap_or_default()
        .trim_end_matches('.')
        .to_ascii_lowercase();

    NATIVE_AD_HOSTS
        .iter()
        .any(|blocked| host == *blocked || host.ends_with(&format!(".{blocked}")))
}

#[cfg(target_os = "windows")]
fn install_native_ad_blocker(webview: &wry::WebView) -> windows::core::Result<()> {
    use webview2_com::{
        take_pwstr, Microsoft::Web::WebView2::Win32::COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
        WebResourceRequestedEventHandler,
    };
    use windows::core::HSTRING;
    use wry::WebViewExtWindows;

    let core_webview = webview.webview();
    let environment = webview.environment();
    for filter in NATIVE_AD_REQUEST_FILTERS {
        unsafe {
            core_webview.AddWebResourceRequestedFilter(
                &HSTRING::from(*filter),
                COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
            )?;
        }
    }

    let handler = WebResourceRequestedEventHandler::create(Box::new(move |_, args| {
        let Some(args) = args else {
            return Ok(());
        };
        let request = unsafe { args.Request()? };
        let mut uri = windows::core::PWSTR::null();
        unsafe {
            request.Uri(&mut uri)?;
        }
        if !is_native_ad_request(&take_pwstr(uri)) {
            return Ok(());
        }

        let reason = HSTRING::from("No Content");
        let headers = HSTRING::from("Cache-Control: no-store\r\nContent-Length: 0\r\n");
        let response =
            unsafe { environment.CreateWebResourceResponse(None, 204, &reason, &headers)? };
        unsafe {
            args.SetResponse(&response)?;
        }
        Ok(())
    }));
    let mut token = 0;
    unsafe {
        core_webview.add_WebResourceRequested(&handler, &mut token)?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
#[derive(serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum UiMessage {
    SetPlugin { name: String, enabled: bool },
    SetTheme { name: Option<String> },
    OpenMultiStream { channels: Vec<String> },
    OpenDevTools,
}

#[cfg(target_os = "windows")]
#[derive(serde::Serialize, serde::Deserialize, Default, Clone)]
#[serde(default)]
struct Settings {
    plugins: std::collections::BTreeSet<String>,
    theme: Option<String>,
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
    ensure_example(
        &plugins_dir,
        "ad-blocker.js",
        include_str!("../plugins/ad-blocker.js"),
    )?;
    upgrade_example_if_unchanged(
        &plugins_dir.join("ad-blocker.js"),
        include_str!("../plugins/ad-blocker-v1.js"),
        include_str!("../plugins/ad-blocker.js"),
    )?;
    repair_adblocker_worker_scope(&plugins_dir.join("ad-blocker.js"))?;
    ensure_example(
        &themes_dir,
        "catppuccin-mocha.css",
        include_str!("../themes/catppuccin-mocha.css"),
    )?;
    create_start_menu_shortcut()?;

    let settings_path = app_data_dir.join("settings.json");
    let mut settings = load_settings(&settings_path)?;
    migrate_adblocker_setting(&settings_path, &mut settings)?;
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
        let frame_guard = if plugin.filename == "ad-blocker.js" {
            ""
        } else {
            "if (window.top !== window) return;\n"
        };
        let script = format!(
            "(() => {{\n\
                {frame_guard}\
                if (location.hostname !== 'twitch.tv' && !location.hostname.endsWith('.twitch.tv')) return;\n\
                {source}\n\
            }})();",
            frame_guard = frame_guard,
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
    if settings.plugins.contains("ad-blocker.js") {
        install_native_ad_blocker(&webview)?;
    }
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
            let frame_guard = if plugin.filename == "ad-blocker.js" {
                ""
            } else {
                "if (window.top !== window) return;\n"
            };
            builder = builder.with_initialization_script(format!(
                "(() => {{\n\
                    {frame_guard}\
                    if (location.hostname !== 'twitch.tv' && !location.hostname.endsWith('.twitch.tv')) return;\n\
                    {source}\n\
                }})();",
                frame_guard = frame_guard,
                source = plugin.source,
            ));
        }

        let webview = builder.build_as_child(&window)?;
        if settings.plugins.contains("ad-blocker.js") {
            install_native_ad_blocker(&webview)?;
        }
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
fn upgrade_example_if_unchanged(
    path: &std::path::Path,
    previous_source: &str,
    source: &str,
) -> std::io::Result<()> {
    use std::{fs, io};

    let existing = match fs::read_to_string(path) {
        Ok(existing) => existing,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if existing.replace("\r\n", "\n") == previous_source.replace("\r\n", "\n") {
        fs::write(path, source)?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn repair_adblocker_worker_scope(path: &std::path::Path) -> std::io::Result<()> {
    use std::{fs, io};

    let mut source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    const BROKEN_BOUNDARY: &str = "  installStyle();\n})();\n\n// Adapted from My AdBlock's Twitch worker patch.\n// Copyright (c) 2026 Jakub L. Licensed under MIT; see\n// licenses/my-adblock-LICENSE.txt.\n(() => {\n  \"use strict\";\n";
    const FIXED_BOUNDARY: &str = "  installStyle();\n\n// Adapted from My AdBlock's Twitch worker patch.\n// Copyright (c) 2026 Jakub L. Licensed under MIT; see\n// licenses/my-adblock-LICENSE.txt.\n";
    let line_ending = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let broken_boundary = BROKEN_BOUNDARY.replace('\n', line_ending);
    if let Some(start) = source.find(&broken_boundary) {
        source.replace_range(
            start..start + broken_boundary.len(),
            &FIXED_BOUNDARY.replace('\n', line_ending),
        );
        fs::write(path, source)?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn migrate_adblocker_setting(
    settings_path: &std::path::Path,
    settings: &mut Settings,
) -> std::io::Result<()> {
    if settings.plugins.remove("ad-blocker.yaml") {
        settings.plugins.insert("ad-blocker.js".to_owned());
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
        let source = fs::read_to_string(&path).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("Could not read plugin {}: {error}", path.display()),
            )
        })?;
        plugins.push(Plugin {
            filename: entry.file_name().to_string_lossy().into_owned(),
            source,
        });
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
        apply_ui_message, is_native_ad_request, load_settings, migrate_adblocker_setting,
        repair_adblocker_worker_scope, save_settings, upgrade_example_if_unchanged, Settings,
        UiMessage,
    };
    use std::collections::BTreeSet;

    #[test]
    fn plugins_are_disabled_by_default() {
        assert!(Settings::default().plugins.is_empty());
    }

    #[test]
    fn native_ad_filter_matches_ad_hosts_without_matching_stream_hosts() {
        assert!(is_native_ad_request(
            "https://edge.ads.twitch.tv/ads?channel=example"
        ));
        assert!(is_native_ad_request(
            "https://sub.doubleclick.net/pagead/ads"
        ));
        assert!(!is_native_ad_request(
            "https://doubleclick.net.evil.example/ads"
        ));
        assert!(!is_native_ad_request(
            "https://example.com/edge.ads.twitch.tv/ads"
        ));
        assert!(!is_native_ad_request("https://gql.twitch.tv/gql"));
        assert!(!is_native_ad_request(
            "https://usher.ttvnw.net/api/channel/hls/example.m3u8"
        ));
    }

    #[test]
    fn repairs_worker_scope_in_existing_plugin_without_replacing_custom_content() {
        let directory = std::env::temp_dir().join(format!(
            "twitch-client-adblock-scope-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("ad-blocker.js");
        let broken = "(() => {\n  installStyle();\n})();\n\n// Adapted from My AdBlock's Twitch worker patch.\n// Copyright (c) 2026 Jakub L. Licensed under MIT; see\n// licenses/my-adblock-LICENSE.txt.\n(() => {\n  \"use strict\";\n  function isPlayerWorker() { return rules.twitch.worker_identifiers; }\n  // user customization\n})();\n";
        let expected = "(() => {\n  installStyle();\n\n// Adapted from My AdBlock's Twitch worker patch.\n// Copyright (c) 2026 Jakub L. Licensed under MIT; see\n// licenses/my-adblock-LICENSE.txt.\n  function isPlayerWorker() { return rules.twitch.worker_identifiers; }\n  // user customization\n})();\n";
        std::fs::write(&path, broken.replace('\n', "\r\n")).unwrap();

        repair_adblocker_worker_scope(&path).unwrap();
        let repaired = std::fs::read_to_string(&path).unwrap();
        assert_eq!(repaired, expected.replace('\n', "\r\n"));
        repair_adblocker_worker_scope(&path).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), repaired);
        std::fs::remove_dir_all(directory).unwrap();
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
    fn bundled_plugin_upgrade_preserves_custom_edits() {
        let directory = std::env::temp_dir().join(format!(
            "twitch-client-adblock-migration-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("ad-blocker.js");
        std::fs::write(&path, "old bundled source").unwrap();
        upgrade_example_if_unchanged(&path, "old bundled source", "updated source").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "updated source");

        std::fs::write(&path, "custom source").unwrap();
        upgrade_example_if_unchanged(&path, "old bundled source", "newer source").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "custom source");
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn yaml_plugin_setting_migrates_to_the_bundled_javascript_plugin() {
        let directory =
            std::env::temp_dir().join(format!("twitch-client-adblock-yaml-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let settings_path = directory.join("settings.json");
        let mut settings = Settings {
            plugins: BTreeSet::from(["ad-blocker.yaml".to_owned(), "custom.js".to_owned()]),
            ..Settings::default()
        };
        save_settings(&settings_path, &settings).unwrap();
        migrate_adblocker_setting(&settings_path, &mut settings).unwrap();
        assert!(settings.plugins.contains("ad-blocker.js"));
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
