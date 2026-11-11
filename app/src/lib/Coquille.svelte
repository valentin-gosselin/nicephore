<script lang="ts">
  // Application shell: checks the connection, then shows the header,
  // the sidebar and the current page; otherwise the login screen.
  import { LoadingSpinner, Text } from '@immich/ui';
  import type { Snippet } from 'svelte';
  import { onMount } from 'svelte';
  import { mdiArrowLeft, mdiArrowRight } from '@mdi/js';
  import { IconButton } from '@immich/ui';
  import { afterNavigate, goto } from '$app/navigation';
  import { avance, historique, noterNavigation, retour } from './navigation.svelte';
  import { page } from '$app/state';
  import { t } from './i18n.svelte';
  import BarreLaterale from './BarreLaterale.svelte';
  import BarreRecherche from './BarreRecherche.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import BarreFenetre from './BarreFenetre.svelte';
  import BoutonsFenetre from './BoutonsFenetre.svelte';
  import Connexion from './Connexion.svelte';
  import MenuCompte from './MenuCompte.svelte';
  import Deverrouillage from './Deverrouillage.svelte';
  import Notifications from './Notifications.svelte';
  import { compteAjoute, session, verifierSession } from './session.svelte';

  let { children }: { children: Snippet } = $props();

  let barreRecherche = $state<BarreRecherche>();

  afterNavigate((navigation) =>
    noterNavigation(navigation.type, navigation.type === 'popstate' ? navigation.delta : undefined),
  );

  /** Side buttons of the mouse (back = 3, forward = 4). */
  function surBoutonSouris(e: MouseEvent) {
    if (e.button !== 3 && e.button !== 4) return;
    e.preventDefault();
    if (e.type === 'mouseup') (e.button === 3 ? retour : avance)();
  }

  function surTouche(e: KeyboardEvent) {
    if (e.altKey && (e.key === 'ArrowLeft' || e.key === 'ArrowRight')) {
      e.preventDefault();
      (e.key === 'ArrowLeft' ? retour : avance)();
      return;
    }
    if ((e.ctrlKey && e.key.toLowerCase() === 'k') || (e.key === '/' && !(e.target instanceof HTMLInputElement))) {
      e.preventDefault();
      barreRecherche?.focaliser();
    }
  }

  let largeur = $state(1280);
  const repliee = $derived(largeur < 960);

  // A newer version found by the startup check.
  let versionDisponible = $state<string | null>(null);

  /** Opens photos of this computer (default viewer). */
  function ouvrirFichiers(fichiers: string[]) {
    if (!fichiers.length) return;
    // The viewer sizes the window to the photo and shows it; just in case,
    // the window shows anyway after a few seconds.
    void goto(`/fichier?chemin=${encodeURIComponent(fichiers[0])}&ajuster=1`);
    setTimeout(async () => {
      if (!(await getCurrentWindow().isVisible())) await getCurrentWindow().show();
    }, 5000);
  }

  onMount(() => {
    void verifierSession();
    invoke<string[]>('fichiers_au_demarrage').then(ouvrirFichiers).catch(() => {});
    const arretFichiers = listen<string[]>('ouvrir-fichiers', ({ payload }) => ouvrirFichiers(payload));
    const arretVersion = listen<{ version: string }>('version-disponible', ({ payload }) => (versionDisponible = payload.version));
    // Portable mode scenario (scripts/banc.sh --portable): runs from the
    // login or unlock screen, before any timeline exists.
    invoke<boolean>('mode_banc').then(async (banc) => {
      const scenario = banc ? await invoke<string>('scenario_banc') : '';
      if (scenario.startsWith('portable')) {
        const { lancerScenarioPortable } = await import('./scenario-portable');
        void lancerScenarioPortable(scenario);
      } else if (scenario === 'fichier') {
        const { lancerScenarioFichier } = await import('./scenario-fichier');
        void lancerScenarioFichier();
      } else if (scenario === 'comptes') {
        const { lancerScenarioComptes } = await import('./scenario-comptes');
        void lancerScenarioComptes();
      } else if (scenario === 'vitrine') {
        while (!session.connexion) await new Promise((r) => setTimeout(r, 200));
        const { lancerScenarioVitrine } = await import('./scenario-vitrine');
        void lancerScenarioVitrine();
      } else if (scenario === 'mise-a-jour') {
        // Once connected (development key of the disposable server).
        while (!session.connexion) await new Promise((r) => setTimeout(r, 200));
        const { lancerScenarioMiseAJour } = await import('./scenario-mise-a-jour');
        void lancerScenarioMiseAJour();
      }
    });
    return () => {
      void arretVersion.then((f) => f());
      void arretFichiers.then((f) => f());
    };
  });
</script>

<svelte:window bind:innerWidth={largeur} onkeydown={surTouche} onmousedown={surBoutonSouris} onmouseup={surBoutonSouris} />

<div class="flex h-full flex-col">
  {#if page.url.pathname === '/fichier'}
    <!-- A photo of this computer: shown even without a connection. -->
    {@render children()}
  {:else if session.ajout}
    <BarreFenetre />
    <Connexion onconnecte={() => compteAjoute()} onannuler={() => (session.ajout = false)} />
  {:else if session.verification}
    <BarreFenetre />
    <div class="flex flex-1 items-center justify-center"><LoadingSpinner size="giant" /></div>
  {:else if session.connexion}
    <!-- The header is the title bar: empty areas move the window, a double
         click maximizes it. -->
    <!-- Laid out like the Immich web navbar: a first column as wide as the
         sidebar, then the search bar (up to 64rem), then the account. -->
    <header
      class="barre-navigation flex shrink-0 items-center border-b border-gray-200 py-2 pe-3 dark:border-gray-800"
      data-tauri-drag-region
    >
      <div class="flex shrink-0 items-center gap-3 ps-3 {repliee ? 'pe-3' : 'w-64'}" data-tauri-drag-region>
      <div class="flex">
        <IconButton
          icon={mdiArrowLeft}
          aria-label={t('Retour (Alt+←)')}
          variant="ghost"
          color="secondary"
          shape="round"
          disabled={historique.position === 0}
          onclick={retour}
        />
        <IconButton
          icon={mdiArrowRight}
          aria-label={t('Avancer (Alt+→)')}
          variant="ghost"
          color="secondary"
          shape="round"
          disabled={historique.position >= historique.maximum}
          onclick={avance}
        />
      </div>
      <img src="/icone.svg" alt="" class="pointer-events-none h-7 w-7" />
      {#if !repliee}<span class="pointer-events-none select-none text-lg font-semibold tracking-tight">Nicéphore</span>{/if}
      </div>
      <!-- The search takes the free width up to 64rem; only what is left
           beyond goes to the drag area. -->
      <div class="min-w-0 max-w-5xl" style:flex="1000 1 0%">
        <BarreRecherche bind:this={barreRecherche} />
      </div>
      {#if versionDisponible}
        <a href="/reglages" class="nouvelle ms-4 shrink-0 rounded-full px-3 py-1 text-xs font-medium" onclick={() => (versionDisponible = null)}>
          {t('Nicéphore {version} disponible', { version: versionDisponible })}
        </a>
      {/if}
      <span class="h-full min-w-6 flex-1" data-tauri-drag-region></span>
      <MenuCompte connexion={session.connexion} />
      <span class="mx-1 h-6 w-px bg-gray-300 dark:bg-gray-700"></span>
      <BoutonsFenetre />
    </header>
    <div class="flex min-h-0 flex-1">
      <BarreLaterale {repliee} />
      <main class="min-w-0 flex-1">
        {@render children()}
      </main>
    </div>
  {:else}
    <BarreFenetre />
    {#if session.erreur}<div class="p-4"><Text color="danger">{session.erreur}</Text></div>{/if}
    {#if session.verrouille}
      <Deverrouillage
        serveur={session.verrouille}
        onconnecte={(c) => (session.connexion = c)}
        onautrement={() => (session.verrouille = null)}
      />
    {:else}
      <Connexion onconnecte={(c) => (session.connexion = c)} />
    {/if}
  {/if}
</div>

<Notifications />

<style>
  /* Height of the Immich web navbar. */
  .barre-navigation {
    height: calc(4.5rem + 4px);
  }
  .nouvelle {
    color: var(--immich-ui-primary-500);
    background: color-mix(in oklab, var(--immich-ui-primary-500) 14%, transparent);
  }
</style>
