//! Tray icon: keeps Nicéphore reachable while folders are sent with the
//! window closed. Its menu shows the upload state, pauses or resumes, opens
//! the window or quits. (On Linux the tray only opens its menu on click.)

use envoi::{EtatEnvoi, Phase};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

use crate::Etat;

pub struct Zone {
    etat: MenuItem<Wry>,
    pause: MenuItem<Wry>,
}

pub fn creer(app: &AppHandle) -> tauri::Result<Zone> {
    let ouvrir = MenuItem::with_id(app, "ouvrir", "Ouvrir Nicéphore", true, None::<&str>)?;
    let etat = MenuItem::with_id(app, "etat", "Envoi : en attente", false, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "Mettre l’envoi en pause", true, None::<&str>)?;
    let quitter = MenuItem::with_id(app, "quitter", "Quitter Nicéphore", true, None::<&str>)?;
    let separateur = PredefinedMenuItem::separator(app)?;
    let separateur2 = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&ouvrir, &separateur, &etat, &pause, &separateur2, &quitter])?;
    let mut constructeur = TrayIconBuilder::with_id("principal")
        .tooltip("Nicéphore")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, evenement| match evenement.id().as_ref() {
            "ouvrir" => montrer(app),
            "pause" => {
                if let Some(etat) = app.try_state::<Etat>() {
                    let pause = !etat.envoi.instantane().en_pause;
                    etat.envoi.mettre_en_pause(pause);
                }
            }
            "quitter" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|icone, evenement| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = evenement {
                montrer(icone.app_handle());
            }
        });
    if let Some(icone) = app.default_window_icon() {
        constructeur = constructeur.icon(icone.clone());
    }
    constructeur.build(app)?;
    Ok(Zone { etat, pause })
}

/// Shows and focuses the main window.
pub fn montrer(app: &AppHandle) {
    if let Some(fenetre) = app.get_webview_window("main") {
        let _ = fenetre.show();
        let _ = fenetre.unminimize();
        let _ = fenetre.set_focus();
    }
}

/// One line describing the upload state, for the tray menu.
pub fn resume(etat: &EtatEnvoi) -> String {
    if !etat.connecte {
        return "Envoi : non connecté".into();
    }
    if etat.en_pause {
        return "Envoi : en pause".into();
    }
    if let Some(raison) = &etat.attente {
        return format!("Envoi : en attente ({raison})");
    }
    if etat.erreur.is_some() {
        return "Envoi : arrêté, voir la page Envoi".into();
    }
    match etat.phase {
        Phase::Inactif => "Envoi : à jour".into(),
        Phase::Envoi if etat.total > 0 => format!("Envoi : {} sur {}", etat.faits, etat.total),
        _ => "Envoi : analyse des dossiers".into(),
    }
}

impl Zone {
    pub fn mettre_a_jour(&self, etat: &EtatEnvoi) {
        let _ = self.etat.set_text(resume(etat));
        let _ = self.pause.set_text(if etat.en_pause { "Reprendre l’envoi" } else { "Mettre l’envoi en pause" });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resumes() {
        let mut e = EtatEnvoi { connecte: true, ..Default::default() };
        assert_eq!(resume(&e), "Envoi : à jour");
        e.phase = Phase::Envoi;
        e.faits = 2;
        e.total = 5;
        assert_eq!(resume(&e), "Envoi : 2 sur 5");
        e.attente = Some("sur batterie".into());
        assert_eq!(resume(&e), "Envoi : en attente (sur batterie)");
        e.en_pause = true;
        assert_eq!(resume(&e), "Envoi : en pause");
    }
}
