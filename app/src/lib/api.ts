// Thin wrappers around the Tauri commands exposed by the Rust core.
import { invoke } from '@tauri-apps/api/core';

export type Utilisateur = { id: string; name: string; email: string };
export type Connexion = { urlServeur: string; version: string; utilisateur: Utilisateur };
export type Mois = { timeBucket: string; count: number };

/** Columnar month description: index `i` of every array describes asset `i`. */
export type DetailMois = {
  id: string[];
  ratio: number[];
  isImage: boolean[];
  isFavorite: boolean[];
  thumbhash: (string | null)[];
  duration: (number | string | null)[];
  fileCreatedAt: string[];
  localOffsetHours: number[];
  livePhotoVideoId: (string | null)[];
  stack?: (string[] | null)[] | null;
};

export const etatConnexion = () => invoke<Connexion | null>('etat_connexion');
export const seConnecter = (url: string, cle: string) => invoke<Connexion>('se_connecter', { url, cle });
export const listeMois = () => invoke<Mois[]>('liste_mois');
export const detailMois = (mois: string) => invoke<DetailMois>('detail_mois', { mois });
export const infosAsset = (id: string) => invoke<Record<string, any>>('infos_asset', { id });
export const urlVideo = (id: string) => invoke<string>('url_video', { id });

export type TailleImage = 'vignette' | 'apercu' | 'pleine';

// Custom protocols are exposed as `http://<scheme>.localhost` on Windows.
const baseImages = navigator.userAgent.includes('Windows')
  ? 'http://vignette.localhost'
  : 'vignette://localhost';

/** URL of a cached image; `version` (the thumbhash) busts the cache after edits. */
export function urlImage(taille: TailleImage, id: string, version: string | null): string {
  return `${baseImages}/${taille}/${id}?v=${encodeURIComponent(version ?? '')}`;
}
