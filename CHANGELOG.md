# Changelog

## [1.0.3-pre.1] - 2026-11-21

### Added

- Add a resource limiter plugin with native CPU and RAM controls.
- Automatically install and discover bundled plugins and themes.

### Changed

- Replace the mini-player page window with WebView2 Picture-in-Picture for a
  browser-style floating video window and native controls.

### Fixed

## [1.0.2] - 2026-11-14

### Added

- Add a per-user Windows Start Menu shortcut.
- Add F12 and in-app controls for opening WebView2 Developer Tools, with a
  console warning about inspecting signed-in session data.
- Add a draggable always-on-top mini player and a resizable 2-4 stream grid.
- Keep multi-stream audio limited to the first stream.

## [1.0.1] - 2026-11-14

### Added

- Add a Twitch header settings panel for enabling plugins and selecting themes.
- Add local CSS themes, including a Catppuccin Mocha example.
- Add an application icon and GitHub Actions packaging for Windows releases.
- Persist theme and plugin preferences locally; plugins are disabled by default.

### Changed

- Restrict settings messages to Twitch origins and known local plugin/theme
  files.
- Include the changelog in the Windows release archive.

### Fixed

- Correct plugin initialization and mount the settings panel in Twitch's
  top navigation.
- Expand the example theme to cover Twitch page surfaces and controls.

## [1.0.0] - 2026-11-14

### Added

- Initial lightweight Windows Twitch client using the WebView2 runtime.
