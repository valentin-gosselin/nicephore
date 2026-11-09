<script lang="ts">
  // A photo or video of this computer (Nicéphore as the default viewer):
  // the file and its neighbours in the folder, and whether it is on Immich
  // (cloud badge, like the mobile app) with a button to send it now.
  import { Button, IconButton, LoadingSpinner } from '@immich/ui';
  import { mdiChevronLeft, mdiChevronRight, mdiCloudCheckOutline, mdiCloudOffOutline, mdiCloudUploadOutline } from '@mdi/js';
  import { convertFileSrc, invoke } from '@tauri-apps/api/core';
  import { availableMonitors, currentMonitor, getCurrentWindow, LogicalSize, primaryMonitor } from '@tauri-apps/api/window';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import {
    apercuLocal,
    envoyerFichierLocal,
    etatFichierLocal,
    ouvrirFichierLocal,
    type ApercuLocal,
    type EtatLocal,
    type FichierOuvert,
  } from '../../lib/api';
  import BoutonsFenetre from '../../lib/BoutonsFenetre.svelte';
  import { notifier } from '../../lib/notifications.svelte';
  import { session } from '../../lib/session.svelte';

  const chemin = $derived(page.url.searchParams.get('chemin') ?? '');
  let fichier = $state<FichierOuvert | null>(null);
  let erreur = $state<string | null>(null);
  let etat = $state<EtatLocal | null>(null);
  let envoi = $state(false);
  let apercu = $state<ApercuLocal | null>(null);
  let zoneL = $state(0);
  let zoneH = $state(0);
  /** Opened from outside (file manager): the window takes the photo's size. */
  let ajusterFenetre = false;

  const BARRE = 48;
  /** Narrow window (portrait photo): icons only in the bar. */
  const compact = $derived(zoneL > 0 && zoneL < 760);

  /** Window sized to the photo (within the screen), centred, then shown. */
  async function ajuster(largeur: number, hauteur: number) {
    if (!ajusterFenetre) return;
    ajusterFenetre = false;
    const fenetre = getCurrentWindow();
    try {
      // A hidden window has no screen yet (Wayland): the primary one then.
      const ecran = (await currentMonitor()) ?? (await primaryMonitor()) ?? (await availableMonitors())[0];
      if (ecran && largeur > 0 && hauteur > 0 && !(await fenetre.isMaximized()) && !(await fenetre.isFullscreen())) {
        const maxL = (ecran.size.width / ecran.scaleFactor) * 0.9;
        const maxH = (ecran.size.height / ecran.scaleFactor) * 0.85 - BARRE;
        const r = Math.min(1, maxL / largeur, maxH / hauteur);
        await fenetre.setSize(new LogicalSize(Math.max(480, Math.round(largeur * r)), Math.max(360, Math.round(hauteur * r) + BARRE)));
        await fenetre.center();
      }
    } catch (e) {
      await invoke('journal', { message: `taille de fenêtre : ${e}` }).catch(() => {});
    } finally {
      await fenetre.show();
      await fenetre.setFocus();
    }
  }

  /** CSS rotation of a RAW preview from its EXIF orientation. */
  const rotation = (o: number) => (o === 3 ? 180 : o === 6 ? 90 : o === 8 ? 270 : 0);

  $effect(() => {
    const demande = chemin;
    // Each opening from outside asks again; the arrows never do.
    ajusterFenetre = page.url.searchParams.get('ajuster') === '1';
    fichier = null;
    erreur = null;
    etat = null;
    if (!demande) return;
    apercu = null;
    ouvrirFichierLocal(demande)
      .then(async (f) => {
        if (demande !== chemin) return;
        fichier = f;
        if (f.raw) {
          const a = await apercuLocal(demande).catch(() => null);
          if (demande !== chemin) return;
          apercu = a;
          if (a) {
            const tourne = rotation(a.orientation) % 180 !== 0;
            void ajuster(tourne ? a.hauteur : a.largeur, tourne ? a.largeur : a.hauteur);
          }
        }
        if (!f.affichable && !apercu) void ajuster(0, 0);
        return etatFichierLocal(demande).then((e) => demande === chemin && (etat = e));
      })
      .catch((e) => {
        erreur = String(e);
        void ajuster(0, 0);
      });
  });

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
      notifier(`« ${fichier.nom} » envoyée vers Immich`, 'succes');
      etat = await etatFichierLocal(fichier.chemin);
    } catch (e) {
      notifier(`Envoi impossible : ${e}`, 'erreur');
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
    o >= 1_048_576 ? `${(o / 1_048_576).toLocaleString('fr-FR', { maximumFractionDigits: 1 })} Mo` : `${Math.ceil(o / 1024)} ko`;
</script>

<svelte:window onkeydown={touche} />
<svelte:head><title>{fichier?.nom ?? 'Photo'} · Nicéphore</title></svelte:head>

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
        <span class="statut"><LoadingSpinner size="small" />{#if !compact} Vérification…{/if}</span>
      {:else if !etat.connecte}
        {@render statut('opacity-70', mdiCloudOffOutline, 'Non connecté', 'Connectez-vous à Immich pour savoir si elle y est')}
      {:else if etat.surServeur}
        {@render statut(
          'sur-serveur',
          mdiCloudCheckOutline,
          etat.dansLaCorbeille ? 'Dans la corbeille d’Immich' : 'Sur Immich',
          etat.dansLaCorbeille ? 'Sur Immich, dans sa corbeille' : 'Déjà sur Immich',
        )}
      {:else}
        {@render statut('absente', mdiCloudOffOutline, 'Pas sur Immich', 'Cette photo n’est pas encore sur Immich')}
        {#if compact}
          <IconButton icon={mdiCloudUploadOutline} aria-label="Envoyer maintenant" title="Envoyer maintenant" size="small" shape="round" disabled={envoi} onclick={envoyer} />
        {:else}
          <Button size="small" leadingIcon={mdiCloudUploadOutline} loading={envoi} onclick={envoyer}>Envoyer maintenant</Button>
        {/if}
      {/if}
    {/if}
    <BoutonsFenetre />
  </div>

  <div class="relative min-h-0 flex-1 overflow-hidden" bind:clientWidth={zoneL} bind:clientHeight={zoneH}>
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
        Aperçu du RAW ({apercu.largeur} × {apercu.hauteur})
      </span>
    {:else if !fichier.affichable}
      <div class="flex h-full flex-col items-center justify-center gap-2 text-center opacity-80">
        <p>Aperçu indisponible pour ce format ({fichier.nom.split('.').pop()?.toUpperCase()}).</p>
        <p class="text-sm opacity-70">Il peut tout de même être envoyé vers Immich, qui sait l’afficher.</p>
      </div>
    {:else if fichier.video}
      {#key fichier.chemin}
        <!-- svelte-ignore a11y_media_has_caption -->
        <video
          class="h-full w-full object-contain"
          src={convertFileSrc(fichier.chemin)}
          controls
          autoplay
          onloadedmetadata={(e) => ajuster(e.currentTarget.videoWidth, e.currentTarget.videoHeight)}
          onerror={() => ajuster(0, 0)}
        ></video>
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
      <button class="fleche left-2" aria-label="Précédente" onclick={() => aller(-1)}>
        <svg viewBox="0 0 24 24" width="32" height="32" aria-hidden="true"><path fill="currentColor" d={mdiChevronLeft} /></svg>
      </button>
      <button class="fleche right-2" aria-label="Suivante" onclick={() => aller(1)}>
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
    opacity: 0.7;
  }
  .fleche:hover {
    opacity: 1;
  }
</style>
