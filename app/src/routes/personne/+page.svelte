<script lang="ts">
  // Timeline of one person (filtered with `personId`).
  import { page } from '$app/state';
  import Chronologie from '../../lib/Chronologie.svelte';
  import type { Vue } from '../../lib/vues';

  const id = $derived(page.url.searchParams.get('id') ?? '');
  const nom = $derived(page.url.searchParams.get('nom') || 'Sans nom');
  const vue = $derived<Vue>({
    id: 'personne',
    titre: nom,
    chemin: '/personne',
    filtre: { personne: id },
    vide: 'Aucune photo de cette personne.',
  });
</script>

<svelte:head><title>{nom} · Nicéphore</title></svelte:head>

{#key id}
  <Chronologie {vue} />
{/key}
