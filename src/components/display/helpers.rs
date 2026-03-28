use crate::services::config::AppConfig;

const SRGB_ICM: &[u8] = include_bytes!("../../../assets/icm/ASUS_sRGB.icm");
const DCIP3_ICM: &[u8] = include_bytes!("../../../assets/icm/ASUS_DCIP3.icm");
const DISPLAYP3_ICM: &[u8] = include_bytes!("../../../assets/icm/ASUS_DisplayP3.icm");

pub(crate) async fn setup_icm_profiles() -> Result<std::path::PathBuf, String> {
    let basis = AppConfig::config_dir()
        .ok_or_else(|| "Konnte Config-Verzeichnis nicht bestimmen".to_string())?
        .join("icm");

    let basis_clone = basis.clone();
    tokio::task::spawn_blocking(move || {
        std::fs::create_dir_all(&basis_clone)
            .map_err(|e| format!("ICM-Verzeichnis erstellen fehlgeschlagen: {e}"))?;

        for (name, data) in [
            ("ASUS_sRGB.icm", SRGB_ICM),
            ("ASUS_DCIP3.icm", DCIP3_ICM),
            ("ASUS_DisplayP3.icm", DISPLAYP3_ICM),
        ] {
            std::fs::write(basis_clone.join(name), data)
                .map_err(|e| format!("ICM-Profil '{name}' schreiben fehlgeschlagen: {e}"))?;
        }
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| format!("spawn_blocking fehlgeschlagen: {e}"))??;

    Ok(basis)
}

pub(crate) async fn icm_profil_reset() -> Result<(), String> {
    let result = tokio::task::spawn_blocking(|| {
        std::process::Command::new("kscreen-doctor")
            .arg("output.eDP-1.colorProfileSource.EDID")
            .status()
    })
    .await;

    match result {
        Ok(Ok(status)) if status.success() => Ok(()),
        Ok(Ok(status)) => Err(format!(
            "kscreen-doctor profile.reset fehlgeschlagen mit Exit-Code: {}",
            status.code().unwrap_or(-1)
        )),
        Ok(Err(e)) => Err(format!("kscreen-doctor starten fehlgeschlagen: {e}")),
        Err(e) => Err(format!("spawn_blocking fehlgeschlagen: {e}")),
    }
}

pub(crate) async fn icm_profil_anwenden(
    dateiname: &str,
    basis_pfad: &std::path::Path,
) -> Result<(), String> {
    let absoluter_pfad = basis_pfad.join(dateiname);
    let arg = format!("output.eDP-1.iccprofile.{}", absoluter_pfad.display());

    let result = tokio::task::spawn_blocking(move || {
        std::process::Command::new("kscreen-doctor")
            .arg(&arg)
            .status()
    })
    .await;

    match result {
        Ok(Ok(status)) if status.success() => Ok(()),
        Ok(Ok(status)) => Err(format!(
            "kscreen-doctor fehlgeschlagen mit Exit-Code: {}",
            status.code().unwrap_or(-1)
        )),
        Ok(Err(e)) => Err(format!("kscreen-doctor starten fehlgeschlagen: {e}")),
        Err(e) => Err(format!("spawn_blocking fehlgeschlagen: {e}")),
    }
}

/// Fallback: versucht qdbus-qt6, dann qdbus.
pub(crate) async fn qdbus_ausfuehren(args: Vec<String>) -> Result<(), String> {
    let result = tokio::task::spawn_blocking(move || {
        let status = std::process::Command::new("qdbus-qt6").args(&args).status();
        match status {
            Ok(s) => Ok(("qdbus-qt6", s)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Fallback auf qdbus
                std::process::Command::new("qdbus")
                    .args(&args)
                    .status()
                    .map(|s| ("qdbus", s))
            }
            Err(e) => Err(e),
        }
    })
    .await;

    match result {
        Ok(Ok((_, status))) if status.success() => Ok(()),
        Ok(Ok((cmd, status))) => Err(format!(
            "{cmd} fehlgeschlagen mit Exit-Code: {}",
            status.code().unwrap_or(-1)
        )),
        Ok(Err(e)) => Err(format!("qdbus starten fehlgeschlagen: {e}")),
        Err(e) => Err(format!("spawn_blocking fehlgeschlagen: {e}")),
    }
}

pub(crate) async fn kwriteconfig_ausfuehren(args: &[&str]) -> Result<(), String> {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let result = tokio::task::spawn_blocking(move || {
        std::process::Command::new("kwriteconfig6")
            .args(&args)
            .status()
    })
    .await;

    match result {
        Ok(Ok(status)) if status.success() => Ok(()),
        Ok(Ok(status)) => Err(format!(
            "kwriteconfig6 fehlgeschlagen mit Exit-Code: {}",
            status.code().unwrap_or(-1)
        )),
        Ok(Err(e)) => Err(format!("kwriteconfig6 starten fehlgeschlagen: {e}")),
        Err(e) => Err(format!("spawn_blocking fehlgeschlagen: {e}")),
    }
}
