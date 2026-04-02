# image-viewer-v2

A minimal keyboard-driven image viewer built with Tauri v2.  
Supports ZIP archives and directories. No buttons, no toolbar.

## Usage

```bash
image-viewer-v2 /path/to/archive.zip
image-viewer-v2 /path/to/image-directory
```

## Keyboard Shortcuts

| Key | Action |
|---|---|
| `PageDown` | Scroll down / next image (at bottom) |
| `PageUp` | Scroll up / previous image (at top) |
| `Home` | First image |
| `End` | Last image |
| `[` | Previous archive or directory |
| `]` | Next archive or directory |
| `F9` | Toggle sort order (modified ↔ name) |
| `F11` | Toggle fullscreen |

## Mouse

| Action | Effect |
|---|---|
| Scroll wheel | Zoom in / out (centered on cursor) |

## Sort Modes

- **modified** (default) — sorted by modification date, newest first
- **name** — sorted alphabetically

Current sort mode is shown in the bottom-left corner along with the page index.

## Supported Formats

- ZIP archives (`.zip`)
- Directories containing images

Supported image types: `jpg`, `jpeg`, `png`, `webp`

## Build

```bash
npm run tauri build
```

## Version

2.0.1
