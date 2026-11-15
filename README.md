# Twitch Windows Client

A lightweight Rust desktop shell that opens the official Twitch website in a
native Windows window. Twitch provides the browsing, streaming, chat, and
account features; the client does not require developer-provided Twitch API
credentials.

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
On first run, the app adds a shortcut to your per-user Windows Start Menu.

## GitHub releases

Push a version tag matching the version in `Cargo.toml` (for example,
`v1.0.2`) or run the workflow manually to build the Windows release. Each run
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
until you explicitly enable it. Its implementation is the
`%LOCALAPPDATA%\TwitchWindowsClient\plugins\ad-blocker.js` script. On upgrade,
an unchanged bundled script is updated; customized plugin files are left
untouched.

When enabled, the bundled ad blocker uses WebView2's native
`WebResourceRequested` interception to return an empty response for known
client-side ad hosts. It deliberately does not block Twitch's GraphQL,
usher/HLS playlist, or media-segment requests. The player-worker hook handles
stitched HLS ads separately: it identifies marked ad segments and, when
available, substitutes only a same-time, same-quality live segment from an
alternate Twitch player playlist. It preserves the original playlist's
sequence, timestamps, map, and encryption tags; if it cannot find a safe
match, it leaves the segment alone rather than risking a stalled player. This
is best-effort: Twitch changes its ad delivery, and no client-side blocker can
promise to remove every ad without any playback risk.

Use **Picture-in-Picture** in Client settings to pop the playing video out into
WebView2's native floating video window. It stays above other windows and
provides the browser's built-in Picture-in-Picture controls. Enter 2-4
comma-separated channel names in the multi-stream field to open them together
in a resizable grid window; audio is enabled for the first stream only.
until you explicitly enable it. Restart the app after adding or removing files.
On upgrade, an unchanged copy of the previous bundled ad blocker is updated;
customized plugin files are left untouched.

The bundled ad blocker filters third-party ad requests and banner elements. It detects
ad markers in video playlists, tries alternate Twitch player playlists, and
falls back to removing marked ad segments. Twitch may change its player or ad
delivery at any time, so stream-ad blocking is not guaranteed.

Use **Pop out current stream** in Client settings on a channel page to open a
draggable, always-on-top mini player. Enter 2-4 comma-separated channel names
in the multi-stream field to open them together in a resizable grid window;
audio is enabled for the first stream only.

Press **F12** or choose **Inspect page** in Client settings to open Developer
Tools. The console logs a warning because DevTools can inspect page content and
your signed-in session. Do not copy or share cookies or tokens.

Plugins execute as Twitch page scripts and may act through your signed-in
session, so only enable code you trust. Themes are CSS-only but can still load
remote resources; use trusted stylesheets. The app does not expose Twitch
credentials to native code or add its own token store. Keep your Windows
account protected and use device encryption such as BitLocker when appropriate.
