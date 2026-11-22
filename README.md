# Twitch Windows Client

A lightweight Rust desktop shell that opens the official Twitch website in a
native Windows window. Twitch provides the browsing, streaming, chat, and
account features; the client does not require developer-provided Twitch API
credentials.

## Requirements

- Windows 10 or later with the Microsoft Edge WebView2 Runtime
- Rust stable with the MSVC toolchain
- Visual Studio C++ Build Tools

## Themes and plugins

Use **Client settings** in Twitch's top navigation bar to choose a theme and
enable or disable plugins. Changes restart the Twitch page to take effect; your
WebView2 sign-in profile is preserved.

Custom themes are `.css` files in
`%LOCALAPPDATA%\TwitchWindowsClient\themes`. The bundled `catppuccin-mocha.css`
is installed automatically. Bundled `.js` plugins and `.css` themes are
installed and discovered automatically; plugins remain disabled until enabled
in Client settings, and themes remain selectable there. Custom plugins are
`.js` files in
`%LOCALAPPDATA%\TwitchWindowsClient\plugins`.

Use **Picture-in-Picture** in Client settings to pop the playing video out into
WebView2's native floating video window. It stays above other windows and
provides the browser's built-in Picture-in-Picture controls. Enter 2-4
comma-separated channel names in the multi-stream field to open them together
in a resizable grid window; audio is enabled for the first stream only.

Use **Pop out current stream** in Client settings on a channel page to open a
draggable, always-on-top mini player. Enter 2-4 comma-separated channel names
in the multi-stream field to open them together in a resizable grid window;
audio is enabled for the first stream only.

Press **F12** or choose **Inspect page** in Client settings to open Developer
Tools. The console logs a warning because DevTools can inspect page content and
your signed-in session. Do not copy or share cookies or tokens.

The built-in **Resource limiter** plugin can apply CPU and RAM caps to the
Windows client process through Client settings. It is installed automatically
in `%LOCALAPPDATA%\TwitchWindowsClient\plugins` and may be enabled or disabled
like other plugins. WebView2 child processes may remain outside the client
process job.

Plugins execute as Twitch page scripts and may act through your signed-in
session, so only enable code you trust. Themes are CSS-only but can still load
remote resources; use trusted stylesheets. The app does not expose Twitch
credentials to native code or add its own token store. Keep your Windows
account protected and use device encryption such as BitLocker when appropriate.
