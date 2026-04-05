use futures_util::StreamExt;
use gtk4 as gtk;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

use crate::components::display::helpers::DISPLAY_NAME;
use crate::services::commands::run_command_blocking;
use crate::services::config::AppConfig;

#[zbus::proxy(
    interface = "org.kde.Solid.PowerManagement.Actions.BrightnessControl",
    default_service = "org.kde.Solid.PowerManagement",
    default_path = "/org/kde/Solid/PowerManagement/Actions/BrightnessControl"
)]
trait BrightnessControl {
    #[zbus(signal, name = "brightnessChanged")]
    fn brightness_changed(&self, brightness: i32) -> zbus::Result<()>;
}

pub struct OledDimmingModel {
    helligkeit: u32,
}

#[derive(Debug)]
pub enum OledDimmingMsg {
    HelligkeitSetzen(u32),
}

#[derive(Debug)]
pub enum OledDimmingCommandOutput {
    Gesetzt(u32),
    Fehler(String),
    HelligkeitGeaendert,
}

#[relm4::component(pub)]
impl Component for OledDimmingModel {
    type Init = ();
    type Input = OledDimmingMsg;
    type Output = String;
    type CommandOutput = OledDimmingCommandOutput;

    view! {
        adw::PreferencesGroup {
            set_title: &t!("oled_dimming_group_title"),
            set_description: Some(&t!("oled_dimming_group_desc")),

            add = &gtk::Label {
                set_label: &t!("oled_dimming_warning"),
                add_css_class: "error",
                set_wrap: true,
                set_xalign: 0.0,
                set_margin_top: 8,
                set_margin_start: 12,
                set_margin_end: 12,
                set_margin_bottom: 4,
            },

            add = &adw::ActionRow {
                set_title: &t!("oled_dimming_slider_title"),

                add_suffix = &gtk::Scale {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_range: (10.0, 100.0),
                    set_increments: (5.0, 10.0),
                    set_round_digits: 0,
                    set_value: model.helligkeit as f64,
                    set_width_request: 200,
                    connect_value_changed[sender] => move |scale| {
                        sender.input(OledDimmingMsg::HelligkeitSetzen(scale.value() as u32));
                    },
                },

                add_suffix = &gtk::Label {
                    #[watch]
                    set_label: &format!("{}%", model.helligkeit),
                    set_width_chars: 4,
                    set_xalign: 1.0,
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let config = AppConfig::load();
        let helligkeit = config.oled_dc_dimming;

        let model = OledDimmingModel { helligkeit };
        let widgets = view_output!();

        if helligkeit < 100 {
            sender.command(move |out, shutdown| {
                shutdown
                    .register(async move {
                        match kscreen_doctor_helligkeit(helligkeit).await {
                            Ok(()) => out.emit(OledDimmingCommandOutput::Gesetzt(helligkeit)),
                            Err(e) => out.emit(OledDimmingCommandOutput::Fehler(e)),
                        }
                    })
                    .drop_on_shutdown()
            });
        }

        let out = sender.command_sender().clone();
        tokio::spawn(start_brightness_listener(out));

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: OledDimmingMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            OledDimmingMsg::HelligkeitSetzen(wert) => {
                if wert == self.helligkeit {
                    return;
                }
                self.helligkeit = wert;
                AppConfig::update(|c| c.oled_dc_dimming = wert);

                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            match kscreen_doctor_helligkeit(wert).await {
                                Ok(()) => out.emit(OledDimmingCommandOutput::Gesetzt(wert)),
                                Err(e) => out.emit(OledDimmingCommandOutput::Fehler(e)),
                            }
                        })
                        .drop_on_shutdown()
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: OledDimmingCommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            OledDimmingCommandOutput::Gesetzt(wert) => {
                eprintln!("{}", t!("oled_dimming_set", value = wert.to_string()));
            }
            OledDimmingCommandOutput::Fehler(e) => {
                let _ = sender.output(e);
            }
            OledDimmingCommandOutput::HelligkeitGeaendert => {
                let wert = self.helligkeit;
                if wert < 100 {
                    sender.command(move |out, shutdown| {
                        shutdown
                            .register(async move {
                                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                                match kscreen_doctor_helligkeit(wert).await {
                                    Ok(()) => out.emit(OledDimmingCommandOutput::Gesetzt(wert)),
                                    Err(e) => out.emit(OledDimmingCommandOutput::Fehler(e)),
                                }
                            })
                            .drop_on_shutdown()
                    });
                }
            }
        }
    }
}

async fn kscreen_doctor_helligkeit(wert: u32) -> Result<(), String> {
    let arg = format!("output.{}.dimming.{}", DISPLAY_NAME, wert);
    run_command_blocking("kscreen-doctor", &[&arg]).await
}

async fn start_brightness_listener(out: relm4::Sender<OledDimmingCommandOutput>) {
    let conn = match zbus::Connection::session().await {
        Ok(c) => c,
        Err(_) => return,
    };
    let proxy = match BrightnessControlProxy::new(&conn).await {
        Ok(p) => p,
        Err(_) => return,
    };
    let mut stream = match proxy.receive_brightness_changed().await {
        Ok(s) => s,
        Err(_) => return,
    };
    while stream.next().await.is_some() {
        out.emit(OledDimmingCommandOutput::HelligkeitGeaendert);
    }
}
