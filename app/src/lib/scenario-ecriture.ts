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

/** Types digits into a date or time segment of Immich, as on a keyboard. */
async function taperSegment(segment: HTMLElement | null, chiffres: string) {
  if (!segment) return;
  segment.focus();
  for (const c of chiffres) {
    segment.dispatchEvent(new KeyboardEvent('keydown', { key: c, code: `Digit${c}`, bubbles: true, cancelable: true }));
    await attendre(40);
  }
}

/** Types a "YYYY-MM-DD" date into the Immich date field named `groupe`. */
async function taperDate(groupe: string, date: string) {
  const segment = (type: string) => element<HTMLElement>(`[role="dialog"] [aria-label="${groupe}"] [data-segment="${type}"]`);
  const [annee, mois, jour] = date.split('-');
  await taperSegment(await segment('day'), jour);
  await taperSegment(await segment('month'), mois);
  await taperSegment(await segment('year'), annee);
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
    // Menus of Immich, not of the system: the sort menu opens and sorts.
    await verifier('aucune liste déroulante du système', document.querySelectorAll('select').length === 0);
    const tri = await element<HTMLButtonElement>('[data-select-trigger]');
    tri?.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, cancelable: true, button: 0, pointerType: 'mouse' }));
    const choixTri = await element<HTMLElement>('[role="option"]', (o) => (o.textContent ?? '').includes('Nom, de A à Z'));
    await capture('e1b-menu-tri');
    choixTri?.dispatchEvent(new PointerEvent('pointerup', { bubbles: true, cancelable: true, button: 0, pointerType: 'mouse' }));
    choixTri?.click();
    await attendre(500);
    await verifier(
      'menu de tri façon Immich',
      localStorage.getItem('nicephore-tri-albums') === 'az' && (tri?.textContent ?? '').includes('Nom, de A à Z'),
      `${localStorage.getItem('nicephore-tri-albums')} ${tri?.textContent?.trim() ?? 'pas de menu'}`,
    );
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
    await verifier('date et heure façon Immich', !document.querySelector('[role="dialog"] input[type="datetime-local"]'));
    const segment = (groupe: string, type: string) => element<HTMLElement>(`[role="dialog"] [aria-label="${groupe}"] [data-segment="${type}"]`);
    await taperDate('Date', '2020-06-15');
    await taperSegment(await segment('Heure', 'hour'), '14');
    await taperSegment(await segment('Heure', 'minute'), '30');
    await capture('e4b-date');
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
      (await element<HTMLElement>('[aria-label="Inclure les fichiers RAW"]'))?.click();
      await attendre(1200);
      type VueRaw = { sens: string; sansRaw: boolean };
      await verifier('fichiers RAW laissés de côté depuis la carte', (await invoke<VueRaw[]>('regles_envoi'))[0]?.sansRaw === true && page().includes('Inclure les fichiers RAW (DNG, ARW, CR2…) : laissés de côté'));
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
      await element('input[name="sens"]');
      await verifier(
        'types de synchro nommés court',
        ['Sauvegarde', 'Copie locale', 'Fichiers virtuels', 'Synchro complète'].every((t) => page().includes(t)),
      );
      await capture('e11-types-synchro');
      (await element<HTMLInputElement>('input[name="sens"][value="descendante"]'))?.click();
      await attendre(300);
      (await element<HTMLInputElement>('input[name="quoi"][value="periode"]'))?.click();
      await attendre(300);
      await taperDate('Du', '2025-02-01');
      await taperDate('Au', '2025-02-28');
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

      // 11c bis. Two ways with the library (February 2025 standing for
      // it): the dialog, then a folder holding a renamed copy of a server
      // photo and a new one (prepared by banc.sh).
      (await element<HTMLButtonElement>('button', (b) => b.textContent?.includes('Ajouter un dossier') ?? false))?.click();
      (await element<HTMLInputElement>('input[name="sens"][value="double"]'))?.click();
      await attendre(500);
      await verifier(
        'deux sens : photothèque proposée par défaut',
        !!document.querySelector<HTMLInputElement>('input[name="avec"][value="bibliotheque"]')?.checked &&
          boite().includes('Où ranger les photos reçues') &&
          boite().includes('Depuis Immich') &&
          boite().includes('reconnue à son contenu'),
        boite().slice(0, 300),
      );
      await capture('e6d0-boite-deux-sens');
      (await element<HTMLButtonElement>('[role="dialog"] button', (b) => b.textContent?.trim() === 'Annuler'))?.click();
      await attendre(500);
      const deuxBib = await invoke<string | null>('dossier_banc_vide', { nom: 'deuxbib' });
      const fevrier = await invoke<{ fichiers: number }>('estimer_reception', { selection: periode, dossier: deuxBib });
      await invoke('ajouter_double_bibliotheque', {
        dossier: deuxBib,
        cible: { genre: 'bibliotheque' },
        selection: periode,
        dossierRecus: 'Depuis Immich',
      });
      type VueBib = { sens: string; cible: { genre: string }; bilan: { envoyes: number; presents: number; recus: number; aRecevoir: number; retiresServeur: number } };
      const bib = async () => (await invoke<VueBib[]>('regles_envoi')).find((r) => r.sens === 'double' && r.cible.genre !== 'album');
      for (let i = 0; i < 80 && ((await bib())?.bilan.recus ?? 0) < fevrier.fichiers - 1; i++) await attendre(500);
      await attendre(1500);
      const vueBib = await bib();
      await verifier(
        'deux sens photothèque : la nouvelle part, la copie renommée ne redescend pas',
        vueBib?.bilan.envoyes === 1 && vueBib.bilan.recus === fevrier.fichiers - 1 && vueBib.bilan.aRecevoir === 0 && vueBib.bilan.retiresServeur === 0,
        JSON.stringify(vueBib?.bilan),
      );
      await verifier('deux sens photothèque affiché', page().includes('Synchronisé avec') && page().includes('reçues dans « Depuis Immich »'), page().slice(0, 300));
      await capture('e6d1-deux-sens-phototheque');
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

    {
      // 15. Sidebar like Immich: recent albums under "Albums", storage, server.
      await aller('/');
      await attendre(1500);
      const barre = () => document.querySelector('nav[aria-label="Navigation principale"]')?.textContent?.replace(/\s+/g, ' ') ?? '';
      await verifier('albums récents sous « Albums »', !!document.querySelector('nav .albums-recents a[href^="/album?id="]'), barre().slice(0, 200));
      await verifier('stockage du serveur', /Stockage .* sur .* utilisé/.test(barre()), barre().slice(-160));
      await verifier('serveur en ligne et sa version', /Serveur en ligne v\d+\.\d+\.\d+/.test(barre()), barre().slice(-80));
      await capture('e10-barre-laterale');

      // 16. Search panel of the Immich web app, then the chips of the results.
      const champ = await element<HTMLInputElement>('input[aria-label="Rechercher"]');
      champ?.focus();
      champ?.click();
      await attendre(500);
      const panneau = () => document.getElementById('panneau-recherche');
      const texte = () => panneau()?.textContent?.replace(/\s+/g, ' ') ?? '';
      await verifier(
        'panneau de recherche façon Immich',
        ['Filtrer par', 'Rechercher par type', 'Personnes', 'Date', 'Lieux', 'Média', 'Filtres avancés', 'Effacer tout', 'Contexte'].every((t) => texte().includes(t)),
        texte().slice(0, 200),
      );
      const avancesOuverts = () => !panneau()?.querySelector('[inert]');
      await verifier('par défaut, filtres avancés repliés comme Immich', !avancesOuverts());
      const pastilles = () => [...(panneau()?.querySelectorAll('button') ?? [])].map((b) => (b.textContent ?? '').trim());
      await verifier('« Étiquettes » masqué quand le compte les désactive', !pastilles().includes('Étiquettes'), pastilles().join(' | '));
      await capture('e10b-panneau-defaut');
      const bouton = (libelle: string) =>
        element<HTMLButtonElement>('#panneau-recherche button', (b) => (b.textContent ?? '').trim() === libelle);
      (await bouton('Lieux'))?.click();
      await attendre(500);
      const pays = await element<HTMLInputElement>('#panneau-recherche [data-libelle="Pays"] input');
      pays?.focus();
      await attendre(600);
      const liste = document.querySelector<HTMLElement>('#panneau-recherche [data-libelle="Pays"] [role="listbox"]');
      const r = liste?.getBoundingClientRect();
      const visible = !!r && r.height > 0 && document.elementFromPoint(r.left + r.width / 2, r.bottom - 4)?.closest('[role="listbox"]') === liste;
      await verifier('liste des pays entière, non rognée par le panneau', visible && getComputedStyle(liste!).position === 'fixed', r ? `${Math.round(r.top)}-${Math.round(r.bottom)}` : 'pas de liste');
      await capture('e10c-liste-pays');
      // A click on the panel's text: the panel stays open, the list closes.
      const textePanneau = panneau()?.querySelector('p, span') as HTMLElement | null;
      textePanneau?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
      (panneau()?.closest('[tabindex="-1"]') as HTMLElement | null)?.focus();
      await attendre(400);
      await verifier('un clic dans le panneau le laisse ouvert', !!panneau() && !document.querySelector('[data-libelle="Pays"] [role="listbox"]'));
      (await bouton('Média'))?.click();
      await attendre(300);
      (await bouton('Image'))?.click();
      await attendre(300);
      (await bouton('Filtres avancés'))?.click();
      await attendre(800);
      await verifier('filtres avancés dépliés : appareil photo et bibliothèque', avancesOuverts() && ['Appareil photo', 'Marque', 'Modèle d\'objectif', 'Pas dans un album'].every((t) => texte().includes(t)));
      const marques = await invoke<string[]>('suggestions_recherche', { genre: 'marque' }).then(
        (v) => v,
        () => null,
      );
      await verifier('suggestions du serveur (marques)', Array.isArray(marques), JSON.stringify(marques));
      (await bouton('Favoris'))?.click();
      await attendre(300);
      await capture('e11-panneau-recherche');
      (await bouton('Recherche'))?.click();
      await attendre(2000);
      const puces = () => [...document.querySelectorAll('.puce')].map((p) => p.getAttribute('data-cle'));
      await verifier('recherche lancée avec les filtres', location.pathname === '/recherche' && location.search.includes('type=image') && location.search.includes('favoris=1'), location.search);
      await verifier('puces des filtres façon Immich', puces().includes('type') && puces().includes('favoris'), puces().join(','));
      await verifier('panneau refermé après la recherche', !panneau());
      await capture('e12-puces-recherche');
      (await element<HTMLButtonElement>('.puce[data-cle="favoris"] button'))?.click();
      await attendre(1500);
      await verifier('retirer une puce retire le filtre', !location.search.includes('favoris') && puces().join(',') === 'type', location.search);

    }

    await journal(`scénario terminé : ${reussites} réussites, ${echecs} échecs`);
  } catch (e) {
    await journal(`scénario interrompu : ${e}`);
  }
  await invoke('quitter');
}
