use zbus::{interface, object_server::SignalContext};
use syd_core::NotifData;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use zbus::zvariant::Value;


pub struct NotificationService {
    history: Arc<Mutex<Vec<NotifData>>>,
}

impl NotificationService {
    pub fn new() -> (Self, Arc<Mutex<Vec<NotifData>>>) {
        let h = Arc::new(Mutex::new(Vec::new()));
        (Self { history: h.clone() }, h)
    }
}

#[interface(name = "org.syd.Notifications")]
impl NotificationService {
    #[zbus(signal)] 
    async fn received(&self, ctxt: &SignalContext<'_>, note: NotifData) -> zbus::Result<()>;

    async fn get_history(&self) -> Vec<NotifData> {
        self.history.lock().unwrap().clone()
    }

    async fn clear_history(&self) {
        self.history.lock().unwrap().clear();
    }
    
    async fn close(&self, _id: u32) {}
}


pub struct FdoNotificationServer {
    history_ref: Arc<Mutex<Vec<NotifData>>>,
}

impl FdoNotificationServer {
    pub fn new(h: Arc<Mutex<Vec<NotifData>>>) -> Self {
        Self { history_ref: h }
    }
}

#[interface(name = "org.freedesktop.Notifications")]
impl FdoNotificationServer {
    fn get_capabilities(&self) -> Vec<String> { vec!["body".into()] }
    fn get_server_information(&self) -> (String, String, String, String) { ("Syd".into(), "Syd".into(), "0.1".into(), "1.2".into()) }
    
    async fn notify(&self, app_name: String, _r: u32, _i: String, summary: String, body: String, _a: Vec<String>, _h: HashMap<String, Value<'_>>, _e: i32, #[zbus(signal_context)] ctxt: SignalContext<'_>) -> u32 {
        let n = NotifData { app_name, summary, body };
        
        
        {
            let mut h = self.history_ref.lock().unwrap();
            h.insert(0, n.clone());
            if h.len() > 50 { h.pop(); }
        }

        
        let conn = ctxt.connection();
        if let Ok(iface) = conn.object_server().interface::<_, NotificationService>("/org/syd/Notifications").await {
            let _ = NotificationService::received(&*iface.get().await, iface.signal_context(), n).await;
        }
        1
    }
    fn close_notification(&self, _id: u32) {}
}
