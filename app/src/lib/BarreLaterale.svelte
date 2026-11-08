<script lang="ts">
  // Sidebar: the timeline views, the sections planned for the next lots
  // (shown disabled so the roadmap is visible), and the settings.
  import { Icon } from '@immich/ui';
  import {
    mdiAccountMultipleOutline,
    mdiArchiveArrowDownOutline,
    mdiCogOutline,
    mdiCompassOutline,
    mdiHeartOutline,
    mdiImageAlbum,
    mdiImageMultipleOutline,
    mdiMapOutline,
    mdiTrashCanOutline,
  } from '@mdi/js';
  import { page } from '$app/state';

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
  ];

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
      <Icon icon={entree.icone} size="22" />
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
  <span class="flex-1"></span>
  {@render lien({ libelle: 'Réglages', icone: mdiCogOutline, chemin: '/reglages' })}
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
  .bientot {
    margin-left: auto;
    font-size: 11px;
    font-weight: 400;
  }
</style>
