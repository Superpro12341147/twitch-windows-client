# Twitch Windows Client

A lightweight Rust desktop shell that opens the official Twitch website in a
native Windows window. Twitch provides the browsing, streaming, chat, and
account features; the client does not use Twitch API credentials.

## Requirements

- Windows 10 or later with the Microsoft Edge WebView2 Runtime
- Rust stable with the MSVC toolchain
- Visual Studio C++ Build Tools

## Run

```powershell
cargo run
```

Build an optimized executable with:

```powershell
cargo build --release
```

The standalone GUI executable is
`target\release\twitch-windows-client.exe`. It opens without a terminal window;
startup errors are shown in a dialog.

The WebView2 browser profile, including Twitch sign-in data, is stored under
`%LOCALAPPDATA%\TwitchWindowsClient\WebView2`.

## Plugins

On first launch, the example ad-blocker plugin is copied to
`%LOCALAPPDATA%\TwitchWindowsClient\plugins`. The client loads `.js` files in
that folder in filename order at startup. Add or edit a plugin there, then
restart the client. Remove `ad-blocker.js` to disable the example.

Plugins are local JavaScript injected into Twitch pages and can access page
content and session data. Only install scripts you trust. The example blocks
requests to several common third-party advertising domains and hides matching
banner elements; it does not block Twitch's in-stream video ads.
