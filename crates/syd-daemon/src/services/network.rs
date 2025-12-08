use zbus::{interface, object_server::SignalContext, Connection};
use tokio::process::Command;
use tokio::time::{sleep, Duration};
use syd_core::WifiNet;
use std::collections::HashSet;

pub struct NetworkService;
#[interface(name = "org.syd.Network")]
impl NetworkService {
    async fn get_state(&self) -> String { read_state_async().await }
    
    async fn set_wifi(&self, e: bool) { 
        let _ = Command::new("nmcli").args(&["radio", "wifi", if e{"on"}else{"off"}]).output().await; 
    }
    
    async fn scan(&self) -> Vec<WifiNet> {
        let active_ssid = get_active_ssid().await;
        let known_ssids = get_known_ssids().await;

        if let Ok(o) = Command::new("nmcli")
            .env("LC_ALL", "C")
            .args(&["-t", "-f", "SSID,SIGNAL,SECURITY", "dev", "wifi"])
            .output().await 
        {
            let mut seen = HashSet::new();
            let mut res = Vec::new();
            
            for line in String::from_utf8_lossy(&o.stdout).lines() {
                let p: Vec<&str> = line.split(':').collect();
                if p.len() < 3 { continue; }
                
                let ssid = p[0].to_string();
                if ssid.is_empty() || seen.contains(&ssid) { continue; }
                seen.insert(ssid.clone());
                
                let known = known_ssids.contains(&ssid);
                let active = ssid == active_ssid;
                let strength = p[1].parse().unwrap_or(0);
                let security = p[2].to_string();

                res.push(WifiNet { ssid, strength, security, known, active });
            }
            res.sort_by(|a, b| {
                if a.active != b.active { return b.active.cmp(&a.active); }
                b.strength.cmp(&a.strength)
            });
            return res;
        }
        vec![]
    }

    async fn connect(&self, ssid: String, pass: String) -> String {
        if !pass.is_empty() {
             let _ = Command::new("nmcli").args(&["con", "delete", &ssid]).output().await;
        }
        let mut cmd = Command::new("nmcli");
        cmd.env("LC_ALL", "C");
        cmd.args(&["dev", "wifi", "connect", &ssid]);
        if !pass.is_empty() { cmd.args(&["password", &pass]); }
        
        if let Ok(o) = cmd.output().await {
            if o.status.success() { return "OK".into(); }
            let err = String::from_utf8_lossy(&o.stderr);
            if err.contains("Secrets were required") { return "PASS_REQ".into(); }
            return err.to_string();
        }
        "Error".into()
    }

    async fn forget(&self, ssid: String) {
        let _ = Command::new("nmcli").args(&["con", "delete", &ssid]).output().await;
    }

    #[zbus(signal)] async fn state_changed(&self, ctxt: &SignalContext<'_>, state: String) -> zbus::Result<()>;
}

async fn get_active_ssid() -> String {
    if let Ok(o) = Command::new("nmcli").env("LC_ALL", "C").args(&["-t", "-f", "NAME", "con", "show", "--active"]).output().await {
        for l in String::from_utf8_lossy(&o.stdout).lines() {
             if l != "lo" && !l.contains("docker") && !l.contains("virbr") { return l.to_string(); }
        }
    }
    "".into()
}

async fn get_known_ssids() -> HashSet<String> {
    let mut s = HashSet::new();
    if let Ok(o) = Command::new("nmcli").env("LC_ALL", "C").args(&["-t", "-f", "NAME", "con", "show"]).output().await {
        for l in String::from_utf8_lossy(&o.stdout).lines() { s.insert(l.to_string()); }
    }
    s
}

async fn read_state_async() -> String {
    if let Ok(o) = Command::new("nmcli").env("LC_ALL", "C").args(&["general", "status"]).output().await {
        if String::from_utf8_lossy(&o.stdout).contains("connected") { return "Connected".into(); }
    }
    "Disconnected".into()
}

pub async fn monitor(conn: Connection) {
    let iface = conn.object_server().interface::<_, NetworkService>("/org/syd/Network").await.unwrap();
    let mut last = "Unknown".to_string();
    loop {
        let curr = read_state_async().await;
        if curr != last {
            let _ = NetworkService::state_changed(&*iface.get().await, iface.signal_context(), curr.clone()).await;
            last = curr;
        }
        sleep(Duration::from_secs(3)).await;
    }
}