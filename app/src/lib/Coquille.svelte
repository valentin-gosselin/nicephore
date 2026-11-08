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
  import Connexion from './Connexion.svelte';
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

  onMount(() => {
    void verifierSession();
    // Portable mode scenario (scripts/banc.sh --portable): runs from the
    // login or unlock screen, before any timeline exists.
    invoke<boolean>('mode_banc').then(async (banc) => {
      const scenario = banc ? await invoke<string>('scenario_banc') : '';
      if (scenario.startsWith('portable')) {
        const { lancerScenarioPortable } = await import('./scenario-portable');
        void lancerScenarioPortable(scenario);
      }
    });
  });
</script>

<svelte:window bind:innerWidth={largeur} onkeydown={surTouche} onmousedown={surBoutonSouris} onmouseup={surBoutonSouris} />

<div class="flex h-full flex-col">
  {#if session.verification}
    <div class="flex flex-1 items-center justify-center"><LoadingSpinner size="giant" /></div>
  {:else if session.connexion}
    <header class="flex h-14 shrink-0 items-center gap-3 border-b border-gray-200 px-3 dark:border-gray-800">
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
      <img src="/icone.svg" alt="" class="h-7 w-7" />
      <span class="text-lg font-semibold tracking-tight">Nicéphore</span>
      <form class="recherche mx-auto flex w-full max-w-xl items-center" role="search" onsubmit={chercher}>
        <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true" class="loupe"><path fill="currentColor" d={mdiMagnify} /></svg>
        <input
          bind:this={champRecherche}
          bind:value={texteRecherche}
          type="search"
          placeholder="Rechercher dans vos photos (Ctrl+K)"
          aria-label="Rechercher"
        />
      </form>
      <a href="/reglages" class="text-sm opacity-70 hover:opacity-100">{session.connexion.utilisateur.name}</a>
    </header>
    <div class="flex min-h-0 flex-1">
      <BarreLaterale {repliee} />
      <main class="min-w-0 flex-1">
        {@render children()}
      </main>
    </div>
  {:else}
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
