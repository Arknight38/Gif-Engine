# Gif-Engine Manager

> **Your Desktop, Animated.**  
> A lightweight, high-performance manager for bringing your desktop to life with transparent GIFs and APNGs.

Main Interface

<img width="1005" height="734" alt="image" src="https://github.com/user-attachments/assets/ce029d6b-4f2d-456d-b8fa-7e5d3758f7f8" />


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

### Prerequisites
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
*   **Store**: Application state is persisted in `store.json` using `serde`. This includes the list of imported GIFs and their individual settings (scale, FPS, position).
*   **Concurrency**: Shared state within the manager is protected by `Arc<Mutex<...>>` (or `RwLock`), allowing the GUI thread and background tasks (like the tray icon handler) to access data safely.

### GUI Framework
The manager UI is built with **egui**, an immediate mode GUI library for Rust.
*   It's lightweight and embeds directly into the application executable.
*   It allows for rapid iteration on the UI layout without complex XAML/HTML/CSS boilerplate.

---

## Contributing

We welcome contributions! Whether it's fixing bugs, adding features, or improving documentation.

1.  Fork the Project
2.  Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3.  Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4.  Push to the Branch (`git push origin feature/AmazingFeature`)
5.  Open a Pull Request
