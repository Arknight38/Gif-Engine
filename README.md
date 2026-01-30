# GifEngine Manager

A lightweight, high-performance desktop manager built in Rust for playing transparent GIFs and APNGs as desktop overlays and mascots.

## Demonstration

![Main Interface Placeholder](docs/images/interface.png)
*The main manager interface for organizing your collection.*

![Desktop Overlay Placeholder](docs/images/overlay_demo.gif)
*Transparent animations running on the desktop.*

## Features

*   **Transparent Rendering**: Seamlessly renders GIF and APNG animations with transparency directly on your desktop.
*   **Desktop Overlay**: Windows are borderless, transparent, and can be set to "Always on Top" (Overlay mode).
*   **Multi-Process Architecture**: Each animation runs in its own isolated process, ensuring the main manager remains responsive.
*   **Library Management**:
    *   **Bulk Import**: Scan entire directories for compatible animation files.
    *   **Drag & Drop**: (Supported via path input)
    *   **Persistence**: Your library and configuration are saved automatically.
*   **Customization**:
    *   **Positioning**: Presets (Top-Left, Center, etc.) or precise Custom X/Y coordinates.
    *   **Scaling**: Resize animations from 0.1x to 2.0x.
    *   **Frame Rate Control**: Override the native FPS of any GIF.
    *   **Monitor Selection**: Support for multi-monitor setups.
*   **System Integration**:
    *   **Run on Startup**: Option to automatically launch the manager and your animations when Windows starts.
    *   **System Tray**: Minimizes to the system tray to keep your taskbar clean.
    *   **Stealth Mode**: Animation windows do not appear in the taskbar or Alt-Tab menu.
*   **Theming**: Includes Dark and Light themes.

## Installation

### Prerequisites

*   **Rust**: Ensure you have the latest stable version of Rust installed. [Install Rust](https://www.rust-lang.org/tools/install)
*   **OS**: Windows 10/11 (Primary target for windowing features).

### Building from Source

1.  Clone the repository:
    ```bash
    git clone https://github.com/yourusername/gif-engine.git
    cd gif-engine
    ```

2.  Build the project in release mode:
    ```bash
    cargo build --release
    ```

3.  The executable will be located in `target/release/gif-engine.exe`.

## Usage

### Getting Started

1.  Run the application:
    ```bash
    cargo run --release
    ```
2.  **Add Animations**:
    *   Click the **Folder Icon (📂)** to select a single `.gif`, `.png`, or `.apng` file.
    *   Click the **Scan Icon (📁+)** to import all compatible files from a specific directory.
    *   Alternatively, paste a file path into the text box and press Enter.

### Managing Animations

*   **Play**: Select an animation from the library list and click **▶ Play**.
*   **Stop**: Click **⏹ Stop** to close the animation window.
*   **Restart**: Click **🔄 Restart** to apply new settings to a running animation.
*   **Delete**: Click **🗑 Delete** to remove the animation from your library.

### Configuration

Select an animation to view its settings in the right panel:
*   **Overlay**: Check "Always on top" to keep the animation above other windows.
*   **Target FPS**: Adjust the playback speed.
*   **Scale**: Resize the animation.
*   **Alignment**: Snap to screen corners or use **Custom** for manual X/Y positioning.

### Global Settings

Click **⚙ Settings** at the bottom of the sidebar to:
*   Enable/Disable **Run on Startup**.
*   Switch between **Dark** and **Light** themes.
*   **Clean Dead Processes**: Remove zombie entries if the app wasn't closed properly.
*   **Reset Library**: Clear all data (Danger Zone).

## Project Structure

*   `src/main.rs`: Entry point and CLI argument parsing.
*   `src/gui/`: Main application GUI logic (egui).
*   `src/tui/`: Backend logic, store management, and process handling.
*   `src/renderer/`: Window creation and frame rendering logic.
*   `src/decoder/`: GIF/APNG decoding and caching.
*   `src/cache/`: Frame buffer management.

## Contributing

Contributions are welcome! Please follow these steps to contribute:

1.  **Fork the Repository**: Create a fork of the project on GitHub.
2.  **Create a Feature Branch**:
    ```bash
    git checkout -b feature/AmazingFeature
    ```
3.  **Commit your Changes**:
    ```bash
    git commit -m 'Add some AmazingFeature'
    ```
4.  **Push to the Branch**:
    ```bash
    git push origin feature/AmazingFeature
    ```
5.  **Open a Pull Request**: Submit a pull request to the `main` branch.

### Guidelines
*   Ensure your code adheres to the existing style (run `cargo fmt`).
*   Check for any warnings or errors (run `cargo check`).
*   If adding new features, please include a brief description in your PR.

## License

[MIT License](LICENSE)
