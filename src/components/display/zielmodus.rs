use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

use super::helpers::qdbus_ausfuehren;
use crate::services::commands::{is_kde_desktop, run_command_blocking};
use crate::services::config::AppConfig;

pub struct ZielmodusModel {
    aktiv: bool,
    kde_verfuegbar: bool,
}

#[derive(Debug)]
pub enum ZielmodusMsg {
    AktivSetzen(bool),
}

#[derive(Debug)]
pub enum ZielmodusCommandOutput {
    AktivGelesen(bool),
    AktivGesetzt(bool),
    Fehler(String),
}

#[relm4::component(pub)]
impl Component for ZielmodusModel {
    type Init = ();
    type Input = ZielmodusMsg;
    type Output = String;
    type CommandOutput = ZielmodusCommandOutput;

    view! {
        adw::PreferencesGroup {
            set_title: &t!("zielmodus_group_title"),
            set_description: Some(&t!("zielmodus_group_desc")),

            add = &gtk::Label {
                #[watch]
                set_visible: !model.kde_verfuegbar,
                set_label: &t!("zielmodus_kde_required"),
                add_css_class: "error",
                set_wrap: true,
                set_xalign: 0.0,
                set_margin_top: 8,
                set_margin_start: 12,
                set_margin_end: 12,
                set_margin_bottom: 4,
            },

            add = &adw::SwitchRow {
                set_title: &t!("zielmodus_switch_title"),
                set_subtitle: &t!("zielmodus_switch_subtitle"),

                #[watch]
                set_active: model.aktiv,
                #[watch]
                set_sensitive: model.kde_verfuegbar,

                connect_active_notify[sender] => move |switch| {
                    sender.input(ZielmodusMsg::AktivSetzen(switch.is_active()));
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
        let kde_verfuegbar = is_kde_desktop();

        let model = ZielmodusModel {
            aktiv: config.zielmodus_aktiv,
            kde_verfuegbar,
        };
        let widgets = view_output!();

        if kde_verfuegbar {
            let fallback = config.zielmodus_aktiv;
            sender.command(move |out, shutdown| {
                shutdown
                    .register(async move {
                        let aktiv = tokio::task::spawn_blocking(move || {
                            lese_kwin_bool("Plugins", "diminactiveEnabled")
                        })
                        .await
                        .ok()
                        .flatten()
                        .unwrap_or(fallback);
                        out.emit(ZielmodusCommandOutput::AktivGelesen(aktiv));
                    })
                    .drop_on_shutdown()
            });
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: ZielmodusMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            ZielmodusMsg::AktivSetzen(aktiv) => {
                if aktiv == self.aktiv {
                    return;
                }
                self.aktiv = aktiv;
                AppConfig::update(|c| c.zielmodus_aktiv = aktiv);

                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            match kwin_effekt_setzen(aktiv).await {
                                Ok(()) => out.emit(ZielmodusCommandOutput::AktivGesetzt(aktiv)),
                                Err(e) => out.emit(ZielmodusCommandOutput::Fehler(e)),
                            }
                        })
                        .drop_on_shutdown()
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: ZielmodusCommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            ZielmodusCommandOutput::AktivGelesen(aktiv) => {
                self.aktiv = aktiv;
                AppConfig::update(|c| c.zielmodus_aktiv = aktiv);
            }
            ZielmodusCommandOutput::AktivGesetzt(aktiv) => {
                eprintln!("{}", t!("zielmodus_aktiv_set", value = aktiv.to_string()));
            }
            ZielmodusCommandOutput::Fehler(e) => {
                let _ = sender.output(e);
            }
        }
    }
}

async fn kwin_effekt_setzen(aktiv: bool) -> Result<(), String> {
    let wert = if aktiv { "true" } else { "false" };
    run_command_blocking(
        "kwriteconfig6",
        &[
            "--file",
            "kwinrc",
            "--group",
            "Plugins",
            "--key",
            "diminactiveEnabled",
            "--type",
            "bool",
            wert,
        ],
    )
    .await?;

    let method = if aktiv { "loadEffect" } else { "unloadEffect" };
    qdbus_ausfuehren(vec![
        "org.kde.KWin".to_string(),
        "/Effects".to_string(),
        method.to_string(),
        "diminactive".to_string(),
    ])
    .await
}

fn lese_kwin_bool(group: &str, key: &str) -> Option<bool> {
    let output = std::process::Command::new("kreadconfig6")
        .args([
            "--file",
            "kwinrc",
            "--group",
            group,
            "--key",
            key,
            "--default",
            "false",
        ])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_lowercase();
    Some(s == "true")
}
