use zbus::Connection;
use futures::StreamExt;
use std::sync::Arc;
use syd_core::*;

#[derive(Debug)]
pub enum SydEvent {
    Volume(u32),
    Brightness(u32),
    Media { status: String, title: String, artist: String },
    Battery(u32, String),
    NetworkState(String),
    BtPower(bool),
    Notification(NotifData),
    TrayItem(String),
}

pub struct Syd {
    pub audio: AudioProxy<'static>,
    pub brightness: BrightnessProxy<'static>,
    pub media: MediaProxy<'static>,
    pub system: SystemProxy<'static>,
    pub network: NetworkProxy<'static>,
    pub bluetooth: BluetoothProxy<'static>,
    pub notif: NotificationsProxy<'static>,
    pub tray_watcher: StatusNotifierWatcherProxy<'static>,
}

impl Syd {
    pub async fn connect() -> zbus::Result<Arc<Self>> {
        let c = Connection::session().await?;
        Ok(Arc::new(Self {
            audio: AudioProxy::new(&c).await?,
            brightness: BrightnessProxy::new(&c).await?,
            media: MediaProxy::new(&c).await?,
            system: SystemProxy::new(&c).await?,
            network: NetworkProxy::new(&c).await?,
            bluetooth: BluetoothProxy::new(&c).await?,
            notif: NotificationsProxy::new(&c).await?,
            tray_watcher: StatusNotifierWatcherProxy::builder(&c).path("/StatusNotifierWatcher")?.build().await?,
        }))
    }

    pub async fn events(&self) -> impl futures::Stream<Item = SydEvent> + '_ {
        let mut s1 = self.audio.receive_volume_changed().await.unwrap();
        let mut s2 = self.brightness.receive_brightness_changed().await.unwrap();
        let mut s3 = self.media.receive_metadata_changed().await.unwrap();
        let mut s4 = self.system.receive_battery_changed().await.unwrap();
        let mut s5 = self.network.receive_state_changed().await.unwrap();
        let mut s6 = self.bluetooth.receive_power_changed().await.unwrap();
        let mut s7 = self.notif.receive_received().await.unwrap();
        let mut s8 = self.tray_watcher.receive_status_notifier_item_registered().await.unwrap();

        async_stream::stream! {
            loop {
                tokio::select! {
                    Some(m) = s1.next() => if let Ok(a) = m.args() { yield SydEvent::Volume(a.new_vol); },
                    Some(m) = s2.next() => if let Ok(a) = m.args() { yield SydEvent::Brightness(a.new_val); },
                    Some(m) = s3.next() => if let Ok(a) = m.args() { yield SydEvent::Media { status: a.status, title: a.title, artist: a.artist }; },
                    Some(m) = s4.next() => if let Ok(a) = m.args() { yield SydEvent::Battery(a.percent, a.state); },
                    Some(m) = s5.next() => if let Ok(a) = m.args() { yield SydEvent::NetworkState(a.state); },
                    Some(m) = s6.next() => if let Ok(a) = m.args() { yield SydEvent::BtPower(a.enabled); },
                    Some(m) = s7.next() => if let Ok(a) = m.args() { yield SydEvent::Notification(a.note); },
                    
                    Some(m) = s8.next() => if let Ok(a) = m.args() { yield SydEvent::TrayItem(a.service_name); },
                }
            }
        }
    }
}
