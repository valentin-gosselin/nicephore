<script lang="ts">
  import { LoadingSpinner, Text } from '@immich/ui';
  import { onMount } from 'svelte';
  import { etatConnexion, type Connexion as InfosConnexion } from '../lib/api';
  import Chronologie from '../lib/Chronologie.svelte';
  import Connexion from '../lib/Connexion.svelte';

  let connexion = $state<InfosConnexion | null>(null);
  let verification = $state(true);
  let erreur = $state<string | null>(null);
  let nombre = $state<number | null>(null);

  onMount(() => {
    etatConnexion()
      .then((c) => (connexion = c))
      .catch((e) => (erreur = String(e)))
      .finally(() => (verification = false));
  });
</script>

<svelte:head><title>Nicéphore</title></svelte:head>

<div class="flex h-full flex-col">
  {#if verification}
    <div class="flex flex-1 items-center justify-center"><LoadingSpinner size="giant" /></div>
  {:else if connexion}
    <header class="flex h-14 shrink-0 items-center gap-4 border-b border-gray-200 px-5 dark:border-gray-800">
      <span class="text-lg font-semibold">Nicéphore</span>
      <span class="rounded-full bg-primary/15 px-2 py-0.5 text-xs font-medium text-primary">prototype</span>
      <span class="flex-1"></span>
      {#if nombre != null}
        <Text size="small" color="muted">{nombre.toLocaleString('fr-FR')} photos et vidéos</Text>
      {/if}
      <Text size="small" color="muted">{connexion.utilisateur.name} · Immich {connexion.version}</Text>
    </header>
    <main class="min-h-0 flex-1">
      <Chronologie onnombre={(n) => (nombre = n)} />
    </main>
  {:else}
    {#if erreur}<div class="p-4"><Text color="danger">{erreur}</Text></div>{/if}
    <Connexion onconnecte={(c) => (connexion = c)} />
  {/if}
</div>
