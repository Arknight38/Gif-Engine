# Gif-Engine

![Build Status](https://github.com/Arknight38/Gif-Engine/workflows/Rust/badge.svg)
![Version](https://img.shields.io/badge/version-2.0.0-blue)
![License](https://img.shields.io/badge/license-MIT-green)

**Desktop animation manager for Windows - built for performance and stability.**

## About

I started this project because I wanted something better than Anima Engine. The concept was great, but I ran into performance issues and wanted more control over how it worked and looked. So I rebuilt it from scratch in Rust with a focus on stability through multi-process architecture (still working on making it bulletproof) and keeping the codebase open for anyone to contribute to or learn from.

## Screenshots

<img width="1002" height="736" alt="image" src="https://github.com/user-attachments/assets/6f3419a2-5d38-42b1-a154-c6a720cb5368" />

<img width="2559" height="1439" alt="image" src="https://github.com/user-attachments/assets/8b29ccb7-0ae5-46f1-8dec-5f93e41c39d7" />

<p align="center">
  <img src="https://github.com/user-attachments/assets/55d98732-69e1-40e6-a611-964de794a1bf" alt="demo">
</p>

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

- **Community Hub** - Browse and download animation packs directly from the app
- **Broad Format Support** - Supports GIF, APNG, WebP, PNG, JPG, and more
- **Desktop Overlays** - Transparent animations rendered directly on your screen
- **Multi-process Architecture** - Each animation runs independently for maximum stability
- **Advanced Window Anchoring** - "Pet Mode": Attach animations to specific windows (they follow the window and stay just above it)
- **Stealth Mode** - Animations are hidden from the taskbar to keep your workspace clean
- **Behaviors** - Make your animations Bounce or Wander around the screen
- **Interactive Mode** - Click-through by default, but hold **Ctrl** or **Alt** to drag/interact
- **Smart Library** - Bulk import, tags, search, and automatic file management
- **System Integration** - System tray support, auto-start, and hotkeys

---

## Using Gif-Engine

### Adding animations
Click the folder icon for single files, or use the folder+ icon to scan entire directories. 
Supported formats: **GIF, APNG, WebP, PNG, JPG**.
Animations get automatically copied to `%APPDATA%\gif-engine\gifs\`, so you can reorganize your original files without breaking anything.

### Community Hub
Discover new animations in the **Community** tab! 
- Browse a growing collection of user-submitted packs
- One-click download and install
- Search by name, author, or tags
- Automatically adds downloaded packs to your library

### Organizing with tags
Tag your animations for easy organization! In the animation settings, you can:
- Add tags by typing in the tag field and pressing Enter or clicking the + button
- Remove tags by clicking the ✖ button on any tag
- Tags are displayed below animation names in the library list

### Searching your library
Use the search bar at the top of the library panel to quickly find animations:
- Search by animation name (case-insensitive)
- Search by tags
- Results update in real-time as you type
- Click the ✖ button to clear your search

### Playback and customization
Select any animation and hit Play. From the settings panel you can adjust:
- **Anchoring**: Select a running application to attach the animation to (it will move with the window!)
- **Behavior**: Choose between **None** (Static), **Bounce** (DVD screensaver style), or **Wander** (Random movement)
- **Target FPS**: Control speed and CPU usage
- **Scale**: Resize without losing quality
- **Alignment**: Position relative to screen or anchored window
- **Always on Top**: Force the animation to stay above everything else

### Interactive Mode (Click-Through)
By default, animations are "click-through" - you can click on things behind them.
- **Hold Ctrl or Alt**: Temporarily enables interaction. You can now drag the animation to move it.
- **Right-Click (while holding Ctrl/Alt)**: Open the context menu to Pause/Stop/Settings.

### Managing active animations
Switch to the **Active** tab to see all running animations:
- View all active animations with their process IDs
- See runtime for each animation
- Stop individual animations
- Running animations are also marked in the library with a ▶ indicator

<img width="588" height="330" alt="image" src="https://github.com/user-attachments/assets/f67f0867-c62b-4de0-a22e-b69aca2a9237" />

### Export and Import
Backup your entire library or share it with others:

**Export:**
- Click "📤 Export Library & Settings" in the Settings panel
- Creates a ZIP file containing all your animations and settings
- Perfect for backups or sharing with friends
- Works across different devices - paths are automatically updated on import

**Import:**
- Click "📥 Import Library & Settings" in the Settings panel
- Select a ZIP file exported from Gif-Engine
- Animations are extracted to your local gifs directory
- Settings and library entries are merged with your existing data
- All file paths are automatically updated to work on your system

<img width="589" height="387" alt="image" src="https://github.com/user-attachments/assets/3c662c00-735e-41f7-bfbb-9ff72bfd9801" />

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
- Tags and search functionality
- Active animations management
- Export/Import library and settings (ZIP format)
- Community Pack Browser
- Window Anchoring & Behaviors

Want to contribute? Check out issues tagged [`enhancement`](https://github.com/Arknight38/Gif-Engine/issues?q=is%3Aissue+is%3Aopen+label%3Aenhancement) or [`good first issue`](https://github.com/Arknight38/Gif-Engine/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22).

---

## Technical Overview

### Architecture
The manager process handles the UI and configuration while spawning separate processes for each animation. This isolation means a problematic GIF won't take down your entire session - only its own window crashes.

### Rendering
Windows are created with `WS_EX_LAYERED` for per-pixel alpha blending and `WS_EX_TOOLWINDOW` to stay hidden from the taskbar. Frames get decoded through the `image` crate, composited to handle disposal methods correctly, then presented via `UpdateLayeredWindow` for proper transparency.

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

**Want to share your library with someone else?**  
Use the Export feature in Settings to create a ZIP file containing all your animations and settings. The recipient can use Import to add everything to their library - paths are automatically updated for their system.

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
- [x] Tags and search for the library
- [x] Global hotkeys (pause all, resume all, toggle visibility)
- [x] Export / import library and settings
- [ ] Basic logging window for errors (failed loads, crashes, etc.)
- [x] Add a “minimal CPU mode” preset
- [x] Support for more formats (WebP, PNG, JPG)
- [x] Window Anchoring (Pet Mode)
- [x] Community Pack Browser

---

## Credits

This wouldn't exist without the Rust ecosystem. Key dependencies include `egui`, `winit`, `image`, and `tray-icon`. Full credit list in `Cargo.toml`.

Original inspiration from Anima Engine, though this is a complete new codebase.

---

If you find this useful, consider giving it a star ⭐ - it helps others discover the project.

---

## Project Stats

![GitHub stars](https://img.shields.io/github/stars/Arknight38/Gif-Engine?style=social)
![GitHub forks](https://img.shields.io/github/forks/Arknight38/Gif-Engine?style=social)
![GitHub issues](https://img.shields.io/github/issues/Arknight38/Gif-Engine)
![GitHub pull requests](https://img.shields.io/github/issues-pr/Arknight38/Gif-Engine)
![GitHub last commit](https://img.shields.io/github/last-commit/Arknight38/Gif-Engine)

### Contributors

Thanks to everyone who has contributed to this project!

[![Contributors](https://contrib.rocks/image?repo=Arknight38/Gif-Engine)](https://github.com/Arknight38/Gif-Engine/graphs/contributors)

### Star History

[![Star History Chart](https://api.star-history.com/svg?repos=Arknight38/Gif-Engine&type=Date)](https://star-history.com/#Arknight38/Gif-Engine&Date)

---

## License

MIT License - see LICENSE file for details.
