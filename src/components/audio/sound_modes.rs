use gtk4 as gtk;
use gtk4::gio;
use gtk4::glib;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;
use std::path::PathBuf;

use crate::services::commands::run_command_blocking;
use crate::services::config::AppConfig;

const PRESET_MUSIK: &str = include_str!("../../../assets/presets/Music.json");
const PRESET_FILM: &str = include_str!("../../../assets/presets/Movie.json");
const PRESET_VIDEO: &str = include_str!("../../../assets/presets/Video.json");
const PRESET_SPRACHE: &str = include_str!("../../../assets/presets/Voice.json");
const PRESET_ANGEPASST: &str = include_str!("../../../assets/presets/Perfect_EQ.json");

const PRESETS: &[(&str, &str)] = &[
    ("Movie", PRESET_FILM),
    ("Music", PRESET_MUSIK),
    ("Perfect_EQ", PRESET_ANGEPASST),
    ("Video", PRESET_VIDEO),
    ("Voice", PRESET_SPRACHE),
];

// Index 0..6: Movie, Music, None(bypass), Perfect_EQ, Video, Voice, Custom
// Index 2 = None (kein Preset, nur Bypass an)
const NONE_IDX: u32 = 2;
const CUSTOM_IDX: u32 = 6;
const PRESET_NAMEN: &[&str] = &["Movie", "Music", "Perfect_EQ", "Video", "Voice"];
const EASYEFFECTS_STARTUP_DELAY_MS: u64 = 1500;

pub struct SoundModesModel {
    ee_installiert: bool,
    aktuelles_profil: u32,
    vorheriges_profil: u32,
    dropdown: gtk::DropDown,
}

#[derive(Debug)]
pub enum AudioMsg {
    ProfilWechseln(u32),
    CustomPresetPfadGewaehlt(PathBuf),
    CustomAbgebrochen(u32),
}

#[derive(Debug)]
pub enum AudioCommandOutput {
    EeGeprueft(bool),
    PresetsInstalliert,
    ProfilGesetzt(u32),
    CustomPresetGeladen(String),
    Fehler(String),
}

#[relm4::component(pub)]
impl Component for SoundModesModel {
    type Init = ();
    type Input = AudioMsg;
    type Output = String;
    type CommandOutput = AudioCommandOutput;

    view! {
        adw::PreferencesGroup {
            set_title: &t!("audio_profiles_title"),
            set_description: Some(&t!("audio_profiles_desc")),

            add = &gtk::Label {
                #[watch]
                set_visible: !model.ee_installiert,
                set_label: &t!("ee_missing_warning"),
                add_css_class: "error",
                set_wrap: true,
                set_xalign: 0.0,
                set_margin_top: 8,
                set_margin_start: 12,
                set_margin_end: 12,
                set_margin_bottom: 4,
            },

            add = &adw::ActionRow {
                set_title: &t!("audio_profile_label"),
                add_suffix = &model.dropdown.clone(),
                set_activatable_widget: Some(&model.dropdown),
                #[watch]
                set_sensitive: model.ee_installiert,
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let config = AppConfig::load();

        let optionen = gtk::StringList::new(&[
            &t!("audio_profile_film"),
            &t!("audio_profile_musik"),
            &t!("audio_profile_none"),
            &t!("audio_profile_optimiert"),
            &t!("audio_profile_video"),
            &t!("audio_profile_sprache"),
            &t!("audio_profile_custom"),
        ]);
        let dropdown = gtk::DropDown::new(Some(optionen), gtk::Expression::NONE);
        dropdown.set_valign(gtk::Align::Center);
        dropdown.set_selected(config.audio_profil);
        dropdown.set_sensitive(false); // bis EE-Check abgeschlossen

        {
            let sender = sender.clone();
            dropdown.connect_selected_notify(move |dd| {
                sender.input(AudioMsg::ProfilWechseln(dd.selected()));
            });
        }

        let model = SoundModesModel {
            ee_installiert: false,
            aktuelles_profil: config.audio_profil,
            vorheriges_profil: config.audio_profil,
            dropdown,
        };

        let widgets = view_output!();

        // EasyEffects-Check
        sender.command(move |out, shutdown| {
            shutdown
                .register(async move {
                    let installiert = tokio::task::spawn_blocking(|| {
                        std::process::Command::new("which")
                            .arg("easyeffects")
                            .status()
                            .map(|s| s.success())
                            .unwrap_or(false)
                    })
                    .await
                    .unwrap_or(false);
                    out.emit(AudioCommandOutput::EeGeprueft(installiert));
                })
                .drop_on_shutdown()
        });

        // Presets installieren
        sender.command(move |out, shutdown| {
            shutdown
                .register(async move {
                    match presets_installieren().await {
                        Ok(()) => out.emit(AudioCommandOutput::PresetsInstalliert),
                        Err(e) => out.emit(AudioCommandOutput::Fehler(
                            t!("audio_preset_install_error", error = e).to_string(),
                        )),
                    }
                })
                .drop_on_shutdown()
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: AudioMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            AudioMsg::ProfilWechseln(idx) => {
                if idx == self.aktuelles_profil {
                    return;
                }

                if idx == CUSTOM_IDX {
                    let vorheriges = self.aktuelles_profil;
                    self.aktuelles_profil = CUSTOM_IDX;

                    let sender_clone = sender.clone();
                    let dialog = gtk::FileDialog::builder()
                        .title(t!("audio_profile_custom").as_ref())
                        .accept_label("Open")
                        .build();
                    let filter = gtk::FileFilter::new();
                    filter.add_pattern("*.json");
                    filter.set_name(Some("JSON"));
                    let store = gio::ListStore::new::<gtk::FileFilter>();
                    store.append(&filter);
                    dialog.set_filters(Some(&store));

                    glib::spawn_future_local(async move {
                        match dialog.open_future(None::<&gtk::Window>).await {
                            Ok(file) => {
                                if let Some(path) = file.path() {
                                    sender_clone.input(AudioMsg::CustomPresetPfadGewaehlt(path));
                                } else {
                                    sender_clone.input(AudioMsg::CustomAbgebrochen(vorheriges));
                                }
                            }
                            Err(_) => {
                                sender_clone.input(AudioMsg::CustomAbgebrochen(vorheriges));
                            }
                        }
                    });
                    return;
                }

                self.vorheriges_profil = self.aktuelles_profil;
                self.aktuelles_profil = idx;
                AppConfig::update(|c| c.audio_profil = idx);

                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            if let Err(e) = easyeffects_profil_setzen(idx, None).await {
                                out.emit(AudioCommandOutput::Fehler(e));
                                return;
                            }
                            out.emit(AudioCommandOutput::ProfilGesetzt(idx));
                        })
                        .drop_on_shutdown()
                });
            }

            AudioMsg::CustomPresetPfadGewaehlt(path) => {
                let name = match extract_file_stem(&path) {
                    Ok(n) => n,
                    Err(_) => {
                        sender.input(AudioMsg::CustomAbgebrochen(self.vorheriges_profil));
                        return;
                    }
                };

                AppConfig::update(|c| {
                    c.audio_profil = CUSTOM_IDX;
                    c.custom_preset_name = Some(name.clone());
                });

                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            match custom_preset_laden(path).await {
                                Ok(n) => out.emit(AudioCommandOutput::CustomPresetGeladen(n)),
                                Err(e) => out.emit(AudioCommandOutput::Fehler(e)),
                            }
                        })
                        .drop_on_shutdown()
                });
            }

            AudioMsg::CustomAbgebrochen(vorheriges) => {
                self.aktuelles_profil = vorheriges;
                self.dropdown.set_selected(vorheriges);
                AppConfig::update(|c| c.audio_profil = vorheriges);
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: AudioCommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            AudioCommandOutput::EeGeprueft(installiert) => {
                self.ee_installiert = installiert;
            }
            AudioCommandOutput::PresetsInstalliert => {}
            AudioCommandOutput::ProfilGesetzt(idx) => {
                eprintln!("{}", t!("audio_profile_set", profile = idx));
            }
            AudioCommandOutput::CustomPresetGeladen(name) => {
                eprintln!("{}", t!("audio_profile_set", profile = name));
            }
            AudioCommandOutput::Fehler(e) => {
                let _ = sender.output(e);
            }
        }
    }
}

async fn ensure_easyeffects_running() {
    let daemon_laeuft = tokio::task::spawn_blocking(|| {
        std::process::Command::new("pgrep")
            .args(["-x", "easyeffects"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
    .await
    .unwrap_or(false);

    if !daemon_laeuft {
        let _ = tokio::process::Command::new("easyeffects")
            .arg("--gapplication-service")
            .spawn();
        tokio::time::sleep(tokio::time::Duration::from_millis(EASYEFFECTS_STARTUP_DELAY_MS)).await;
    }
}

fn easyeffects_output_dir() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|e| e.to_string())?;
    Ok(PathBuf::from(home).join(".config/easyeffects/output"))
}

fn extract_file_stem(path: &std::path::Path) -> Result<String, String> {
    path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .ok_or_else(|| "Invalid file name".to_string())
}

async fn easyeffects_profil_setzen(idx: u32, custom_name: Option<String>) -> Result<(), String> {
    ensure_easyeffects_running().await;

    if idx == NONE_IDX {
        run_command_blocking("easyeffects", &["-b", "1"]).await?;
    } else if idx == CUSTOM_IDX {
        if let Some(name) = custom_name {
            run_command_blocking("easyeffects", &["-b", "2"]).await?;
            run_command_blocking("easyeffects", &["-l", &name]).await?;
        }
    } else {
        run_command_blocking("easyeffects", &["-b", "2"]).await?;
        let preset_idx = if idx < NONE_IDX { idx } else { idx - 1 } as usize;
        run_command_blocking("easyeffects", &["-l", PRESET_NAMEN[preset_idx]]).await?;
    }

    Ok(())
}

async fn custom_preset_laden(path: PathBuf) -> Result<String, String> {
    let name = extract_file_stem(&path)?;

    let dest = easyeffects_output_dir()?.join(format!("{name}.json"));
    tokio::fs::copy(&path, &dest)
        .await
        .map_err(|e| e.to_string())?;

    ensure_easyeffects_running().await;

    run_command_blocking("easyeffects", &["-b", "2"]).await?;
    run_command_blocking("easyeffects", &["-l", &name]).await?;

    Ok(name)
}

async fn presets_installieren() -> Result<(), String> {
    let dir = easyeffects_output_dir()?;
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| e.to_string())?;
    for (name, content) in PRESETS {
        let path = dir.join(format!("{}.json", name));
        if !path.exists() {
            tokio::fs::write(&path, content)
                .await
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
