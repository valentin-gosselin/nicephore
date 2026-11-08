// Thin wrappers around the Tauri commands exposed by the Rust core.
import { invoke } from '@tauri-apps/api/core';

export type Utilisateur = { id: string; name: string; email: string };
/** Where the connection comes from: development variables, OS keyring, or this session only. */
export type Origine = 'environnement' | 'trousseau' | 'memoire';
export type Connexion = {
  urlServeur: string;
  version: string;
  utilisateur: Utilisateur;
  origine: Origine;
  /** Session opened with email and password (rather than an API key). */
  session: boolean;
};

/** Login methods offered by a server, read before logging in. */
export type ModesConnexion = { passwordLogin: boolean; oauth: boolean; urlServeur: string };

/** Which assets a timeline shows. */
export type Filtre = {
  visibilite?: 'timeline' | 'archive';
  favori?: boolean;
  corbeille?: boolean;
  personne?: string;
  album?: string;
};

/** Asset of a search result, reduced to what the grid needs. */
export type AssetResume = {
  id: string;
  ratio: number;
  isImage: boolean;
  isFavorite: boolean;
  thumbhash: string | null;
  duration: number | null;
  fileCreatedAt: string;
  localDateTime: string;
};
export type PageResultats = { assets: AssetResume[]; pageSuivante: number | null; total: number };

/** contenu: smart search on what the image shows; the others match text fields. */
export type ModeRecherche = 'contenu' | 'fichierNom' | 'description' | 'texte';
export type CriteresRecherche = {
  texte?: string;
  mode?: ModeRecherche;
  typeMedia?: 'image' | 'video';
  personnes?: string[];
  ville?: string;
  pays?: string;
  du?: string;
  au?: string;
  favoris?: boolean;
  archive?: boolean;
};
export type Lieu = { ville: string; asset: AssetResume };
export type Personne = {
  id: string;
  name: string;
  isFavorite: boolean;
  isHidden: boolean;
  birthDate: string | null;
  updatedAt: string | null;
};
export type ModificationPersonne = { name?: string; isFavorite?: boolean; isHidden?: boolean };

export type Album = {
  id: string;
  albumName: string;
  description: string;
  assetCount: number;
  albumThumbnailAssetId: string | null;
  startDate: string | null;
  endDate: string | null;
  shared: boolean;
  updatedAt: string | null;
  lastModifiedAssetTimestamp: string | null;
  /** Owned by the user (false: shared with them). */
  possede: boolean;
};
export type BilanAlbum = { ajoutes: number; dejaPresents: number; echecs: number };
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
export const modesConnexion = (url: string) => invoke<ModesConnexion>('modes_connexion', { url });
export const seConnecterMotDePasse = (url: string, email: string, motDePasse: string, memoriser: boolean) =>
  invoke<Connexion>('se_connecter_mot_de_passe', { url, email, motDePasse, memoriser });
export const seConnecter = (url: string, cle: string, memoriser: boolean) =>
  invoke<Connexion>('se_connecter', { url, cle, memoriser });
export const seDeconnecter = () => invoke<void>('se_deconnecter');
export const listeMois = (filtre: Filtre) => invoke<Mois[]>('liste_mois', { filtre });
export const detailMois = (mois: string, filtre: Filtre) => invoke<DetailMois>('detail_mois', { mois, filtre });
export const definirFavori = (ids: string[], favori: boolean) => invoke<void>('definir_favori', { ids, favori });
export const definirArchive = (ids: string[], archive: boolean) => invoke<void>('definir_archive', { ids, archive });
export const mettreCorbeille = (ids: string[]) => invoke<void>('mettre_corbeille', { ids });

/** Changes to the information of assets; absent fields are left as they are. */
export type ModificationInfos = {
  description?: string;
  /** 1 to 5 stars, -1 rejected, null to remove. */
  note?: number | null;
  /** ISO with offset. */
  date?: string;
  fuseau?: string;
  latitude?: number;
  longitude?: number;
};
export const modifierInfos = (ids: string[], modification: ModificationInfos) =>
  invoke<void>('modifier_infos', { ids, modification });
export type Tag = { id: string; name: string; value: string; color: string | null };
export const tags = () => invoke<Tag[]>('tags');
export const etiqueter = (nom: string, ids: string[], retirer = false) => invoke<Tag>('etiqueter', { nom, ids, retirer });
export const restaurerCorbeille = (ids: string[]) => invoke<void>('restaurer_corbeille', { ids });

export type Bilan = { reussis: number; echecs: string[]; dossier: string };
export const telecharger = (ids: string[], dossier: string) => invoke<Bilan>('telecharger', { ids, dossier });

export type OccupationCache = { octets: number; fichiers: number; budget: number };
export type InfosCache = { vignettes: OccupationCache; apercus: OccupationCache; dossier: string };
export const infosCache = () => invoke<InfosCache>('infos_cache');
export const viderCache = () => invoke<void>('vider_cache');

/** Background download of every grid thumbnail. */
export type EtatPrechargement = {
  actif: boolean;
  enCours: boolean;
  termine: boolean;
  total: number;
  presentes: number;
  erreurs: number;
  /** The last finished pass had every thumbnail on disk. */
  complet: boolean;
};
export const etatPrechargement = () => invoke<EtatPrechargement>('etat_prechargement');
export const definirPrechargement = (actif: boolean) => invoke<void>('definir_prechargement', { actif });
export const signalerDefilement = () => invoke<void>('signaler_defilement');
export const rechercher = (criteres: CriteresRecherche, page: number) =>
  invoke<PageResultats>('rechercher', { criteres, page });
export const lieux = () => invoke<Lieu[]>('lieux');
export const personnes = (avecMasquees = false) => invoke<Personne[]>('personnes', { avecMasquees });
export const personne = (id: string) => invoke<Personne>('personne', { id });
export const personnesProches = (id: string) => invoke<Personne[]>('personnes_proches', { id });
export const modifierPersonne = (id: string, modification: ModificationPersonne) =>
  invoke<Personne>('modifier_personne', { id, modification });
export const fusionnerPersonnes = (cible: string, autres: string[]) =>
  invoke<number>('fusionner_personnes', { cible, autres });

export type Marqueur = { id: string; lat: number; lon: number; city: string | null };
export type StylesCarte = { mapLightStyleUrl: string; mapDarkStyleUrl: string };
export type Souvenir = {
  id: string;
  /** `on_this_day` or `birthday`. */
  genre: string;
  annee: number;
  personne: string | null;
  date: string;
  assets: AssetResume[];
};
export const marqueurs = () => invoke<Marqueur[]>('marqueurs');
export const stylesCarte = () => invoke<StylesCarte>('styles_carte');
export const souvenirs = (jour: string) => invoke<Souvenir[]>('souvenirs', { jour });
export const souvenir = (id: string) => invoke<Souvenir>('souvenir', { id });
export const assetsParIds = (ids: string[]) => invoke<AssetResume[]>('assets_par_ids', { ids });

export const albums = () => invoke<Album[]>('albums');
export const album = (id: string) => invoke<Album>('album', { id });
export const creerAlbum = (nom: string, description: string, ids: string[]) =>
  invoke<Album>('creer_album', { nom, description, ids });
export const modifierAlbum = (id: string, modification: { nom?: string; description?: string }) =>
  invoke<void>('modifier_album', { id, modification });
export const supprimerAlbum = (id: string) => invoke<void>('supprimer_album', { id });
export const ajouterALAlbum = (id: string, ids: string[]) => invoke<BilanAlbum>('ajouter_a_l_album', { id, ids });
export const retirerDeLAlbum = (id: string, ids: string[]) => invoke<void>('retirer_de_l_album', { id, ids });
export const nombresPersonnes = (ids: string[]) => invoke<Record<string, number>>('nombres_personnes', { ids });
export const nombresVilles = (villes: string[]) => invoke<Record<string, number>>('nombres_villes', { villes });
export const infosAsset = (id: string) => invoke<Record<string, any>>('infos_asset', { id });
export const urlVideo = (id: string) => invoke<string>('url_video', { id });

export type TailleImage = 'vignette' | 'apercu' | 'pleine' | 'personne';

// Custom protocols are exposed as `http://<scheme>.localhost` on Windows.
const baseImages = navigator.userAgent.includes('Windows')
  ? 'http://vignette.localhost'
  : 'vignette://localhost';

/** URL of a cached image; `version` (the thumbhash) busts the cache after edits. */
export function urlImage(taille: TailleImage, id: string, version: string | null): string {
  return `${baseImages}/${taille}/${id}?v=${encodeURIComponent(version ?? '')}`;
}

// ---- Upload of local folders ----

/** Where the photos of a folder go. */
export type Cible =
  | { genre: 'bibliotheque' }
  | { genre: 'album'; id: string; nom: string }
  | { genre: 'albumParDossier' };
export type BilanEnvoi = { envoyes: number; presents: number; enAttente: number; echecs: number; ignores: number };
export type RegleEnvoi = {
  id: number;
  dossier: string;
  cible: Cible;
  actif: boolean;
  creeLe: number;
  bilan: BilanEnvoi;
  dossierAbsent: boolean;
};
export type PhaseEnvoi = 'inactif' | 'analyse' | 'empreintes' | 'verification' | 'envoi' | 'albums';
export type EtatEnvoi = {
  phase: PhaseEnvoi;
  regle: number | null;
  fichier: string | null;
  faits: number;
  total: number;
  octetsFaits: number;
  octetsTotal: number;
  enPause: boolean;
  connecte: boolean;
  erreur: string | null;
  erreursRegles: Record<string, string>;
  dernierPassage: number | null;
};
export type EntreeJournal = {
  id: number;
  horodatage: number;
  regle: number | null;
  chemin: string | null;
  action: string;
  detail: string | null;
  assetId: string | null;
};
export type FichierEnEchec = { chemin: string; erreur: string | null; essais: number };

export const reglesEnvoi = () => invoke<RegleEnvoi[]>('regles_envoi');
export const ajouterRegleEnvoi = (dossier: string, cible: Cible) => invoke<RegleEnvoi>('ajouter_regle_envoi', { dossier, cible });
export const activerRegleEnvoi = (id: number, actif: boolean) => invoke<void>('activer_regle_envoi', { id, actif });
export const supprimerRegleEnvoi = (id: number) => invoke<void>('supprimer_regle_envoi', { id });
export const etatEnvoi = () => invoke<EtatEnvoi>('etat_envoi');
export const lancerEnvoi = () => invoke<void>('lancer_envoi');
export const pauseEnvoi = (pause: boolean) => invoke<void>('pause_envoi', { pause });
export const journalEnvoi = (limite: number) => invoke<EntreeJournal[]>('journal_envoi', { limite });
export const echecsEnvoi = (regle: number) => invoke<FichierEnEchec[]>('echecs_envoi', { regle });
export const reessayerEnvoi = (regle: number) => invoke<number>('reessayer_envoi', { regle });
