//! Tray icon: keeps Nicéphore reachable while folders are sent with the
//! window closed. Its menu shows the upload state, pauses or resumes, opens
//! the window or quits.
//!
//! On Linux the icon is published directly as a StatusNotifierItem (ksni):
//! the tray of Tauri goes through libappindicator, which gives Plasma a
//! temporary file path and no title, so the icon vanished from the expanded
//! tray popup and the entry was named "nicephore". Here Plasma gets the
//! title and the icon pixels themselves. Windows keeps the tray of Tauri.

use envoi::{EtatEnvoi, Phase};
use tauri::{AppHandle, Manager};

use crate::Etat;

pub use plateforme::{Zone, creer};

/// Shows and focuses the main window.
pub fn montrer(app: &AppHandle) {
    if let Some(fenetre) = app.get_webview_window("main") {
        let _ = fenetre.show();
        let _ = fenetre.unminimize();
        let _ = fenetre.set_focus();
    }
}

fn basculer_pause(app: &AppHandle) {
    if let Some(etat) = app.try_state::<Etat>() {
        let pause = !etat.envoi.instantane().en_pause;
        etat.envoi.mettre_en_pause(pause);
    }
}

fn libelle_pause(en_pause: bool) -> &'static str {
    if en_pause { "Reprendre l’envoi" } else { "Mettre l’envoi en pause" }
}

#[cfg(target_os = "linux")]
mod plateforme {
    use super::*;
    use ksni::TrayMethods;
    use ksni::menu::{MenuItem, StandardItem};
    use std::sync::Mutex;

    struct Icone {
        app: AppHandle,
        resume: String,
        en_pause: bool,
        images: Vec<ksni::Icon>,
    }

    /// RGBA pixels (from Tauri) to the ARGB32 big-endian pixels of the spec.
    fn argb(image: tauri::image::Image<'_>) -> ksni::Icon {
        let data = image.rgba().as_chunks::<4>().0.iter().flat_map(|&[r, g, b, a]| [a, r, g, b]).collect();
        ksni::Icon { width: image.width() as i32, height: image.height() as i32, data }
    }

    impl ksni::Tray for Icone {
        fn id(&self) -> String {
            "nicephore".into()
        }

        fn title(&self) -> String {
            "Nicéphore".into()
        }

        fn icon_pixmap(&self) -> Vec<ksni::Icon> {
            self.images.clone()
        }

        fn tool_tip(&self) -> ksni::ToolTip {
            ksni::ToolTip { title: "Nicéphore".into(), description: self.resume.clone(), ..Default::default() }
        }

        fn activate(&mut self, _x: i32, _y: i32) {
            montrer(&self.app);
        }

        fn menu(&self) -> Vec<MenuItem<Self>> {
            vec![
                StandardItem { label: "Ouvrir Nicéphore".into(), activate: Box::new(|i: &mut Self| montrer(&i.app)), ..Default::default() }
                    .into(),
                MenuItem::Separator,
                StandardItem { label: self.resume.clone(), enabled: false, ..Default::default() }.into(),
                StandardItem {
                    label: libelle_pause(self.en_pause).into(),
                    activate: Box::new(|i: &mut Self| basculer_pause(&i.app)),
                    ..Default::default()
                }
                .into(),
                MenuItem::Separator,
                StandardItem {
                    label: "Quitter Nicéphore".into(),
                    icon_name: "application-exit".into(),
                    activate: Box::new(|i: &mut Self| i.app.exit(0)),
                    ..Default::default()
                }
                .into(),
            ]
        }
    }

    pub struct Zone {
        poignee: ksni::Handle<Icone>,
        dernier: Mutex<(String, bool)>,
    }

    /// Fails when no tray host is running (closing the window then quits).
    pub fn creer(app: &AppHandle) -> Result<Zone, String> {
        let images = [
            tauri::include_image!("icons/32x32.png"),
            tauri::include_image!("icons/64x64.png"),
            tauri::include_image!("icons/128x128.png"),
            tauri::include_image!("icons/128x128@2x.png"),
        ]
        .into_iter()
        .map(argb)
        .collect();
        let resume = "Envoi : en attente".to_string();
        let icone = Icone { app: app.clone(), resume: resume.clone(), en_pause: false, images };
        let poignee = tauri::async_runtime::block_on(icone.spawn()).map_err(|e| e.to_string())?;
        Ok(Zone { poignee, dernier: Mutex::new((resume, false)) })
    }

    impl Zone {
        pub fn mettre_a_jour(&self, etat: &EtatEnvoi) {
            let nouveau = (resume(etat), etat.en_pause);
            {
                let mut dernier = self.dernier.lock().expect("verrou de la zone");
                if *dernier == nouveau {
                    return;
                }
                *dernier = nouveau.clone();
            }
            let poignee = self.poignee.clone();
            tauri::async_runtime::spawn(async move {
                poignee
                    .update(|i| {
                        i.resume = nouveau.0;
                        i.en_pause = nouveau.1;
                    })
                    .await;
            });
        }
    }
}

#[cfg(not(target_os = "linux"))]
mod plateforme {
    use super::*;
    use tauri::Wry;
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    pub struct Zone {
        etat: MenuItem<Wry>,
        pause: MenuItem<Wry>,
    }

    pub fn creer(app: &AppHandle) -> Result<Zone, String> {
        creer_tauri(app).map_err(|e| e.to_string())
    }

    fn creer_tauri(app: &AppHandle) -> tauri::Result<Zone> {
        let ouvrir = MenuItem::with_id(app, "ouvrir", "Ouvrir Nicéphore", true, None::<&str>)?;
        let etat = MenuItem::with_id(app, "etat", "Envoi : en attente", false, None::<&str>)?;
        let pause = MenuItem::with_id(app, "pause", libelle_pause(false), true, None::<&str>)?;
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
                "pause" => basculer_pause(app),
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

    impl Zone {
        pub fn mettre_a_jour(&self, etat: &EtatEnvoi) {
            let _ = self.etat.set_text(resume(etat));
            let _ = self.pause.set_text(libelle_pause(etat.en_pause));
        }
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
