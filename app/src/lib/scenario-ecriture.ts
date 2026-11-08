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
    await verifier('corbeille vide après annulation', document.body.textContent?.includes('La corbeille est vide') ?? false);

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
    await capture('e5-fin');

    await journal(`scénario terminé : ${reussites} réussites, ${echecs} échecs`);
  } catch (e) {
    await journal(`scénario interrompu : ${e}`);
  }
  await invoke('quitter');
}
