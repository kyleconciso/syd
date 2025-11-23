use zbus::{interface, object_server::SignalContext, Connection};
use std::sync::{Arc, atomic::{AtomicU32, Ordering}};
use tokio::process::Command as TokioCommand;
use std::process::Command as StdCommand;
use tokio::time::{sleep, Duration};

pub struct BrightnessService { val: Arc<AtomicU32> }
impl BrightnessService {
    pub fn new() -> (Self, Arc<AtomicU32>) {
        let v = Arc::new(AtomicU32::new(0));
        (Self { val: v.clone() }, v)
    }
}

#[interface(name = "org.syd.Brightness")]
impl BrightnessService {
    async fn set_brightness(&self, p: u32) {
        
        let _ = StdCommand::new("brightnessctl").arg("set").arg(format!("{}%", p)).output();
        self.val.store(p, Ordering::Relaxed);
    }
    async fn get_brightness(&self) -> u32 { self.val.load(Ordering::Relaxed) }
    #[zbus(signal)] async fn brightness_changed(&self, ctxt: &SignalContext<'_>, new_val: u32) -> zbus::Result<()>;
}

pub async fn monitor(conn: Connection, cache: Arc<AtomicU32>) {
    let iface = conn.object_server().interface::<_, BrightnessService>("/org/syd/Brightness").await.unwrap();
    let mut last = 0;
    loop {
        if let Ok(o) = TokioCommand::new("brightnessctl").args(&["-m", "info"]).output().await {
            if let Some(l) = String::from_utf8_lossy(&o.stdout).split(',').last() {
                let cur = l.trim().replace('%',"").parse().unwrap_or(0);
                if cur != last {
                    cache.store(cur, Ordering::Relaxed);
                    let _ = BrightnessService::brightness_changed(&*iface.get().await, iface.signal_context(), cur).await;
                    last = cur;
                }
            }
        }
        sleep(Duration::from_secs(1)).await;
    }
}
