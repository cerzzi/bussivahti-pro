# 🚌 Bussivahti Pro

A high-performance, real-time public transport tracking suite for **Tampere (Nysse)** region. This project demonstrates a powerful **Rust-based architecture** that delivers both a lightweight terminal user interface (TUI) and a modern graphical user interface (GUI) with map visualizations from a single codebase.

![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)
![Egui](https://img.shields.io/badge/GUI-egui_0.29-blue.svg)
![Ratatui](https://img.shields.io/badge/CLI-ratatui-green.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)

## ✨ Features

### 🖥️ GUI Version (Map Visualization)
* **Real-Time Map:** Visualizes bus stops on an interactive OpenStreetMap (via `walkers` crate).
* **Color-Coded Markers:** Stops change color based on departure urgency (Red < 2min, Yellow < 5min, Green > 5min).
* **Search Functionality:** Built-in Geocoding API search to find and add new stops dynamically.
* **Responsive UI:** Zoomable map and scalable UI elements for 4K/HiDPI screens.
* **Stop Details:** Hover over any marker to see a detailed schedule of upcoming departures.

### 📟 CLI Version (Terminal Dashboard)
* **Resource Efficient:** Runs comfortably on low-end hardware (e.g., Raspberry Pi Zero) via SSH.
* **ASCII Visualization:** Graphical progress bars rendered in pure text for departure times.
* **Keyboard Navigation:** Fast, shortcut-driven interface.

## 🛠️ Tech Stack

The project is structured as a Rust workspace with shared business logic (`lib.rs`) powering two distinct binaries.

### Core (Shared Logic)
* **Language:** Rust 🦀
* **Async Runtime:** `tokio` (for non-blocking API polling)
* **HTTP Client:** `reqwest` (fetching data from Digitransit GraphQL API)
* **State Management:** `Arc<RwLock>` / `Mutex` for thread-safe data sharing across async tasks.
* **Configuration:** `config-rs` (TOML-based settings).

### Interfaces
| Component | Crate / Library | Description |
|-----------|----------------|-------------|
| **GUI** | `eframe` & `egui` | Immediate Mode GUI framework (v0.29). |
| **Map** | `walkers` | OpenStreetMap rendering widget for egui. |
| **CLI** | `ratatui` | Terminal UI library for creating dashboards. |
| **Events** | `crossterm` | Cross-platform terminal event handling. |

## 🏗️ Architecture

```mermaid
graph TD
    API["Digitransit GraphQL API"] -- JSON --> Network["Network Module (Shared)"]
    Network -- "StopData Structs" --> State["Shared AppState (Arc<Mutex>)"]
    
    subgraph Binaries
        State --> GUI["GUI Binary (eframe)"]
        State --> CLI["CLI Binary (ratatui)"]
    end
    
    GUI -- "User Actions (Search/Map)" --> Network
    CLI -- "Key Events" --> Network
