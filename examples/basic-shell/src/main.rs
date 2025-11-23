use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Orientation, Scale, Label, Box as GtkBox, Align, Button, Switch, Revealer, Popover, ListBox, DropDown, StringList, PasswordEntry, Image, ScrolledWindow, GestureClick};
use syd_client::{Syd, SydEvent};
use glib::clone;
use std::sync::Arc;
use tokio::sync::mpsc;
use std::time::Duration;
use futures::StreamExt;
use std::cell::RefCell;
use std::rc::Rc;
use syd_core::StatusNotifierItemProxy;

fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _g = rt.enter();
    let app = Application::builder().application_id("org.syd.Shell").build();
    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let w = ApplicationWindow::builder().application(app).title("Syd").default_width(360).default_height(700).build();
    let c = GtkBox::new(Orientation::Vertical, 10);
    c.set_margin_top(20); c.set_margin_start(20); c.set_margin_end(20); c.set_margin_bottom(20);
    
    let overlay = gtk4::Overlay::new();
    overlay.set_child(Some(&c));
    w.set_child(Some(&overlay));

    glib::MainContext::default().spawn_local(clone!(@weak overlay, @weak c, @weak w => async move {
        if let Ok(syd) = Syd::connect().await {
            setup_full_ui(&c, &overlay, &w, syd);
        }
    }));
    w.present();
}

fn setup_full_ui(c: &GtkBox, ov: &gtk4::Overlay, win: &ApplicationWindow, syd: Arc<Syd>) {
    
    let (toast, t_lbl) = create_toast();
    ov.add_overlay(&toast);

    
    let head = GtkBox::new(Orientation::Horizontal, 10);
    let (user_lbl, bat_lbl) = create_header_labels();
    
    let notif_btn = Button::with_label("🔔");
    let notif_pop = create_notif_history(syd.clone());
    notif_pop.set_parent(&notif_btn);
    notif_btn.connect_clicked(move |_| notif_pop.popup());
    
    head.append(&user_lbl);
    head.append(&bat_lbl);
    head.append(&notif_btn);
    c.append(&head);
    c.append(&gtk4::Separator::new(Orientation::Horizontal));

    
    let (m_lbl, m_box, m_seek, m_time) = create_media(syd.clone());
    c.append(&m_box);
    c.append(&gtk4::Separator::new(Orientation::Horizontal));

    
    let (net_box, wifi_sw, bt_sw) = create_net_bt_row(win, syd.clone());
    c.append(&net_box);
    c.append(&gtk4::Separator::new(Orientation::Horizontal));

    
    let io_box = create_io_selectors(syd.clone());
    c.append(&io_box);

    
    let (v_scale, v_box) = create_slider("Volume", syd.clone(), true);
    c.append(&v_box);
    let (b_scale, b_box) = create_slider("Brightness", syd.clone(), false);
    c.append(&b_box);

    
    c.append(&gtk4::Separator::new(Orientation::Horizontal));
    c.append(&create_power_section(syd.clone()));
    
    c.append(&Label::new(Some("System Tray")));
    let tray_box = GtkBox::new(Orientation::Horizontal, 5);
    let tray_scroll = ScrolledWindow::builder().min_content_height(50).child(&tray_box).build();
    c.append(&tray_scroll);

    
    let is_playing = Rc::new(RefCell::new(false));

    glib::MainContext::default().spawn_local(clone!(@weak m_seek, @weak m_time, @strong is_playing, @weak m_lbl, @weak tray_box => async move {
        
        if let Ok(u) = syd.system.get_user().await { user_lbl.set_label(&format!("Hello, {}", u)); }
        if let Ok((p, s)) = syd.system.get_battery().await { bat_lbl.set_label(&format!("{}% {}", p, s)); }
        if let Ok(v) = syd.audio.get_volume().await { v_scale.set_value(v as f64); }
        if let Ok(v) = syd.brightness.get_brightness().await { b_scale.set_value(v as f64); }
        if let Ok(st) = syd.network.get_state().await { wifi_sw.set_active(st == "Connected"); }
        if let Ok(p) = syd.bluetooth.get_power().await { bt_sw.set_active(p); }

        if let Ok((status, title, artist)) = syd.media.get_metadata().await {
             let icon = if status == "Playing" { "🎵" } else { "⏸" };
             m_lbl.set_label(&format!("{} {} - {}", icon, title, artist));
             *is_playing.borrow_mut() = status == "Playing";
             if let Ok(len) = syd.media.get_length().await {
                 if len > 0.0 {
                     m_seek.set_range(0.0, len);
                     if let Ok(pos) = syd.media.get_position().await {
                         m_seek.set_value(pos);
                         m_time.set_label(&format!("{}/{}", fmt_time(pos), fmt_time(len)));
                     }
                 }
             }
        }

        let mut evts = Box::pin(syd.events().await);
        let s_media = syd.clone();
        let playing_flag = is_playing.clone();
        
        glib::timeout_add_local(Duration::from_secs(1), clone!(@weak m_seek, @weak m_time => @default-return glib::ControlFlow::Break, move || {
             if *playing_flag.borrow() {
                 let s = s_media.clone();
                 glib::MainContext::default().spawn_local(async move {
                     if let Ok(pos) = s.media.get_position().await {
                         if let Ok(len) = s.media.get_length().await {
                            if len > 0.0 && !m_seek.has_focus() {
                                m_seek.set_range(0.0, len);
                                m_seek.set_value(pos);
                                m_time.set_label(&format!("{}/{}", fmt_time(pos), fmt_time(len)));
                            }
                         }
                     }
                 });
             }
             glib::ControlFlow::Continue
        }));

        while let Some(e) = evts.next().await {
            match e {
                SydEvent::Volume(v) => if !v_scale.has_focus() { v_scale.set_value(v as f64); },
                SydEvent::Brightness(v) => if !b_scale.has_focus() { b_scale.set_value(v as f64); },
                SydEvent::Media{title, artist, status} => {
                    let icon = if status == "Playing" { "🎵" } else { "⏸" };
                    *is_playing.borrow_mut() = status == "Playing";
                    m_lbl.set_label(&format!("{} {} - {}", icon, title, artist));
                },
                SydEvent::Battery(p, s) => bat_lbl.set_label(&format!("{}% {}", p, s)),
                SydEvent::NetworkState(s) => wifi_sw.set_active(s == "Connected"),
                SydEvent::BtPower(p) => bt_sw.set_active(p),
                SydEvent::Notification(n) => {
                    t_lbl.set_label(&format!("{}: {}", n.app_name, n.summary));
                    toast.set_reveal_child(true);
                    let t = toast.clone();
                    glib::timeout_add_seconds_local(3, move || { t.set_reveal_child(false); glib::ControlFlow::Break });
                },
                SydEvent::TrayItem(service) => {
                    
                    spawn_tray_item(service, tray_box.clone());
                }
            }
        }
    }));
}

fn spawn_tray_item(service: String, container: GtkBox) {
    glib::MainContext::default().spawn_local(async move {
        let conn = zbus::Connection::session().await.unwrap();
        if let Ok(item) = StatusNotifierItemProxy::builder(&conn)
            .destination(service.clone()).unwrap()
            .path("/StatusNotifierItem").unwrap()
            .build().await 
        {
            let icon_name = item.icon_name().await.unwrap_or("image-missing".into());
            
            let btn = Button::builder().has_frame(false).build();
            let img = Image::from_icon_name(&icon_name);
            img.set_pixel_size(24);
            btn.set_child(Some(&img));
            
            let i_clone = item.clone();
            btn.connect_clicked(move |_| {
                let i = i_clone.clone();
                glib::MainContext::default().spawn_local(async move { let _=i.activate(0,0).await; });
            });
            
            let gesture = GestureClick::new();
            gesture.set_button(3);
            let i_clone2 = item.clone();
            gesture.connect_pressed(move |_, _, _, _| {
                let i = i_clone2.clone();
                glib::MainContext::default().spawn_local(async move { let _=i.context_menu(0,0).await; });
            });
            btn.add_controller(gesture);

            container.append(&btn);
        }
    });
}

fn create_media(syd: Arc<Syd>) -> (Label, GtkBox, Scale, Label) {
    let b = GtkBox::new(Orientation::Vertical, 10);
    let l = Label::new(Some("No Media")); l.set_wrap(true);
    let seek = Scale::with_range(Orientation::Horizontal, 0.0, 100.0, 1.0);
    let time = Label::new(Some("00:00 / 00:00")); time.add_css_class("caption");
    
    let s = syd.clone();
    seek.connect_value_changed(clone!(@weak seek => move |sc| {
        if sc.has_focus() {
             let s = s.clone(); let v = sc.value();
             glib::MainContext::default().spawn_local(async move { let _=s.media.set_position(v).await; });
        }
    }));

    let row = GtkBox::new(Orientation::Horizontal, 20); row.set_halign(Align::Center);
    let prev = Button::with_label("⏮");
    let play = Button::with_label("⏯");
    let next = Button::with_label("⏭");
    prev.set_valign(Align::Center); play.set_valign(Align::Center); next.set_valign(Align::Center);
    
    let s=syd.clone(); prev.connect_clicked(move |_| { let s=s.clone(); glib::MainContext::default().spawn_local(async move {let _=s.media.prev().await;}); });
    let s=syd.clone(); play.connect_clicked(move |_| { let s=s.clone(); glib::MainContext::default().spawn_local(async move {let _=s.media.play_pause().await;}); });
    let s=syd.clone(); next.connect_clicked(move |_| { let s=s.clone(); glib::MainContext::default().spawn_local(async move {let _=s.media.next().await;}); });
    
    row.append(&prev); row.append(&play); row.append(&next);
    b.append(&l); b.append(&seek); b.append(&time); b.append(&row);
    (l, b, seek, time)
}

fn create_io_selectors(syd: Arc<Syd>) -> GtkBox {
    let b = GtkBox::new(Orientation::Vertical, 5);
    
    b.append(&Label::new(Some("Output")));
    let sink_dd = DropDown::new(None::<StringList>, None::<gtk4::Expression>);
    let sink_names = Rc::new(RefCell::new(Vec::<String>::new()));
    let sl = StringList::new(&[]);
    sink_dd.set_model(Some(&sl));
    b.append(&sink_dd);

    let s = syd.clone(); let sn = sink_names.clone();
    glib::MainContext::default().spawn_local(async move {
        if let Ok(devs) = s.audio.get_sinks().await {
            for d in devs { 
                sn.borrow_mut().push(d.name);
                sl.append(&d.description); 
            }
        }
    });
    
    let s = syd.clone(); let sn = sink_names.clone();
    sink_dd.connect_selected_notify(move |d| {
        let idx = d.selected() as usize;
        if let Some(name) = sn.borrow().get(idx) {
            let n = name.clone(); let s = s.clone();
            glib::MainContext::default().spawn_local(async move { let _=s.audio.set_default_sink(n).await; });
        }
    });

    b.append(&Label::new(Some("Input")));
    let src_dd = DropDown::new(None::<StringList>, None::<gtk4::Expression>);
    let src_names = Rc::new(RefCell::new(Vec::<String>::new()));
    let sl_src = StringList::new(&[]);
    src_dd.set_model(Some(&sl_src));
    b.append(&src_dd);

    let s = syd.clone(); let sn = src_names.clone();
    glib::MainContext::default().spawn_local(async move {
        if let Ok(devs) = s.audio.get_sources().await {
            for d in devs { 
                sn.borrow_mut().push(d.name);
                sl_src.append(&d.description); 
            }
        }
    });
    let s = syd.clone(); let sn = src_names.clone();
    src_dd.connect_selected_notify(move |d| {
        let idx = d.selected() as usize;
        if let Some(name) = sn.borrow().get(idx) {
            let n = name.clone(); let s = s.clone();
            glib::MainContext::default().spawn_local(async move { let _=s.audio.set_default_source(n).await; });
        }
    });
    b
}

fn fmt_time(secs: f64) -> String {
    let s = secs as u64;
    format!("{:02}:{:02}", s / 60, s % 60)
}
fn create_header_labels() -> (Label, Label) {
    let u = Label::new(Some("User")); u.set_hexpand(true); u.set_halign(Align::Start); u.add_css_class("title-3");
    let b = Label::new(Some("Bat: --%"));
    (u, b)
}
fn create_notif_history(syd: Arc<Syd>) -> Popover {
    let p = Popover::builder().child(&GtkBox::new(Orientation::Vertical, 5)).build();
    let b = p.child().unwrap().downcast::<GtkBox>().unwrap();
    b.set_width_request(300);
    let list = ListBox::new();
    let scroll = ScrolledWindow::builder().min_content_height(300).child(&list).build();
    b.append(&scroll);
    let clear = Button::with_label("Clear All");
    let s = syd.clone(); let l_weak = list.clone();
    clear.connect_clicked(move |_| {
        let s = s.clone(); let l = l_weak.clone();
        glib::MainContext::default().spawn_local(async move {
            let _ = s.notif.clear_history().await;
            while let Some(c) = l.first_child() { l.remove(&c); }
        });
    });
    b.append(&clear);
    let s = syd.clone(); let l = list.clone();
    p.connect_map(move |_| {
        let s = s.clone(); let l = l.clone();
        glib::MainContext::default().spawn_local(async move {
            while let Some(c) = l.first_child() { l.remove(&c); }
            if let Ok(hist) = s.notif.get_history().await {
                for n in hist {
                    let row = GtkBox::new(Orientation::Vertical, 2);
                    let lbl_title = Label::new(None);
                    lbl_title.set_markup(&format!("<b>{}</b>", n.app_name));
                    lbl_title.set_halign(Align::Start);
                    let lbl_sum = Label::new(Some(&n.summary)); lbl_sum.set_halign(Align::Start); lbl_sum.set_wrap(true);
                    let lbl_body = Label::new(Some(&n.body)); lbl_body.set_halign(Align::Start); lbl_body.set_wrap(true); lbl_body.add_css_class("caption");
                    row.append(&lbl_title); row.append(&lbl_sum); row.append(&lbl_body);
                    row.append(&gtk4::Separator::new(Orientation::Horizontal));
                    l.append(&row);
                }
            }
        });
    });
    p
}
fn create_net_bt_row(win: &ApplicationWindow, syd: Arc<Syd>) -> (GtkBox, Switch, Switch) {
    let b = GtkBox::new(Orientation::Horizontal, 10);
    let wifi_btn = Button::with_label("Wi-Fi >");
    let wifi_pop = Popover::builder().child(&GtkBox::new(Orientation::Vertical, 5)).build();
    wifi_pop.set_parent(&wifi_btn);
    let w_list = ListBox::new();
    let scroll = ScrolledWindow::builder().min_content_height(250).min_content_width(280).child(&w_list).build();
    let head = GtkBox::new(Orientation::Horizontal, 5); head.append(&Label::new(Some("Wi-Fi")));
    let sw = Switch::new(); sw.set_valign(Align::Center); let s_sw = syd.clone();
    sw.connect_state_set(move |_, st| { let s=s_sw.clone(); glib::MainContext::default().spawn_local(async move{let _=s.network.set_wifi(st).await;}); glib::Propagation::Proceed });
    head.append(&sw);
    let pb = wifi_pop.child().unwrap().downcast::<GtkBox>().unwrap(); pb.append(&head); pb.append(&scroll);
    let s = syd.clone(); let wl = w_list.clone(); let w_win = win.clone();
    wifi_btn.connect_clicked(move |_| {
        wifi_pop.popup();
        while let Some(c) = wl.first_child() { wl.remove(&c); }
        wl.append(&Label::new(Some("Scanning...")));
        let s=s.clone(); let wl=wl.clone(); let w_win=w_win.clone();
        glib::MainContext::default().spawn_local(async move {
            if let Ok(nets) = s.network.scan().await {
                while let Some(c) = wl.first_child() { wl.remove(&c); }
                if nets.is_empty() { wl.append(&Label::new(Some("No networks"))); }
                for n in nets {
                    let row = GtkBox::new(Orientation::Horizontal, 5);
                    let txt = format!("{} ({}%)", n.ssid, n.strength);
                    let lbl = Label::new(Some(&txt)); lbl.set_hexpand(true); lbl.set_halign(Align::Start); row.append(&lbl);
                    if n.security != "" { row.append(&Label::new(Some("🔒"))); }
                    if n.active {
                        row.append(&Label::new(Some("✅")));
                        let b = Button::with_label("Disconnect"); let s=s.clone();
                        b.connect_clicked(move |_| { let s=s.clone(); glib::MainContext::default().spawn_local(async move {let _=s.network.set_wifi(false).await;}); }); 
                        row.append(&b);
                    } else {
                         let b = Button::with_label("Connect"); let s=s.clone(); let ssid=n.ssid.clone(); let w=w_win.clone();
                         b.connect_clicked(move |_| { let s=s.clone(); let ssid=ssid.clone(); let w=w.clone();
                             glib::MainContext::default().spawn_local(async move {
                                 let res = s.network.connect(ssid.clone(), "".into()).await.unwrap_or("Err".into());
                                 if res == "PASS_REQ" { if let Some(pass) = prompt_pass(&w, &ssid).await { let _ = s.network.connect(ssid, pass).await; } }
                             });
                         });
                         row.append(&b);
                    }
                    if n.known {
                         let b = Button::with_label("Forget"); let s=s.clone(); let ssid=n.ssid.clone();
                         b.connect_clicked(move |_| { let s=s.clone(); let ssid=ssid.clone(); glib::MainContext::default().spawn_local(async move {let _=s.network.forget(ssid).await;}); });
                         row.append(&b);
                    }
                    wl.append(&row);
                }
            }
        });
    });
    b.append(&wifi_btn);

    let bt_btn = Button::with_label("Bluetooth >");
    let bt_pop = Popover::builder().child(&GtkBox::new(Orientation::Vertical, 5)).build();
    bt_pop.set_parent(&bt_btn);
    let b_list = ListBox::new();
    let scroll_bt = ScrolledWindow::builder().min_content_height(250).min_content_width(280).child(&b_list).build();
    let head_bt = GtkBox::new(Orientation::Horizontal, 5); head_bt.append(&Label::new(Some("Bluetooth")));
    let sw_bt = Switch::new(); sw_bt.set_valign(Align::Center); let s_bt = syd.clone();
    sw_bt.connect_state_set(move |_, st| { let s=s_bt.clone(); glib::MainContext::default().spawn_local(async move{let _=s.bluetooth.set_power(st).await;}); glib::Propagation::Proceed });
    head_bt.append(&sw_bt);
    let pb = bt_pop.child().unwrap().downcast::<GtkBox>().unwrap(); pb.append(&head_bt); pb.append(&scroll_bt);
    let s = syd.clone(); let bl = b_list.clone(); let sw_bt_c = sw_bt.clone();
    bt_btn.connect_clicked(move |_| {
        bt_pop.popup();
        while let Some(c) = bl.first_child() { bl.remove(&c); }
        bl.append(&Label::new(Some("Loading...")));
        let s=s.clone(); let bl=bl.clone(); let sw_bt=sw_bt_c.clone();
        glib::MainContext::default().spawn_local(async move {
            if let Ok(p) = s.bluetooth.get_power().await { sw_bt.set_active(p); }
            if let Ok(devs) = s.bluetooth.get_devices().await {
                while let Some(c) = bl.first_child() { bl.remove(&c); }
                if devs.is_empty() { bl.append(&Label::new(Some("No devices"))); }
                for d in devs {
                    let row = GtkBox::new(Orientation::Horizontal, 10);
                    let icon = if d.connected { "🟢" } else { "⚪" };
                    let lbl = Label::new(Some(&format!("{} {}", icon, d.name))); lbl.set_hexpand(true); lbl.set_halign(Align::Start); row.append(&lbl);
                    let b = Button::with_label(if d.connected { "Disconnect" } else { "Connect" });
                    let s=s.clone(); let mac=d.mac.clone(); let do_con = !d.connected;
                    b.connect_clicked(move |_| { let s=s.clone(); let m=mac.clone(); glib::MainContext::default().spawn_local(async move{let _=s.bluetooth.connect_device(m, do_con).await;}); });
                    row.append(&b);
                    bl.append(&row);
                }
            }
        });
    });
    b.append(&bt_btn);
    (b, sw, sw_bt)
}
fn create_power_section(syd: Arc<Syd>) -> GtkBox {
    let b = GtkBox::new(Orientation::Vertical, 10);
    let row = GtkBox::new(Orientation::Horizontal, 10); row.append(&Label::new(Some("Profile:")));
    let dd = DropDown::new(None::<StringList>, None::<gtk4::Expression>);
    let sl = StringList::new(&[]); dd.set_model(Some(&sl));
    let profiles_store = Rc::new(RefCell::new(Vec::<String>::new()));
    let s = syd.clone(); let ps = profiles_store.clone(); let dd_weak = dd.clone();
    glib::MainContext::default().spawn_local(async move {
        if let Ok(profs) = s.system.get_power_profiles().await {
            for p in profs { ps.borrow_mut().push(p.clone()); sl.append(&p); }
            if let Ok(cur) = s.system.get_current_profile().await {
                if let Some(idx) = ps.borrow().iter().position(|x| x == &cur) { dd_weak.set_selected(idx as u32); }
            }
        }
    });
    let s = syd.clone(); let ps = profiles_store.clone();
    dd.connect_selected_notify(move |d| {
        let idx = d.selected() as usize;
        if let Some(p) = ps.borrow().get(idx) {
             let s=s.clone(); let p=p.clone(); glib::MainContext::default().spawn_local(async move { let _=s.system.set_profile(p).await; });
        }
    });
    row.append(&dd); b.append(&row);
    let acts = GtkBox::new(Orientation::Horizontal, 20); acts.set_halign(Align::Center);
    let off = Button::with_label("Power Off"); let reb = Button::with_label("Reboot");
    let s=syd.clone(); off.connect_clicked(move |_| { let s=s.clone(); glib::MainContext::default().spawn_local(async move{let _=s.system.power_off().await;}); });
    let s=syd.clone(); reb.connect_clicked(move |_| { let s=s.clone(); glib::MainContext::default().spawn_local(async move{let _=s.system.reboot().await;}); });
    acts.append(&off); acts.append(&reb); b.append(&acts);
    b
}
fn create_slider(name: &str, syd: Arc<Syd>, is_vol: bool) -> (Scale, GtkBox) {
    let b = GtkBox::new(Orientation::Vertical, 5); b.append(&Label::new(Some(name)));
    let s = Scale::with_range(Orientation::Horizontal, 0.0, 100.0, 1.0); b.append(&s);
    let (tx, mut rx) = mpsc::channel::<u32>(10);
    let bg_syd = syd.clone();
    glib::MainContext::default().spawn_local(async move {
        while let Some(val) = rx.recv().await {
            if is_vol { let _ = bg_syd.audio.set_volume(val).await; } else { let _ = bg_syd.brightness.set_brightness(val).await; }
            let mut last = val; while let Ok(v) = rx.try_recv() { last = v; }
            if last != val { if is_vol { let _ = bg_syd.audio.set_volume(last).await; } else { let _ = bg_syd.brightness.set_brightness(last).await; } }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    });
    s.connect_value_changed(move |sc| { let _ = tx.try_send(sc.value() as u32); });
    (s, b)
}
fn create_toast() -> (Revealer, Label) {
    let r = Revealer::builder().valign(Align::Start).halign(Align::Center).transition_type(gtk4::RevealerTransitionType::SlideDown).build();
    let b = GtkBox::new(Orientation::Vertical, 5); b.add_css_class("osd"); b.set_width_request(250);
    let l = Label::new(None); l.set_wrap(true);
    b.append(&l); r.set_child(Some(&b));
    (r, l)
}
async fn prompt_pass(parent: &ApplicationWindow, ssid: &str) -> Option<String> {
    let d = gtk4::Window::builder().transient_for(parent).modal(true).title("Password").default_width(300).build();
    let b = GtkBox::new(Orientation::Vertical, 10); b.set_margin_top(20); b.set_margin_start(20); b.set_margin_end(20); b.set_margin_bottom(20);
    b.append(&Label::new(Some(&format!("Enter password for {}", ssid))));
    let e = PasswordEntry::new(); e.set_activates_default(true); b.append(&e);
    let row = GtkBox::new(Orientation::Horizontal, 10); row.set_halign(Align::End);
    let cn = Button::with_label("Cancel"); let ok = Button::with_label("Connect"); ok.add_css_class("suggested-action"); d.set_default_widget(Some(&ok));
    row.append(&cn); row.append(&ok); b.append(&row); d.set_child(Some(&b)); d.present();
    let (tx, rx) = futures::channel::oneshot::channel(); let tx = Rc::new(RefCell::new(Some(tx)));
    let tx1 = tx.clone(); let d1 = d.clone(); let e1 = e.clone();
    ok.connect_clicked(move |_| { if let Some(t) = tx1.borrow_mut().take() { let _=t.send(Some(e1.text().to_string())); } d1.close(); });
    let tx2 = tx.clone(); let d2 = d.clone();
    cn.connect_clicked(move |_| { if let Some(t) = tx2.borrow_mut().take() { let _=t.send(None); } d2.close(); });
    rx.await.unwrap_or(None)
}
