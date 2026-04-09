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

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;

fn default_brighten_threshold() -> f64 {
    12.0
}
fn default_dim_threshold() -> f64 {
    35.0
}
fn default_touchpad_active() -> bool {
    true
}
fn default_dc_dimming() -> u32 {
    100
}
fn default_language() -> String {
    "en".to_string()
}
/// Persistent application configuration stored as JSON at `~/.config/asus-hub/config.json`.
///
/// All fields are serialised with `serde`. Fields added in later versions carry `#[serde(default)]`
/// so that existing config files remain valid after an upgrade.
#[derive(Serialize, Deserialize, Clone)]
pub struct AppConfig {
    /// Selected color gamut profile index (0 = native, 1 = sRGB, 2 = DCI-P3, 3 = Display P3).
    pub color_profile_index: u32,
    /// Whether the OLED pixel-refresh idle timer is enabled.
    pub oled_care_pixel_refresh: bool,
    /// Whether the KDE panel auto-hide feature is active.
    pub oled_care_panel_autohide: bool,
    /// Whether the KDE panel transparency effect is active.
    pub oled_care_transparency: bool,
    /// Whether suspend-to-RAM uses `deep` sleep instead of the default `s2idle`.
    pub battery_deep_sleep_active: bool,
    /// Active fan profile index matching [`crate::services::dbus::FanProfile`] repr values.
    pub fan_profile: u32,
    /// Whether touchpad edge gestures are active.
    pub input_gestures_active: bool,
    /// Whether the FN key is locked (media keys require FN modifier when `true`).
    pub input_fn_key_locked: bool,
    /// Whether auto-brighten (on low ambient light) is enabled for keyboard backlight.
    #[serde(default)]
    pub kbd_brighten_active: bool,
    /// Whether auto-dim (on high ambient light) is enabled for keyboard backlight.
    #[serde(default)]
    pub kbd_dim_active: bool,
    /// Keyboard backlight idle timeout mode (0 = never, 1 = battery+AC, 2 = battery only).
    #[serde(default)]
    pub kbd_timeout_mode: u32,
    /// Timeout dropdown index used when on battery and AC power.
    #[serde(default)]
    pub kbd_timeout_battery_ac_index: u32,
    /// Timeout dropdown index used when on battery only.
    #[serde(default)]
    pub kbd_timeout_battery_only_index: u32,
    /// Ambient light threshold (lux) below which keyboard backlight is brightened (default 12).
    #[serde(default = "default_brighten_threshold")]
    pub kbd_brighten_threshold: f64,
    /// Ambient light threshold (lux) above which keyboard backlight is dimmed (default 35).
    #[serde(default = "default_dim_threshold")]
    pub kbd_dim_threshold: f64,
    /// Whether the touchpad is enabled (default `true`).
    #[serde(default = "default_touchpad_active")]
    pub touchpad_active: bool,
    /// UI language code, e.g. `"en"` or `"de"` (default `"en"`).
    #[serde(default = "default_language")]
    pub language: String,
    /// Selected EasyEffects audio profile index.
    #[serde(default)]
    pub audio_profile: u32,
    /// OLED DC dimming level (10–100, default 100 = no dimming).
    #[serde(default = "default_dc_dimming")]
    pub oled_dc_dimming: u32,
    /// Whether the KDE "Diminish Inactive Windows" effect is active.
    #[serde(default)]
    pub target_mode_active: bool,
    /// Last GPU mode set by the user, stored as the `GfxMode` repr value (default `0` = Hybrid).
    #[serde(default)]
    pub gpu_mode: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            color_profile_index: 0,
            oled_care_pixel_refresh: false,
            oled_care_panel_autohide: false,
            oled_care_transparency: false,
            battery_deep_sleep_active: false,
            fan_profile: 0,
            input_gestures_active: false,
            input_fn_key_locked: false,
            kbd_brighten_active: false,
            kbd_dim_active: false,
            kbd_timeout_mode: 0,
            kbd_timeout_battery_ac_index: 0,
            kbd_timeout_battery_only_index: 0,
            kbd_brighten_threshold: default_brighten_threshold(),
            kbd_dim_threshold: default_dim_threshold(),
            touchpad_active: default_touchpad_active(),
            language: default_language(),
            audio_profile: 0,
            oled_dc_dimming: default_dc_dimming(),
            target_mode_active: false,
            gpu_mode: 0,
        }
    }
}

impl AppConfig {
    /// Returns the application's config directory (e.g. `~/.config/asus-hub`).
    pub fn config_dir() -> Option<std::path::PathBuf> {
        ProjectDirs::from("", "", "asus-hub").map(|dirs| dirs.config_dir().to_path_buf())
    }

    /// Returns the full path to `config.json` inside the config directory.
    fn config_path() -> Option<std::path::PathBuf> {
        Self::config_dir().map(|dir| dir.join("config.json"))
    }

    /// Loads the config from disk, falling back to [`Default`] if the file is absent or invalid.
    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Serialises the config to `config.json`, silently ignoring all I/O errors.
    pub fn save(&self) {
        let Some(path) = Self::config_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(&path, json);
        }
    }

    /// Loads the config, applies `f` to mutate it, then saves it back to disk.
    pub fn update(f: impl FnOnce(&mut Self)) {
        let mut config = Self::load();
        f(&mut config);
        config.save();
    }
}
