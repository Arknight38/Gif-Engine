# Gif-Engine Manager

![Build Status](https://github.com/Arknight38/Gif-Engine/workflows/Rust/badge.svg)
![Version](https://img.shields.io/badge/version-1.3.0-blue)
![License](https://img.shields.io/badge/license-MIT-green)

> **Your Desktop, Animated.**  
> A lightweight, high-performance manager for bringing your desktop to life with transparent GIFs and APNGs.

## About

This project took inspiration from Anima Engine, but I was unhappy with the state of Anima Engine and decided to build my own version with better performance, a multi-process architecture for stability, and an open source codebase I can grow over time.

Main Interface

<img width="1006" height="734" alt="image" src="https://github.com/user-attachments/assets/cfbb6257-f11b-4549-bbfd-55189ed11d41" />

<img width="2559" height="1439" alt="image" src="https://github.com/user-attachments/assets/6d5e950d-e137-43ab-b1b8-03c24d571030" />



---

## Quick Start (No Building Required)

**[Download Latest Release](https://github.com/Arknight38/Gif-Engine/releases/latest)** ⬇️

Just download `gif-engine.exe`, run it, and you're ready to go.

---

## Table of Contents
- [Features](#features)
- [Installation](#installation)
- [Usage Guide](#usage-guide)
- [Configuration & Storage](#configuration--storage)
- [Keyboard & Tray Shortcuts](#keyboard--tray-shortcuts)
- [Roadmap & Status](#roadmap--status)
- [Technical Deep Dive](#technical-deep-dive)
- [Troubleshooting](#troubleshooting)
- [Credits & Inspiration](#credits--inspiration)
- [Contributing](#contributing)
- [License](#license)

---

## Features

*   **Desktop Overlays**: Render transparent GIFs and APNGs directly on your screen.
*   **Performance First**: Multi-process architecture means one heavy animation won't lag the others or the manager.
*   **Customization**:
    *   **Scale**: Resize animations from tiny stickers to massive wallpapers.
    *   **Speed**: Adjust playback FPS to your liking.
    *   **Position**: Snap to corners or place them pixel-perfectly.
    *   **Visibility**: "Always on Top" mode ensures your mascots are never hidden.
*   **Library Management**:
    *   **Bulk Import**: Scan folders to add your entire collection at once.
    *   **Persistence**: Settings are saved automatically.
*   **System Integration**:
    *   **System Tray**: Stays out of your way in the notification area.
    *   **Auto-Start**: Can launch automatically when Windows starts.
    *   **Stealth Mode**: Animation windows don't clutter your taskbar.

---

## Installation

### System Requirements
- Windows 10 (build 1903+) or Windows 11
- ~100MB free disk space
- DirectX 11 compatible GPU (recommended)

### Prerequisites (for building from source)
*   **Windows 10/11**: Currently the primary supported OS.
*   **Rust**: Required to build from source. [Install Rust](https://www.rust-lang.org/tools/install).

### Building from Source

1.  **Clone the repository**:
    ```bash
    git clone https://github.com/Arknight38/Gif-Engine.git
    cd gif-engine
    ```

2.  **Build**:
    ```bash
    cargo build --release
    ```

3.  **Run**:
    The executable will be at `target/release/gif-engine.exe`.

---

## Usage Guide

### 1. Launching
Run the application. You'll see the main **Library** window.

### 2. Adding Animations
*   **Single File**: Click the folder icon to pick a `.gif` or `.apng`.
*   **Folder Scan**: Click the folder+ icon to import all compatible files from a folder.
*   **Manual Path**: Paste a file path in the bottom text box and click **Add**.

### 3. Playing Animations
*   Select an animation from the list.
*   Click **Play**.
*   The animation will appear on your desktop!

### 4. Customizing
Select an active or inactive animation to tweak its settings:
*   **Target FPS**: Speed it up or slow it down.
*   **Scale**: Make it bigger or smaller.
*   **Alignment**: Snap it to the bottom-right (classic mascot spot) or anywhere else.
*   **Overlay**: Toggle "Always on Top" if you want it to float over your browser/games.

---

## Configuration & Storage

Gif-Engine follows Windows conventions and keeps your settings and managed assets under `%APPDATA%`:

- `store.json`  
  - **Path**: `%APPDATA%\gif-engine\store.json`  
  - **What**: Library + per-GIF settings (fps, scale, alignment, etc.) and app settings (theme, minimize-to-tray).

- `running.json`  
  - **Path**: `%APPDATA%\gif-engine\running.json`  
  - **What**: Tracks currently running animation processes so the manager can clean them up or reflect running state.

- `gifs\` directory  
  - **Path**: `%APPDATA%\gif-engine\gifs\`  
  - **What**: When you add a GIF/APNG to the library, Gif-Engine **copies** it here and uses this managed copy.

### What happens if I move/delete the original files?

- After import, Gif-Engine plays from `%APPDATA%\gif-engine\gifs\...`.  
- If you move or delete the original source files, your library **still works**, because the managed copies live under `%APPDATA%`.  
- If you manually delete files from `%APPDATA%\gif-engine\gifs\`, the corresponding entries in the library may fail to load until you re-import.

---

## Keyboard & Tray Shortcuts

### Tray Icon (main manager)

The main Gif-Engine tray icon lives in the Windows notification area:

- **Show Manager**: Restores/opens the main Gif-Engine window.
- **Quit**: Cleanly exits the manager and closes all running animations it controls.

### Keyboard (inside the manager)

- Standard **egui** interactions apply (mouse-driven UI).  
- Global hotkeys are intentionally minimal right now; this section will grow as more shortcuts are added.

---

## Developer Tools

### `gen_gif.exe`
Included in the source is a utility called `gen_gif`. This tool generates a sample validation GIF (`test_assets/sample.gif`) to help test the rendering engine.

**Usage:**
```bash
cargo run --bin gen_gif
```
This is useful if you want to verify that the transparency and frame timing logic is working correctly with a known-good file.

---

## Project Structure

For contributors who want to dive into the code:

*   **`src/main.rs`**: The CLI entry point.
*   **`src/gui/`**: The Manager UI (built with `egui`).
*   **`src/playback/`**: The logic that runs inside the individual animation windows.
*   **`src/renderer/`**: Handles the raw window creation and pixel painting.
*   **`src/decoder/`**: Custom logic for parsing GIF/APNG frames efficiently.

---

## Roadmap & Status

This is an evolving project. Rough status of major features:

- **Done**
  - Multi-process architecture (manager + per-animation players)
  - Auto-copy GIFs/APNGs into `%APPDATA%\gif-engine\gifs\`
  - Basic library management (add, bulk import, delete)
  - Overlay/always-on-top transparent windows
  - Single central tray icon for the manager

- **In Progress / Planned**
  - Scenes / profiles (save multiple layouts of running animations)
  - Tags and search for large libraries
  - Global hotkeys (Pause all / Resume all / Toggle visibility)
  - More advanced animation behaviors (simple motion, opacity control)

Want to help? Check out the [`enhancement` issues](https://github.com/Arknight38/Gif-Engine/issues?q=is%3Aissue+is%3Aopen+label%3Aenhancement) for ideas.

---

## Technical Deep Dive

This section explains the architecture and design decisions behind Gif-Engine for those interested in the internals.

### Multi-Process Architecture
Gif-Engine uses a multi-process architecture to ensure stability and performance.
*   **Manager Process**: The main GUI application (the "Library"). It handles user interaction, configuration (stored in `store.json`), and process management.
*   **Child Processes**: Each animation runs in a completely separate process.
    *   *Why?* If one animation crashes or hangs (e.g., due to a malformed GIF), it doesn't affect the manager or other animations. It also allows the OS to schedule CPU time more effectively across cores.
    *   *Communication*: The manager launches child processes with specific CLI arguments (e.g., `--file path/to/gif --x 100 --y 100`).

### Rendering Pipeline
Rendering transparent windows on Windows is non-trivial. We use the **Windows API (Win32)** directly for window management.
1.  **Window Creation**: We create a layered window (`WS_EX_LAYERED`) which supports per-pixel alpha blending.
2.  **Decoding**: We use the `image` crate and custom logic to decode GIF/APNG frames into an RGBA buffer.
3.  **Compositing**: Frames are composited onto a `FrameBuffer`. This handles "disposal methods" (how the previous frame is cleared before drawing the next one).
4.  **Presentation**: The final buffer is blitted to the window using GDI functions (`UpdateLayeredWindow`), which preserves the alpha channel for true desktop transparency.

### State Management
*   **Store**: Application state is persisted in `store.json` using `serde`. This file is stored in `%APPDATA%\gif-engine` (e.g., `C:\Users\YourName\AppData\Roaming\gif-engine`), following Windows standards. This ensures your settings are safe and separate from the executable.
*   **GIF Storage**: When you add a GIF or APNG to the library, it is automatically copied to `%APPDATA%\gif-engine\gifs\` directory. This ensures your animations are preserved even if the original files are moved or deleted, and keeps everything organized in one location alongside the configuration files.
*   **Running Processes**: Active animation windows are tracked in `running.json`, allowing the manager to know which animations are currently alive and to clean them up if needed.
*   **Concurrency**: Shared state within the manager is protected by `Arc<Mutex<...>>` (or `RwLock`), allowing the GUI thread and background tasks (like the tray icon handler) to access data safely.

### GUI Framework
The manager UI is built with **egui** / **eframe**, an immediate mode GUI stack for Rust.
*   It's lightweight and embeds directly into the application executable.
*   It allows for rapid iteration on the UI layout without complex XAML/HTML/CSS boilerplate.

---

## Troubleshooting

**Q: Animations aren't showing up**  
**A:** Make sure the animation is set to **Overlay / Always on Top** in the settings, and that it isn't hidden behind full-screen applications that capture the entire display.

**Q: Performance issues (high CPU usage)?**  
**A:** Try:
- Reducing the **Target FPS** for busy animations.
- Lowering **Scale** for very large GIFs.
- Limiting the number of simultaneously running animations.

**Q: My GIF disappeared after I moved the original file**  
**A:** The manager uses its own copy under `%APPDATA%\gif-engine\gifs\`. If that copy was deleted manually, re-add the GIF through the Library.

---

## Credits & Inspiration

Gif-Engine wouldn't exist without the awesome Rust ecosystem and prior work:

- **Inspiration**
  - Anima Engine – the project that inspired this tool and motivated a new, more maintainable take.

- **Key crates**
  - `egui`, `eframe` – for the GUI.
  - `winit`, `softbuffer` – for windowing and rendering surfaces.
  - `image`, `gif`, `png` – for decoding image formats.
  - `tray-icon`, `menu_rs` – for the system tray integration.
  - `serde`, `serde_json` – for configuration and state persistence.

---

## Contributing

We welcome contributions! Whether it's fixing bugs, adding features, or improving documentation:

1.  Fork the project.
2.  Create your feature branch (`git checkout -b feature/AmazingFeature`).
3.  Commit your changes (`git commit -m 'Add some AmazingFeature'`).
4.  Push to the branch (`git push origin feature/AmazingFeature`).
5.  Open a Pull Request.

If you're looking for ideas, check the [`good first issue`](https://github.com/Arknight38/Gif-Engine/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22) and [`enhancement`](https://github.com/Arknight38/Gif-Engine/issues?q=is%3Aissue+is%3Aopen+label%3Aenhancement) labels.

---

## License

This project is licensed under the **MIT License**. See the `LICENSE` file for details.
