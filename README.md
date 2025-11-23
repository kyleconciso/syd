# Syd 🦀

> **S**yd **Y**ields **D**ata.


Syd is a session daemon that sits in the background, monitors your system state (Volume, Battery, Network, Tray) and pushes updates over DBus only when things change

It’s a backend for your custom shell. You handle the UI, Syd handles the logic


## Architecture

The project is split into three crates:

*   **`syd-daemon`**: The engine, Run this in the background
*   **`syd-client`**: The Rust SDK, provides a unified `Syd::events()` stream
*   **`syd-core`**: The raw DBus protocol definitions if you want to connect using Python, JS, or Lua

## Installation

### From Source
```bash
# 1. Clone the repo
git clone https://github.com/kyleconciso/syd
cd syd

# 2. Install the daemon
cargo install --path crates/syd-daemon

# 3. Run it
syd-daemon &
```

## Building a Shell

No raw DBus handling, use the client library

Add this to your `Cargo.toml`:
```toml
[dependencies]
syd-client = { path = "../path/to/syd/crates/syd-client" } # Or version from crates.io
tokio = { version = "1", features = ["full"] }
```

**Example `main.rs`:**

```rust
use syd_client::{Syd, SydEvent};

#[tokio::main]
async fn main() {
    // Connect to the running daemon
    let syd = Syd::connect().await.unwrap();

    // The stream
    let mut events = syd.events().await;

    while let Some(event) = events.next().await {
        match event {
            SydEvent::Volume(vol) => println!("Volume is now {}%", vol),
            SydEvent::Media { title, .. } => println!("Now playing: {}", title),
            SydEvent::TrayItem(id) => println!("New Tray Icon: {}", id),
            SydEvent::Notification(n) => println!("New Notification: {}", n.summary),
            _ => {}
        }
    }
}
```

## Services Supported

- [x] **Audio** (PulseAudio/PipeWire) - Volume, Mute, Sinks, Sources
- [x] **Media** (MPRIS) - Play/Pause, Seek, Metadata (Spotify, Firefox, etc)
- [x] **Network** (NetworkManager) - Wifi scanning, connecting, and passwords
- [x] **System** (UPower/Sysfs) - Battery, Power Profiles, User info
- [x] **Bluetooth** (BlueZ) - Power, Device lists, connection toggling
- [x] **Notifications** - History, Dismissal, and popups

## License

MIT
