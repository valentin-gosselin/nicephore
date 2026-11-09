<script lang="ts">
  // Sidebar: the timeline views, the library and this computer's uploads
  // (settings are in the account menu, top right).
  import { Icon } from '@immich/ui';
  import {
    mdiAccountMultipleOutline,
    mdiArchiveArrowDownOutline,
    mdiCompassOutline,
    mdiHeartOutline,
    mdiImageAlbum,
    mdiImageMultipleOutline,
    mdiMapOutline,
    mdiSync,
    mdiTrashCanOutline,
  } from '@mdi/js';
  import { page } from '$app/state';
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import { etatEnvoi, type EtatEnvoi } from './api';

  let { repliee }: { repliee: boolean } = $props();

  type Entree = { libelle: string; icone: string; chemin?: string; bientot?: string };

  const groupes: { titre?: string; entrees: Entree[] }[] = [
    {
      entrees: [
        { libelle: 'Photos', icone: mdiImageMultipleOutline, chemin: '/' },
        { libelle: 'Explorer', icone: mdiCompassOutline, chemin: '/explorer' },
        { libelle: 'Carte', icone: mdiMapOutline, chemin: '/carte' },
        { libelle: 'Personnes', icone: mdiAccountMultipleOutline, chemin: '/personnes' },
        { libelle: 'Albums', icone: mdiImageAlbum, chemin: '/albums' },
      ],
    },
    {
      titre: 'Bibliothèque',
      entrees: [
        { libelle: 'Favoris', icone: mdiHeartOutline, chemin: '/favoris' },
        { libelle: 'Archive', icone: mdiArchiveArrowDownOutline, chemin: '/archive' },
        { libelle: 'Corbeille', icone: mdiTrashCanOutline, chemin: '/corbeille' },
      ],
    },
    {
      titre: 'Cet ordinateur',
      entrees: [{ libelle: 'Synchro', icone: mdiSync, chemin: '/synchro' }],
    },
  ];

  // A dot on "Envoi" while files are being sent.
  let envoi = $state<EtatEnvoi | null>(null);
  const envoiActif = $derived(!!envoi && !envoi.enPause && envoi.phase !== 'inactif');
  onMount(() => {
    etatEnvoi().then((e) => (envoi = e)).catch(() => {});
    const arret = listen<EtatEnvoi>('envoi', ({ payload }) => (envoi = payload));
    return () => void arret.then((f) => f());
  });

  const actif = (chemin: string) =>
    page.url.pathname === chemin ||
    (chemin === '/personnes' && page.url.pathname === '/personne') ||
    (chemin === '/albums' && page.url.pathname === '/album');
</script>

{#snippet lien(entree: Entree)}
  {#if entree.chemin}
    <a
      href={entree.chemin}
      class="entree"
      class:actif={actif(entree.chemin)}
      aria-current={actif(entree.chemin) ? 'page' : undefined}
      title={repliee ? entree.libelle : undefined}
    >
      <span class="relative flex">
        <Icon icon={entree.icone} size="22" />
        {#if entree.chemin === '/synchro' && envoiActif}<span class="temoin" title="Synchro en cours"></span>{/if}
      </span>
      {#if !repliee}<span>{entree.libelle}</span>{/if}
    </a>
  {:else}
    <span class="entree desactivee" title={entree.bientot}>
      <Icon icon={entree.icone} size="22" />
      {#if !repliee}<span>{entree.libelle}</span><span class="bientot">bientôt</span>{/if}
    </span>
  {/if}
{/snippet}

<nav class="flex h-full flex-col gap-1 overflow-y-auto py-3" class:repliee aria-label="Navigation principale">
  {#each groupes as groupe, i (i)}
    {#if groupe.titre && !repliee}
      <div class="mt-4 px-5 pb-1 text-xs font-semibold uppercase tracking-wide text-gray-500">{groupe.titre}</div>
    {:else if i > 0}
      <div class="mx-4 my-2 border-t border-gray-200 dark:border-gray-800"></div>
    {/if}
    {#each groupe.entrees as entree (entree.libelle)}
      {@render lien(entree)}
    {/each}
  {/each}
</nav>

<style>
  nav {
    width: 232px;
    transition: width 150ms;
  }
  nav.repliee {
    width: 68px;
  }
  .entree {
    display: flex;
    align-items: center;
    gap: 14px;
    margin: 0 10px;
    padding: 9px 12px;
    border-radius: 9999px;
    font-size: 14px;
    font-weight: 500;
    white-space: nowrap;
  }
  .repliee .entree {
    justify-content: center;
    padding: 9px 0;
  }
  a.entree:hover {
    background: color-mix(in oklab, currentColor 8%, transparent);
  }
  .entree.actif {
    color: var(--immich-ui-primary-500);
    background: color-mix(in oklab, var(--immich-ui-primary-500) 14%, transparent);
  }
  .desactivee {
    opacity: 0.45;
    cursor: default;
  }
  .temoin {
    position: absolute;
    right: -2px;
    top: -2px;
    width: 8px;
    height: 8px;
    border-radius: 9999px;
    background: var(--immich-ui-primary-500);
    animation: pulsation 1.4s ease-in-out infinite;
  }
  @keyframes pulsation {
    50% {
      opacity: 0.35;
    }
  }
  .bientot {
    margin-left: auto;
    font-size: 11px;
    font-weight: 400;
  }
</style>
