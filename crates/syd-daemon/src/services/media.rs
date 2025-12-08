use zbus::{interface, object_server::SignalContext, Connection};
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use std::process::Stdio;

pub struct MediaService;
#[interface(name = "org.syd.Media")]
impl MediaService {
    async fn play_pause(&self) { let _ = Command::new("playerctl").arg("play-pause").output().await; }
    async fn next(&self) { let _ = Command::new("playerctl").arg("next").output().await; }
    async fn prev(&self) { let _ = Command::new("playerctl").arg("previous").output().await; }
    
    async fn get_position(&self) -> f64 {
        if let Ok(o) = Command::new("playerctl").env("LC_ALL", "C").arg("position").output().await {
            return String::from_utf8_lossy(&o.stdout).trim().parse().unwrap_or(0.0);
        }
        0.0
    }

    async fn get_length(&self) -> f64 {
        if let Ok(o) = Command::new("playerctl").env("LC_ALL", "C").args(&["metadata", "mpris:length"]).output().await {
             let micros = String::from_utf8_lossy(&o.stdout).trim().parse::<f64>().unwrap_or(0.0);
             return micros / 1_000_000.0;
        }
        0.0
    }

    async fn set_position(&self, sec: f64) {
        let _ = Command::new("playerctl").args(&["position", &sec.to_string()]).output().await;
    }
    
    async fn get_metadata(&self) -> (String, String, String) {
        if let Ok(o) = Command::new("playerctl").env("LC_ALL", "C").args(&["metadata", "--format", "{{status}}|{{title}}|{{artist}}"]).output().await {
            let s = String::from_utf8_lossy(&o.stdout).to_string();
            let parts: Vec<&str> = s.trim().split('|').collect();
            if parts.len() >= 3 {
                return (parts[0].into(), parts[1].into(), parts[2].into());
            }
        }
        ("Stopped".into(), "No Media".into(), "".into())
    }

    #[zbus(signal)] async fn metadata_changed(&self, ctxt: &SignalContext<'_>, status: String, title: String, artist: String) -> zbus::Result<()>;
}

pub async fn monitor(conn: Connection) {
    let iface = conn.object_server().interface::<_, MediaService>("/org/syd/Media").await.unwrap();

    if let Ok(o) = Command::new("playerctl").env("LC_ALL", "C").args(&["metadata", "--format", "{{status}}|{{title}}|{{artist}}"]).output().await {
        let s = String::from_utf8_lossy(&o.stdout);
        if !s.trim().is_empty() {
            let p: Vec<&str> = s.trim().split('|').collect();
            if p.len() >= 3 {
                 let _ = MediaService::metadata_changed(&*iface.get().await, iface.signal_context(), p[0].into(), p[1].into(), p[2].into()).await;
            }
        }
    }

    let mut child = Command::new("playerctl")
        .env("LC_ALL", "C")
        .args(&["metadata", "--follow", "--format", "{{status}}|{{title}}|{{artist}}"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn playerctl monitor");

    let mut reader = BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    
    while reader.read_line(&mut line).await.is_ok() {
        let p: Vec<&str> = line.trim().split('|').collect();
        if p.len() >= 3 {
            let _ = MediaService::metadata_changed(&*iface.get().await, iface.signal_context(), p[0].into(), p[1].into(), p[2].into()).await;
        }
        line.clear();
    }
}
