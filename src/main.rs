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
fn run() -> Result<(), Box<dyn std::error::Error>> {
    use std::{env, fs, io, path::PathBuf};
    use tao::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::WindowBuilder,
    };
    use wry::{WebContext, WebViewBuilder};

    let local_app_data = env::var_os("LOCALAPPDATA")
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "LOCALAPPDATA is not set"))?;
    let app_data_dir = PathBuf::from(local_app_data).join("TwitchWindowsClient");
    let webview_data_dir = app_data_dir.join("WebView2");
    fs::create_dir_all(&webview_data_dir)?;

    let plugins_dir = app_data_dir.join("plugins");
    match fs::create_dir(&plugins_dir) {
        Ok(()) => fs::write(
            plugins_dir.join("ad-blocker.js"),
            include_str!("../plugins/ad-blocker.js"),
        )?,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.into()),
    }
    let plugins = load_plugins(&plugins_dir)?;

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Twitch")
        .with_inner_size(tao::dpi::LogicalSize::new(1280.0, 800.0))
        .with_min_inner_size(tao::dpi::LogicalSize::new(800.0, 500.0))
        .build(&event_loop)?;

    let mut web_context = WebContext::new(Some(webview_data_dir));
    let mut webview_builder = WebViewBuilder::new_with_web_context(&mut web_context);
    for (_, source) in plugins {
        let script = format!(
            "(() => {{\n\
                if (window.top !== window) return;\n\
                if (location.hostname !== 'twitch.tv' && !location.hostname.endsWith('.twitch.tv')) return;\n\
                {source}\n\
            }})();"
        );
        webview_builder = webview_builder.with_initialization_script(script);
    }
    let _webview = webview_builder
        .with_url("https://www.twitch.tv/")
        .build(&window)?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
            *control_flow = ControlFlow::Exit;
        }
    });
}

#[cfg(target_os = "windows")]
fn load_plugins(plugin_dir: &std::path::Path) -> std::io::Result<Vec<(String, String)>> {
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
        plugins.push((entry.file_name().to_string_lossy().into_owned(), source));
    }

    plugins.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(plugins)
}

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("This Twitch client currently supports Windows only.");
}
