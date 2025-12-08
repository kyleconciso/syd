use zbus::{interface, object_server::SignalContext, Connection};
use std::sync::{Arc, atomic::{AtomicU32, Ordering}};
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use std::process::Stdio;
use syd_core::AudioDevice;
use serde::Deserialize;

#[derive(Deserialize)]
struct PactlNode {
    name: String,
    description: String,
}

pub struct AudioService { vol: Arc<AtomicU32> }
impl AudioService {
    pub fn new() -> (Self, Arc<AtomicU32>) {
        let c = Arc::new(AtomicU32::new(0));
        (Self { vol: c.clone() }, c)
    }
}

#[interface(name = "org.syd.Audio")]
impl AudioService {
    async fn set_volume(&self, p: u32) {
        let _ = Command::new("pactl")
            .env("LC_ALL", "C")
            .args(&["set-sink-volume", "@DEFAULT_SINK@", &format!("{}%", p.min(100))])
            .output().await;
        self.vol.store(p, Ordering::Relaxed);
    }

    async fn get_volume(&self) -> u32 { self.vol.load(Ordering::Relaxed) }
    
    async fn get_sinks(&self) -> Vec<AudioDevice> { get_pactl_list("sinks").await }
    async fn get_sources(&self) -> Vec<AudioDevice> { get_pactl_list("sources").await }
    
    async fn set_default_sink(&self, name: String) {
        let _ = Command::new("pactl").args(&["set-default-sink", &name]).output().await;
    }
    async fn set_default_source(&self, name: String) {
        let _ = Command::new("pactl").args(&["set-default-source", &name]).output().await;
    }

    #[zbus(signal)] async fn volume_changed(&self, ctxt: &SignalContext<'_>, new_vol: u32) -> zbus::Result<()>;
}

async fn get_pactl_list(kind: &str) -> Vec<AudioDevice> {
    if let Ok(o) = Command::new("pactl")
        .env("LC_ALL", "C")
        .args(&["-f", "json", "list", kind])
        .output().await 
    {
        if let Ok(nodes) = serde_json::from_slice::<Vec<PactlNode>>(&o.stdout) {
            return nodes.into_iter()
                .filter(|n| !n.name.contains(".monitor")) 
                .map(|n| AudioDevice { name: n.name, description: n.description })
                .collect();
        }
    }
    vec![]
}

pub async fn monitor(conn: Connection, cache: Arc<AtomicU32>) {
    if let Ok(o) = Command::new("pactl").env("LC_ALL", "C").args(&["get-sink-volume", "@DEFAULT_SINK@"]).output().await {
        if let Some(p) = String::from_utf8_lossy(&o.stdout).split('/').nth(1) {
            let v = p.trim().replace('%', "").parse().unwrap_or(0);
            cache.store(v, Ordering::Relaxed);
        }
    }

    let mut child = Command::new("pactl")
        .env("LC_ALL", "C")
        .arg("subscribe")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn pactl monitor");

    let mut reader = BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    let iface = conn.object_server().interface::<_, AudioService>("/org/syd/Audio").await.unwrap();

    while reader.read_line(&mut line).await.is_ok() {
        if line.contains("sink") && line.contains("change") {
             if let Ok(o) = Command::new("pactl").env("LC_ALL", "C").args(&["get-sink-volume", "@DEFAULT_SINK@"]).output().await {
                if let Some(p) = String::from_utf8_lossy(&o.stdout).split('/').nth(1) {
                    let v = p.trim().replace('%', "").parse().unwrap_or(0);
                    cache.store(v, Ordering::Relaxed);
                    let _ = AudioService::volume_changed(&*iface.get().await, iface.signal_context(), v).await;
                }
            }
        }
        line.clear();
    }
}