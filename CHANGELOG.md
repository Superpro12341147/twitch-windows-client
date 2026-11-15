# Changelog

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

### Known limitations

- The example ad blocker filters some third-party ad requests and banner
  elements, but does not block Twitch in-stream ads.

## [1.0.0] - 2026-11-14

### Added

- Initial lightweight Windows Twitch client using the WebView2 runtime.
