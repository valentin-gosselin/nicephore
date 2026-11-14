<script lang="ts">
  // A photo or video of this computer (Nicéphore as the default viewer):
  // the file and its neighbours in the folder, zoom, rotation, full screen,
  // slideshow, details, a strip of the folder, the usual file actions, and
  // whether it is on Immich (cloud button, like the mobile app).
  import { IconButton, LoadingSpinner, MenuItemType, type MenuItems } from '@immich/ui';
  import { startDrag } from '@crabnebula/tauri-plugin-drag';
  import {
    mdiChevronLeft,
    mdiChevronRight,
    mdiCloudCheckOutline,
    mdiCloudOffOutline,
    mdiCloudUploadOutline,
    mdiContentCopy,
    mdiDotsVertical,
    mdiFolderOpenOutline,
    mdiFullscreen,
    mdiFullscreenExit,
    mdiImageAlbum,
    mdiImageMultipleOutline,
    mdiInformationOutline,
    mdiOpenInApp,
    mdiOpenInNew,
    mdiPause,
    mdiPlay,
    mdiRotateLeft,
    mdiRotateRight,
    mdiTrashCanOutline,
    mdiViewCarouselOutline,
  } from '@mdi/js';
  import { convertFileSrc, invoke } from '@tauri-apps/api/core';
  import {
    availableMonitors,
    currentMonitor,
    getCurrentWindow,
    LogicalPosition,
    LogicalSize,
    primaryMonitor,
  } from '@tauri-apps/api/window';
  import { onMount, untrack } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import {
    afficherDansDossier,
    ajouterALAlbum,
    apercuLocal,
    copierImageLocale,
    copierTexte,
    corbeilleLocale,
    creerAlbum,
    envoyerFichierLocal,
    etatFichierLocal,
    ouvrirAvec,
    ouvrirFichierLocal,
    pivoterFichierLocal,
    restaurerCorbeilleLocale,
    urlVideoLocale,
    vignetteLocale,
    type Album,
    type ApercuLocal,
    type EtatLocal,
    type FichierOuvert,
  } from '../../lib/api';
  import BandeVignettes, { oublierVignette } from '../../lib/BandeVignettes.svelte';
  import BoutonsFenetre from '../../lib/BoutonsFenetre.svelte';
  import ChoixAlbum from '../../lib/ChoixAlbum.svelte';
  import { desarmerFilet } from '../../lib/filet-fenetre';
  import LecteurVideo from '../../lib/LecteurVideo.svelte';
  import { menuActif, ouvrirMenu } from '../../lib/menu-contextuel';
  import PanneauInfosLocal from '../../lib/PanneauInfosLocal.svelte';
  import { locale, t } from '../../lib/i18n.svelte';
  import { notifier } from '../../lib/notifications.svelte';
  import { session } from '../../lib/session.svelte';
  import { Zoom } from '../../lib/zoom.svelte';

  const chemin = $derived(page.url.searchParams.get('chemin') ?? '');
  let fichier = $state<FichierOuvert | null>(null);
  let erreur = $state<string | null>(null);
  let etat = $state<EtatLocal | null>(null);
  let envoi = $state(false);
  let apercu = $state<ApercuLocal | null>(null);
  let video = $state<string | null>(null);
  let videoIlisible = $state(false);
  /** Size of the photo shown (before rotation), once known. */
  let naturel = $state<{ l: number; h: number } | null>(null);
  /** Quarter turns asked by the user (display only, the file is untouched). */
  let quarts = $state(0);
  let vignetteCourante: string | null = null;
  /** Quarter turns not written yet (in the file and on Immich). */
  let aPivoter = 0;
  let minuteurRotation: ReturnType<typeof setTimeout> | undefined;
  let rotations: Promise<void> = Promise.resolve();
  const DELAI_ROTATION = 900;
  /**
   * Counts the openings: the same photo opened again (from the file manager)
   * is loaded again, so its size, the window and its showing follow.
   */
  let ouverture = $state(0);
  let zoneL = $state(0);
  let zoneH = $state(0);
  /** The folder has no photo left (the last one went to the trash). */
  let dossierVide = $state(false);
  let choixAlbum = $state(false);
  /** Opened from outside (file manager): the window takes the photo's size. */
  let ajusterFenetre = false;
  const zoom = new Zoom();

  const PREFERENCE_BANDE = 'nicephore-bande-vignettes';
  let bande = $state(lirePreference());
  let infos = $state(false);
  let lecture = $state(false);
  let pleinEcran = $state(false);
  /** Full screen: bar, strip and pointer show while the mouse moves. */
  let actif = $state(true);
  let minuteurActif: ReturnType<typeof setTimeout> | undefined;
  let indicateur = $state(false);
  let minuteurIndicateur: ReturnType<typeof setTimeout> | undefined;

  const DUREE_DIAPO = 5000;
  const BARRE = 48;
  const HAUTEUR_BANDE = 76;
  /** Narrow window (portrait photo): fewer buttons in the bar. */
  const compact = $derived(zoneL > 0 && zoneL < 620);

  function lirePreference() {
    try {
      return localStorage.getItem(PREFERENCE_BANDE) !== '0';
    } catch {
      return true;
    }
  }

  function basculerBande() {
    bande = !bande;
    try {
      localStorage.setItem(PREFERENCE_BANDE, bande ? '1' : '0');
    } catch {
      // Private storage unavailable: the choice lasts for this window.
    }
  }

  /**
   * Window sized to each photo (within the screen), like a photo viewer:
   * from portrait to landscape, the window changes, not the photo. Opened
   * from outside, it is centred then shown; afterwards it keeps its centre
   * (where the system lets an app move its window: not under Wayland).
   */
  async function ajuster(largeur: number, hauteur: number) {
    const premiere = ajusterFenetre;
    ajusterFenetre = false;
    const fenetre = getCurrentWindow();
    try {
      // A hidden window has no screen yet (Wayland): the primary one then.
      const ecran = (await currentMonitor()) ?? (await primaryMonitor()) ?? (await availableMonitors())[0];
      if (ecran && largeur > 0 && hauteur > 0 && !(await fenetre.isMaximized()) && !(await fenetre.isFullscreen())) {
        const habillage = BARRE + (bande ? HAUTEUR_BANDE : 0);
        const panneau = infos ? 320 : 0;
        const maxL = (ecran.size.width / ecran.scaleFactor) * 0.9 - panneau;
        const maxH = (ecran.size.height / ecran.scaleFactor) * 0.85 - habillage;
        const r = Math.min(1, maxL / largeur, maxH / hauteur);
        const l = Math.max(480, Math.round(largeur * r) + panneau);
        const h = Math.max(400, Math.round(hauteur * r) + habillage);
        if (premiere) {
          await fenetre.setSize(new LogicalSize(l, h));
          await fenetre.center();
        } else {
          const echelle = await fenetre.scaleFactor();
          const position = (await fenetre.outerPosition()).toLogical(echelle);
          const avant = (await fenetre.outerSize()).toLogical(echelle);
          await fenetre.setSize(new LogicalSize(l, h));
          const ecranX = ecran.position.x / ecran.scaleFactor;
          const ecranY = ecran.position.y / ecran.scaleFactor;
          const x = Math.max(ecranX, position.x + avant.width / 2 - l / 2);
          const y = Math.max(ecranY, position.y + avant.height / 2 - h / 2);
          await fenetre.setPosition(new LogicalPosition(Math.round(x), Math.round(y))).catch(() => {});
        }
      }
    } catch (e) {
      await invoke('journal', { message: `taille de fenêtre : ${e}` }).catch(() => {});
    } finally {
      if (premiere) {
        // Shown here: the safety net must not show it again once closed.
        desarmerFilet();
        await fenetre.show();
        await fenetre.setFocus();
      }
    }
  }

  /** The window follows the panel, the strip and the rotation, as it follows each photo. */
  function reajuster() {
    if (naturel) void ajuster(tourne ? naturel.h : naturel.l, tourne ? naturel.l : naturel.h);
  }
  let premierPassage = true;
  $effect(() => {
    void infos;
    void bande;
    void quarts;
    if (premierPassage) premierPassage = false;
    else untrack(reajuster);
  });

  /** Address of the photo, new after each write (the web view keeps images). */
  const urlPhoto = (cheminPhoto: string, version: number) => `${convertFileSrc(cheminPhoto)}?v=${version}`;

  /** Rotation of a RAW preview from its EXIF orientation. */
  const rotation = (o: number) => (o === 3 ? 180 : o === 6 ? 90 : o === 8 ? 270 : 0);
  const angle = $derived(((apercu && !fichier?.affichable ? rotation(apercu.orientation) : 0) + quarts * 90) % 360);
  const tourne = $derived(angle % 180 !== 0);
  /** Fit of the photo in the area, never enlarged beyond its size. */
  const ajustement = $derived.by(() => {
    if (!naturel || !zoneL || !zoneH) return 0;
    const l = tourne ? naturel.h : naturel.l;
    const h = tourne ? naturel.l : naturel.h;
    return Math.min(1, zoneL / l, zoneH / h);
  });

  $effect(() => {
    if (!naturel || !ajustement) return;
    const s = ajustement;
    const l = (tourne ? naturel.h : naturel.l) * s;
    const h = (tourne ? naturel.l : naturel.h) * s;
    zoom.dimensionner(l, h, zoneL, zoneH, 1 / (s * (window.devicePixelRatio || 1)));
  });

  // The zoom level shows for a moment after each change.
  let echellePrecedente = 1;
  $effect(() => {
    const echelle = zoom.echelle;
    if (echelle === echellePrecedente) return;
    echellePrecedente = echelle;
    indicateur = true;
    clearTimeout(minuteurIndicateur);
    minuteurIndicateur = setTimeout(() => (indicateur = false), 1200);
  });

  $effect(() => {
    const demande = chemin;
    // A turn not written yet belongs to the photo shown until now.
    untrack(() => void enregistrerRotation());
    // Each opening from outside asks again; the arrows never do.
    ajusterFenetre = page.url.searchParams.get('ajuster') === '1';
    erreur = null;
    if (!demande) {
      fichier = null;
      return;
    }
    // The previous photo stays until the next one is ready (its RAW preview
    // or video included): no message or empty frame in between.
    ouvrirFichierLocal(demande)
      .then(async (f) => {
        if (demande !== chemin) return;
        const url = f.video ? await urlVideoLocale(demande).catch(() => null) : null;
        const a = f.raw ? await apercuLocal(demande).catch(() => null) : null;
        if (demande !== chemin) return;
        fichier = f;
        ouverture++;
        dossierVide = false;
        etat = null;
        apercu = a;
        video = url;
        videoIlisible = f.video && !url;
        quarts = 0;
        zoom.reinitialiser();
        naturel = a && !f.affichable ? { l: a.largeur, h: a.hauteur } : null;
        vignetteCourante = null;
        if (!f.video) void vignetteLocale(demande).then((v) => demande === chemin && (vignetteCourante = v)).catch(() => {});
        if (a && !f.affichable) {
          const r = rotation(a.orientation) % 180 !== 0;
          void ajuster(r ? a.hauteur : a.largeur, r ? a.largeur : a.hauteur);
        } else if (videoIlisible || !f.affichable) {
          void ajuster(0, 0);
        }
        prechargerVoisins(f);
        return etatFichierLocal(demande).then((e) => demande === chemin && (etat = e));
      })
      .catch((e) => {
        erreur = String(e);
        void ajuster(0, 0);
      });
  });

  const EST_RAW = /\.(dng|arw|cr2|cr3|nef|orf|raf|rw2|srw|pef)$/i;
  /** The RAW previews of the previous and next files, extracted ahead. */
  function prechargerVoisins(f: FichierOuvert) {
    const n = f.voisins.length;
    if (n < 2) return;
    for (const pas of [1, -1]) {
      const voisin = f.voisins[(f.index + pas + n) % n];
      if (EST_RAW.test(voisin)) void apercuLocal(voisin).catch(() => null);
    }
  }

  function ouvrir(cheminVoisin: string) {
    void enregistrerRotation();
    void goto(`/fichier?chemin=${encodeURIComponent(cheminVoisin)}`, { replaceState: true });
  }

  function aller(sens: 1 | -1) {
    if (!fichier || fichier.voisins.length < 2) return;
    ouvrir(fichier.voisins[(fichier.index + sens + fichier.voisins.length) % fichier.voisins.length]);
  }

  function allerA(i: number) {
    if (fichier && i !== fichier.index && fichier.voisins[i]) ouvrir(fichier.voisins[i]);
  }

  // ---- Immich ----

  async function envoyer(album?: { existant: Album } | { nouveau: string }) {
    if (!fichier) return;
    const courant = fichier;
    envoi = true;
    try {
      const id = await envoyerFichierLocal(courant.chemin);
      if (album && 'existant' in album) {
        await ajouterALAlbum(album.existant.id, [id]);
        notifier(t('« {nom} » envoyée dans l’album « {album} »', { nom: courant.nom, album: album.existant.albumName }), 'succes');
      } else if (album) {
        await creerAlbum(album.nouveau, '', [id]);
        notifier(t('« {nom} » envoyée dans l’album « {album} »', { nom: courant.nom, album: album.nouveau }), 'succes');
      } else {
        notifier(t('« {nom} » envoyée vers Immich', { nom: courant.nom }), 'succes');
      }
      if (courant.chemin === fichier?.chemin) etat = await etatFichierLocal(courant.chemin);
    } catch (e) {
      notifier(t('Envoi impossible : {e}', { e: String(e) }), 'erreur');
    } finally {
      envoi = false;
    }
  }

  function ouvrirDansLaBibliotheque() {
    if (etat?.assetId) void goto(`/photo?id=${encodeURIComponent(etat.assetId)}`);
  }

  // ---- File actions ----

  const copiable = $derived(!!fichier && !fichier.video && (fichier.affichable || !!apercu));

  async function copierImage() {
    if (!fichier || !copiable) return;
    try {
      await copierImageLocale(fichier.chemin, quarts);
      notifier(t('Image copiée'), 'succes');
    } catch (e) {
      notifier(t('Copie impossible : {e}', { e: String(e) }), 'erreur');
    }
  }

  async function copierChemin() {
    if (!fichier) return;
    try {
      await copierTexte(fichier.chemin);
      notifier(t('Chemin copié'), 'succes');
    } catch (e) {
      notifier(t('Copie impossible : {e}', { e: String(e) }), 'erreur');
    }
  }

  const action = (faire: () => Promise<void>) => () => void faire().catch((e) => notifier(String(e), 'erreur'));

  /** To the system trash, then the next photo; « Annuler » puts it back. */
  async function corbeille() {
    if (!fichier) return;
    const courant = fichier;
    lecture = false;
    try {
      await enregistrerRotation();
      await corbeilleLocale(courant.chemin);
    } catch (e) {
      notifier(t('Corbeille impossible : {e}', { e: String(e) }), 'erreur');
      return;
    }
    notifier(t('« {nom} » mise à la corbeille', { nom: courant.nom }), 'info', {
      libelle: t('Annuler'),
      executer: async () => {
        await restaurerCorbeilleLocale(courant.chemin);
        ouvrir(courant.chemin);
      },
    });
    const restants = courant.voisins.filter((v) => v !== courant.chemin);
    if (!restants.length) {
      dossierVide = true;
      return;
    }
    ouvrir(restants[Math.min(courant.index, restants.length - 1)]);
  }

  // ---- Display ----

  function pivoter(sens: 1 | -1) {
    if (!fichier || fichier.video) return;
    quarts = (quarts + sens + 4) % 4;
    aPivoter = (aPivoter + sens + 4) % 4;
    zoom.reinitialiser();
    clearTimeout(minuteurRotation);
    minuteurRotation = setTimeout(() => void enregistrerRotation(), DELAI_ROTATION);
  }

  /**
   * The turns asked since the last write, written in the file and on Immich
   * (several turns in a row: one write). Taken at once, so another photo
   * opened meanwhile gets nothing of them; the writes follow each other.
   */
  function enregistrerRotation(): Promise<void> {
    clearTimeout(minuteurRotation);
    const courant = fichier;
    const delta = aPivoter;
    aPivoter = 0;
    if (!courant || !delta) return rotations;
    rotations = rotations.then(() => ecrireRotation(courant, delta));
    return rotations;
  }

  async function ecrireRotation(courant: FichierOuvert, delta: number) {
    try {
      const r = await pivoterFichierLocal(courant.chemin, delta);
      if (r.erreurImmich) notifier(t('Pivotée ici, mais pas sur Immich : {e}', { e: r.erreurImmich }), 'erreur');
      if (!r.fichier) return;
      void oublierVignette(courant.chemin);
      if (fichier?.chemin !== courant.chemin) return;
      // The file holds the turn now: shown as it is written, decoded first
      // so the change does not show.
      const prochaine = new Image();
      prochaine.src = urlPhoto(courant.chemin, ouverture + 1);
      await prochaine.decode().catch(() => {});
      if (fichier?.chemin !== courant.chemin) return;
      if (delta % 2 && naturel) naturel = { l: naturel.h, h: naturel.l };
      quarts = (quarts - delta + 4) % 4;
      ouverture++;
      void vignetteLocale(courant.chemin).then((v) => fichier?.chemin === courant.chemin && (vignetteCourante = v));
    } catch (e) {
      notifier(t('Rotation non enregistrée : {e}', { e: String(e) }), 'erreur');
    }
  }

  /** The window itself in full screen (no web page full screen: a key or a click of any kind may ask for it). */
  async function basculerPleinEcran() {
    const fenetre = getCurrentWindow();
    await fenetre.setFullscreen(!(await fenetre.isFullscreen()));
    pleinEcran = await fenetre.isFullscreen();
    reveiller();
  }

  function reveiller() {
    actif = true;
    clearTimeout(minuteurActif);
    if (pleinEcran) minuteurActif = setTimeout(() => (actif = false), 2500);
  }

  // Slideshow: photos advance every 5 s once shown, videos at their end.
  $effect(() => {
    if (!lecture || !fichier || (fichier.video && !videoIlisible) || (!naturel && fichier.affichable)) return;
    const minuteur = setTimeout(() => aller(1), DUREE_DIAPO);
    return () => clearTimeout(minuteur);
  });

  onMount(() => {
    // Also when the system leaves full screen by itself.
    const fenetre = getCurrentWindow();
    const arret = fenetre.onResized(async () => {
      const etat = await fenetre.isFullscreen();
      if (etat !== pleinEcran) {
        pleinEcran = etat;
        reveiller();
      }
    });
    return () => {
      void enregistrerRotation();
      void arret.then((f) => f());
      // Leaving the viewer (library, home) leaves full screen too.
      void fenetre.isFullscreen().then((p) => (p ? fenetre.setFullscreen(false) : undefined));
    };
  });

  function glisserVersAppli(e: DragEvent) {
    e.preventDefault();
    if (!fichier || zoom.zoome) return;
    // A 1-pixel image when no thumbnail exists (videos).
    const icone =
      vignetteCourante ?? 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=';
    void startDrag({ item: [fichier.chemin], icon: icone, mode: 'copy' }).catch((err) =>
      notifier(t('Glisser impossible : {e}', { e: String(err) }), 'erreur'),
    );
  }

  function menu(e: MouseEvent) {
    if (!fichier) return;
    const surImmich = !!etat?.connecte && etat.surServeur;
    const aEnvoyer = !!etat?.connecte && !etat.surServeur && !envoi;
    const image = !fichier.video;
    const items: MenuItems = [
      { title: t('Copier l’image (Ctrl+C)'), icon: mdiContentCopy, onAction: () => void copierImage(), $if: () => copiable },
      { title: t('Copier le chemin'), icon: mdiContentCopy, onAction: () => void copierChemin() },
      MenuItemType.Divider,
      { title: t('Afficher dans le dossier'), icon: mdiFolderOpenOutline, onAction: action(() => afficherDansDossier(fichier!.chemin)) },
      { title: t('Ouvrir avec…'), icon: mdiOpenInApp, onAction: action(() => ouvrirAvec(fichier!.chemin)) },
      MenuItemType.Divider,
      { title: t('Pivoter à droite (R)'), icon: mdiRotateRight, onAction: () => pivoter(1), $if: () => image },
      { title: t('Pivoter à gauche (Maj+R)'), icon: mdiRotateLeft, onAction: () => pivoter(-1), $if: () => image },
      { title: pleinEcran ? t('Quitter le plein écran (F)') : t('Plein écran (F)'), icon: pleinEcran ? mdiFullscreenExit : mdiFullscreen, onAction: () => void basculerPleinEcran() },
      { title: lecture ? t('Arrêter le diaporama (D)') : t('Diaporama (D)'), icon: lecture ? mdiPause : mdiPlay, onAction: () => (lecture = !lecture), $if: () => fichier!.voisins.length > 1 },
      { title: t('Infos (I)'), icon: mdiInformationOutline, onAction: () => (infos = !infos) },
      MenuItemType.Divider,
      { title: t('Envoyer vers Immich'), icon: mdiCloudUploadOutline, onAction: () => void envoyer(), $if: () => aEnvoyer },
      { title: t('Envoyer dans un album…'), icon: mdiImageAlbum, onAction: () => (choixAlbum = true), $if: () => aEnvoyer },
      { title: t('Ouvrir dans la bibliothèque'), icon: mdiOpenInNew, onAction: ouvrirDansLaBibliotheque, $if: () => surImmich && !!etat?.assetId },
      MenuItemType.Divider,
      { title: t('Mettre à la corbeille (Suppr)'), icon: mdiTrashCanOutline, color: 'danger', onAction: () => void corbeille() },
    ];
    ouvrirMenu(e, items);
  }

  function touche(e: KeyboardEvent) {
    // An open menu or dialog keeps its keys (Escape closes it, not the viewer).
    if (choixAlbum || e.altKey || e.target instanceof HTMLInputElement || e.defaultPrevented || menuActif()) return;
    const cle = e.key.toLowerCase();
    if (e.ctrlKey && cle === 'c') {
      if (e.shiftKey) void copierChemin();
      else void copierImage();
    } else if (e.ctrlKey) return;
    else if (e.key === 'ArrowRight') aller(1);
    else if (e.key === 'ArrowLeft') aller(-1);
    else if (e.key === 'Home' && fichier) allerA(0);
    else if (e.key === 'End' && fichier) allerA(fichier.voisins.length - 1);
    else if (cle === 'r') pivoter(e.shiftKey ? -1 : 1);
    else if (cle === 'f' || e.key === 'F11') void basculerPleinEcran();
    else if (cle === 'd') lecture = !lecture;
    else if (cle === 'i') infos = !infos;
    else if (e.key === 'Delete') void corbeille();
    else if ((e.key === '+' || e.key === '=') && naturel) zoom.pas(1);
    else if (e.key === '-' && naturel) zoom.pas(-1);
    else if (e.key === '0') zoom.reinitialiser();
    else if (e.key === '1' && naturel) zoom.zoomerVers(zoom.reel);
    else if (e.key === 'Escape') {
      // Full screen first, then the zoom, the slideshow, the viewer.
      if (pleinEcran) void basculerPleinEcran();
      else if (zoom.zoome) zoom.reinitialiser();
      else if (lecture) lecture = false;
      else if (session.connexion) void goto('/');
      else return;
    } else return;
    e.preventDefault();
  }

  const taille = (o: number) =>
    o >= 1_048_576
      ? t('{taille} Mo', { taille: (o / 1_048_576).toLocaleString(locale(), { maximumFractionDigits: 1 }) })
      : t('{n} ko', { n: Math.ceil(o / 1024) });
</script>

<svelte:window onkeydown={touche} />
<svelte:head><title>{fichier?.nom ?? t('Photo')} · Nicéphore</title></svelte:head>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="visionneuse-locale fixed inset-0 flex flex-col bg-black text-white"
  class:plein-ecran={pleinEcran}
  class:inactif={pleinEcran && !actif}
  onpointermove={reveiller}
>
  <div class="barre flex h-12 shrink-0 items-center gap-2 px-3" data-tauri-drag-region>
    <img src="/icone.svg" alt="" class="pointer-events-none h-6 w-6" />
    {#if fichier}
      <span class="pointer-events-none min-w-0 truncate text-sm font-medium" title={fichier.chemin}>{fichier.nom}</span>
      <span class="pointer-events-none shrink-0 text-xs opacity-60">
        {fichier.index + 1} / {fichier.voisins.length}{compact ? '' : ` · ${taille(fichier.taille)}`}
      </span>
    {/if}
    <span class="h-full flex-1" data-tauri-drag-region></span>
    {#if fichier && !dossierVide}
      <!-- On Immich or not: one icon, its meaning in the tooltip. -->
      {#if !etat}
        <span class="statut" title={t('Vérification sur Immich…')}><LoadingSpinner size="small" /></span>
      {:else if !etat.connecte}
        <span class="statut opacity-60" title={t('Non connecté : connectez-vous à Immich pour savoir si elle y est')}>
          <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true"><path fill="currentColor" d={mdiCloudOffOutline} /></svg>
        </span>
      {:else if etat.surServeur}
        <IconButton
          class="sur-serveur"
          icon={mdiCloudCheckOutline}
          aria-label={etat.dansLaCorbeille ? t('Sur Immich, dans sa corbeille') : t('Déjà sur Immich : l’ouvrir dans la bibliothèque')}
          variant="ghost"
          color="secondary"
          shape="round"
          disabled={etat.dansLaCorbeille || !etat.assetId}
          onclick={ouvrirDansLaBibliotheque}
        />
      {:else}
        <IconButton
          class="absente"
          icon={mdiCloudUploadOutline}
          aria-label={t('Pas encore sur Immich : l’envoyer maintenant')}
          variant="ghost"
          color="secondary"
          shape="round"
          loading={envoi}
          onclick={() => void envoyer()}
        />
      {/if}
      <span class="separateur" aria-hidden="true"></span>
      {#if !compact && !fichier.video}
        <IconButton icon={mdiRotateRight} aria-label={t('Pivoter (R) : à l’affichage, le fichier n’est pas modifié')} variant="ghost" color="secondary" shape="round" onclick={() => pivoter(1)} />
      {/if}
      {#if !compact && fichier.voisins.length > 1}
        <IconButton
          icon={lecture ? mdiPause : mdiPlay}
          aria-label={lecture ? t('Arrêter le diaporama (D)') : t('Diaporama (D)')}
          variant="ghost"
          color="secondary"
          shape="round"
          onclick={() => (lecture = !lecture)}
        />
      {/if}
      <IconButton
        class={bande ? 'allume' : ''}
        icon={mdiViewCarouselOutline}
        aria-label={bande ? t('Masquer les photos du dossier') : t('Afficher les photos du dossier')}
        variant="ghost"
        color="secondary"
        shape="round"
        onclick={basculerBande}
      />
      <IconButton
        class={infos ? 'allume' : ''}
        icon={mdiInformationOutline}
        aria-label={t('Infos (I)')}
        variant="ghost"
        color="secondary"
        shape="round"
        onclick={() => (infos = !infos)}
      />
      <IconButton
        icon={pleinEcran ? mdiFullscreenExit : mdiFullscreen}
        aria-label={pleinEcran ? t('Quitter le plein écran (F)') : t('Plein écran (F)')}
        variant="ghost"
        color="secondary"
        shape="round"
        onclick={() => void basculerPleinEcran()}
      />
      <IconButton icon={mdiDotsVertical} aria-label={t('Plus d’actions')} variant="ghost" color="secondary" shape="round" onclick={menu} />
    {/if}
    {#if !pleinEcran}<BoutonsFenetre />{/if}
  </div>

  <div class="milieu flex min-h-0 flex-1">
    <div class="zone relative min-w-0 flex-1 overflow-hidden" bind:clientWidth={zoneL} bind:clientHeight={zoneH} oncontextmenu={menu}>
      {#if erreur}
        <p class="p-8 text-center text-red-400">{erreur}</p>
      {:else if dossierVide}
        <div class="flex h-full flex-col items-center justify-center gap-2 text-center opacity-80">
          <svg viewBox="0 0 24 24" width="40" height="40" aria-hidden="true"><path fill="currentColor" d={mdiImageMultipleOutline} /></svg>
          <p>{t('Plus aucune photo dans ce dossier.')}</p>
        </div>
      {:else if !fichier}
        <div class="flex h-full items-center justify-center"><LoadingSpinner size="giant" /></div>
      {:else if !fichier.affichable && !apercu}
        <div class="flex h-full flex-col items-center justify-center gap-2 text-center opacity-80">
          <p>{t('Aperçu indisponible pour ce format ({format}).', { format: fichier.nom.split('.').pop()?.toUpperCase() ?? '' })}</p>
          <p class="text-sm opacity-70">{t('Il peut tout de même être envoyé vers Immich, qui sait l’afficher.')}</p>
        </div>
      {:else if fichier.video && videoIlisible}
        <div class="flex h-full flex-col items-center justify-center gap-2 px-6 text-center opacity-80">
          <p>{t('Cette vidéo ne peut pas être lue ici : son format n’est pas pris en charge par le système.')}</p>
          <p class="text-sm opacity-70">{t('Elle peut tout de même être envoyée vers Immich, qui la convertit pour la lecture.')}</p>
        </div>
      {:else if fichier.video && video}
        {#key `${video}|${ouverture}`}
          <LecteurVideo
            src={video}
            classeVideo="video-locale h-full w-full object-contain"
            onloadedmetadata={(v: HTMLVideoElement) => ajuster(v.videoWidth, v.videoHeight)}
            onerror={() => {
              videoIlisible = true;
              void ajuster(0, 0);
            }}
            onended={() => lecture && aller(1)}
          />
        {/key}
      {:else}
        {@const source = !fichier.affichable && apercu ? apercu.chemin : fichier.chemin}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="scene absolute inset-0"
          class:zoomee={zoom.zoome}
          onwheel={(e) => naturel && zoom.molette(e)}
          ondblclick={(e) => naturel && zoom.doubleClic(e)}
          onpointerdown={(e) => zoom.appui(e)}
          onpointermove={(e) => zoom.mouvement(e)}
          onpointerup={() => zoom.relache()}
          onpointercancel={() => zoom.relache()}
        >
          {#key `${source}|${ouverture}`}
            <img
              class="photo"
              class:invisible={!naturel}
              style:width={naturel ? `${naturel.l * ajustement * zoom.echelle}px` : undefined}
              style:height={naturel ? `${naturel.h * ajustement * zoom.echelle}px` : undefined}
              style:transform="translate(-50%, -50%) translate({zoom.x}px, {zoom.y}px) rotate({angle}deg)"
              src={urlPhoto(source, ouverture)}
              alt={fichier.nom}
              draggable={!zoom.zoome}
              ondragstart={glisserVersAppli}
              onload={(e) => {
                const image = e.currentTarget as HTMLImageElement;
                if (fichier?.affichable) {
                  naturel = { l: image.naturalWidth, h: image.naturalHeight };
                  void ajuster(image.naturalWidth, image.naturalHeight);
                }
              }}
              onerror={() => ajuster(0, 0)}
            />
          {/key}
        </div>
        <div class="mentions pointer-events-none absolute bottom-3 left-1/2 flex -translate-x-1/2 gap-2">
          {#if !fichier.affichable && apercu}
            <span class="mention">{t('Aperçu du RAW ({largeur} × {hauteur})', { largeur: apercu.largeur, hauteur: apercu.hauteur })}</span>
          {/if}
          {#if naturel && (indicateur || zoom.zoome)}
            <span class="mention" class:discrete={!indicateur}>{zoom.pourcentage} %</span>
          {/if}
        </div>
      {/if}
      {#if lecture && fichier && !fichier.video && naturel}
        {#key fichier.chemin}
          <div class="progression" style:animation-duration="{DUREE_DIAPO}ms"></div>
        {/key}
      {/if}
      {#if fichier && !dossierVide && fichier.voisins.length > 1}
        <button class="fleche left-2" aria-label={t('Précédente')} title={t('Précédente')} onclick={() => aller(-1)}>
          <svg viewBox="0 0 24 24" width="32" height="32" aria-hidden="true"><path fill="currentColor" d={mdiChevronLeft} /></svg>
        </button>
        <button class="fleche right-2" aria-label={t('Suivante')} title={t('Suivante')} onclick={() => aller(1)}>
          <svg viewBox="0 0 24 24" width="32" height="32" aria-hidden="true"><path fill="currentColor" d={mdiChevronRight} /></svg>
        </button>
      {/if}
    </div>
    {#if infos && fichier && !dossierVide}
      <PanneauInfosLocal {fichier} dimensions={naturel && (tourne ? { l: naturel.h, h: naturel.l } : naturel)} />
    {/if}
  </div>

  {#if bande && fichier && !dossierVide && fichier.voisins.length > 1}
    <div class="bas shrink-0" style:height="{HAUTEUR_BANDE}px">
      <BandeVignettes voisins={fichier.voisins} index={fichier.index} onchoisir={allerA} />
    </div>
  {/if}
</div>

{#if choixAlbum}
  <ChoixAlbum
    nombre={1}
    onchoisir={(a) => {
      choixAlbum = false;
      void envoyer({ existant: a });
    }}
    oncreer={(nom) => {
      choixAlbum = false;
      void envoyer({ nouveau: nom });
    }}
    onfermer={() => (choixAlbum = false)}
  />
{/if}

<style>
  .barre,
  .bas {
    background: rgb(0 0 0 / 0.6);
    transition: opacity 200ms;
  }
  /* Full screen: bar and strip float over the photo and fade when idle. */
  .plein-ecran .barre,
  .plein-ecran .bas {
    position: absolute;
    left: 0;
    right: 0;
    z-index: 3;
  }
  .plein-ecran .barre {
    top: 0;
  }
  .plein-ecran .bas {
    bottom: 0;
  }
  .inactif,
  .inactif :global(*) {
    cursor: none !important;
  }
  .inactif .barre,
  .inactif .bas,
  .inactif .fleche {
    opacity: 0 !important;
    pointer-events: none;
  }
  .statut {
    display: inline-flex;
    width: 36px;
    height: 36px;
    flex-shrink: 0;
    align-items: center;
    justify-content: center;
  }
  .barre :global(.sur-serveur) {
    color: #6ee7a8;
  }
  .barre :global(.absente) {
    color: #fbbf24;
  }
  .barre :global(.allume) {
    background: rgb(255 255 255 / 0.14);
  }
  .separateur {
    width: 1px;
    height: 20px;
    margin: 0 4px;
    background: rgb(255 255 255 / 0.18);
  }
  .scene {
    cursor: zoom-in;
    touch-action: none;
  }
  .scene.zoomee {
    cursor: grab;
  }
  .scene.zoomee:active {
    cursor: grabbing;
  }
  .photo {
    position: absolute;
    left: 50%;
    top: 50%;
    max-width: none;
    user-select: none;
    -webkit-user-drag: element;
  }
  .mention {
    border-radius: 9999px;
    padding: 4px 12px;
    font-size: 12px;
    background: rgb(0 0 0 / 0.55);
    transition: opacity 300ms;
  }
  .mention.discrete {
    opacity: 0.55;
  }
  .progression {
    position: absolute;
    left: 0;
    top: 0;
    z-index: 2;
    height: 3px;
    background: white;
    animation-name: avance;
    animation-timing-function: linear;
    animation-fill-mode: forwards;
  }
  @keyframes avance {
    from {
      width: 0;
    }
    to {
      width: 100%;
    }
  }
  .fleche {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    width: 52px;
    height: 52px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 9999px;
    background: rgb(0 0 0 / 0.35);
    opacity: 0;
    transition: opacity 150ms;
  }
  /* Arrows only while the pointer is over the photo (or on keyboard focus). */
  .zone:hover .fleche,
  .fleche:focus-visible {
    opacity: 0.7;
  }
  .zone .fleche:hover {
    opacity: 1;
  }
</style>
