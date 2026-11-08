<script lang="ts">
  // Application shell: checks the connection, then shows the header,
  // the sidebar and the current page; otherwise the login screen.
  import { LoadingSpinner, Text } from '@immich/ui';
  import type { Snippet } from 'svelte';
  import { onMount } from 'svelte';
  import BarreLaterale from './BarreLaterale.svelte';
  import Connexion from './Connexion.svelte';
  import Notifications from './Notifications.svelte';
  import { session, verifierSession } from './session.svelte';

  let { children }: { children: Snippet } = $props();

  let largeur = $state(1280);
  const repliee = $derived(largeur < 960);

  onMount(() => void verifierSession());
</script>

<svelte:window bind:innerWidth={largeur} />

<div class="flex h-full flex-col">
  {#if session.verification}
    <div class="flex flex-1 items-center justify-center"><LoadingSpinner size="giant" /></div>
  {:else if session.connexion}
    <header class="flex h-14 shrink-0 items-center gap-3 border-b border-gray-200 px-5 dark:border-gray-800">
      <img src="/icone.svg" alt="" class="h-7 w-7" />
      <span class="text-lg font-semibold tracking-tight">Nicéphore</span>
      <span class="flex-1"></span>
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
    <Connexion onconnecte={(c) => (session.connexion = c)} />
  {/if}
</div>

<Notifications />
