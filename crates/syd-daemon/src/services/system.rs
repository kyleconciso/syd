use zbus::{interface, object_server::SignalContext, Connection};
use tokio::time::{sleep, Duration};
use std::process::Command as StdCommand;

pub struct SystemService;
#[interface(name = "org.syd.System")]
impl SystemService {
    
    async fn get_battery(&self) -> (u32, String) { get_bat_real().await }
    
    async fn power_off(&self) { let _ = StdCommand::new("poweroff").output(); }
    async fn reboot(&self) { let _ = StdCommand::new("reboot").output(); }
    
    async fn get_user(&self) -> String {
        std::env::var("USER").unwrap_or_else(|_| "User".into())
    }

    async fn get_power_profiles(&self) -> Vec<String> {
        let mut profiles = Vec::new();
        if let Ok(o) = StdCommand::new("powerprofilesctl").arg("list").output() {
            let raw = String::from_utf8_lossy(&o.stdout);
            for line in raw.lines() {
                let line = line.trim();
                if line.ends_with(':') {
                    let name = line.trim_start_matches('*').trim().trim_end_matches(':');
                    profiles.push(name.to_string());
                }
            }
        }
        if profiles.is_empty() {
            vec!["balanced".into(), "power-saver".into()]
        } else {
            profiles
        }
    }

    async fn get_current_profile(&self) -> String {
        if let Ok(o) = StdCommand::new("powerprofilesctl").arg("get").output() {
            return String::from_utf8_lossy(&o.stdout).trim().to_string();
        }
        "balanced".into()
    }

    async fn set_profile(&self, profile: String) {
        let _ = StdCommand::new("powerprofilesctl").args(&["set", &profile]).output();
    }

    #[zbus(signal)] async fn battery_changed(&self, ctxt: &SignalContext<'_>, percent: u32, state: String) -> zbus::Result<()>;
}


async fn get_bat_real() -> (u32, String) {
    
    if let Ok(o) = StdCommand::new("upower").args(&["-i", "/org/freedesktop/UPower/devices/DisplayDevice"]).output() {
        let s = String::from_utf8_lossy(&o.stdout);
        let mut pct = 0;
        let mut state = "Unknown".to_string();
        
        for line in s.lines() {
            let line = line.trim();
            if line.starts_with("percentage:") {
                if let Some(val) = line.split(':').nth(1) {
                    pct = val.trim().trim_end_matches('%').parse().unwrap_or(0);
                }
            } else if line.starts_with("state:") {
                if let Some(val) = line.split(':').nth(1) {
                    
                    let raw = val.trim();
                    if !raw.is_empty() {
                        let mut chars = raw.chars();
                        if let Some(f) = chars.next() {
                            state = f.to_uppercase().collect::<String>() + chars.as_str();
                        }
                    }
                }
            }
        }
        
        
        if pct > 0 || state != "Unknown" {
            return (pct, state);
        }
    }
    
    
    if let Ok(cap) = std::fs::read_to_string("/sys/class/power_supply/BAT0/capacity") {
        let pct = cap.trim().parse().unwrap_or(0);
        let status = std::fs::read_to_string("/sys/class/power_supply/BAT0/status")
            .unwrap_or("Unknown".into())
            .trim()
            .to_string();
        return (pct, status);
    }

    (0, "No Battery".into())
}

pub async fn monitor(conn: Connection) {
    let iface = conn.object_server().interface::<_, SystemService>("/org/syd/System").await.unwrap();
    let mut last_p = 0;
    let mut last_s = String::new();

    loop {
        let (p, s) = get_bat_real().await;
        
        
        if p != last_p || s != last_s {
            let _ = SystemService::battery_changed(&*iface.get().await, iface.signal_context(), p, s.clone()).await;
            last_p = p;
            last_s = s;
        }
        sleep(Duration::from_secs(5)).await;
    }
}
