<script lang="ts">
  // Upload by drag and drop, like Immich web: files and folders dropped
  // anywhere on the window are sent to Immich. The window dims under the
  // drag, a dropped folder asks for its album (except on an album page,
  // which receives them), and a panel in the corner follows the upload.
  import { Icon, IconButton } from '@immich/ui';
  import { mdiAlertCircleOutline, mdiCheckCircleOutline, mdiClose, mdiTrayArrowUp } from '@mdi/js';
  import { getCurrentWebview } from '@tauri-apps/api/webview';
  import { onMount } from 'svelte';
  import { annulerEnvois, effacerEnvois } from './api';
  import ChoixAlbum from './ChoixAlbum.svelte';
  import { albumAffiche, envois, lancerEnvoi, proposerEnvoi, suivreEnvois } from './envois.svelte';
  import { t, tp } from './i18n.svelte';

  let { actif }: { actif: boolean } = $props();

  const etat = $derived(envois.etat);
  /** The panel shows from the first lot until closed. */
  const visible = $derived(!!etat && etat.total > 0);
  const progression = $derived(
    etat ? (etat.octetsTotal > 0 ? etat.octetsFaits / etat.octetsTotal : etat.total > 0 ? etat.traites / etat.total : 0) : 0,
  );
  let erreursOuvertes = $state(false);
  const versAlbum = $derived(envois.survol && !!albumAffiche());

  onMount(() => {
    suivreEnvois();
    const arret = getCurrentWebview().onDragDropEvent((e) => {
      if (!actif) {
        envois.survol = false;
        return;
      }
      const type = e.payload.type;
      if (type === 'enter') envois.survol = e.payload.paths.length > 0;
      else if (type === 'leave') envois.survol = false;
      else if (type === 'drop') {
        envois.survol = false;
        void proposerEnvoi(e.payload.paths);
      }
    });
    return () => void arret.then((f) => f());
  });

  function fermer() {
    erreursOuvertes = false;
    void effacerEnvois();
  }
</script>

{#if envois.survol}
  <div class="depot pointer-events-none fixed inset-0 z-[80] flex items-center justify-center p-6" aria-live="polite">
    <div class="cadre flex flex-col items-center gap-3 rounded-3xl px-12 py-10 text-center">
      <Icon icon={mdiTrayArrowUp} size="56" />
      <p class="text-xl font-semibold">{t('Déposez les fichiers n’importe où pour envoyer')}</p>
      <p class="text-sm opacity-80">{versAlbum ? t('Ils iront aussi dans cet album.') : t('Les dossiers sont envoyés avec tout leur contenu.')}</p>
    </div>
  </div>
{/if}

{#if envois.demande}
  {@const demande = envois.demande}
  <ChoixAlbum
    nombre={0}
    titre={demande.dossiers > 1 ? t('Où ranger les photos de ces {n} dossiers ?', { n: demande.dossiers }) : t('Où ranger les photos de ce dossier ?')}
    nomPropose={demande.nomPropose}
    onbibliotheque={() => void lancerEnvoi(demande.chemins, null)}
    onchoisir={(a) => void lancerEnvoi(demande.chemins, { type: 'existant', id: a.id, nom: a.albumName })}
    oncreer={(nom) => void lancerEnvoi(demande.chemins, { type: 'nouveau', nom })}
    onfermer={() => (envois.demande = null)}
  />
{/if}

{#if visible && etat}
  <section class="panneau-envoi fixed bottom-4 right-4 z-[60] w-[22rem] rounded-2xl p-4 text-sm shadow-xl" aria-label={t('Envois')}>
    <div class="flex items-center gap-2">
      {#if etat.actif}
        <Icon icon={mdiTrayArrowUp} size="20" class="text-primary" />
        <h2 class="flex-1 font-semibold">{t('Envoi')}</h2>
      {:else if etat.nombreErreurs}
        <Icon icon={mdiAlertCircleOutline} size="20" class="text-danger" />
        <h2 class="flex-1 font-semibold">{t('Envoi fini')}</h2>
      {:else}
        <Icon icon={mdiCheckCircleOutline} size="20" class="text-success" />
        <h2 class="flex-1 font-semibold">{t('Envoi fini')}</h2>
      {/if}
      <span class="tabular-nums opacity-70">{t('{traites} / {total}', { traites: etat.traites, total: etat.total })}</span>
      {#if etat.actif}
        <button class="annuler rounded-full px-2 py-0.5 text-xs font-medium" onclick={() => void annulerEnvois()}>{t('Annuler')}</button>
      {:else}
        <IconButton icon={mdiClose} aria-label={t('Fermer')} variant="ghost" color="secondary" shape="round" size="small" onclick={fermer} />
      {/if}
    </div>
    <div class="barre mt-3 h-1.5 overflow-hidden rounded-full">
      <div class="h-full rounded-full bg-primary transition-[width] duration-200" style:width="{Math.round(progression * 100)}%"></div>
    </div>
    <dl class="mt-3 grid grid-cols-3 gap-2 text-center">
      <div><dt>{t('Envoyé')}</dt><dd class="envoyes">{etat.envoyes}</dd></div>
      <div><dt>{t('Doublons')}</dt><dd class="doublons">{etat.doublons}</dd></div>
      <div><dt>{t('Erreurs')}</dt><dd class="erreurs" class:text-danger={etat.nombreErreurs > 0}>{etat.nombreErreurs}</dd></div>
    </dl>
    {#if etat.albums.length}
      <p class="mt-3 truncate opacity-80">
        {tp(etat.albums.length, 'Dans l’album « {albums} »', 'Dans les albums « {albums} »', { albums: [...new Set(etat.albums)].join(' », « ') })}
      </p>
    {/if}
    {#if etat.actif && etat.fichier}
      <p class="mt-1 truncate opacity-60">{etat.fichier}</p>
    {:else if etat.annule}
      <p class="mt-1 opacity-70">{t('Envoi annulé : les photos déjà envoyées restent sur Immich.')}</p>
    {/if}
    {#if etat.erreurs.length}
      <button class="mt-2 text-xs text-primary hover:underline" onclick={() => (erreursOuvertes = !erreursOuvertes)}>
        {erreursOuvertes ? t('Masquer les erreurs') : t('Voir les erreurs')}
      </button>
      {#if erreursOuvertes}
        <ul class="mt-2 max-h-40 space-y-1 overflow-y-auto text-xs">
          {#each etat.erreurs as e, i (i)}
            <li><span class="font-medium">{e.nom || t('Envoi')}</span> : <span class="opacity-75">{e.message}</span></li>
          {/each}
        </ul>
      {/if}
    {/if}
  </section>
{/if}

<style>
  .depot {
    background: rgb(0 0 0 / 0.45);
    backdrop-filter: blur(2px);
  }
  .cadre {
    border: 2px dashed rgb(255 255 255 / 0.8);
    background: color-mix(in oklab, var(--color-primary, rgb(66 80 175)) 85%, transparent);
    color: white;
  }
  .panneau-envoi {
    background: var(--immich-ui-light, white);
    color: var(--immich-ui-dark, black);
    border: 1px solid rgb(127 127 127 / 0.2);
  }
  :global(.dark) .panneau-envoi {
    background: #1c1f24;
    color: #e8e8e8;
  }
  .barre {
    background: rgb(127 127 127 / 0.2);
  }
  dt {
    font-size: 11px;
    opacity: 0.6;
  }
  dd {
    font-size: 18px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .annuler {
    background: rgb(127 127 127 / 0.15);
  }
  .annuler:hover {
    background: rgb(127 127 127 / 0.28);
  }
</style>
