// Manual upload, like Immich web: files and folders dropped on the window or
// picked with the « Envoyer » button. On an album page they go into that
// album; elsewhere, a dropped folder asks where its photos go (library only,
// a new album named after it, or an existing one).
import { page } from '$app/state';
import { listen } from '@tauri-apps/api/event';
import { album, decrireDepot, envoyerLot, etatEnvois, type AlbumEnvoi, type EtatEnvoiManuel } from './api';
import { t, tp } from './i18n.svelte';
import { notifier } from './notifications.svelte';

/** A drop waiting for its album. */
export interface Demande {
  chemins: string[];
  /** Name offered for a new album: the folder's, when one was dropped. */
  nomPropose: string;
  dossiers: number;
}

export const envois = $state<{ etat: EtatEnvoiManuel | null; demande: Demande | null; survol: boolean }>({
  etat: null,
  demande: null,
  survol: false,
});

let ecoute = false;
/** Follows the progress sent by the app (once). */
export function suivreEnvois() {
  if (ecoute) return;
  ecoute = true;
  void listen<EtatEnvoiManuel>('envoi-manuel', ({ payload }) => (envois.etat = payload));
  void etatEnvois()
    .then((e) => (envois.etat = e))
    .catch(() => {});
}

/** The album shown, when the page is one. */
export function albumAffiche(): string | null {
  return page.url.pathname === '/album' ? page.url.searchParams.get('id') || null : null;
}

/** Files and folders to send: straight into the album shown, or asks first for folders. */
export async function proposerEnvoi(chemins: string[]) {
  if (!chemins.length) return;
  const idAlbum = albumAffiche();
  if (idAlbum) {
    const nom = await album(idAlbum)
      .then((a) => a.albumName)
      .catch(() => '');
    return lancerEnvoi(chemins, { type: 'existant', id: idAlbum, nom });
  }
  const depot = await decrireDepot(chemins);
  if (depot.dossiers.length) {
    envois.demande = { chemins, nomPropose: depot.dossiers.length === 1 ? depot.dossiers[0] : '', dossiers: depot.dossiers.length };
  } else {
    await lancerEnvoi(chemins, null);
  }
}

export async function lancerEnvoi(chemins: string[], cible: AlbumEnvoi | null) {
  envois.demande = null;
  try {
    const lot = await envoyerLot(chemins, cible);
    if (!lot.medias) notifier(t('Aucune photo ni vidéo à envoyer'), 'info');
    else if (lot.ecartes)
      notifier(tp(lot.ecartes, '{n} fichier laissé de côté : format refusé par le serveur', '{n} fichiers laissés de côté : format refusé par le serveur'), 'info');
  } catch (e) {
    notifier(t('Envoi impossible : {e}', { e: String(e) }), 'erreur');
  }
}
