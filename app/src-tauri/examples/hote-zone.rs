//! Test bench only: a minimal tray host (StatusNotifierWatcher) on the
//! private D-Bus session of `scripts/banc.sh`. It records the tray item
//! Nicéphore publishes and writes what a panel reads from it (title, icon
//! sizes, menu labels) to the file given as argument, each time the item
//! registers or its menu changes. `<file>.pret` appears once the host is up.

//! Linux only: zbus and the StatusNotifier protocol do not exist on Windows.

#[cfg(target_os = "linux")]
fn main() -> zbus::Result<()> {
    hote::main()
}

#[cfg(not(target_os = "linux"))]
fn main() {}

#[cfg(target_os = "linux")]
mod hote {
    use serde::Deserialize;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
    use zbus::message::Header;
    use zbus::zvariant::{OwnedValue, Type, Value};
    use zbus::{Connection, interface};

    struct Veilleur {
        items: Arc<Mutex<Vec<String>>>,
        nouveaux: UnboundedSender<(String, String)>,
    }

    #[interface(name = "org.kde.StatusNotifierWatcher")]
    impl Veilleur {
        fn register_status_notifier_item(&self, service: String, #[zbus(header)] entete: Header<'_>) {
            let expediteur = entete.sender().map(|s| s.to_string()).unwrap_or_default();
            let (bus, chemin) =
                if service.starts_with('/') { (expediteur, service) } else { (service, "/StatusNotifierItem".to_string()) };
            self.items.lock().expect("verrou").push(format!("{bus}{chemin}"));
            let _ = self.nouveaux.send((bus, chemin));
        }

        fn register_status_notifier_host(&self, _service: String) {}

        #[zbus(property)]
        fn registered_status_notifier_items(&self) -> Vec<String> {
            self.items.lock().expect("verrou").clone()
        }

        #[zbus(property)]
        fn is_status_notifier_host_registered(&self) -> bool {
            true
        }

        #[zbus(property)]
        fn protocol_version(&self) -> i32 {
            0
        }
    }

    /// A dbusmenu layout node: id, properties, children (as variants).
    #[derive(Deserialize, Type)]
    struct Noeud {
        _id: i32,
        proprietes: HashMap<String, OwnedValue>,
        enfants: Vec<OwnedValue>,
    }

    fn libelles(noeud: &Noeud, sortie: &mut Vec<String>) {
        for enfant in &noeud.enfants {
            let valeur: &Value = enfant;
            let valeur = match valeur {
                Value::Value(dedans) => dedans.as_ref(),
                v => v,
            };
            let Ok(structure) = valeur.try_clone() else { continue };
            let Ok((id, proprietes, petits)) = <(i32, HashMap<String, OwnedValue>, Vec<OwnedValue>)>::try_from(structure) else {
                continue;
            };
            let texte = |cle: &str| proprietes.get(cle).and_then(|v| String::try_from(v.try_clone().ok()?).ok());
            let actif = proprietes.get("enabled").and_then(|v| bool::try_from(v.try_clone().ok()?).ok()).unwrap_or(true);
            match (texte("type").as_deref(), texte("label")) {
                (Some("separator"), _) => sortie.push("---".into()),
                (_, Some(l)) => sortie.push(if actif { l } else { format!("{l} (grisé)") }),
                _ => {}
            }
            libelles(&Noeud { _id: id, proprietes: HashMap::new(), enfants: petits }, sortie);
        }
        let _ = &noeud.proprietes;
    }

    async fn lire(connexion: &Connection, bus: &str, chemin: &str) -> zbus::Result<String> {
        let proprietes = zbus::fdo::PropertiesProxy::builder(connexion).destination(bus)?.path(chemin)?.build().await?;
        let tout = proprietes.get_all("org.kde.StatusNotifierItem".try_into()?).await?;
        let texte = |cle: &str| tout.get(cle).and_then(|v| String::try_from(v.try_clone().ok()?).ok()).unwrap_or_default();
        let mut lignes = vec![format!("id={}", texte("Id")), format!("titre={}", texte("Title")), format!("nom_icone={}", texte("IconName"))];
        let images = tout
            .get("IconPixmap")
            .and_then(|v| Vec::<(i32, i32, Vec<u8>)>::try_from(v.try_clone().ok()?).ok())
            .unwrap_or_default();
        let tailles: Vec<String> = images
            .iter()
            .map(|(l, h, d)| {
                let complete = d.len() == (*l as usize) * (*h as usize) * 4;
                let opaque = d.chunks(4).filter(|p| p[0] > 0).count();
                format!("{l}x{h}{}{}", if complete { "" } else { "(incomplète)" }, if opaque > 0 { "" } else { "(vide)" })
            })
            .collect();
        lignes.push(format!("images={}", tailles.join(",")));
        let menu = tout.get("Menu").and_then(|v| zbus::zvariant::OwnedObjectPath::try_from(v.try_clone().ok()?).ok());
        if let Some(menu) = menu {
            let reponse = connexion
                .call_method(Some(bus), menu.as_str(), Some("com.canonical.dbusmenu"), "GetLayout", &(0i32, -1i32, Vec::<&str>::new()))
                .await?;
            let (_revision, racine): (u32, Noeud) = reponse.body().deserialize()?;
            let mut sortie = Vec::new();
            libelles(&racine, &mut sortie);
            lignes.push(format!("menu={}", sortie.join(" | ")));
        }
        Ok(lignes.join("\n") + "\n")
    }

    #[tokio::main]
    pub async fn main() -> zbus::Result<()> {
        let sortie = std::env::args().nth(1).expect("fichier de sortie");
        let (nouveaux, mut arrivees) = unbounded_channel();
        let veilleur = Veilleur { items: Arc::default(), nouveaux };
        let connexion = zbus::connection::Builder::session()?
            .name("org.kde.StatusNotifierWatcher")?
            .serve_at("/StatusNotifierWatcher", veilleur)?
            .build()
            .await?;
        std::fs::write(format!("{sortie}.pret"), "").expect("fichier prêt");
        let Some((bus, chemin)) = arrivees.recv().await else { return Ok(()) };
        // Read again every second: the menu follows the upload state.
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            match lire(&connexion, &bus, &chemin).await {
                Ok(texte) => std::fs::write(&sortie, texte).expect("fichier de sortie"),
                Err(_) => return Ok(()),
            }
        }
    }
}
