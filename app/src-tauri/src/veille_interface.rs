//! Watches the main (interface) thread: when it stops answering for a few
//! seconds, a warning goes to the log, and another one when it answers again
//! with how long it was frozen. A frozen main thread stops reading the
//! display server, which may then drop the connection (the app closes).

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;
use tauri::AppHandle;

/// Beyond this, the interface counts as frozen.
const SEUIL_MS: i64 = 3_000;

fn maintenant_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

pub fn lancer(app: AppHandle) {
    let reponse = Arc::new(AtomicI64::new(maintenant_ms()));
    let _ = std::thread::Builder::new().name("veille-interface".into()).spawn(move || {
        let mut fige_depuis: Option<i64> = None;
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let r = Arc::clone(&reponse);
            if app.run_on_main_thread(move || r.store(maintenant_ms(), Ordering::SeqCst)).is_err() {
                return;
            }
            let derniere = reponse.load(Ordering::SeqCst);
            let retard = maintenant_ms() - derniere;
            match fige_depuis {
                None if retard > SEUIL_MS => {
                    tracing::warn!("interface figée depuis {} s (pid {})", retard / 1000, std::process::id());
                    fige_depuis = Some(derniere);
                }
                Some(debut) if retard < 1_500 => {
                    tracing::warn!("interface de nouveau réactive après {:.1} s de gel", (derniere - debut) as f64 / 1000.0);
                    fige_depuis = None;
                }
                _ => {}
            }
        }
    });
}
