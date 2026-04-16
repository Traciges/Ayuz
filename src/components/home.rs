// Asus Hub - Unofficial Control Center for Asus Laptops
// Copyright (C) 2026 Guido Philipp
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see https://www.gnu.org/licenses/.

use gtk4 as gtk;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

use crate::services::commands::pkexec_read_file;
use crate::sys_paths::*;

pub struct HomeModel {
    product_name_label: gtk::Label,
    board_row: adw::ActionRow,
    bios_row: adw::ActionRow,
    kernel_row: adw::ActionRow,
    serial_row: adw::ActionRow,
    reveal_button: gtk::Button,
    metrics_box: gtk::Box,
    battery_label: gtk::Label,
    cpu_label: gtk::Label,
    ram_label: gtk::Label,
    disk_label: gtk::Label,
}

#[derive(Debug)]
pub enum HomeMsg {
    RevealSerial,
}

#[derive(Debug)]
pub enum HomeCommandOutput {
    DataLoaded {
        product_name: String,
        board_name: String,
        bios_version: String,
        bios_date: String,
        kernel: String,
    },
    SerialRevealed(Result<String, String>),
    MetricsRefreshed {
        battery: String,
        cpu: String,
        ram: String,
        disk: String,
    },
}

fn metric_card(icon_name: &str, title: &str) -> (gtk::Box, gtk::Label) {
    let value_label = gtk::Label::builder()
        .css_classes(["title-2", "dim-label"])
        .halign(gtk::Align::Start)
        .label("…")
        .build();

    let icon = gtk::Image::from_icon_name(icon_name);
    icon.set_pixel_size(16);
    let title_label = gtk::Label::new(Some(title));

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    header.append(&icon);
    header.append(&title_label);

    let inner = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .build();
    inner.append(&header);
    inner.append(&value_label);

    let card = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .valign(gtk::Align::Start)
        .build();
    card.add_css_class("card");
    card.append(&inner);

    (card, value_label)
}

async fn fetch_metrics() -> HomeCommandOutput {
    let battery = {
        let b0 = tokio::fs::read_to_string(SYS_BATTERY0_CAPACITY)
            .await
            .ok()
            .and_then(|s| s.trim().parse::<u8>().ok());
        let b1 = tokio::fs::read_to_string(SYS_BATTERY1_CAPACITY)
            .await
            .ok()
            .and_then(|s| s.trim().parse::<u8>().ok());
        match (b0, b1) {
            (Some(a), Some(b)) => format!("{}%", (a as u16 + b as u16) / 2),
            (Some(a), None) | (None, Some(a)) => format!("{}%", a),
            (None, None) => "N/A".to_string(),
        }
    };

    let cpu = {
        let load = tokio::fs::read_to_string(SYS_LOAD_AVG)
            .await
            .map(|s| {
                s.split_whitespace()
                    .next()
                    .unwrap_or("?")
                    .to_string()
            })
            .unwrap_or_else(|_| "?".to_string());

        let temp = tokio::fs::read_to_string(SYS_THERMAL_ZONE0_TEMP)
            .await
            .map(|s| {
                let millideg: i32 = s.trim().parse().unwrap_or(0);
                format!("{}°C", millideg / 1000)
            })
            .unwrap_or_else(|_| "?°C".to_string());

        format!("{} | {}", load, temp)
    };

    let ram = tokio::fs::read_to_string(SYS_MEM_INFO)
        .await
        .map(|s| {
            let mut total: u64 = 0;
            let mut available: u64 = 0;
            for line in s.lines() {
                if line.starts_with("MemTotal:") {
                    total = line
                        .split_whitespace()
                        .nth(1)
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0);
                } else if line.starts_with("MemAvailable:") {
                    available = line
                        .split_whitespace()
                        .nth(1)
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0);
                }
                if total > 0 && available > 0 {
                    break;
                }
            }
            if total > 0 && available <= total {
                format!("{}%", (total - available) * 100 / total)
            } else {
                "N/A".to_string()
            }
        })
        .unwrap_or_else(|_| "N/A".to_string());

    let disk = tokio::process::Command::new("df")
        .args(["-h", "/"])
        .output()
        .await
        .map(|o| {
            let stdout = String::from_utf8_lossy(&o.stdout);
            stdout
                .lines()
                .nth(1)
                .and_then(|line| line.split_whitespace().nth(4))
                .map(|s| s.to_string())
                .unwrap_or_else(|| "N/A".to_string())
        })
        .unwrap_or_else(|_| "N/A".to_string());

    HomeCommandOutput::MetricsRefreshed {
        battery,
        cpu,
        ram,
        disk,
    }
}

#[relm4::component(pub)]
impl Component for HomeModel {
    type Init = ();
    type Input = HomeMsg;
    type Output = String;
    type CommandOutput = HomeCommandOutput;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 24,
            set_margin_top: 24,
            set_margin_bottom: 32,
            set_margin_start: 32,
            set_margin_end: 32,

            append = &adw::PreferencesGroup {
                add = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 24,

                    append = &model.product_name_label.clone(),

                    append = &gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 32,

                        append = &gtk::Image {
                            set_icon_name: Some("computer-symbolic"),
                            set_pixel_size: 192,
                            set_valign: gtk::Align::Center,
                        },

                        append = &adw::PreferencesGroup {
                            set_valign: gtk::Align::Center,
                            set_hexpand: true,

                            add = &model.board_row.clone(),
                            add = &model.bios_row.clone(),
                            add = &model.kernel_row.clone(),
                            add = &model.serial_row.clone(),
                        },
                    },
                },
            },

            append = &model.metrics_box.clone(),

            // Profiles placeholder
            append = &adw::PreferencesGroup {
                set_title: &t!("home_profiles_title"),

                add = &gtk::Label {
                    set_label: &t!("home_profiles_placeholder"),
                    set_margin_top: 12,
                    set_margin_bottom: 12,
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let product_name_label = gtk::Label::new(Some(&t!("home_loading")));
        product_name_label.add_css_class("title-1");
        product_name_label.set_halign(gtk::Align::Start);

        let board_row = adw::ActionRow::new();
        board_row.set_title(&t!("home_board_title"));
        board_row.set_selectable(false);

        let bios_row = adw::ActionRow::new();
        bios_row.set_title(&t!("home_bios_title"));
        bios_row.set_selectable(false);

        let kernel_row = adw::ActionRow::new();
        kernel_row.set_title(&t!("home_kernel_title"));
        kernel_row.set_selectable(false);

        let serial_row = adw::ActionRow::new();
        serial_row.set_title(&t!("home_serial_title"));
        serial_row.set_subtitle(&t!("home_serial_hidden"));
        serial_row.set_selectable(false);

        let reveal_button = gtk::Button::with_label(&t!("home_serial_reveal"));
        reveal_button.set_valign(gtk::Align::Center);
        reveal_button.add_css_class("flat");
        {
            let sender = sender.clone();
            reveal_button.connect_clicked(move |_| {
                sender.input(HomeMsg::RevealSerial);
            });
        }
        serial_row.add_suffix(&reveal_button);

        let (battery_card, battery_label) = metric_card("battery-symbolic", "Battery");
        let (cpu_card, cpu_label) = metric_card("system-run-symbolic", "CPU");
        let (ram_card, ram_label) = metric_card("media-flash-symbolic", "Memory");
        let (disk_card, disk_label) = metric_card("drive-harddisk-symbolic", "Disk");

        let metrics_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(16)
            .homogeneous(true)
            .build();
        metrics_box.append(&battery_card);
        metrics_box.append(&cpu_card);
        metrics_box.append(&ram_card);
        metrics_box.append(&disk_card);

        let model = HomeModel {
            product_name_label,
            board_row,
            bios_row,
            kernel_row,
            serial_row,
            reveal_button,
            metrics_box,
            battery_label,
            cpu_label,
            ram_label,
            disk_label,
        };

        let widgets = view_output!();

        // Load device info
        sender.command(move |out, shutdown| {
            shutdown
                .register(async move {
                    let product_name = tokio::fs::read_to_string(SYS_PRODUCT_NAME)
                        .await
                        .map(|s| s.trim().to_string())
                        .unwrap_or_default();

                    let board_name = tokio::fs::read_to_string(SYS_BOARD_NAME)
                        .await
                        .map(|s| s.trim().to_string())
                        .unwrap_or_default();

                    let bios_version = tokio::fs::read_to_string(SYS_BIOS_VERSION)
                        .await
                        .map(|s| s.trim().to_string())
                        .unwrap_or_default();

                    let bios_date = tokio::fs::read_to_string(SYS_BIOS_DATE)
                        .await
                        .map(|s| s.trim().to_string())
                        .unwrap_or_default();

                    let kernel = tokio::process::Command::new("uname")
                        .arg("-r")
                        .output()
                        .await
                        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                        .unwrap_or_default();

                    out.emit(HomeCommandOutput::DataLoaded {
                        product_name,
                        board_name,
                        bios_version,
                        bios_date,
                        kernel,
                    });
                })
                .drop_on_shutdown()
        });

        // Fetch metrics immediately, then every 5 seconds, cancelled on shutdown.
        sender.command(move |out, shutdown| {
            shutdown
                .register(async move {
                    loop {
                        out.emit(fetch_metrics().await);
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                })
                .drop_on_shutdown()
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: HomeMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            HomeMsg::RevealSerial => {
                self.reveal_button.set_sensitive(false);
                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            let result = pkexec_read_file(SYS_PRODUCT_SERIAL).await;
                            out.emit(HomeCommandOutput::SerialRevealed(result));
                        })
                        .drop_on_shutdown()
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: HomeCommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            HomeCommandOutput::DataLoaded {
                product_name,
                board_name,
                bios_version,
                bios_date,
                kernel,
            } => {
                self.product_name_label.set_label(&product_name);
                self.board_row.set_subtitle(&board_name);
                self.bios_row
                    .set_subtitle(&format!("{bios_version} / {bios_date}"));
                self.kernel_row.set_subtitle(&kernel);
            }
            HomeCommandOutput::SerialRevealed(Ok(serial)) => {
                self.serial_row.set_subtitle(&serial);
            }
            HomeCommandOutput::SerialRevealed(Err(e)) => {
                self.reveal_button.set_sensitive(true);
                let _ = sender.output(e);
            }
            HomeCommandOutput::MetricsRefreshed {
                battery,
                cpu,
                ram,
                disk,
            } => {
                self.battery_label.set_label(&battery);
                self.cpu_label.set_label(&cpu);
                self.ram_label.set_label(&ram);
                self.disk_label.set_label(&disk);
            }
        }
    }
}
