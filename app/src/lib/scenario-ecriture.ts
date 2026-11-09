// In-app write scenario, driven through the real UI like a person would:
// albums, favorites, trash with undo, archive. Runs only in benchmark mode
// with NICEPHORE_BANC_SCENARIO=ecriture, and refuses any server that is not
// local (the disposable Immich of scripts/immich-jetable.sh).
import { goto } from '$app/navigation';
import { invoke } from '@tauri-apps/api/core';
import { etatConnexion } from './api';

const journal = (message: string) => invoke('journal', { message });
const capture = (nom: string) => invoke('capture', { nom }).catch(() => {});
const attendre = (ms: number) => new Promise((r) => setTimeout(r, ms));

let reussites = 0;
let echecs = 0;

async function verifier(nom: string, condition: boolean, detail = '') {
  if (condition) reussites++;
  else echecs++;
  await journal(`${condition ? 'ok' : 'ÉCHEC'} : ${nom}${detail ? ` (${detail})` : ''}`);
}

/** Waits for an element (up to 8 s). */
async function element<T extends Element>(selecteur: string, filtre?: (e: T) => boolean): Promise<T | null> {
  for (let i = 0; i < 80; i++) {
    const trouves = [...document.querySelectorAll<T>(selecteur)].filter((e) => !filtre || filtre(e));
    if (trouves.length) return trouves[0];
    await attendre(100);
  }
  return null;
}

const bouton = (libelle: string) => element<HTMLButtonElement>(`[aria-label="${libelle}"]`);

async function cliquer(libelle: string) {
  const b = await bouton(libelle);
  b?.click();
  await attendre(900);
  return !!b;
}

/** Types into a field bound with Svelte (`bind:value` listens to input). */
function saisir(champ: HTMLInputElement | null, texte: string) {
  if (!champ) return;
  champ.value = texte;
  champ.dispatchEvent(new Event('input', { bubbles: true }));
}

const entete = () => document.querySelector('.entete')?.textContent?.replace(/\s+/g, ' ').trim() ?? '';
const notifications = () => [...document.querySelectorAll('[role="status"], [role="alert"]')].map((n) => n.textContent ?? '');

/** Selects the first `n` tiles visible in the timeline. */
async function selectionner(n: number) {
  await element('.tuile .coche');
  const coches = [...document.querySelectorAll<HTMLButtonElement>('.tuile .coche')].slice(0, n);
  for (const c of coches) c.click();
  await attendre(400);
  return coches.length;
}

async function aller(chemin: string) {
  await goto(chemin);
  await attendre(2200);
}

export async function lancerScenarioEcriture() {
  try {
    const connexion = await etatConnexion();
    const url = connexion?.urlServeur ?? '';
    if (!/^https?:\/\/(127\.0\.0\.1|localhost)(:|\/|$)/.test(url)) {
      await journal(`scénario refusé : ${url} n'est pas un serveur local jetable`);
      return;
    }
    await journal(`scénario d'écriture sur ${url}`);
    const nomAlbum = `Album banc ${Date.now() % 100000}`;

    // 1. New album from a selection.
    await aller('/');
    await selectionner(3);
    await cliquer('Ajouter à un album');
    saisir(await element<HTMLInputElement>('[role="dialog"] input'), nomAlbum);
    (await element<HTMLButtonElement>('[role="dialog"] button[type="submit"]'))?.click();
    await attendre(1500);
    await verifier('album créé depuis la sélection', notifications().some((t) => t.includes(nomAlbum)));
    await capture('e1-album-cree');

    // 2. Open it from the Albums page.
    await aller('/albums');
    const carte = await element<HTMLAnchorElement>('a.album', (a) => (a.textContent ?? '').includes(nomAlbum));
    carte?.click();
    await attendre(2500);
    await verifier('album ouvert avec ses 3 éléments', entete().includes('3 éléments'), entete());

    // 3. Rename and describe by clicking the title and the description.
    (await element<HTMLButtonElement>('.texte.grand'))?.click();
    await attendre(300);
    const champTitre = await element<HTMLInputElement>('input.champ.grand');
    saisir(champTitre, `${nomAlbum} renommé`);
    champTitre?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    await attendre(1500);
    await verifier('titre renommé par un clic', entete().includes(`${nomAlbum} renommé`), entete());
    (await element<HTMLButtonElement>('.texte:not(.grand)'))?.click();
    await attendre(300);
    const champDescription = await element<HTMLInputElement>('input.champ:not(.grand)');
    saisir(champDescription, 'Créé par le banc');
    champDescription?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    await attendre(1500);
    await verifier('description enregistrée', entete().includes('Créé par le banc'), entete());

    // 4. Remove one photo from the album.
    await selectionner(1);
    await cliquer('Retirer de l’album');
    await attendre(1200);
    await verifier('photo retirée de l’album', entete().includes('2 éléments'), entete());
    await capture('e2-album-modifie');

    // 5. Delete the album (its photos stay).
    await cliquer('Supprimer l’album');
    const confirmer = await element<HTMLButtonElement>('[role="dialog"] button', (b) => b.textContent?.trim() === 'Supprimer l’album');
    confirmer?.click();
    await attendre(2500);
    await attendre(800);
    const restants = [...document.querySelectorAll('a.album')].filter((a) => (a.textContent ?? '').includes(nomAlbum));
    await verifier('album supprimé, retour à la liste', location.pathname === '/albums' && restants.length === 0, location.pathname);

    // 6. Favorites: add two, then remove them all from the Favorites view
    // (counts compared, the test server may hold leftovers of earlier runs).
    const compte = () => Number(entete().match(/(\d[\d\s\u202f]*) élément/)?.[1].replace(/\D/g, '') ?? 0);
    await aller('/favoris');
    const favorisAvant = compte();
    await aller('/');
    await selectionner(2);
    await cliquer('Ajouter aux favoris');
    await aller('/favoris');
    await verifier('deux favoris ajoutés', compte() === favorisAvant + 2, `${favorisAvant} puis ${compte()}`);
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'a', ctrlKey: true }));
    await attendre(400);
    await cliquer('Retirer des favoris');
    await attendre(1200);
    await verifier('favoris retirés', compte() === 0, entete());

    // 7. Trash, then undo from the toast.
    await aller('/corbeille');
    const corbeilleAvant = compte();
    await aller('/');
    const avant = entete();
    await selectionner(1);
    await cliquer('Mettre à la corbeille (Suppr)');
    await attendre(800);
    await verifier('mise à la corbeille', entete() !== avant, `${avant} puis ${entete()}`);
    const annuler = await element<HTMLButtonElement>('button', (b) => b.textContent?.trim() === 'Annuler');
    annuler?.click();
    await attendre(2000);
    await verifier('annulation depuis la notification', entete() === avant, entete());
    await aller('/corbeille');
    await verifier('corbeille inchangée après annulation', compte() === corbeilleAvant, `${corbeilleAvant} puis ${compte()}`);

    // 8. Archive, then unarchive from the Archive view (counts compared,
    // the test server may hold leftovers of earlier runs).
    await aller('/archive');
    const archiveAvant = compte();
    await aller('/');
    await selectionner(1);
    await cliquer('Archiver');
    await aller('/archive');
    await verifier('élément archivé', compte() === archiveAvant + 1, `${archiveAvant} puis ${compte()}`);
    await selectionner(1);
    await cliquer('Désarchiver');
    await attendre(1200);
    await verifier('élément désarchivé', compte() === archiveAvant, `${compte()}`);
    await capture('e3-archive');

    // 8b. A desktop app: selecting everything selects nothing.
    const style = getComputedStyle(document.body);
    const selection = style.userSelect || style.getPropertyValue('-webkit-user-select');
    await verifier('rien ne se sélectionne comme sur une page web', selection === 'none', selection);

    // 9. Editing in the viewer's info panel.
    await aller('/');
    (await element<HTMLButtonElement>('.tuile .image-tuile'))?.click();
    await attendre(1500);
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'i' }));
    await attendre(1500);
    const panneau = () => document.querySelector('.visionneuse .panneau')?.textContent?.replace(/\s+/g, ' ') ?? '';
    (await element<HTMLButtonElement>('.visionneuse .panneau .texte'))?.click();
    await attendre(300);
    const champ = await element<HTMLInputElement>('.visionneuse .panneau input.champ');
    saisir(champ, 'Décrite par le banc');
    champ?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    await attendre(1800);
    await verifier('description modifiée dans le panneau', panneau().includes('Décrite par le banc'));
    (await bouton('4 étoiles'))?.click();
    await attendre(1800);
    await verifier('note de 4 étoiles', document.querySelectorAll('.visionneuse .etoile.allumee').length === 4);
    (await bouton('4 étoiles'))?.click();
    await attendre(1800);
    await verifier('note retirée', document.querySelectorAll('.visionneuse .etoile.allumee').length === 0);
    (await element<HTMLButtonElement>('.visionneuse .panneau .tag.ajout'))?.click();
    saisir(await element<HTMLInputElement>('[role="dialog"] input'), 'Banc');
    (await element<HTMLButtonElement>('[role="dialog"] button[type="submit"]'))?.click();
    await attendre(2000);
    await verifier('tag ajouté', panneau().includes('Banc'), panneau().slice(0, 120));
    (await bouton('Retirer le tag Banc'))?.click();
    await attendre(2000);
    await verifier('tag retiré', !panneau().includes('Retirer le tag Banc') && !document.querySelector('[aria-label="Retirer le tag Banc"]'));
    (await element<HTMLButtonElement>('.visionneuse .panneau .modifiable'))?.click();
    const champDate = await element<HTMLInputElement>('[role="dialog"] input[type="datetime-local"]');
    saisir(champDate, '2020-06-15T14:30');
    (await element<HTMLButtonElement>('[role="dialog"] button[type="submit"]'))?.click();
    await attendre(3500);
    await verifier('date modifiée', panneau().includes('15 juin 2020'), panneau().slice(0, 160));
    await capture('e4-infos');
    (await element<HTMLButtonElement>('.visionneuse [aria-label="Ajouter aux favoris"]'))?.click();
    await attendre(1200);
    await verifier('favori depuis la visionneuse', !!document.querySelector('.visionneuse [aria-label="Retirer des favoris"]'));
    document.querySelector<HTMLButtonElement>('.visionneuse [aria-label="Retirer des favoris"]')?.click();
    await attendre(1000);
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    await attendre(2500);
    await verifier('la photo redatée reste dans la chronologie', entete() === avant, `${avant} puis ${entete()}`);

    // 10. A tag on a whole selection.
    await selectionner(2);
    await cliquer('Ajouter un tag');
    saisir(await element<HTMLInputElement>('[role="dialog"] input'), 'Lot banc');
    (await element<HTMLButtonElement>('[role="dialog"] button[type="submit"]'))?.click();
    await attendre(1800);
    await verifier('tag en lot', notifications().some((t) => t.includes('Lot banc') && t.includes('2 éléments')), notifications().join(' | ').slice(0, 160));
    await capture('e5-tags');

    // 11. Upload of a local folder (the folder picker is the only step
    // skipped: the rule is added with the folder prepared by banc.sh).
    await aller('/synchro');
    const page = () => document.querySelector('main')?.textContent?.replace(/\s+/g, ' ') ?? document.body.textContent ?? '';
    await verifier('page Envoi vide au départ', page().includes('Aucun dossier n’est envoyé'));
    const dossier = await invoke<string | null>('dossier_photos_banc');
    await verifier('dossier de photos préparé', !!dossier, dossier ?? '');
    if (dossier) {
      const estimation = await invoke<{ fichiers: number; octets: number }>('estimer_dossier', { dossier });
      await verifier('estimation avant ajout (4 photos, texte exclu)', estimation.fichiers === 4 && estimation.octets > 0, `${estimation.fichiers} fichiers, ${estimation.octets} octets`);
      await invoke('ajouter_regle_envoi', { dossier, cible: { genre: 'albumParDossier' } });
      let envoyes = false;
      for (let i = 0; i < 60 && !envoyes; i++) {
        await attendre(500);
        envoyes = page().includes('3 envoyés') && page().includes('À jour');
      }
      await verifier('trois photos envoyées, la copie reconnue', envoyes && page().includes('1 déjà sur le serveur'), page().slice(0, 300));
      await verifier('fichier texte ignoré sans erreur', !page().includes('non envoyé'));
      await verifier('albums créés dans le journal', page().includes('Album créé') && page().includes('Envoyé'));
      await capture('e6-envoi');
      (await element<HTMLButtonElement>('button', (b) => b.textContent?.trim() === 'Fichiers RAW inclus'))?.click();
      await attendre(1200);
      type VueRaw = { sens: string; sansRaw: boolean };
      await verifier('fichiers RAW laissés de côté depuis la carte', (await invoke<VueRaw[]>('regles_envoi'))[0]?.sansRaw === true && page().includes('Fichiers RAW laissés de côté'));
      await cliquer('Mettre l’envoi en pause');
      await verifier('pause', page().includes('En pause.'));
      await cliquer('Reprendre l’envoi');
      await attendre(1500);
      await verifier('reprise', !page().includes('En pause.'));
      await aller('/albums');
      const balade = await element<HTMLAnchorElement>('a.album', (a) => (a.textContent ?? '').includes('Balade banc'));
      await verifier('album du sous-dossier', !!balade && (balade.textContent ?? '').includes('2 éléments'), balade?.textContent?.replace(/\s+/g, ' ') ?? '');
      // 11b. Receiving: the dialog up to its estimate, then a period of the
      // server received by year and month.
      await aller('/synchro');
      (await element<HTMLButtonElement>('button', (b) => b.textContent?.includes('Ajouter un dossier') ?? false))?.click();
      (await element<HTMLInputElement>('input[name="sens"][value="descendante"]'))?.click();
      await attendre(300);
      (await element<HTMLInputElement>('input[name="quoi"][value="periode"]'))?.click();
      await attendre(300);
      saisir(await element<HTMLInputElement>('input[aria-label="Du"]'), '2025-02-01');
      saisir(await element<HTMLInputElement>('input[aria-label="Au"]'), '2025-02-28');
      await attendre(2500);
      const boite = () => document.querySelector('[role="dialog"]')?.textContent?.replace(/\s+/g, ' ') ?? '';
      await verifier('estimation de la réception dans la boîte', /\d+ photos? .* à recevoir/.test(boite()), boite().slice(0, 200));
      await verifier('rangement par année et mois proposé pour une période', !!document.querySelector<HTMLInputElement>('input[name="rangement"][value="anneeMois"]')?.checked);
      await verifier('case RAW dans la boîte, cochée', !!document.querySelector<HTMLInputElement>('[role="dialog"] input[type="checkbox"]')?.checked && boite().includes('Inclure les fichiers RAW'));
      await capture('e6b-boite-reception');
      (await element<HTMLButtonElement>('[role="dialog"] button', (b) => b.textContent?.trim() === 'Annuler'))?.click();
      await attendre(500);
      const recu = await invoke<string | null>('dossier_banc_vide', { nom: 'recu' });
      const periode = { genre: 'periode', du: '2025-02-01', au: '2025-02-28' };
      const attendu = await invoke<{ fichiers: number }>('estimer_reception', { selection: periode, dossier: recu });
      await invoke('ajouter_reception', { dossier: recu, selection: periode, rangement: 'anneeMois' });
      type VueRegle = { sens: string; bilan: { recus: number } };
      let recus = 0;
      for (let i = 0; i < 60 && recus < attendu.fichiers; i++) {
        await attendre(500);
        recus = (await invoke<VueRegle[]>('regles_envoi')).find((r) => r.sens === 'descendante')?.bilan.recus ?? 0;
      }
      await verifier('période reçue en entier', recus === attendu.fichiers && recus > 0, `${recus} sur ${attendu.fichiers}`);
      await attendre(1500);
      await verifier('réception affichée sur la page', page().includes('Reçoit du') && page().includes(`${attendu.fichiers} reçu`), page().slice(0, 300));
      await capture('e6c-reception');
      (await element<HTMLButtonElement>('[aria-label^="Ne plus recevoir dans"]'))?.click();
      (await element<HTMLButtonElement>('[role="dialog"] button', (b) => b.textContent?.trim() === 'Ne plus recevoir'))?.click();
      await attendre(1500);

      // 11c. Two ways: an empty folder kept like the album made in step 11.
      const albumBalade = (await invoke<{ id: string; albumName: string }[]>('albums')).find((x) => x.albumName.startsWith('Balade banc'));
      const deux = await invoke<string | null>('dossier_banc_vide', { nom: 'deuxsens' });
      await invoke('ajouter_double', { dossier: deux, album: albumBalade?.id ?? null, nouvelAlbum: null, miroir: false });
      type VueDouble = { sens: string; miroir: boolean; bilan: { recus: number } };
      const double = async () => (await invoke<VueDouble[]>('regles_envoi')).find((r) => r.sens === 'double');
      for (let i = 0; i < 60 && ((await double())?.bilan.recus ?? 0) < 2; i++) await attendre(500);
      await verifier('deux sens : les photos de l’album arrivent dans le dossier', (await double())?.bilan.recus === 2);
      await attendre(1500);
      await verifier('deux sens affiché sur la page', page().includes('Identique à l’album'));
      (await element<HTMLButtonElement>('button', (b) => b.textContent?.trim() === 'Suppressions non répercutées'))?.click();
      await attendre(1200);
      await verifier('miroir activé depuis la page', (await double())?.miroir === true && page().includes('Suppressions répercutées'));
      await capture('e6d-deux-sens');
      (await element<HTMLButtonElement>('[aria-label^="Ne plus synchroniser"]'))?.click();
      (await element<HTMLButtonElement>('[role="dialog"] button', (b) => b.textContent?.trim() === 'Ne plus synchroniser'))?.click();
      await attendre(1500);

      // 11d. Virtual folder: February 2025 shown without downloading.
      const pointVirtuel = await invoke<string | null>('dossier_banc_vide', { nom: 'virtuel' });
      await invoke('ajouter_virtuel', { point: pointVirtuel, selection: periode, rangement: 'anneeMois' });
      type VueVirtuel = { monte: boolean; fichiers: number; erreur: string | null };
      const vv = (await invoke<VueVirtuel[]>('virtuels'))[0];
      await verifier('dossier virtuel monté avec ses fichiers', !!vv?.monte && vv.fichiers === attendu.fichiers, JSON.stringify(vv));
      await aller('/albums');
      await aller('/synchro');
      await verifier('dossier virtuel affiché', page().includes('Dossiers virtuels') && page().includes('visibles'));
      await capture('e6e-virtuel');
      (await element<HTMLButtonElement>('[aria-label^="Retirer le dossier virtuel"]'))?.click();
      await attendre(1500);
      await verifier('dossier virtuel retiré', (await invoke<VueVirtuel[]>('virtuels')).length === 0);
      // Another one stays: it must still be served once the app is closed
      // (checked by banc.sh).
      const persistant = await invoke<string | null>('dossier_banc_vide', { nom: 'persistant' });
      await invoke('ajouter_virtuel', { point: persistant, selection: periode, rangement: 'plat' });
      await verifier('dossier virtuel persistant monté', (await invoke<VueVirtuel[]>('virtuels'))[0]?.monte === true);

      // 12. Background: settings, then closing the window into the tray
      // while a folder is sent.
      await aller('/reglages');
      await verifier('section Envoi en arrière-plan', page().includes('Envoi en arrière-plan'));
      type Vue = { demarrageAuto: boolean; zoneDisponible: boolean; pauseSurBatterie: boolean };
      const reglages = () => invoke<Vue>('reglages_app');
      const interrupteur = (texte: string) =>
        element<HTMLElement>('label', (l) => (l.textContent ?? '').includes(texte)).then((l) => l?.querySelector<HTMLElement>('[role="switch"], button, input'));
      const demarrageAvant = (await reglages()).demarrageAuto;
      (await interrupteur('Lancer Nicéphore à l’ouverture de session'))?.click();
      await attendre(800);
      await verifier('démarrage avec la session activé', (await reglages()).demarrageAuto === !demarrageAvant);
      (await interrupteur('Lancer Nicéphore à l’ouverture de session'))?.click();
      await attendre(800);
      await verifier('démarrage avec la session désactivé', (await reglages()).demarrageAuto === demarrageAvant);
      (await interrupteur('Attendre le secteur'))?.click();
      await attendre(800);
      await verifier('réglage batterie retenu', (await reglages()).pauseSurBatterie === false);
      (await interrupteur('Attendre le secteur'))?.click();
      await attendre(800);
      await capture('e7-reglages');
      const zone = (await reglages()).zoneDisponible;
      await verifier('icône dans la zone de notification', zone);
      if (zone) {
        await invoke('fermer_fenetre');
        await attendre(1500);
        await verifier('fenêtre fermée, Nicéphore reste ouvert', !(await invoke<boolean>('fenetre_visible')));
        await invoke('montrer_fenetre');
        await attendre(1000);
        await verifier('rouverte depuis la zone de notification', await invoke<boolean>('fenetre_visible'));
      }

      await aller('/synchro');
      (await element<HTMLButtonElement>('[aria-label^="Ne plus envoyer"]'))?.click();
      const confirmerRetrait = await element<HTMLButtonElement>('[role="dialog"] button', (b) => b.textContent?.trim() === 'Ne plus envoyer');
      confirmerRetrait?.click();
      await attendre(1500);
      await verifier('dossier retiré', page().includes('Aucun dossier n’est envoyé') && page().includes('Dossier retiré'));
      // Without any folder synced, closing still goes to the tray.
      await invoke('fermer_fenetre');
      await attendre(1500);
      await verifier('fermer range dans la zone de notification même sans dossier', !(await invoke<boolean>('fenetre_visible')));
      await invoke('montrer_fenetre');
      await attendre(1000);
    }
    await capture('e7-fin');

    // 13. Account menu at the top right.
    (await element<HTMLButtonElement>('[aria-label^="Compte de"]'))?.click();
    await attendre(500);
    await verifier('menu du compte', !!document.querySelector('[role="menu"]') && (document.querySelector('[role="menu"]')?.textContent ?? '').includes('Réglages'));
    await capture('e8-menu-compte');
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    await attendre(300);

    // 14. Title bar drawn by the app: maximize, restore, minimize.
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    const fenetre = getCurrentWindow();
    await verifier('boutons de fenêtre dans l’en-tête', !!document.querySelector('header [aria-label="Fermer la fenêtre"]'));
    await cliquer('Agrandir la fenêtre');
    await attendre(800);
    await verifier('fenêtre agrandie', (await fenetre.isMaximized()) && !!document.querySelector('[aria-label="Restaurer la fenêtre"]'));
    await capture('e9-agrandie');
    await cliquer('Restaurer la fenêtre');
    await attendre(800);
    await verifier('fenêtre restaurée', !(await fenetre.isMaximized()));
    // Wayland does not tell an app it is minimized: only the call is checked.
    const reduite = await fenetre.minimize().then(() => true, () => false);
    await attendre(800);
    await verifier('réduction acceptée', reduite);
    await invoke('montrer_fenetre');
    await attendre(800);

    await journal(`scénario terminé : ${reussites} réussites, ${echecs} échecs`);
  } catch (e) {
    await journal(`scénario interrompu : ${e}`);
  }
  await invoke('quitter');
}
