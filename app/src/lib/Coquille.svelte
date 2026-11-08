<script lang="ts">
  // Application shell: checks the connection, then shows the header,
  // the sidebar and the current page; otherwise the login screen.
  import { LoadingSpinner, Text } from '@immich/ui';
  import type { Snippet } from 'svelte';
  import { onMount } from 'svelte';
  import { mdiArrowLeft, mdiArrowRight, mdiMagnify } from '@mdi/js';
  import { IconButton } from '@immich/ui';
  import { afterNavigate, goto } from '$app/navigation';
  import { avance, historique, noterNavigation, retour } from './navigation.svelte';
  import { page } from '$app/state';
  import { lireCriteres, nomsPersonnes, urlRecherche } from './criteres-url';
  import BarreLaterale from './BarreLaterale.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import BarreFenetre from './BarreFenetre.svelte';
  import BoutonsFenetre from './BoutonsFenetre.svelte';
  import Connexion from './Connexion.svelte';
  import MenuCompte from './MenuCompte.svelte';
  import Deverrouillage from './Deverrouillage.svelte';
  import Notifications from './Notifications.svelte';
  import { session, verifierSession } from './session.svelte';

  let { children }: { children: Snippet } = $props();

  let champRecherche = $state<HTMLInputElement>();
  let texteRecherche = $state('');

  // Keep the field in sync with the search being shown.
  $effect(() => {
    if (page.url.pathname === '/recherche') texteRecherche = page.url.searchParams.get('texte') ?? '';
  });

  function chercher(e: SubmitEvent) {
    e.preventDefault();
    const criteres = page.url.pathname === '/recherche' ? lireCriteres(page.url.searchParams) : {};
    void goto(urlRecherche({ ...criteres, texte: texteRecherche }, nomsPersonnes(page.url.searchParams)));
    champRecherche?.blur();
  }

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
      champRecherche?.focus();
      champRecherche?.select();
    }
  }

  let largeur = $state(1280);
  const repliee = $derived(largeur < 960);

  // A newer version found by the startup check.
  let versionDisponible = $state<string | null>(null);

  onMount(() => {
    void verifierSession();
    const arretVersion = listen<{ version: string }>('version-disponible', ({ payload }) => (versionDisponible = payload.version));
    // Portable mode scenario (scripts/banc.sh --portable): runs from the
    // login or unlock screen, before any timeline exists.
    invoke<boolean>('mode_banc').then(async (banc) => {
      const scenario = banc ? await invoke<string>('scenario_banc') : '';
      if (scenario.startsWith('portable')) {
        const { lancerScenarioPortable } = await import('./scenario-portable');
        void lancerScenarioPortable(scenario);
      } else if (scenario === 'mise-a-jour') {
        // Once connected (development key of the disposable server).
        while (!session.connexion) await new Promise((r) => setTimeout(r, 200));
        const { lancerScenarioMiseAJour } = await import('./scenario-mise-a-jour');
        void lancerScenarioMiseAJour();
      }
    });
    return () => void arretVersion.then((f) => f());
  });
</script>

<svelte:window bind:innerWidth={largeur} onkeydown={surTouche} onmousedown={surBoutonSouris} onmouseup={surBoutonSouris} />

<div class="flex h-full flex-col">
  {#if session.verification}
    <BarreFenetre />
    <div class="flex flex-1 items-center justify-center"><LoadingSpinner size="giant" /></div>
  {:else if session.connexion}
    <!-- The header is the title bar: empty areas move the window, a double
         click maximizes it. -->
    <header class="flex h-14 shrink-0 items-center gap-3 border-b border-gray-200 px-3 dark:border-gray-800" data-tauri-drag-region>
      <div class="flex">
        <IconButton
          icon={mdiArrowLeft}
          aria-label="Retour (Alt+←)"
          variant="ghost"
          color="secondary"
          shape="round"
          disabled={historique.position === 0}
          onclick={retour}
        />
        <IconButton
          icon={mdiArrowRight}
          aria-label="Avancer (Alt+→)"
          variant="ghost"
          color="secondary"
          shape="round"
          disabled={historique.position >= historique.maximum}
          onclick={avance}
        />
      </div>
      <img src="/icone.svg" alt="" class="pointer-events-none h-7 w-7" />
      <span class="pointer-events-none select-none text-lg font-semibold tracking-tight">Nicéphore</span>
      <span class="h-full flex-1" data-tauri-drag-region></span>
      <form class="recherche flex w-full max-w-xl items-center" role="search" onsubmit={chercher}>
        <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true" class="loupe"><path fill="currentColor" d={mdiMagnify} /></svg>
        <input
          bind:this={champRecherche}
          bind:value={texteRecherche}
          type="search"
          placeholder="Rechercher dans vos photos (Ctrl+K)"
          aria-label="Rechercher"
        />
      </form>
      {#if versionDisponible}
        <a href="/reglages" class="nouvelle shrink-0 rounded-full px-3 py-1 text-xs font-medium" onclick={() => (versionDisponible = null)}>
          Nicéphore {versionDisponible} disponible
        </a>
      {/if}
      <span class="h-full flex-1" data-tauri-drag-region></span>
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
  .nouvelle {
    color: var(--immich-ui-primary-500);
    background: color-mix(in oklab, var(--immich-ui-primary-500) 14%, transparent);
  }
  .recherche {
    position: relative;
  }
  .recherche .loupe {
    position: absolute;
    left: 12px;
    opacity: 0.55;
    pointer-events: none;
  }
  .recherche input {
    width: 100%;
    border-radius: 9999px;
    padding: 7px 16px 7px 40px;
    font-size: 14px;
    background: color-mix(in oklab, currentColor 7%, transparent);
    border: 1px solid transparent;
    outline: none;
  }
  .recherche input:focus {
    border-color: var(--immich-ui-primary-500);
    background: transparent;
  }
</style>
