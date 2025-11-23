use zbus::{proxy, zvariant};
use serde::{Deserialize, Serialize};



#[derive(Debug, Serialize, Deserialize, zbus::zvariant::Type, Clone, Default)]
pub struct WifiNet {
    pub ssid: String,
    pub strength: u8,
    pub security: String,
    pub known: bool,
    pub active: bool,
}

#[derive(Debug, Serialize, Deserialize, zbus::zvariant::Type, Clone, Default)]
pub struct AudioDevice {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, zbus::zvariant::Type, Clone, Default)]
pub struct BtDevice {
    pub mac: String,
    pub name: String,
    pub connected: bool,
}

#[derive(Debug, Serialize, Deserialize, zbus::zvariant::Type, Clone, Default)]
pub struct NotifData {
    pub summary: String,
    pub body: String,
    pub app_name: String,
}



#[proxy(interface = "org.syd.Notifications", default_service = "org.syd.Daemon", default_path = "/org/syd/Notifications")]
pub trait Notifications {
    #[zbus(signal)] fn received(&self, note: NotifData) -> zbus::Result<()>;
    fn get_history(&self) -> zbus::Result<Vec<NotifData>>;
    fn clear_history(&self) -> zbus::Result<()>;
    fn close(&self, id: u32) -> zbus::Result<()>;
}



#[proxy(interface = "org.kde.StatusNotifierWatcher", default_service = "org.syd.Daemon", default_path = "/StatusNotifierWatcher")]
pub trait StatusNotifierWatcher {
    #[zbus(signal)] 
    fn status_notifier_item_registered(&self, service_name: String) -> zbus::Result<()>;
    
    fn register_status_notifier_item(&self, service: String) -> zbus::Result<()>;
}


#[proxy(interface = "org.kde.StatusNotifierItem")]
pub trait StatusNotifierItem {
    fn activate(&self, x: i32, y: i32) -> zbus::Result<()>;
    fn context_menu(&self, x: i32, y: i32) -> zbus::Result<()>;
    fn scroll(&self, delta: i32, orientation: &str) -> zbus::Result<()>;

    #[zbus(property)] fn id(&self) -> zbus::Result<String>;
    #[zbus(property)] fn title(&self) -> zbus::Result<String>;
    #[zbus(property)] fn icon_name(&self) -> zbus::Result<String>;
    #[zbus(property)] fn menu(&self) -> zbus::Result<zvariant::OwnedObjectPath>;
}


#[proxy(interface = "org.syd.Audio", default_service = "org.syd.Daemon", default_path = "/org/syd/Audio")]
pub trait Audio {
    fn set_volume(&self, percentage: u32) -> zbus::Result<()>;
    fn get_volume(&self) -> zbus::Result<u32>;
    fn get_sinks(&self) -> zbus::Result<Vec<AudioDevice>>;
    fn get_sources(&self) -> zbus::Result<Vec<AudioDevice>>;
    fn set_default_sink(&self, name: String) -> zbus::Result<()>;
    fn set_default_source(&self, name: String) -> zbus::Result<()>;
    #[zbus(signal)] fn volume_changed(&self, new_vol: u32) -> zbus::Result<()>;
}

#[proxy(interface = "org.syd.System", default_service = "org.syd.Daemon", default_path = "/org/syd/System")]
pub trait System {
    fn get_battery(&self) -> zbus::Result<(u32, String)>;
    fn power_off(&self) -> zbus::Result<()>;
    fn reboot(&self) -> zbus::Result<()>;
    fn get_user(&self) -> zbus::Result<String>;
    fn get_power_profiles(&self) -> zbus::Result<Vec<String>>;
    fn get_current_profile(&self) -> zbus::Result<String>;
    fn set_profile(&self, profile: String) -> zbus::Result<()>;
    #[zbus(signal)] fn battery_changed(&self, percent: u32, state: String) -> zbus::Result<()>;
}

#[proxy(interface = "org.syd.Media", default_service = "org.syd.Daemon", default_path = "/org/syd/Media")]
pub trait Media {
    fn play_pause(&self) -> zbus::Result<()>;
    fn next(&self) -> zbus::Result<()>;
    fn prev(&self) -> zbus::Result<()>;
    fn get_position(&self) -> zbus::Result<f64>;
    fn get_length(&self) -> zbus::Result<f64>;
    fn set_position(&self, sec: f64) -> zbus::Result<()>;
    fn get_metadata(&self) -> zbus::Result<(String, String, String)>;
    #[zbus(signal)] fn metadata_changed(&self, status: String, title: String, artist: String) -> zbus::Result<()>;
}

#[proxy(interface = "org.syd.Brightness", default_service = "org.syd.Daemon", default_path = "/org/syd/Brightness")]
pub trait Brightness {
    fn set_brightness(&self, percentage: u32) -> zbus::Result<()>;
    fn get_brightness(&self) -> zbus::Result<u32>;
    #[zbus(signal)] fn brightness_changed(&self, new_val: u32) -> zbus::Result<()>;
}

#[proxy(interface = "org.syd.Bluetooth", default_service = "org.syd.Daemon", default_path = "/org/syd/Bluetooth")]
pub trait Bluetooth {
    fn get_power(&self) -> zbus::Result<bool>;
    fn set_power(&self, enabled: bool) -> zbus::Result<()>;
    fn get_devices(&self) -> zbus::Result<Vec<BtDevice>>;
    fn connect_device(&self, mac: String, connect: bool) -> zbus::Result<()>;
    #[zbus(signal)] fn power_changed(&self, enabled: bool) -> zbus::Result<()>;
}

#[proxy(interface = "org.syd.Network", default_service = "org.syd.Daemon", default_path = "/org/syd/Network")]
pub trait Network {
    fn get_state(&self) -> zbus::Result<String>;
    fn set_wifi(&self, enabled: bool) -> zbus::Result<()>;
    fn scan(&self) -> zbus::Result<Vec<WifiNet>>;
    fn connect(&self, ssid: String, pass: String) -> zbus::Result<String>;
    fn forget(&self, ssid: String) -> zbus::Result<()>;
    #[zbus(signal)] fn state_changed(&self, state: String) -> zbus::Result<()>;
}
