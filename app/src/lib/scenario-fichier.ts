// In-app scenario of the default viewer (scripts/banc.sh --fichier): a
// photo opened from the command line like the file manager does, its
// neighbour, the cloud button, sending it now, a second launch that goes to
// the running window, zoom, rotation, full screen, slideshow, the folder
// strip, details, the right-click menu, the system trash with undo, the
// library viewer, and a window closed early that stays closed. Disposable
// local server and the bench's own folder and trash only.
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
const photo = () => document.querySelector<HTMLImageElement>('img.photo');
const touche = (key: string, options: KeyboardEventInit = {}) =>
  window.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...options }));
const chemin = () => decodeURIComponent(new URLSearchParams(location.search).get('chemin') ?? '');
const PAS_SUR_IMMICH = 'Pas encore sur Immich : l’envoyer maintenant';
const SUR_IMMICH = 'Déjà sur Immich : l’ouvrir dans la bibliothèque';
const HABILLAGE = 48 + 76;

export async function lancerScenarioFichier() {
  try {
    await journal('scénario visionneuse locale');
    const affichee = await quand(() => location.pathname === '/fichier' && (photo()?.naturalWidth ?? 0) > 0);
    await verifier('photo ouverte depuis la ligne de commande et affichée', affichee, location.search.slice(0, 120));
    await verifier('position dans le dossier', texte().includes('2 / 5'));
    // The window took the photo's proportions (portrait 1067 x 1600), bar
    // and folder strip apart.
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await attendre(800);
    const fenetre = getCurrentWindow();
    const dimensions = async () => (await fenetre.innerSize()).toLogical(await fenetre.scaleFactor());
    const taille = await dimensions();
    const rapport = taille.width / (taille.height - HABILLAGE);
    await verifier('fenêtre aux proportions de la photo', Math.abs(rapport - 1067 / 1600) < 0.05, `${Math.round(taille.width)} × ${Math.round(taille.height)}`);
    await verifier('nuage : pas sur Immich', await quand(() => !!bouton(PAS_SUR_IMMICH)));
    const envoi = bouton(PAS_SUR_IMMICH);
    await verifier('icône seule, son texte en infobulle', !!envoi && !envoi.textContent?.trim() && envoi.getAttribute('aria-label') === PAS_SUR_IMMICH);
    await capture('f1-pas-sur-immich');
    envoi?.click();
    await verifier('envoyée, puis nuage coché', await quand(() => !!bouton(SUR_IMMICH) && !bouton(PAS_SUR_IMMICH), 150));
    await capture('f2-envoyee');
    bouton('Précédente')?.click();
    await verifier('voisine du dossier', await quand(() => location.search.includes('a-sur-serveur')));
    await verifier('voisine déjà sur Immich', await quand(() => !!bouton(SUR_IMMICH), 150));

    // The library viewer, from the green cloud, and back.
    bouton(SUR_IMMICH)?.click();
    await verifier('ouverte dans la bibliothèque', await quand(() => location.pathname === '/photo' && !!document.querySelector('.visionneuse'), 150));
    const menuImmich = new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: 300, clientY: 300 });
    document.querySelector('.visionneuse > div')?.dispatchEvent(menuImmich);
    await verifier('clic droit dans la visionneuse de la bibliothèque', await quand(() => texte().includes('Enregistrer l’original…') && texte().includes('Ajouter à un album…')));
    await capture('f3-menu-bibliotheque');
    document.querySelector<HTMLElement>('[role="menu"]')?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await quand(() => !document.querySelector('[role="menu"]'));
    await attendre(300);
    bouton('Fermer')?.click();
    await verifier('retour à la visionneuse locale', await quand(() => location.pathname === '/fichier' && (photo()?.naturalWidth ?? 0) > 0, 150));

    const nouvelle = chemin().replace('a-sur-serveur', 'c-autre');
    await invoke('lancer_seconde_instance', { chemin: nouvelle });
    await verifier('seconde ouverture dans la même fenêtre', await quand(() => location.search.includes('c-autre') && (photo()?.naturalWidth ?? 0) > 0, 150), location.search.slice(0, 120));
    await capture('f4-seconde-ouverture');

    // Zoom: the wheel around the pointer, keys, double click back to fit.
    const largeurAjustee = photo()?.getBoundingClientRect().width ?? 0;
    const scene = document.querySelector<HTMLElement>('.scene');
    const boite = scene?.getBoundingClientRect();
    for (let i = 0; i < 4; i++) {
      scene?.dispatchEvent(new WheelEvent('wheel', { deltaY: -100, clientX: (boite?.left ?? 0) + 80, clientY: (boite?.top ?? 0) + 80, bubbles: true, cancelable: true }));
    }
    await attendre(300);
    const largeurZoomee = photo()?.getBoundingClientRect().width ?? 0;
    await verifier('zoom à la molette', largeurZoomee > largeurAjustee * 1.5, `${Math.round(largeurAjustee)} → ${Math.round(largeurZoomee)} px`);
    await verifier('niveau de zoom affiché', /\d+ %/.test(document.querySelector('.mentions')?.textContent ?? ''));
    await capture('f5-zoom');
    scene?.dispatchEvent(new MouseEvent('dblclick', { bubbles: true, clientX: 200, clientY: 200 }));
    await attendre(200);
    await verifier('double clic : retour à l’ajustement', Math.abs((photo()?.getBoundingClientRect().width ?? 0) - largeurAjustee) < 2);
    touche('+');
    touche('+');
    await attendre(200);
    const auClavier = (photo()?.getBoundingClientRect().width ?? 0) > largeurAjustee * 1.3;
    touche('0');
    await attendre(200);
    await verifier('zoom au clavier (+, 0)', auClavier && Math.abs((photo()?.getBoundingClientRect().width ?? 0) - largeurAjustee) < 2);

    // Rotation: shown at once, then written in the file (EXIF orientation
    // of the JPEG) after a pause; the photo is then shown as written.
    const enPaysage = () => (photo()?.naturalWidth ?? 0) > (photo()?.naturalHeight ?? 0);
    const droite = () => photo()?.style.transform.includes('rotate(0deg)') ?? false;
    touche('r');
    await attendre(200);
    await verifier('pivoter (R)', photo()?.style.transform.includes('rotate(90deg)') ?? false);
    await capture('f6-pivotee');
    await attendre(800);
    const pivotee = await dimensions();
    await verifier('la fenêtre suit la rotation', pivotee.width > pivotee.height - HABILLAGE, `${Math.round(pivotee.width)} × ${Math.round(pivotee.height)}`);
    await verifier('rotation écrite dans le fichier, photo rechargée telle quelle', await quand(() => enPaysage() && droite(), 50), photo()?.src.slice(-40) ?? '');
    touche('R', { shiftKey: true });
    await attendre(200);
    await verifier('pivoter dans l’autre sens (Maj+R)', photo()?.style.transform.includes('rotate(270deg)') ?? false);
    await verifier('retour à l’orientation d’origine, écrit aussi', await quand(() => !enPaysage() && droite(), 50));
    const taillePivotee = await dimensions();
    await verifier('fenêtre revenue au portrait', taillePivotee.height > taillePivotee.width, `${Math.round(taillePivotee.width)} × ${Math.round(taillePivotee.height)}`);

    // A photo already on Immich: turned on the server too (checked by
    // banc.sh), and still known there once its file changed.
    touche('ArrowLeft');
    await quand(() => location.search.includes('b-nouvelle') && (photo()?.naturalWidth ?? 0) > 0, 150);
    await quand(() => !!bouton(SUR_IMMICH), 150);
    touche('r');
    await verifier('photo d’Immich pivotée, fichier écrit', await quand(() => enPaysage() && droite(), 60));
    touche('ArrowRight');
    await quand(() => location.search.includes('c-autre') && (photo()?.naturalWidth ?? 0) > 0, 150);
    touche('ArrowLeft');
    await quand(() => location.search.includes('b-nouvelle') && (photo()?.naturalWidth ?? 0) > 0, 150);
    await verifier('fichier pivoté toujours reconnu sur Immich', await quand(() => !!bouton(SUR_IMMICH) && !bouton(PAS_SUR_IMMICH), 150));
    await verifier('pivotée à la réouverture', enPaysage());
    const etatPivotee = await invoke<{ assetId?: string }>('etat_fichier_local', { chemin: chemin() }).catch(() => null);
    await journal(`asset pivoté : ${etatPivotee?.assetId ?? 'inconnu'}`);
    await capture('f6b-pivotee-immich');
    touche('ArrowRight');
    await quand(() => location.search.includes('c-autre') && (photo()?.naturalWidth ?? 0) > 0, 150);

    // Details of the file: the window widens for the panel.
    const sansPanneau = (await dimensions()).width;
    touche('i');
    await verifier(
      'panneau d’infos du fichier',
      await quand(() => {
        const panneau = document.querySelector('aside.panneau')?.textContent ?? '';
        return panneau.includes('c-autre.jpg') && panneau.includes('Dossier') && panneau.includes('1067 × 1600');
      }),
    );
    await attendre(800);
    const avecPanneau = (await dimensions()).width;
    await verifier('la fenêtre s’élargit pour le panneau', avecPanneau - sansPanneau > 250, `${Math.round(sansPanneau)} → ${Math.round(avecPanneau)}`);
    await capture('f7-infos');
    touche('i');
    await attendre(800);

    // The folder strip: five tiles, the current one marked; a click jumps.
    const tuiles = () => [...document.querySelectorAll<HTMLButtonElement>('.bande .tuile')];
    await verifier('bande des photos du dossier', await quand(() => tuiles().length === 5 && tuiles()[2]?.classList.contains('courante')));
    await verifier('vignettes de la bande', await quand(() => document.querySelectorAll('.bande .tuile img').length >= 3, 150));
    await capture('f8-bande');
    tuiles()[0]?.click();
    await verifier('saut par la bande', await quand(() => location.search.includes('a-sur-serveur')));
    bouton('Masquer les photos du dossier')?.click();
    await attendre(200);
    let preference = '';
    try {
      preference = localStorage.getItem('nicephore-bande-vignettes') ?? '';
    } catch {
      // No storage: the check below fails and says so.
    }
    await verifier('bande masquée, choix retenu', !document.querySelector('.bande') && preference === '0');
    bouton('Afficher les photos du dossier')?.click();
    await quand(() => !!document.querySelector('.bande'));

    // Right-click menu of the local viewer, and copying the image.
    const clic = new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: 200, clientY: 200 });
    document.querySelector('.zone')?.dispatchEvent(clic);
    await verifier('pas de menu du navigateur au clic droit', clic.defaultPrevented);
    await verifier(
      'menu du clic droit',
      await quand(() => ['Copier l’image (Ctrl+C)', 'Copier le chemin', 'Afficher dans le dossier', 'Ouvrir avec…', 'Mettre à la corbeille (Suppr)'].every((m) => texte().includes(m))),
    );
    await capture('f9-menu');
    document.querySelector<HTMLElement>('[role="menu"]')?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await quand(() => !document.querySelector('[role="menu"]'));
    // Keys stay with a menu for a moment after it closes.
    await attendre(600);
    touche('c', { ctrlKey: true });
    await verifier('image copiée (Ctrl+C)', await quand(() => texte().includes('Image copiée') || texte().includes('Copie impossible'), 100) && texte().includes('Image copiée'), texte().match(/Copie impossible[^«]{0,120}/)?.[0] ?? '');

    // Full screen: bar and strip float, then fade while the mouse rests.
    touche('f');
    await verifier('plein écran (F)', await quand(() => !!document.querySelector('.plein-ecran')) && (await fenetre.isFullscreen()));
    await verifier('barres masquées au repos', await quand(() => !!document.querySelector('.inactif'), 60));
    await capture('f10-plein-ecran');
    touche('f');
    await verifier('sortie du plein écran', await quand(() => !document.querySelector('.plein-ecran')) && !(await fenetre.isFullscreen()));

    // Slideshow: the next photo after 5 s.
    const avant = chemin();
    touche('d');
    await verifier('diaporama (D)', await quand(() => !!document.querySelector('.progression')));
    await verifier('le diaporama avance seul', await quand(() => chemin() !== avant, 80), chemin().split('/').pop());
    touche('d');
    await quand(() => !document.querySelector('.progression'));

    // To the trash (the bench's own), the next photo, then « Annuler ».
    const local = chemin().replace(/[^/\\]+$/, '');
    const cible = `${local}c-autre.jpg`;
    const { goto } = await import('$app/navigation');
    await goto(`/fichier?chemin=${encodeURIComponent(cible)}`, { replaceState: true });
    const affiche = (nom: string) => document.querySelector('.barre span[title]')?.textContent === nom && (photo()?.naturalWidth ?? 0) > 0;
    await quand(() => affiche('c-autre.jpg'));
    touche('Delete');
    await verifier('mise à la corbeille (Suppr), photo suivante', await quand(() => location.search.includes('d-raw') && texte().includes('« c-autre.jpg » mise à la corbeille')));
    const disparue = await invoke('ouvrir_fichier_local', { chemin: cible }).then(() => false).catch(() => true);
    await verifier('fichier sorti du dossier', disparue);
    await verifier('quatre photos restantes', await quand(() => texte().includes('/ 4')));
    await capture('f11-corbeille');
    bouton('Annuler')?.click();
    await verifier('annuler remet la photo', await quand(() => location.search.includes('c-autre') && texte().includes('/ 5'), 100));

    // Watched from the click: the "no preview" message must never show
    // while the RAW preview is being prepared.
    let messageVu = false;
    const guet = new MutationObserver(() => {
      if (document.body.textContent?.includes('Aperçu indisponible')) messageVu = true;
    });
    guet.observe(document.body, { childList: true, subtree: true, characterData: true });
    bouton('Suivante')?.click();
    const raw = await quand(() => location.search.includes('d-raw') && (photo()?.naturalWidth ?? 0) > 0, 150);
    await verifier('aperçu du RAW affiché', raw && texte().includes('Aperçu du RAW (1067 × 1600)'), texte().slice(0, 160));
    guet.disconnect();
    await verifier('pas de message « aperçu indisponible » avant l’aperçu du RAW', !messageVu);
    await verifier('aperçu pivoté selon le RAW', photo()?.style.transform.includes('rotate(90deg)') ?? false);
    // From portrait to landscape (the RAW, rotated), the window follows.
    await attendre(800);
    const paysage = await dimensions();
    await verifier(
      'la fenêtre passe en paysage avec la photo',
      Math.abs(paysage.width / (paysage.height - HABILLAGE) - 1600 / 1067) < 0.05,
      `${Math.round(paysage.width)} × ${Math.round(paysage.height)}`,
    );
    await capture('f12-raw');
    // A video of this computer plays, and the window takes its proportions.
    bouton('Suivante')?.click();
    const lue = await quand(() => {
      const v = document.querySelector<HTMLVideoElement>('video.video-locale');
      return location.search.includes('e-video') && !!v && v.readyState >= 2 && v.videoWidth === 960;
    }, 150);
    await verifier('vidéo de l’ordinateur lue', lue, texte().slice(0, 120));
    // The controls of Immich (media-chrome), not those of the web engine.
    const controleur = document.querySelector('media-controller');
    const lecteur = document.querySelector<HTMLVideoElement>('video.video-locale');
    await verifier(
      'commandes vidéo façon Immich',
      !!controleur?.querySelector('media-play-button') && !!controleur.querySelector('media-time-range') && !lecteur?.controls,
    );
    controleur?.querySelector<HTMLElement>('media-play-button')?.click();
    await attendre(400);
    await verifier('pause par le bouton du lecteur', !!lecteur?.paused);
    await attendre(800);
    const dimVideo = await dimensions();
    await verifier(
      'fenêtre aux proportions de la vidéo',
      Math.abs(dimVideo.width / (dimVideo.height - HABILLAGE) - 960 / 720) < 0.05,
      `${Math.round(dimVideo.width)} × ${Math.round(dimVideo.height)}`,
    );
    await capture('f13-video');

    // A window closed right after opening a photo stays closed (the safety
    // net that shows a slow window must not bring it back).
    const vue = await invoke<{ fermerEnArrierePlan?: boolean; zoneDisponible?: boolean }>('reglages_app').catch(() => null);
    if (vue?.zoneDisponible && vue.fermerEnArrierePlan) {
      await invoke('lancer_seconde_instance', { chemin: `${local}b-nouvelle.jpg` });
      await quand(() => location.search.includes('b-nouvelle'), 150);
      await attendre(300);
      await fenetre.close();
      await attendre(6500);
      await verifier('fenêtre fermée tout de suite : elle ne revient pas seule', !(await fenetre.isVisible()));
      await invoke('lancer_seconde_instance', { chemin: `${local}a-sur-serveur.jpg` });
      const rouverte = await quand(() => location.search.includes('a-sur-serveur'), 150);
      let visible = false;
      for (let i = 0; i < 50 && !visible; i++) {
        visible = await fenetre.isVisible();
        if (!visible) await attendre(100);
      }
      await verifier('rouverte par la photo suivante', rouverte && visible);
      // The same photo opened again after closing: shown, not a black frame.
      await attendre(500);
      await fenetre.close();
      await attendre(500);
      await invoke('lancer_seconde_instance', { chemin: `${local}a-sur-serveur.jpg` });
      let revenue = false;
      for (let i = 0; i < 40 && !revenue; i++) {
        revenue = await fenetre.isVisible();
        if (!revenue) await attendre(100);
      }
      const affichee = await quand(() => (photo()?.naturalWidth ?? 0) > 0 && !photo()?.classList.contains('invisible'), 50);
      await verifier('même photo rouverte après fermeture : affichée, fenêtre montrée tout de suite', revenue && affichee);
    } else {
      await journal('fenêtre fermée tout de suite : non vérifié (pas de zone de notification)');
    }

    // The freeze watchdog: the main thread held 5 s (checked in the log by
    // banc.sh).
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
