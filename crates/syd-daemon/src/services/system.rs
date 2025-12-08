use zbus::{interface, object_server::SignalContext, Connection, Result};
use tokio::time::{sleep, Duration};
use tokio::process::Command;
use std::sync::Arc;

pub struct SystemService;

#[interface(name = "org.syd.System")]
impl SystemService {
    async fn get_battery(&self) -> (u32, String) {
        match get_bat_native().await {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Battery Error: {}", e);
                (0, "Error".to_string())
            }
        }
    }

    async fn power_off(&self) {
        let _ = Command::new("systemctl").arg("poweroff").output().await;
    }

    async fn reboot(&self) {
        let _ = Command::new("systemctl").arg("reboot").output().await;
    }

    async fn get_user(&self) -> String {
        std::env::var("USER").unwrap_or_else(|_| "User".into())
    }

    async fn get_power_profiles(&self) -> Vec<String> {
        if let Ok(o) = Command::new("powerprofilesctl").arg("list").env("LC_ALL", "C").output().await {
            let raw = String::from_utf8_lossy(&o.stdout);
            let mut profiles = Vec::new();
            for line in raw.lines() {
                let line = line.trim();
                if line.ends_with(':') {
                    let name = line.trim_start_matches('*').trim().trim_end_matches(':');
                    profiles.push(name.to_string());
                }
            }
            if !profiles.is_empty() { return profiles; }
        }
        vec!["balanced".into(), "power-saver".into()]
    }

    async fn get_current_profile(&self) -> String {
        if let Ok(o) = Command::new("powerprofilesctl").arg("get").env("LC_ALL", "C").output().await {
            return String::from_utf8_lossy(&o.stdout).trim().to_string();
        }
        "balanced".into()
    }

    async fn set_profile(&self, profile: String) {
        let _ = Command::new("powerprofilesctl").args(&["set", &profile]).output().await;
    }

    #[zbus(signal)]
    async fn battery_changed(&self, ctxt: &SignalContext<'_>, percent: u32, state: String) -> Result<()>;
}


#[zbus::proxy(
    interface = "org.freedesktop.UPower",
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower"
)]
trait UPower {
    fn enumerate_devices(&self) -> Result<Vec<zbus::zvariant::OwnedObjectPath>>;
}

#[zbus::proxy(
    interface = "org.freedesktop.UPower.Device",
    default_service = "org.freedesktop.UPower"
)]
trait UPowerDevice {
    #[zbus(property)] fn percentage(&self) -> Result<f64>;
    #[zbus(property)] fn state(&self) -> Result<u32>;
    #[zbus(property)] fn type_(&self) -> Result<u32>;
}

async fn get_bat_native() -> Result<(u32, String)> {
    let conn = Connection::system().await?;
    let upower = UPowerProxy::new(&conn).await?;
    
    let devices = upower.enumerate_devices().await?;

    for dev_path in devices {
        let dev = UPowerDeviceProxy::builder(&conn).path(dev_path)?.build().await?;
        
        if let Ok(type_) = dev.type_().await {
            if type_ == 2 {
                let pct = dev.percentage().await.unwrap_or(0.0) as u32;
                let state_enum = dev.state().await.unwrap_or(0);
                
                let state = match state_enum {
                    1 => "Charging",
                    2 => "Discharging",
                    3 => "Empty",
                    4 => "Fully Charged",
                    _ => "Unknown",
                };
                
                return Ok((pct, state.to_string()));
            }
        }
    }

    Ok((0, "No Battery".to_string()))
}

pub async fn monitor(conn: Connection) {
    let iface = conn.object_server().interface::<_, SystemService>("/org/syd/System").await.unwrap();
    let mut last_p = 0;
    let mut last_s = String::new();

    loop {
        let (p, s) = match get_bat_native().await {
            Ok(v) => v,
            Err(_) => (0, "Unknown".to_string()), 
        };
        
        if p != last_p || s != last_s {
            let _ = SystemService::battery_changed(&*iface.get().await, iface.signal_context(), p, s.clone()).await;
            last_p = p;
            last_s = s;
        }
        sleep(Duration::from_secs(5)).await;
    }
}