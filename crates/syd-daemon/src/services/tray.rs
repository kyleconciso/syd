use zbus::{interface, object_server::SignalContext};
use tokio::sync::mpsc;

pub struct TrayService {
    tx: mpsc::UnboundedSender<String>,
}

impl TrayService {
    pub fn new(tx: mpsc::UnboundedSender<String>) -> Self {
        Self { tx }
    }
}

#[interface(name = "org.kde.StatusNotifierWatcher")]
impl TrayService {
    
    async fn register_status_notifier_item(&self, service: String) -> zbus::fdo::Result<()> {
        println!("Tray Register: {}", service);
        let _ = self.tx.send(service);
        Ok(())
    }

    
    
    #[zbus(signal)]
    async fn status_notifier_item_registered(&self, ctxt: &SignalContext<'_>, service: String) -> zbus::Result<()>;

    #[zbus(property)] fn is_status_notifier_host_registered(&self) -> bool { true }
    #[zbus(property)] fn protocol_version(&self) -> i32 { 0 }
    
    
    async fn register_status_notifier_host(&self, _service: String) -> zbus::fdo::Result<()> { Ok(()) }
}

pub async fn monitor_tray(conn: zbus::Connection, mut rx: mpsc::UnboundedReceiver<String>) {
    while let Some(service) = rx.recv().await {
        
        let _ = conn.emit_signal(
            Option::<&str>::None,
            "/StatusNotifierWatcher",
            "org.kde.StatusNotifierWatcher",
            "StatusNotifierItemRegistered",
            &(&service,)
        ).await;
    }
}
