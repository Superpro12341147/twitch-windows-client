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
#[derive(serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum UiMessage {
    SetPlugin { name: String, enabled: bool },
    SetTheme { name: Option<String> },
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
#[derive(Clone, Copy)]
enum AppEvent {
    ReloadSettings,
}

#[cfg(target_os = "windows")]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    use std::{env, fs, io, path::PathBuf};
    use tao::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoopBuilder},
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
    ensure_example(
        &themes_dir,
        "catppuccin-mocha.css",
        include_str!("../themes/catppuccin-mocha.css"),
    )?;

    let settings_path = app_data_dir.join("settings.json");
    let plugins = load_plugins(&plugins_dir)?;
    let themes = load_themes(&themes_dir)?;
    let settings = load_settings(&settings_path)?;

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

    event_loop.run(move |event, _, control_flow| {
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
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
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
            window.__twitchClientConfig = {config};\n\
            {ui}\n\
        }})();",
        config = config,
        ui = include_str!("../assets/settings.js"),
    );

    let mut builder = WebViewBuilder::new_with_web_context(web_context)
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

    Ok(builder.with_url("https://www.twitch.tv/").build(window)?)
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
    use super::{apply_ui_message, Settings, UiMessage};
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
}

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("This Twitch client currently supports Windows only.");
}
