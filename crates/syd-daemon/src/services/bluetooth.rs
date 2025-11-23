use zbus::{interface, object_server::SignalContext, Connection};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use tokio::process::Command as TokioCommand;
use std::process::Command as StdCommand;
use tokio::time::{sleep, Duration};
use syd_core::BtDevice;

pub struct BluetoothService { on: Arc<AtomicBool> }
impl BluetoothService {
    pub fn new() -> (Self, Arc<AtomicBool>) { let b = Arc::new(AtomicBool::new(false)); (Self{on:b.clone()}, b) }
}
#[interface(name = "org.syd.Bluetooth")]
impl BluetoothService {
    async fn get_power(&self) -> bool { self.on.load(Ordering::Relaxed) }
    async fn set_power(&self, e: bool) { 
        let _ = StdCommand::new("bluetoothctl").args(&["power", if e{"on"}else{"off"}]).output(); 
        self.on.store(e, Ordering::Relaxed);
    }
    
    
    async fn get_devices(&self) -> Vec<BtDevice> {
        let mut devs = Vec::new();
        if let Ok(o) = StdCommand::new("bluetoothctl").arg("devices").output() {
            for line in String::from_utf8_lossy(&o.stdout).lines() {
                let p: Vec<&str> = line.split_whitespace().collect();
                if p.len() >= 3 {
                    let mac = p[1].to_string();
                    let name = p[2..].join(" ");
                    
                    let connected = if let Ok(info) = StdCommand::new("bluetoothctl").args(&["info", &mac]).output() {
                        String::from_utf8_lossy(&info.stdout).contains("Connected: yes")
                    } else { false };
                    devs.push(BtDevice { mac, name, connected });
                }
            }
        }
        devs
    }

    async fn connect_device(&self, mac: String, connect: bool) {
        let cmd = if connect { "connect" } else { "disconnect" };
        let _ = StdCommand::new("bluetoothctl").args(&[cmd, &mac]).output();
    }

    #[zbus(signal)] async fn power_changed(&self, ctxt: &SignalContext<'_>, enabled: bool) -> zbus::Result<()>;
}

pub async fn monitor(conn: Connection, cache: Arc<AtomicBool>) {
    
    let iface = conn.object_server().interface::<_, BluetoothService>("/org/syd/Bluetooth").await.unwrap();
    let mut last = false;
    loop {
        let mut curr = false;
        if let Ok(o) = TokioCommand::new("bluetoothctl").arg("show").output().await {
             curr = String::from_utf8_lossy(&o.stdout).contains("Powered: yes");
        }
        if curr != last {
             cache.store(curr, Ordering::Relaxed);
             let _ = BluetoothService::power_changed(&*iface.get().await, iface.signal_context(), curr).await;
             last = curr;
        }
        sleep(Duration::from_secs(2)).await;
    }
}
