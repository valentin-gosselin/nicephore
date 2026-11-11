<script lang="ts">
  // Sidebar, laid out like the Immich web one (NavbarItem from @immich/ui):
  // the views, the library with the three recent albums under "Albums",
  // this computer's sync, then the storage and the server status at the
  // bottom. Settings are in the account menu, top right.
  import { Icon, Meter, NavbarGroup, NavbarItem } from '@immich/ui';
  import {
    mdiAccount,
    mdiAccountOutline,
    mdiArchiveArrowDown,
    mdiArchiveArrowDownOutline,
    mdiHeart,
    mdiHeartOutline,
    mdiImageAlbum,
    mdiImageMultiple,
    mdiImageMultipleOutline,
    mdiMagnify,
    mdiMap,
    mdiMapOutline,
    mdiNewBox,
    mdiSync,
    mdiTrashCan,
    mdiTrashCanOutline,
  } from '@mdi/js';
  import { page } from '$app/state';
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import { locale, t } from './i18n.svelte';
  import { albums as chargerAlbums, etatEnvoi, infosServeur, urlImage, type Album, type EtatEnvoi, type InfosServeur } from './api';

  let { repliee }: { repliee: boolean } = $props();

  type Entree = { libelle: string; icone: string; iconeActive?: string; chemin: string; autres?: string[] };

  // $derived: the labels follow a change of language.
  const vues: Entree[] = $derived([
    { libelle: t('Photos'), icone: mdiImageMultipleOutline, iconeActive: mdiImageMultiple, chemin: '/' },
    { libelle: t('Explorer'), icone: mdiMagnify, chemin: '/explorer', autres: ['/recherche'] },
    { libelle: t('Carte'), icone: mdiMapOutline, iconeActive: mdiMap, chemin: '/carte' },
    { libelle: t('Personnes'), icone: mdiAccountOutline, iconeActive: mdiAccount, chemin: '/personnes', autres: ['/personne'] },
  ]);
  const favoris: Entree = $derived({ libelle: t('Favoris'), icone: mdiHeartOutline, iconeActive: mdiHeart, chemin: '/favoris' });
  const albumsEntree: Entree = $derived({ libelle: t('Albums'), icone: mdiImageAlbum, chemin: '/albums', autres: ['/album'] });
  const fin: Entree[] = $derived([
    { libelle: t('Archive'), icone: mdiArchiveArrowDownOutline, iconeActive: mdiArchiveArrowDown, chemin: '/archive' },
    { libelle: t('Corbeille'), icone: mdiTrashCanOutline, iconeActive: mdiTrashCan, chemin: '/corbeille' },
  ]);
  const synchro: Entree = $derived({ libelle: t('Synchro'), icone: mdiSync, chemin: '/synchro' });

  const actif = (e: Entree) => page.url.pathname === e.chemin || (e.autres ?? []).includes(page.url.pathname);

  // The recent albums under "Albums", unfolded or not as in Immich (remembered).
  function lireDeplie() {
    try {
      return localStorage.getItem('nicephore-albums-recents') !== '0';
    } catch {
      return true;
    }
  }
  let deplie = $state(lireDeplie());
  $effect(() => {
    try {
      localStorage.setItem('nicephore-albums-recents', deplie ? '1' : '0');
    } catch {
      // Only a convenience.
    }
  });
  let recents = $state<Album[]>([]);
  const parDate = (a: Album) => Date.parse(a.updatedAt ?? '') || 0;
  async function chargerRecents() {
    try {
      recents = [...(await chargerAlbums())].sort((a, b) => parDate(b) - parDate(a)).slice(0, 3);
    } catch {
      // The list stays as it was.
    }
  }

  // A dot on "Synchro" while files are being sent.
  let envoi = $state<EtatEnvoi | null>(null);
  const envoiActif = $derived(!!envoi && !envoi.enPause && envoi.phase !== 'inactif');

  // Storage and server status, refreshed every minute.
  let serveur = $state<InfosServeur | null>(null);
  let horsLigne = $state(false);
  async function chargerServeur() {
    try {
      serveur = await infosServeur();
      horsLigne = false;
    } catch {
      horsLigne = true;
    }
  }

  onMount(() => {
    etatEnvoi().then((e) => (envoi = e)).catch(() => {});
    const arret = listen<EtatEnvoi>('envoi', ({ payload }) => (envoi = payload));
    void chargerServeur();
    const minuteur = setInterval(() => void chargerServeur(), 60_000);
    return () => {
      void arret.then((f) => f());
      clearInterval(minuteur);
    };
  });
  // Albums change elsewhere in the app: reload them when opening a page.
  $effect(() => {
    void page.url.pathname;
    void chargerRecents();
  });

  const octets = (n: number) => {
    const unites = locale() === 'fr-FR' ? ['o', 'Kio', 'Mio', 'Gio', 'Tio', 'Pio'] : ['B', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB'];
    let i = 0;
    while (n >= 1024 && i < unites.length - 1) {
      n /= 1024;
      i++;
    }
    return `${n.toLocaleString(locale(), { maximumFractionDigits: 1 })} ${unites[i]}`;
  };
  const seuils = [
    { from: 0.8, className: 'bg-warning' },
    { from: 0.95, className: 'bg-danger' },
  ];
</script>

{#snippet item(e: Entree)}
  <NavbarItem title={e.libelle} href={e.chemin} icon={e.icone} activeIcon={e.iconeActive} isActive={() => actif(e)} />
{/snippet}

{#snippet icone(e: Entree)}
  <a href={e.chemin} class="icone" class:actif={actif(e)} aria-current={actif(e) ? 'page' : undefined} title={e.libelle} aria-label={e.libelle}>
    <span class="relative flex">
      <Icon icon={actif(e) && e.iconeActive ? e.iconeActive : e.icone} size="22" />
      {#if e === synchro && envoiActif}<span class="temoin"></span>{/if}
    </span>
  </a>
{/snippet}

{#if repliee}
  <nav class="flex h-full w-[68px] flex-col gap-1 overflow-y-auto py-3" aria-label={t('Navigation principale')}>
    {#each [...vues, favoris, albumsEntree, ...fin] as e (e.chemin)}
      {@render icone(e)}
    {/each}
    <div class="mx-4 my-2 border-t border-gray-200 dark:border-gray-800"></div>
    {@render icone(synchro)}
  </nav>
{:else}
  <nav class="barre flex h-full w-64 shrink-0 flex-col overflow-y-auto pe-4 pt-2" aria-label={t('Navigation principale')}>
    {#each vues as e (e.chemin)}
      {@render item(e)}
    {/each}
    <NavbarGroup title={t('Bibliothèque')} size="tiny" />
    {@render item(favoris)}
    <NavbarItem
      title={albumsEntree.libelle}
      href={albumsEntree.chemin}
      icon={{ icon: mdiImageAlbum, flipped: true }}
      isActive={() => actif(albumsEntree)}
      bind:expanded={deplie}
    >
      {#snippet items()}
        <div class="albums-recents">
          {#each recents as a (a.id)}
            <a
              href={`/album?id=${encodeURIComponent(a.id)}`}
              title={a.albumName}
              class="flex w-full place-items-center gap-4 rounded-e-full py-3 ps-10 pe-4 hover:bg-subtle hover:text-primary"
            >
              <span
                class="size-6 shrink-0 rounded-sm bg-gray-200 bg-cover dark:bg-gray-600"
                style:background-image={a.albumThumbnailAssetId ? `url('${urlImage('vignette', a.albumThumbnailAssetId, null)}')` : undefined}
              ></span>
              <span class="grow truncate text-sm font-medium">{a.albumName}</span>
            </a>
          {/each}
        </div>
      {/snippet}
    </NavbarItem>
    {#each fin as e (e.chemin)}
      {@render item(e)}
    {/each}
    <NavbarGroup title={t('Cet ordinateur')} size="tiny" />
    <div class="relative">
      {@render item(synchro)}
      {#if envoiActif}<span class="temoin temoin-large" title={t('Synchro en cours')}></span>{/if}
    </div>

    <div class="mt-auto pt-4">
      <div class="ms-4 overflow-hidden rounded-lg">
        <div class="stockage bg-light-100 p-4 text-sm" title={serveur ? t('{utilise} utilisés sur {total}', { utilise: octets(serveur.utilise), total: octets(serveur.total) }) : undefined}>
          {#if serveur && serveur.total > 0}
            <Meter
              size="tiny"
              class="bg-light-200"
              containerClass="gap-2 leading-6"
              label={t('Stockage')}
              valueLabel={t('{utilise} sur {total} utilisé', { utilise: octets(serveur.utilise), total: octets(serveur.total) })}
              value={serveur.utilise / serveur.total}
              thresholds={seuils}
            />
          {:else}
            <p class="font-medium">{t('Stockage')}</p>
            <p class="opacity-60">{horsLigne ? t('Indisponible') : '…'}</p>
          {/if}
        </div>
      </div>
      <div class="statut mt-2 mb-6 flex items-center justify-between ps-5 pe-1 text-sm">
        {#if horsLigne}
          <span class="flex items-center gap-2"><span class="point bg-red-500"></span><span class="text-red-500">{t('Serveur hors ligne')}</span></span>
        {:else}
          <span class="flex items-center gap-2"><span class="point bg-green-500"></span>{t('Serveur en ligne')}</span>
          {#if serveur}<span class="opacity-80">v{serveur.version}</span>{/if}
        {/if}
      </div>
      {#if serveur?.nouvelleVersion && !horsLigne}
        <a
          href={`https://github.com/immich-app/immich/releases/tag/v${serveur.nouvelleVersion}`}
          target="_blank"
          rel="noopener noreferrer"
          class="nouvelle-version group ms-4 -mt-3 mb-6 block rounded-lg border p-2.5 text-sm"
        >
          <span class="flex items-center justify-between gap-2">
            <span class="flex items-center gap-2">
              <Icon icon={mdiNewBox} size="16" class="text-primary opacity-80" />
              <span class="text-xs font-medium">v{serveur.nouvelleVersion}</span>
            </span>
            <span class="text-[11px] opacity-70 group-hover:text-primary group-hover:opacity-100">{t('Nouvelle mise à jour !')}</span>
          </span>
        </a>
      {/if}
    </div>
  </nav>
{/if}

<style>
  .icone {
    display: flex;
    justify-content: center;
    margin: 0 10px;
    padding: 9px 0;
    border-radius: 9999px;
  }
  .icone:hover {
    background: color-mix(in oklab, currentColor 8%, transparent);
  }
  .icone.actif {
    color: var(--immich-ui-primary-500);
    background: color-mix(in oklab, var(--immich-ui-primary-500) 14%, transparent);
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
  .temoin-large {
    right: auto;
    left: 36px;
    top: 10px;
  }
  @keyframes pulsation {
    50% {
      opacity: 0.35;
    }
  }
  .point {
    width: 7px;
    height: 7px;
    border-radius: 9999px;
  }
  .nouvelle-version {
    border-color: color-mix(in oklab, currentColor 15%, transparent);
  }
  .nouvelle-version:hover {
    border-color: color-mix(in oklab, var(--immich-ui-primary-500) 40%, transparent);
  }
</style>
