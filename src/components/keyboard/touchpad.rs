use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;

use crate::components::display::helpers::qdbus_ausfuehren;
use crate::services::commands::run_command_blocking;
use crate::services::config::AppConfig;

pub struct TouchpadModel {
    touchpad_aktiv: bool,
    countdown: u8,
    bestaetigung_erforderlich: bool,
    timer_handle: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Debug)]
pub enum TouchpadMsg {
    TouchpadUmschalten(bool),
    BestaetigungGeklickt,
}

#[derive(Debug)]
pub enum TouchpadCommandOutput {
    Fehler(String),
    CountdownTick,
    TimerAbgelaufen,
}

#[relm4::component(pub)]
impl Component for TouchpadModel {
    type Init = ();
    type Input = TouchpadMsg;
    type Output = String;
    type CommandOutput = TouchpadCommandOutput;

    view! {
        adw::PreferencesGroup {
            set_title: "Touchpad",

            add = &gtk::ListBox {
                set_hexpand: true,
                add_css_class: "boxed-list",

                append = &adw::SwitchRow {
                    set_title: "Touchpad aktivieren",
                    set_subtitle: "Schaltet das Touchpad vollständig ein oder aus.",

                    #[watch]
                    set_active: model.touchpad_aktiv,

                    connect_active_notify[sender] => move |s| {
                        sender.input(TouchpadMsg::TouchpadUmschalten(s.is_active()));
                    },
                },

                append = &adw::ActionRow {
                    #[watch]
                    set_visible: model.bestaetigung_erforderlich,

                    #[watch]
                    set_title: &format!(
                        "Touchpad deaktiviert. Automatische Reaktivierung in {} Sekunden...",
                        model.countdown
                    ),

                    add_suffix = &gtk::Button {
                        set_label: "Einstellung bestätigen",
                        add_css_class: "suggested-action",
                        set_valign: gtk::Align::Center,

                        connect_clicked[sender] => move |_| {
                            sender.input(TouchpadMsg::BestaetigungGeklickt);
                        },
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
        let touchpad_aktiv = AppConfig::load().touchpad_aktiv;
        let model = TouchpadModel {
            touchpad_aktiv,
            countdown: 10,
            bestaetigung_erforderlich: false,
            timer_handle: None,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: TouchpadMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            TouchpadMsg::TouchpadUmschalten(aktiv) => {
                if aktiv == self.touchpad_aktiv {
                    return;
                }
                self.touchpad_aktiv = aktiv;

                if let Some(handle) = self.timer_handle.take() {
                    handle.abort();
                }

                if !aktiv {
                    self.bestaetigung_erforderlich = true;
                    self.countdown = 10;

                    let cmd_sender = sender.command_sender().clone();
                    self.timer_handle = Some(tokio::spawn(async move {
                        for _ in 0..10 {
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            cmd_sender.emit(TouchpadCommandOutput::CountdownTick);
                        }
                        cmd_sender.emit(TouchpadCommandOutput::TimerAbgelaufen);
                    }));
                } else {
                    self.bestaetigung_erforderlich = false;
                }

                AppConfig::update(|c| c.touchpad_aktiv = aktiv);

                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            if let Err(e) = touchpad_befehl_ausfuehren(aktiv).await {
                                out.emit(TouchpadCommandOutput::Fehler(e));
                            }
                        })
                        .drop_on_shutdown()
                });
            }
            TouchpadMsg::BestaetigungGeklickt => {
                if let Some(handle) = self.timer_handle.take() {
                    handle.abort();
                }
                self.bestaetigung_erforderlich = false;
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: TouchpadCommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            TouchpadCommandOutput::CountdownTick => {
                self.countdown = self.countdown.saturating_sub(1);
            }
            TouchpadCommandOutput::TimerAbgelaufen => {
                if !self.bestaetigung_erforderlich {
                    return;
                }
                self.touchpad_aktiv = true;
                self.bestaetigung_erforderlich = false;
                self.timer_handle = None;

                AppConfig::update(|c| c.touchpad_aktiv = true);

                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            if let Err(e) = touchpad_befehl_ausfuehren(true).await {
                                out.emit(TouchpadCommandOutput::Fehler(e));
                            }
                        })
                        .drop_on_shutdown()
                });
            }
            TouchpadCommandOutput::Fehler(e) => {
                let _ = sender.output(e);
            }
        }
    }
}

async fn touchpad_befehl_ausfuehren(aktiv: bool) -> Result<(), String> {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_lowercase();

    if desktop.contains("gnome") {
        let wert = if aktiv { "enabled" } else { "disabled" };
        run_command_blocking(
            "gsettings",
            &[
                "set",
                "org.gnome.desktop.peripherals.touchpad",
                "send-events",
                wert,
            ],
        )
        .await
    } else if desktop.contains("kde") {
        let methode = if aktiv {
            "org.kde.touchpad.enable"
        } else {
            "org.kde.touchpad.disable"
        };
        qdbus_ausfuehren(vec![
            "org.kde.kglobalaccel".to_string(),
            "/modules/kded_touchpad".to_string(),
            methode.to_string(),
        ])
        .await
        .map_err(|e| format!("KDE Touchpad-Toggle fehlgeschlagen. {e}"))
    } else {
        Err(format!(
            "Desktop-Umgebung '{desktop}' wird für \
             Touchpad-Steuerung nicht unterstützt."
        ))
    }
}
