// Thin wrappers around the Tauri commands exposed by the Rust core.
import { invoke } from '@tauri-apps/api/core';

export type Utilisateur = {
  id: string;
  name: string;
  email: string;
  /** Empty without a profile picture. */
  profileImagePath: string;
  avatarColor: string | null;
  profileChangedAt: string | null;
};
/** Where the connection comes from: development variables, OS keyring, or this session only. */
export type Origine = 'environnement' | 'trousseau' | 'coffre' | 'memoire';
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
/** `phrase`: portable mode only, seals the remembered connection. */
export const seConnecterMotDePasse = (url: string, email: string, motDePasse: string, memoriser: boolean, phrase?: string) =>
  invoke<Connexion>('se_connecter_mot_de_passe', { url, email, motDePasse, memoriser, phrase: phrase ?? null });
export const seConnecter = (url: string, cle: string, memoriser: boolean, phrase?: string) =>
  invoke<Connexion>('se_connecter', { url, cle, memoriser, phrase: phrase ?? null });
/** Portable mode: server of the sealed connection waiting for its passphrase. */
export const compteVerrouille = () => invoke<string | null>('compte_verrouille');
export const deverrouiller = (phrase: string) => invoke<Connexion>('deverrouiller', { phrase });
/** Portable mode: the folder holding everything, or null. */
export const infosPortable = () => invoke<string | null>('infos_portable');
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

export type TailleImage = 'vignette' | 'apercu' | 'pleine' | 'personne' | 'profil';

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
export type BilanEnvoi = {
  envoyes: number;
  presents: number;
  enAttente: number;
  echecs: number;
  ignores: number;
  recus: number;
  aRecevoir: number;
  retiresIci: number;
  retiresServeur: number;
  echecsReception: number;
};
export type Sens = 'montante' | 'descendante' | 'double';
export type RegleEnvoi = {
  id: number;
  dossier: string;
  sens: Sens;
  cible: Cible;
  selection: Selection | null;
  rangement: Rangement;
  /** Two-way folders: deletions repeated on the other side. */
  miroir: boolean;
  /** RAW files left out. */
  sansRaw: boolean;
  actif: boolean;
  creeLe: number;
  bilan: BilanEnvoi;
  dossierAbsent: boolean;
};
export type PhaseEnvoi = 'inactif' | 'analyse' | 'empreintes' | 'verification' | 'envoi' | 'albums' | 'reception';
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
  /** Why sending waits (on battery, metered network). */
  attente: string | null;
  /** Two-way folders whose deletions wait for a confirmation (count by rule). */
  confirmations: Record<string, number>;
  envoyesPassage: number;
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
export const ajouterRegleEnvoi = (dossier: string, cible: Cible, sansRaw = false) =>
  invoke<RegleEnvoi>('ajouter_regle_envoi', { dossier, cible, sansRaw });
export const activerRegleEnvoi = (id: number, actif: boolean) => invoke<void>('activer_regle_envoi', { id, actif });
export const supprimerRegleEnvoi = (id: number) => invoke<void>('supprimer_regle_envoi', { id });
export const etatEnvoi = () => invoke<EtatEnvoi>('etat_envoi');
export const lancerEnvoi = () => invoke<void>('lancer_envoi');
export const pauseEnvoi = (pause: boolean) => invoke<void>('pause_envoi', { pause });
export const journalEnvoi = (limite: number) => invoke<EntreeJournal[]>('journal_envoi', { limite });
export const echecsEnvoi = (regle: number) => invoke<FichierEnEchec[]>('echecs_envoi', { regle });
export const reessayerEnvoi = (regle: number) => invoke<number>('reessayer_envoi', { regle });

/** Application settings (background, conditions, notifications). */
export type ReglagesApp = {
  fermerEnArrierePlan: boolean;
  pauseSurBatterie: boolean;
  pauseReseauFacture: boolean;
  notifications: boolean;
};
export type ReglagesAppVue = ReglagesApp & { demarrageAuto: boolean; zoneDisponible: boolean; portable: boolean };
export const reglagesApp = () => invoke<ReglagesAppVue>('reglages_app');
export const definirReglagesApp = (reglages: ReglagesApp) => invoke<void>('definir_reglages_app', { reglages });
export const definirDemarrageAuto = (actif: boolean) => invoke<void>('definir_demarrage_auto', { actif });

// ---- Updates ----
export type InfosMiseAJour = { versionActuelle: string; possibles: boolean; installable: boolean; pageDesVersions: string };
export type VersionDisponible = { version: string; notes: string | null; date: string | null };
export const infosMiseAJour = () => invoke<InfosMiseAJour>('infos_mise_a_jour');
export const chercherMiseAJour = () => invoke<VersionDisponible | null>('chercher_mise_a_jour');
export const installerMiseAJour = () => invoke<void>('installer_mise_a_jour');

// ---- Accounts ----
export type CompteMemorise = {
  cle: string;
  urlServeur: string;
  nom: string;
  email: string;
  couleur: string | null;
  /** Portable mode: sealed with a passphrase. */
  scelle: boolean;
  actif: boolean;
};
export const comptes = () => invoke<CompteMemorise[]>('comptes');
/** Null when the account is sealed and waits for its passphrase. */
export const changerCompte = (cle: string) => invoke<Connexion | null>('changer_compte', { cle });

export type EstimationDossier = { fichiers: number; octets: number; illisibles: number };
export const estimerDossier = (dossier: string) => invoke<EstimationDossier>('estimer_dossier', { dossier });

// ---- Receiving ----
export type Selection =
  | { genre: 'album'; id: string; nom: string }
  | { genre: 'personne'; id: string; nom: string }
  | { genre: 'favoris' }
  | { genre: 'periode'; du: string | null; au: string | null }
  | { genre: 'bibliotheque' };
export type Rangement = 'plat' | 'anneeMois';
export type EstimationReception = { fichiers: number; octets: number; libre: number | null };
export const ajouterReception = (dossier: string, selection: Selection, rangement: Rangement, sansRaw = false) =>
  invoke<RegleEnvoi>('ajouter_reception', { dossier, selection, rangement, sansRaw });
export const estimerReception = (selection: Selection, dossier: string | null) =>
  invoke<EstimationReception>('estimer_reception', { selection, dossier });
export const recevoirDeNouveau = (regle: number) => invoke<number>('recevoir_de_nouveau', { regle });

// ---- Two-way folders ----
export type SuppressionEnAttente = { cote: 'ici' | 'serveur'; assetId: string; chemin: string; vuLe: number; confirme: boolean };
export const ajouterDouble = (dossier: string, album: string | null, nouvelAlbum: string | null, miroir: boolean, sansRaw = false) =>
  invoke<RegleEnvoi>('ajouter_double', { dossier, album, nouvelAlbum, miroir, sansRaw });
export const definirSansRaw = (regle: number, sansRaw: boolean) => invoke<void>('definir_sans_raw', { regle, sansRaw });
export const definirMiroir = (regle: number, miroir: boolean) => invoke<void>('definir_miroir', { regle, miroir });
export const suppressionsEnAttente = (regle: number) => invoke<SuppressionEnAttente[]>('suppressions_en_attente', { regle });
export const confirmerSuppressions = (regle: number) => invoke<number>('confirmer_suppressions', { regle });
export const refuserSuppressions = (regle: number) => invoke<number>('refuser_suppressions', { regle });

// ---- Virtual folders ----
export type DossierVirtuel = {
  id: number;
  point: string;
  selection: Selection;
  rangement: Rangement;
  sansRaw: boolean;
  monte: boolean;
  fichiers: number;
  erreur: string | null;
};
export const dossiersVirtuels = () => invoke<DossierVirtuel[]>('virtuels');
export const ajouterVirtuel = (point: string, selection: Selection, rangement: Rangement, sansRaw = false) =>
  invoke<void>('ajouter_virtuel', { point, selection, rangement, sansRaw });
export const retirerVirtuel = (id: number) => invoke<void>('retirer_virtuel', { id });
export const cacheOriginaux = () => invoke<OccupationCache>('cache_originaux');
export const viderCacheOriginaux = () => invoke<void>('vider_cache_originaux');

// ---- Local files (default viewer) ----
export type FichierOuvert = {
  chemin: string;
  nom: string;
  taille: number;
  modifieLe: number;
  affichable: boolean;
  video: boolean;
  voisins: string[];
  index: number;
};
export type EtatLocal = { connecte: boolean; surServeur: boolean; assetId: string | null; dansLaCorbeille: boolean };
export const fichiersAuDemarrage = () => invoke<string[]>('fichiers_au_demarrage');
export const ouvrirFichierLocal = (chemin: string) => invoke<FichierOuvert>('ouvrir_fichier_local', { chemin });
export const etatFichierLocal = (chemin: string) => invoke<EtatLocal>('etat_fichier_local', { chemin });
export const envoyerFichierLocal = (chemin: string) => invoke<string>('envoyer_fichier_local', { chemin });
export const definirVisionneuseParDefaut = () => invoke<string>('definir_visionneuse_par_defaut');
