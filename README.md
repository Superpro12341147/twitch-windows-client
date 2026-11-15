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

## GitHub releases

Push a version tag matching the version in `Cargo.toml` (for example,
`v1.0.1`) or run the workflow manually to build the Windows release. Each run
creates a **draft GitHub Release** containing the executable and a ZIP with the
README, changelog, and license. Manual runs use a unique draft tag based on the
workflow run number. Review the draft and publish it when ready.

The WebView2 browser profile, including Twitch sign-in data, is stored under
`%LOCALAPPDATA%\TwitchWindowsClient\WebView2`.

## Themes and plugins

Use **Client settings** in Twitch's top navigation bar to choose a theme and
enable or disable plugins. Changes restart the Twitch page to take effect; your
WebView2 sign-in profile is preserved.

Custom themes are `.css` files in
`%LOCALAPPDATA%\TwitchWindowsClient\themes`. The bundled `catppuccin-mocha.css`
is an example. Custom plugins are `.js` files in
`%LOCALAPPDATA%\TwitchWindowsClient\plugins`; the bundled ad blocker is off
until you explicitly enable it. Restart the app after adding or removing files.

The bundled ad blocker filters some third-party ad requests and banner elements,
but it cannot block Twitch's in-stream ads, which are delivered through the
player's video stream.

Plugins execute as Twitch page scripts and may act through your signed-in
session, so only enable code you trust. Themes are CSS-only but can still load
remote resources; use trusted stylesheets. The app does not expose Twitch
credentials to native code or add its own token store. Keep your Windows
account protected and use device encryption such as BitLocker when appropriate.
