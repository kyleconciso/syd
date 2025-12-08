# Syd 🦀

**Syd** sits in the background, watches your system (Volume, Battery, Wifi, etc.), and sends a clean stream of events to your app (frontend shell)

If you're building a custom desktop environment, this handles the backend plumbing so you can just focus on the UI

## How it works

*   **`syd-daemon`**: Run this in the background. It wraps standard Linux CLI tools and DBus services into one event loop
*   **`syd-client`**: Exposes `Syd::events()` stream

## Prerequisites

*   `pactl` (Audio)
*   `playerctl` (Media)
*   `brightnessctl` (Screen)
*   `nmcli` (Network)
*   `bluetoothctl` (Bluetooth)
*   `upower` (Battery)

**Build-time dependencies:**
You'll need `libgtk-4-dev` and `libdbus-1-dev` (or your distros equivalent) to compile the examples

## Setup

1. **Get the daemon running:**

```bash
git clone https://github.com/kyleconciso/syd
cd syd
cargo install --path crates/syd-daemon

# Start it up (put this in your sway/hyprland config)
syd-daemon &
```

2. **Use it in your Rust app:**

Add to `Cargo.toml`:
```toml
[dependencies]
syd-client = { path = "/path/to/syd/crates/syd-client" }
tokio = { version = "1", features = ["full"] }
```

3. **Listen for events:**

```rust
use syd_client::{Syd, SydEvent};

#[tokio::main]
async fn main() {
    let syd = Syd::connect().await.unwrap();
    let mut stream = syd.events().await;

    while let Some(event) = stream.next().await {
        match event {
            SydEvent::Volume(v) => println!("Vol: {}%", v),
            SydEvent::NetworkState(s) => println!("Wifi: {}", s),
            SydEvent::Battery(pct, state) => println!("Bat: {}% ({})", pct, state),
            // ... handle media, brightness, bluetooth, notifications, tray, etc
            _ => {}
        }
    }
}
```

## Status
It works on my machine (Arch/Hyprland)

- **Audio:** Pulse/Pipewire supported.
- **Network:** Only NetworkManager for now.
- **System:** Uses UPower for battery, systemd for reboot/shutdown.
- **Tray:** Implements the StatusNotifierItem watcher so tray icons show up

License: MIT
