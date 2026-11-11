<script lang="ts">
  // A photo or video of this computer (Nicéphore as the default viewer):
  // the file and its neighbours in the folder, and whether it is on Immich
  // (cloud badge, like the mobile app) with a button to send it now.
  import { Button, IconButton, LoadingSpinner } from '@immich/ui';
  import { mdiChevronLeft, mdiChevronRight, mdiCloudCheckOutline, mdiCloudOffOutline, mdiCloudUploadOutline } from '@mdi/js';
  import { convertFileSrc, invoke } from '@tauri-apps/api/core';
  import {
    availableMonitors,
    currentMonitor,
    getCurrentWindow,
    LogicalPosition,
    LogicalSize,
    primaryMonitor,
  } from '@tauri-apps/api/window';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import {
    apercuLocal,
    urlVideoLocale,
    envoyerFichierLocal,
    etatFichierLocal,
    ouvrirFichierLocal,
    type ApercuLocal,
    type EtatLocal,
    type FichierOuvert,
  } from '../../lib/api';
  import BoutonsFenetre from '../../lib/BoutonsFenetre.svelte';
  import LecteurVideo from '../../lib/LecteurVideo.svelte';
  import { locale, t } from '../../lib/i18n.svelte';
  import { notifier } from '../../lib/notifications.svelte';
  import { session } from '../../lib/session.svelte';

  const chemin = $derived(page.url.searchParams.get('chemin') ?? '');
  let fichier = $state<FichierOuvert | null>(null);
  let erreur = $state<string | null>(null);
  let etat = $state<EtatLocal | null>(null);
  let envoi = $state(false);
  let apercu = $state<ApercuLocal | null>(null);
  let video = $state<string | null>(null);
  let videoIlisible = $state(false);
  let zoneL = $state(0);
  let zoneH = $state(0);
  /** Opened from outside (file manager): the window takes the photo's size. */
  let ajusterFenetre = false;

  const BARRE = 48;
  /** Narrow window (portrait photo): icons only in the bar. */
  const compact = $derived(zoneL > 0 && zoneL < 760);

  /** Window sized to the photo (within the screen), centred, then shown. */
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
        const maxL = (ecran.size.width / ecran.scaleFactor) * 0.9;
        const maxH = (ecran.size.height / ecran.scaleFactor) * 0.85 - BARRE;
        const r = Math.min(1, maxL / largeur, maxH / hauteur);
        const l = Math.max(480, Math.round(largeur * r));
        const h = Math.max(360, Math.round(hauteur * r) + BARRE);
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
        await fenetre.show();
        await fenetre.setFocus();
      }
    }
  }

  /** CSS rotation of a RAW preview from its EXIF orientation. */
  const rotation = (o: number) => (o === 3 ? 180 : o === 6 ? 90 : o === 8 ? 270 : 0);

  $effect(() => {
    const demande = chemin;
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
        etat = null;
        apercu = a;
        video = url;
        videoIlisible = f.video && !url;
        if (a) {
          const tourne = rotation(a.orientation) % 180 !== 0;
          void ajuster(tourne ? a.hauteur : a.largeur, tourne ? a.largeur : a.hauteur);
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

  function aller(sens: 1 | -1) {
    if (!fichier || fichier.voisins.length < 2) return;
    const i = (fichier.index + sens + fichier.voisins.length) % fichier.voisins.length;
    void goto(`/fichier?chemin=${encodeURIComponent(fichier.voisins[i])}`, { replaceState: true });
  }

  async function envoyer() {
    if (!fichier) return;
    envoi = true;
    try {
      await envoyerFichierLocal(fichier.chemin);
      notifier(t('« {nom} » envoyée vers Immich', { nom: fichier.nom }), 'succes');
      etat = await etatFichierLocal(fichier.chemin);
    } catch (e) {
      notifier(t('Envoi impossible : {e}', { e: String(e) }), 'erreur');
    } finally {
      envoi = false;
    }
  }

  function touche(e: KeyboardEvent) {
    if (e.key === 'ArrowRight') aller(1);
    else if (e.key === 'ArrowLeft') aller(-1);
    else if (e.key === 'Escape' && session.connexion) void goto('/');
  }

  const taille = (o: number) =>
    o >= 1_048_576
      ? t('{taille} Mo', { taille: (o / 1_048_576).toLocaleString(locale(), { maximumFractionDigits: 1 }) })
      : t('{n} ko', { n: Math.ceil(o / 1024) });
</script>

<svelte:window onkeydown={touche} />
<svelte:head><title>{fichier?.nom ?? t('Photo')} · Nicéphore</title></svelte:head>

<div class="visionneuse-locale fixed inset-0 flex flex-col bg-black text-white">
  <div class="barre flex h-12 shrink-0 items-center gap-3 px-3" data-tauri-drag-region>
    <img src="/icone.svg" alt="" class="pointer-events-none h-6 w-6" />
    {#if fichier}
      <span class="pointer-events-none min-w-0 truncate text-sm font-medium" title={fichier.chemin}>{fichier.nom}</span>
      <span class="pointer-events-none shrink-0 text-xs opacity-60">
        {fichier.index + 1} / {fichier.voisins.length}{compact ? '' : ` · ${taille(fichier.taille)}`}
      </span>
    {/if}
    <span class="h-full flex-1" data-tauri-drag-region></span>
    {#if fichier}
      {#snippet statut(classe: string, icone: string, libelle: string, aide: string)}
        <span class="statut {classe}" title={aide} aria-label={compact ? libelle : undefined}>
          <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true"><path fill="currentColor" d={icone} /></svg>
          {#if !compact}{libelle}{/if}
        </span>
      {/snippet}
      {#if !etat}
        <span class="statut"><LoadingSpinner size="small" />{#if !compact} {t('Vérification…')}{/if}</span>
      {:else if !etat.connecte}
        {@render statut('opacity-70', mdiCloudOffOutline, t('Non connecté'), t('Connectez-vous à Immich pour savoir si elle y est'))}
      {:else if etat.surServeur}
        {@render statut(
          'sur-serveur',
          mdiCloudCheckOutline,
          etat.dansLaCorbeille ? t('Dans la corbeille d’Immich') : t('Sur Immich'),
          etat.dansLaCorbeille ? t('Sur Immich, dans sa corbeille') : t('Déjà sur Immich'),
        )}
      {:else}
        {@render statut('absente', mdiCloudOffOutline, t('Pas sur Immich'), t('Cette photo n’est pas encore sur Immich'))}
        {#if compact}
          <IconButton icon={mdiCloudUploadOutline} aria-label={t('Envoyer maintenant')} title={t('Envoyer maintenant')} size="small" shape="round" disabled={envoi} onclick={envoyer} />
        {:else}
          <Button size="small" leadingIcon={mdiCloudUploadOutline} loading={envoi} onclick={envoyer}>{t('Envoyer maintenant')}</Button>
        {/if}
      {/if}
    {/if}
    <BoutonsFenetre />
  </div>

  <div class="zone relative min-h-0 flex-1 overflow-hidden" bind:clientWidth={zoneL} bind:clientHeight={zoneH}>
    {#if erreur}
      <p class="p-8 text-center text-red-400">{erreur}</p>
    {:else if !fichier}
      <div class="flex h-full items-center justify-center"><LoadingSpinner size="giant" /></div>
    {:else if !fichier.affichable && apercu}
      {@const tourne = rotation(apercu.orientation) % 180 !== 0}
      {#key apercu.chemin}
        <img
          class="image apercu-raw absolute left-1/2 top-1/2 object-contain"
          style:width="{tourne ? zoneH : zoneL}px"
          style:height="{tourne ? zoneL : zoneH}px"
          style:transform="translate(-50%, -50%) rotate({rotation(apercu.orientation)}deg)"
          src={convertFileSrc(apercu.chemin)}
          alt={fichier.nom}
        />
      {/key}
      <span class="mention absolute bottom-3 left-1/2 -translate-x-1/2 rounded-full px-3 py-1 text-xs">
        {t('Aperçu du RAW ({largeur} × {hauteur})', { largeur: apercu.largeur, hauteur: apercu.hauteur })}
      </span>
    {:else if !fichier.affichable}
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
      {#key video}
        <LecteurVideo
          src={video}
          classeVideo="video-locale h-full w-full object-contain"
          onloadedmetadata={(v: HTMLVideoElement) => ajuster(v.videoWidth, v.videoHeight)}
          onerror={() => {
            videoIlisible = true;
            void ajuster(0, 0);
          }}
        />
      {/key}
    {:else}
      {#key fichier.chemin}
        <img
          class="image h-full w-full object-contain"
          src={convertFileSrc(fichier.chemin)}
          alt={fichier.nom}
          onload={(e) => {
            const image = e.currentTarget as HTMLImageElement;
            void ajuster(image.naturalWidth, image.naturalHeight);
          }}
          onerror={() => ajuster(0, 0)}
        />
      {/key}
    {/if}
    {#if fichier && fichier.voisins.length > 1}
      <button class="fleche left-2" aria-label={t('Précédente')} onclick={() => aller(-1)}>
        <svg viewBox="0 0 24 24" width="32" height="32" aria-hidden="true"><path fill="currentColor" d={mdiChevronLeft} /></svg>
      </button>
      <button class="fleche right-2" aria-label={t('Suivante')} onclick={() => aller(1)}>
        <svg viewBox="0 0 24 24" width="32" height="32" aria-hidden="true"><path fill="currentColor" d={mdiChevronRight} /></svg>
      </button>
    {/if}
  </div>
</div>

<style>
  .barre {
    background: rgb(0 0 0 / 0.6);
  }
  .statut {
    display: inline-flex;
    flex-shrink: 0;
    white-space: nowrap;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    padding: 4px 10px;
    border-radius: 9999px;
    background: rgb(255 255 255 / 0.08);
  }
  .sur-serveur {
    color: #6ee7a8;
  }
  .absente {
    color: #fbbf24;
  }
  .mention {
    background: rgb(0 0 0 / 0.55);
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
