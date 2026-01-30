# Gif-Engine

![Build Status](https://github.com/Arknight38/Gif-Engine/workflows/Rust/badge.svg)
![Version](https://img.shields.io/badge/version-1.3.0-blue)
![License](https://img.shields.io/badge/license-MIT-green)

**Desktop animation manager for Windows - built for performance and stability.**

## About

I started this project because I wanted something better than Anima Engine. The concept was great, but I ran into performance issues and wanted more control over how it worked. So I rebuilt it from scratch in Rust with a focus on stability through multi-process architecture (still working on making it bulletproof) and keeping the codebase open for anyone to contribute to or learn from.

[Screenshots]

<img width="1006" height="734" alt="image" src="https://github.com/user-attachments/assets/cfbb6257-f11b-4549-bbfd-55189ed11d41" />

<img width="2559" height="1439" alt="image" src="https://github.com/user-attachments/assets/6d5e950d-e137-43ab-b1b8-03c24d571030" />

## Quick Start

**Just want to use it?** [Download the latest release](https://github.com/Arknight38/Gif-Engine/releases/latest) and run the exe.

**Building from source:**
```bash
git clone https://github.com/Arknight38/Gif-Engine.git
cd gif-engine
cargo build --release
```

Executable will be in `target/release/gif-engine.exe`.

**Requirements:** Windows 10 (build 1903+) or Windows 11, ~100MB disk space

---

## Features

- **Desktop overlays** - Transparent GIFs and APNGs rendered directly on your screen
- **Multi-process architecture** - Each animation runs independently, so crashes stay isolated
- **Full customization** - Scale, speed, position, and layer control for every animation
- **Smart library management** - Bulk import with automatic file copying to prevent broken references
- **System integration** - Runs from the tray, optional auto-start, windows stay out of your taskbar

---

## Using Gif-Engine

### Adding animations
Click the folder icon for single files, or use the folder+ icon to scan entire directories. Animations get automatically copied to `%APPDATA%\gif-engine\gifs\`, so you can reorganize your original files without breaking anything.

### Playback and customization
Select any animation and hit Play. From there you can adjust:
- Target FPS for speed control
- Scale for sizing
- Alignment for positioning
- Always on Top for layering

Settings persist automatically between sessions.

---

## File Storage

Everything lives in `%APPDATA%\gif-engine\`:
- `store.json` - Library entries and settings
- `running.json` - Tracks active animation processes
- `gifs\` - Managed copies of your animations

This means your library stays intact even if you move or delete original files. The app works with its own managed copies.

---

## Current Development

This project is actively evolving. Here's where things stand:

**Working now:**
- Multi-process playback system
- Library management with bulk import
- Transparent overlay rendering
- System tray integration

**In progress:**
- Scene/profile system for saving layouts
- Tagging and search for large libraries
- Global hotkeys for pause/resume
- Advanced animation controls

Want to contribute? Check out issues tagged [`enhancement`](https://github.com/Arknight38/Gif-Engine/issues?q=is%3Aissue+is%3Aopen+label%3Aenhancement) or [`good first issue`](https://github.com/Arknight38/Gif-Engine/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22).

---

## Technical Overview

### Architecture
The manager process handles the UI and configuration while spawning separate processes for each animation. This isolation means a problematic GIF won't take down your entire session - only its own window crashes.

### Rendering
Windows are created with `WS_EX_LAYERED` for per-pixel alpha blending. Frames get decoded through the `image` crate, composited to handle disposal methods correctly, then presented via `UpdateLayeredWindow` for proper transparency.

### Stack
Built with `egui`/`eframe` for the UI, standard Windows API for window management, and `serde` for state persistence. The full dependency list is in `Cargo.toml`.

---

## Troubleshooting

**Animations not appearing?**  
Enable "Always on Top" in the animation settings. Some fullscreen applications may still cover them.

**Performance issues?**  
Try reducing target FPS or scale for resource-heavy animations. Running many animations simultaneously will increase CPU usage.

**Missing animations after file reorganization?**  
The app uses copies from `%APPDATA%\gif-engine\gifs\`. If you manually deleted files from there, you'll need to re-import them.

---

## Contributing

I'm building this to be something people actually want to use, so feedback and contributions are genuinely appreciated.

Standard process:
1. Fork the repository
2. Create a feature branch (`git checkout -b feature/YourFeature`)
3. Commit your changes
4. Push and open a PR

---

## Road map/Planned Features

- [ ] Scene profiles (save and switch between different animation layouts)
- [ ] Tags and search for the library
- [ ] Global hotkeys (pause all, resume all, toggle visibility)
- [ ] Per-animation opacity slider
- [ ] Option to hide animations when a window is fullscreen
- [ ] Better preview (scrub timeline, show FPS and resolution)
- [ ] Group animations (folders or collections in the library)
- [ ] Export / import library and settings
- [ ] Optional FPS cap for all animations at once
- [ ] Basic logging window for errors (failed loads, crashes, etc.)
- [ ] More configuration for auto-start behavior
- [ ] Add a “minimal CPU mode” preset
- [ ] Support for more formats (e.g. WebP) if it makes sense
- [ ] Simple in-app “What’s new” / changelog panel

---

## Credits

This wouldn't exist without the Rust ecosystem. Key dependencies include `egui`, `winit`, `image`, and `tray-icon`. Full credit list in `Cargo.toml`.

Original inspiration from Anima Engine, though this is a complete rewrite.

---

If you find this useful, consider giving it a star ⭐ - it helps others discover the project.

---

## Project Stats

![GitHub stars](https://img.shields.io/github/stars/Arknight38/Gif-Engine?style=social)
![GitHub forks](https://img.shields.io/github/forks/Arknight38/Gif-Engine?style=social)
![GitHub issues](https://img.shields.io/github/issues/Arknight38/Gif-Engine)
![GitHub pull requests](https://img.shields.io/github/issues-pr/Arknight38/Gif-Engine)
![GitHub last commit](https://img.shields.io/github/last-commit/Arknight38/Gif-Engine)
![GitHub repo size](https://img.shields.io/github/repo-size/Arknight38/Gif-Engine)
![Lines of code](https://img.shields.io/tokei/lines/github/Arknight38/Gif-Engine)
![GitHub downloads](https://img.shields.io/github/downloads/Arknight38/Gif-Engine/total)

### Contributors

Thanks to everyone who has contributed to this project!

[![Contributors](https://contrib.rocks/image?repo=Arknight38/Gif-Engine)](https://github.com/Arknight38/Gif-Engine/graphs/contributors)

### Star History

[![Star History Chart](https://api.star-history.com/svg?repos=Arknight38/Gif-Engine&type=Date)](https://star-history.com/#Arknight38/Gif-Engine&Date)

---

## License

MIT License - see LICENSE file for details.
