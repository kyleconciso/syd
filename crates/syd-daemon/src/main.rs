mod services;
use zbus::ConnectionBuilder;
use services::{audio, brightness, media, system, network, bluetooth, notifications, tray};
use std::error::Error;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("--- Syd Daemon Starting ---");

    
    let (audio_svc, audio_cache) = audio::AudioService::new();
    let (bright_svc, bright_cache) = brightness::BrightnessService::new();
    let (bt_svc, bt_cache) = bluetooth::BluetoothService::new();
    let media_svc = media::MediaService;
    let system_svc = system::SystemService;
    let network_svc = network::NetworkService;
    
    
    let (notif_svc, notif_hist) = notifications::NotificationService::new();
    let fdo_notif = notifications::FdoNotificationServer::new(notif_hist);
    
    
    let (tray_tx, tray_rx) = mpsc::unbounded_channel();
    let tray_svc = tray::TrayService::new(tray_tx);

    
    let conn = ConnectionBuilder::session()?
        .name("org.syd.Daemon")?
        
        .serve_at("/org/syd/Audio", audio_svc)?
        .serve_at("/org/syd/Brightness", bright_svc)?
        .serve_at("/org/syd/Media", media_svc)?
        .serve_at("/org/syd/System", system_svc)?
        .serve_at("/org/syd/Network", network_svc)?
        .serve_at("/org/syd/Bluetooth", bt_svc)?
        
        .serve_at("/org/syd/Notifications", notif_svc)?
        .serve_at("/org/freedesktop/Notifications", fdo_notif)?
        .name("org.freedesktop.Notifications")?
        
        .serve_at("/StatusNotifierWatcher", tray_svc)? 
        .name("org.kde.StatusNotifierWatcher")?
        .build().await?;

    
    tokio::spawn(audio::monitor(conn.clone(), audio_cache));
    tokio::spawn(brightness::monitor(conn.clone(), bright_cache));
    tokio::spawn(bluetooth::monitor(conn.clone(), bt_cache));
    tokio::spawn(media::monitor(conn.clone()));
    tokio::spawn(system::monitor(conn.clone()));
    tokio::spawn(network::monitor(conn.clone()));
    
    
    tokio::spawn(tray::monitor_tray(conn.clone(), tray_rx));

    println!("--- Syd Daemon Ready ---");
    std::future::pending::<()>().await;
    Ok(())
}
