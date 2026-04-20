// Ayuz - Unofficial Control Center for Asus Laptops
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

use directories::BaseDirs;
use std::path::PathBuf;

use super::config::AppConfig;

fn legacy_config_dir() -> Option<PathBuf> {
    BaseDirs::new().map(|d| d.config_dir().join("asus-hub"))
}

/// Returns true if the legacy `~/.config/asus-hub/` directory exists.
pub fn legacy_dir_exists() -> bool {
    legacy_config_dir()
        .map(|p| p.exists())
        .unwrap_or(false)
}

/// Returns true if a legacy asus-hub config directory exists and the user
/// hasn't previously declined the migration prompt.
pub fn should_prompt() -> bool {
    legacy_dir_exists() && !AppConfig::load().skip_legacy_migration
}

/// Copies `~/.config/asus-hub/config.json` into `~/.config/ayuz/config.json`
/// (overwriting it), then removes the entire `~/.config/asus-hub/` directory.
pub fn perform_migration() -> Result<(), String> {
    let legacy_dir = legacy_config_dir()
        .ok_or_else(|| "Could not determine legacy config directory".to_string())?;

    let legacy_json = legacy_dir.join("config.json");

    if legacy_json.exists() {
        let dest = AppConfig::config_dir()
            .ok_or_else(|| "Could not determine config directory".to_string())?
            .join("config.json");

        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config dir: {e}"))?;
        }

        std::fs::copy(&legacy_json, &dest)
            .map_err(|e| format!("Failed to copy config.json: {e}"))?;
    }

    std::fs::remove_dir_all(&legacy_dir)
        .map_err(|e| format!("Failed to remove legacy config dir: {e}"))?;

    Ok(())
}
