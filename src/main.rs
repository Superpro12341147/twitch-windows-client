#[cfg(target_os = "windows")]
fn main() {
    if let Err(error) = run() {
        eprintln!("Twitch could not start: {error}");
        std::process::exit(1);
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
    let webview_data_dir = PathBuf::from(local_app_data)
        .join("TwitchWindowsClient")
        .join("WebView2");
    fs::create_dir_all(&webview_data_dir)?;

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Twitch")
        .with_inner_size(tao::dpi::LogicalSize::new(1280.0, 800.0))
        .with_min_inner_size(tao::dpi::LogicalSize::new(800.0, 500.0))
        .build(&event_loop)?;

    let mut web_context = WebContext::new(Some(webview_data_dir));
    let _webview = WebViewBuilder::new_with_web_context(&mut web_context)
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

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("This Twitch client currently supports Windows only.");
}
