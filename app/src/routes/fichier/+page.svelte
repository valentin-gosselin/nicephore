<script lang="ts">
  // A photo or video of this computer (Nicéphore as the default viewer):
  // the file and its neighbours in the folder, and whether it is on Immich
  // (cloud badge, like the mobile app) with a button to send it now.
  import { Button, LoadingSpinner } from '@immich/ui';
  import { mdiChevronLeft, mdiChevronRight, mdiCloudCheckOutline, mdiCloudOffOutline, mdiCloudUploadOutline } from '@mdi/js';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import {
    envoyerFichierLocal,
    etatFichierLocal,
    ouvrirFichierLocal,
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

  $effect(() => {
    const demande = chemin;
    fichier = null;
    erreur = null;
    etat = null;
    if (!demande) return;
    ouvrirFichierLocal(demande)
      .then((f) => {
        if (demande !== chemin) return;
        fichier = f;
        return etatFichierLocal(demande).then((e) => demande === chemin && (etat = e));
      })
      .catch((e) => (erreur = String(e)));
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
        {fichier.index + 1} / {fichier.voisins.length} · {taille(fichier.taille)}
      </span>
    {/if}
    <span class="h-full flex-1" data-tauri-drag-region></span>
    {#if fichier}
      {#if !etat}
        <span class="statut"><LoadingSpinner size="small" /> Vérification…</span>
      {:else if !etat.connecte}
        <span class="statut opacity-70" title="Connectez-vous à Immich pour savoir si elle y est">
          <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true"><path fill="currentColor" d={mdiCloudOffOutline} /></svg>
          Non connecté
        </span>
      {:else if etat.surServeur}
        <span class="statut sur-serveur" title={etat.dansLaCorbeille ? 'Sur Immich, dans sa corbeille' : 'Déjà sur Immich'}>
          <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true"><path fill="currentColor" d={mdiCloudCheckOutline} /></svg>
          {etat.dansLaCorbeille ? 'Dans la corbeille d’Immich' : 'Sur Immich'}
        </span>
      {:else}
        <span class="statut absente" title="Cette photo n’est pas encore sur Immich">
          <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true"><path fill="currentColor" d={mdiCloudOffOutline} /></svg>
          Pas sur Immich
        </span>
        <Button size="small" leadingIcon={mdiCloudUploadOutline} loading={envoi} onclick={envoyer}>Envoyer maintenant</Button>
      {/if}
    {/if}
    <BoutonsFenetre />
  </div>

  <div class="relative min-h-0 flex-1">
    {#if erreur}
      <p class="p-8 text-center text-red-400">{erreur}</p>
    {:else if !fichier}
      <div class="flex h-full items-center justify-center"><LoadingSpinner size="giant" /></div>
    {:else if !fichier.affichable}
      <div class="flex h-full flex-col items-center justify-center gap-2 text-center opacity-80">
        <p>Aperçu indisponible pour ce format ({fichier.nom.split('.').pop()?.toUpperCase()}).</p>
        <p class="text-sm opacity-70">Il peut tout de même être envoyé vers Immich, qui sait l’afficher.</p>
      </div>
    {:else if fichier.video}
      {#key fichier.chemin}
        <!-- svelte-ignore a11y_media_has_caption -->
        <video class="h-full w-full object-contain" src={convertFileSrc(fichier.chemin)} controls autoplay></video>
      {/key}
    {:else}
      {#key fichier.chemin}
        <img class="image h-full w-full object-contain" src={convertFileSrc(fichier.chemin)} alt={fichier.nom} />
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
