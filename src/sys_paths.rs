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

pub const SYS_PRODUCT_NAME: &str = "/sys/class/dmi/id/product_name";
pub const SYS_BOARD_NAME: &str = "/sys/class/dmi/id/board_name";
pub const SYS_BIOS_VERSION: &str = "/sys/class/dmi/id/bios_version";
pub const SYS_BIOS_DATE: &str = "/sys/class/dmi/id/bios_date";
pub const SYS_PRODUCT_SERIAL: &str = "/sys/class/dmi/id/product_serial";
/// Battery names differ between models (BAT0, BAT1, BATT, BATC, ...), so
/// batteries are discovered by scanning this directory for `type == Battery`.
pub const SYS_POWER_SUPPLY_DIR: &str = "/sys/class/power_supply";
pub const SYS_LOAD_AVG: &str = "/proc/loadavg";
pub const SYS_MEM_INFO: &str = "/proc/meminfo";
pub const SYS_THERMAL_ZONE0_TEMP: &str = "/sys/class/thermal/thermal_zone0/temp";
pub const SYS_MEM_SLEEP: &str = "/sys/power/mem_sleep";
pub const DEV_UINPUT: &str = "/dev/uinput";
pub const PROC_BUS_INPUT_DEVICES: &str = "/proc/bus/input/devices";

/// Resolves the directory where AniMatrix GIF assets are stored.
///
/// Resolution order:
/// 1. `CARGO_MANIFEST_DIR/assets/anime` - cargo dev runs
/// 2. `$APPDIR/assets/anime` - inside a running AppImage
/// 3. `<binary_parent>/../share/ayuz/anime` - installed via deb/rpm (e.g. `/usr/share/ayuz/anime`)
pub fn anime_assets_dir() -> std::path::PathBuf {
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        return std::path::PathBuf::from(manifest).join("assets/anime");
    }
    if let Ok(appdir) = std::env::var("APPDIR") {
        return std::path::PathBuf::from(appdir).join("assets/anime");
    }
    std::env::current_exe()
        .unwrap_or_default()
        .parent()
        .unwrap_or(std::path::Path::new("/usr/bin"))
        .parent()
        .unwrap_or(std::path::Path::new("/usr"))
        .join("share/ayuz/anime")
}
