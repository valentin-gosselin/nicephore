//! Conditions under which uploads wait: running on battery, or on a metered
//! connection (phone hotspot...). Each check fails open: when the system
//! cannot tell, uploads go on.

use std::path::Path;

/// True when a battery of the computer itself is discharging. Batteries of
/// peripherals (wireless mouse, headset: `scope` = `Device`) are ignored.
pub fn sur_batterie() -> bool {
    #[cfg(target_os = "linux")]
    {
        sur_batterie_linux(Path::new("/sys/class/power_supply"))
    }
    #[cfg(windows)]
    {
        sur_batterie_windows()
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        false
    }
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn sur_batterie_linux(racine: &Path) -> bool {
    let Ok(entrees) = std::fs::read_dir(racine) else { return false };
    let lire = |dossier: &Path, nom: &str| std::fs::read_to_string(dossier.join(nom)).map(|t| t.trim().to_string()).ok();
    entrees.flatten().any(|entree| {
        let dossier = entree.path();
        lire(&dossier, "type").as_deref() == Some("Battery")
            && lire(&dossier, "scope").as_deref() != Some("Device")
            && lire(&dossier, "status").as_deref() == Some("Discharging")
    })
}

#[cfg(windows)]
fn sur_batterie_windows() -> bool {
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut etat: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
    // ACLineStatus: 0 offline, 1 online, 255 unknown.
    unsafe { GetSystemPowerStatus(&mut etat) != 0 && etat.ACLineStatus == 0 }
}

/// True when the network in use is metered (NetworkManager on Linux).
/// Not detected on Windows yet.
pub fn reseau_facture() -> bool {
    #[cfg(target_os = "linux")]
    {
        reseau_facture_linux().unwrap_or(false)
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

#[cfg(target_os = "linux")]
fn reseau_facture_linux() -> zbus::Result<bool> {
    // One system bus connection for the whole run (checked before every file).
    static CONNEXION: std::sync::OnceLock<zbus::blocking::Connection> = std::sync::OnceLock::new();
    let connexion = match CONNEXION.get() {
        Some(c) => c,
        None => {
            let nouvelle = zbus::blocking::Connection::system()?;
            CONNEXION.get_or_init(|| nouvelle)
        }
    };
    let proxy = zbus::blocking::Proxy::new(
        connexion,
        "org.freedesktop.NetworkManager",
        "/org/freedesktop/NetworkManager",
        "org.freedesktop.NetworkManager",
    )?;
    // NMMetered: 1 yes, 3 guessed yes (phone hotspot), 2 and 4 no, 0 unknown.
    let facture: u32 = proxy.get_property("Metered")?;
    Ok(matches!(facture, 1 | 3))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alimentation(racine: &Path, nom: &str, attributs: &[(&str, &str)]) {
        let dossier = racine.join(nom);
        std::fs::create_dir_all(&dossier).unwrap();
        for (fichier, valeur) in attributs {
            std::fs::write(dossier.join(fichier), format!("{valeur}\n")).unwrap();
        }
    }

    #[test]
    fn batterie_de_souris_ignoree() {
        let racine = tempfile::tempdir().unwrap();
        alimentation(racine.path(), "hidpp_battery_0", &[("type", "Battery"), ("scope", "Device"), ("status", "Discharging")]);
        assert!(!sur_batterie_linux(racine.path()));
    }

    #[test]
    fn portable_branche_ou_non() {
        let racine = tempfile::tempdir().unwrap();
        alimentation(racine.path(), "AC", &[("type", "Mains"), ("online", "1")]);
        alimentation(racine.path(), "BAT0", &[("type", "Battery"), ("status", "Charging")]);
        assert!(!sur_batterie_linux(racine.path()));
        alimentation(racine.path(), "BAT0", &[("type", "Battery"), ("status", "Discharging")]);
        assert!(sur_batterie_linux(racine.path()));
    }

    #[test]
    fn rien_de_lisible() {
        assert!(!sur_batterie_linux(Path::new("/nexiste/pas")));
    }
}

#[cfg(all(test, target_os = "linux"))]
mod essai_systeme {
    /// Reads the real NetworkManager (run by hand: `cargo test -- --ignored`).
    #[test]
    #[ignore = "lit l'état réel du système"]
    fn lit_networkmanager_et_l_alimentation() {
        let facture = super::reseau_facture_linux().expect("NetworkManager joignable");
        println!("réseau facturé : {facture}, sur batterie : {}", super::sur_batterie());
    }
}
