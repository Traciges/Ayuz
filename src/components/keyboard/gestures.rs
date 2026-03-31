use gtk::gdk;
use gtk::glib;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use tokio::sync::watch;

use crate::services::config::AppConfig;
use crate::services::edge_gestures;

pub struct GesturenModel {
    aktiv: bool,
    loop_tx: Option<watch::Sender<bool>>,
}

#[derive(Debug)]
pub enum GesturenMsg {
    GestenUmschalten(bool),
}

const GESTURE_IMG: &[u8] = include_bytes!("../../../assets/img/gesture.png");

#[relm4::component(pub)]
impl Component for GesturenModel {
    type Init = ();
    type Input = GesturenMsg;
    type Output = String;
    type CommandOutput = ();

    view! {
        adw::PreferencesGroup {
            set_title: "Intelligente Gesten",
            set_description: Some("Greifen Sie schnell auf häufig verwendete Einstellungen und Apps zu."),

            add = &gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 24,

                #[name = "gesten_bild"]
                append = &gtk::Picture {
                    set_width_request: 300,
                    set_valign: gtk::Align::Start,
                },

                append = &gtk::ListBox {
                    set_hexpand: true,
                    set_valign: gtk::Align::Start,
                    add_css_class: "boxed-list",

                    append = &adw::SwitchRow {
                        set_title: "Erweiterte Gesten aktivieren/deaktivieren",

                        #[watch]
                        set_active: model.aktiv,

                        connect_active_notify[sender] => move |s| {
                            sender.input(GesturenMsg::GestenUmschalten(s.is_active()));
                        },
                    },

                    append = &adw::ActionRow {
                        set_title: "Lautstärke einstellen",
                        set_subtitle: "Wischen Sie mit einem Finger am linken Rand nach oben oder unten.",
                    },

                    append = &adw::ActionRow {
                        set_title: "Helligkeit einstellen",
                        set_subtitle: "Wischen Sie mit einem Finger am rechten Rand nach oben oder unten.",
                    },

                    append = &adw::ActionRow {
                        set_title: "Vor-/Zurückspulen",
                        set_subtitle: "Wischen Sie mit einem Finger am oberen Rand nach links oder rechts.",
                    },
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let aktiv = AppConfig::load().input_gesten_aktiv;
        let loop_tx = if aktiv {
            Some(start_gesture_loop())
        } else {
            None
        };
        let model = GesturenModel { aktiv, loop_tx };
        let widgets = view_output!();

        let bytes = glib::Bytes::from_static(GESTURE_IMG);
        if let Ok(texture) = gdk::Texture::from_bytes(&bytes) {
            widgets.gesten_bild.set_paintable(Some(&texture));
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: GesturenMsg, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            GesturenMsg::GestenUmschalten(aktiv) => {
                if aktiv == self.aktiv {
                    return;
                }
                self.aktiv = aktiv;
                AppConfig::update(|c| c.input_gesten_aktiv = aktiv);

                if aktiv {
                    self.loop_tx = Some(start_gesture_loop());
                } else {
                    // Dropping the sender causes the loop to exit
                    self.loop_tx = None;
                }
            }
        }
    }
}

fn start_gesture_loop() -> watch::Sender<bool> {
    let (tx, rx) = watch::channel(true);
    tokio::spawn(edge_gestures::run_gesture_loop(rx));
    tx
}
