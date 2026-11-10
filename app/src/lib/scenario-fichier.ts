// In-app scenario of the default viewer (scripts/banc.sh --fichier): a
// photo opened from the command line like the file manager does, its
// neighbour, the cloud badge, sending it now, and a second launch that goes
// to the running window. Disposable local server only.
import { invoke } from '@tauri-apps/api/core';

const journal = (message: string) => invoke('journal', { message });
const capture = (nom: string) => invoke('capture', { nom }).catch(() => {});
const attendre = (ms: number) => new Promise((r) => setTimeout(r, ms));
const texte = () => document.body.textContent?.replace(/\s+/g, ' ') ?? '';

let reussites = 0;
let echecs = 0;
async function verifier(nom: string, condition: boolean, detail = '') {
  if (condition) reussites++;
  else echecs++;
  await journal(`${condition ? 'ok' : 'ÉCHEC'} : ${nom}${detail ? ` (${detail})` : ''}`);
}

async function quand(condition: () => boolean, essais = 100) {
  for (let i = 0; i < essais && !condition(); i++) await attendre(100);
  return condition();
}

const bouton = (libelle: string) =>
  [...document.querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent?.trim() === libelle || b.getAttribute('aria-label') === libelle);

export async function lancerScenarioFichier() {
  try {
    await journal('scénario visionneuse locale');
    const affichee = await quand(() => location.pathname === '/fichier' && (document.querySelector<HTMLImageElement>('img.image')?.naturalWidth ?? 0) > 0);
    await verifier('photo ouverte depuis la ligne de commande et affichée', affichee, location.search.slice(0, 120));
    await verifier('position dans le dossier', texte().includes('2 / 5'));
    // The window took the photo's proportions (portrait 1067 x 1600).
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await attendre(800);
    const fenetre = getCurrentWindow();
    const taille = (await fenetre.innerSize()).toLogical(await fenetre.scaleFactor());
    const rapport = taille.width / (taille.height - 48);
    await verifier('fenêtre aux proportions de la photo', Math.abs(rapport - 1067 / 1600) < 0.05, `${Math.round(taille.width)} × ${Math.round(taille.height)}`);
    const etiquettes = () => [...document.querySelectorAll('.statut')].map((e) => `${e.textContent} ${e.getAttribute('aria-label') ?? ''} ${e.getAttribute('title') ?? ''}`).join(' ');
    await verifier('nuage barré : pas sur Immich', await quand(() => etiquettes().includes('Pas sur Immich')));
    await verifier('barre compacte dans une fenêtre étroite', !!document.querySelector('button[aria-label="Envoyer maintenant"]'));
    await capture('f1-pas-sur-immich');
    bouton('Envoyer maintenant')?.click();
    await verifier('envoyée, puis nuage coché', await quand(() => etiquettes().includes('Sur Immich') && !etiquettes().includes('Pas sur Immich'), 150));
    await capture('f2-envoyee');
    bouton('Précédente')?.click();
    await verifier('voisine du dossier', await quand(() => location.search.includes('a-sur-serveur')));
    await verifier('voisine déjà sur Immich', await quand(() => etiquettes().includes('Sur Immich') && !etiquettes().includes('Pas sur Immich'), 150));
    const nouvelle = decodeURIComponent(new URLSearchParams(location.search).get('chemin') ?? '').replace('a-sur-serveur', 'c-autre');
    await invoke('lancer_seconde_instance', { chemin: nouvelle });
    await verifier('seconde ouverture dans la même fenêtre', await quand(() => location.search.includes('c-autre'), 150), location.search.slice(0, 120));
    await capture('f3-seconde-ouverture');
    // Watched from the click: the "no preview" message must never show
    // while the RAW preview is being prepared.
    let messageVu = false;
    const guet = new MutationObserver(() => {
      if (document.body.textContent?.includes('Aperçu indisponible')) messageVu = true;
    });
    guet.observe(document.body, { childList: true, subtree: true, characterData: true });
    bouton('Suivante')?.click();
    const raw = await quand(() => location.search.includes('d-raw') && (document.querySelector<HTMLImageElement>('img.apercu-raw')?.naturalWidth ?? 0) > 0, 150);
    await verifier('aperçu du RAW affiché', raw && texte().includes('Aperçu du RAW (1067 × 1600)'), texte().slice(0, 160));
    guet.disconnect();
    await verifier('pas de message « aperçu indisponible » avant l’aperçu du RAW', !messageVu);
    await verifier('aperçu pivoté selon le RAW', document.querySelector<HTMLElement>('img.apercu-raw')?.style.transform.includes('rotate(90deg)') ?? false);
    // From portrait to landscape (the RAW, rotated), the window follows.
    await attendre(800);
    const paysage = (await fenetre.innerSize()).toLogical(await fenetre.scaleFactor());
    const rapportPaysage = paysage.width / (paysage.height - 48);
    await verifier('la fenêtre passe en paysage avec la photo', Math.abs(rapportPaysage - 1600 / 1067) < 0.05, `${Math.round(paysage.width)} × ${Math.round(paysage.height)}`);
    await capture('f4-raw');
    // A video of this computer plays, and the window takes its proportions.
    bouton('Suivante')?.click();
    const lue = await quand(() => {
      const v = document.querySelector<HTMLVideoElement>('video.video-locale');
      return location.search.includes('e-video') && !!v && v.readyState >= 2 && v.videoWidth === 960;
    }, 150);
    await verifier('vidéo de l’ordinateur lue', lue, texte().slice(0, 120));
    await attendre(800);
    const dimVideo = (await fenetre.innerSize()).toLogical(await fenetre.scaleFactor());
    await verifier('fenêtre aux proportions de la vidéo', Math.abs(dimVideo.width / (dimVideo.height - 48) - 960 / 720) < 0.05, `${Math.round(dimVideo.width)} × ${Math.round(dimVideo.height)}`);
    await capture('f5-video');
    // The freeze watchdog: the main thread held 5 s (checked in the log by
    // banc.sh).
    // Right click: no browser menu outside text fields.
    const clic = new MouseEvent('contextmenu', { bubbles: true, cancelable: true });
    document.querySelector('.zone')?.dispatchEvent(clic);
    await verifier('pas de menu du navigateur au clic droit', clic.defaultPrevented);
    await invoke('geler_interface', { ms: 5000 });
    await attendre(2500);
    // The engine holds its base 5 s while the interface asks for it: the
    // command waits, the interface does not (banc.sh counts one freeze).
    await invoke('tenir_base', { ms: 5000 });
    await attendre(200);
    const debut = Date.now();
    await invoke('suppressions_en_attente', { regle: 0 }).catch(() => null);
    await verifier('la commande a bien attendu la base', Date.now() - debut > 3000, `${Date.now() - debut} ms`);
    await attendre(1500);
  } catch (e) {
    await journal(`scénario interrompu : ${e}`);
  }
  await journal(`scénario terminé : ${reussites} réussites, ${echecs} échecs`);
  await invoke('quitter');
}
