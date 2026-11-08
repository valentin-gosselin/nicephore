<script lang="ts">
  // Albums: the user's own, then those shared with them, sortable and
  // filterable, with a button to create one.
  import { Button, Heading, Input, LoadingSpinner, Text } from '@immich/ui';
  // (Input is used in the creation dialog.)
  import { mdiPlus } from '@mdi/js';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { albums as chargerAlbums, creerAlbum, urlImage, type Album } from '../../lib/api';
  import Dialogue from '../../lib/Dialogue.svelte';
  import { notifier } from '../../lib/notifications.svelte';

  type Tri = 'recent' | 'az' | 'za' | 'dates';
  const TRIS: { valeur: Tri; libelle: string }[] = [
    { valeur: 'recent', libelle: 'Modifiés récemment' },
    { valeur: 'dates', libelle: 'Date des photos' },
    { valeur: 'az', libelle: 'Nom, de A à Z' },
    { valeur: 'za', libelle: 'Nom, de Z à A' },
  ];

  let liste = $state<Album[] | null>(null);
  let erreur = $state<string | null>(null);
  let filtre = $state('');
  let tri = $state<Tri>(lireTri());
  let creation = $state(false);
  let nom = $state('');
  let description = $state('');

  function lireTri(): Tri {
    try {
      const v = localStorage.getItem('nicephore-tri-albums');
      if (v === 'recent' || v === 'az' || v === 'za' || v === 'dates') return v;
    } catch {
      // Default below.
    }
    return 'recent';
  }

  function trier(a: Album, b: Album) {
    switch (tri) {
      case 'az':
        return a.albumName.localeCompare(b.albumName, 'fr', { sensitivity: 'base' });
      case 'za':
        return b.albumName.localeCompare(a.albumName, 'fr', { sensitivity: 'base' });
      case 'dates':
        return (b.endDate ?? '').localeCompare(a.endDate ?? '');
      default:
        return (b.lastModifiedAssetTimestamp ?? b.updatedAt ?? '').localeCompare(a.lastModifiedAssetTimestamp ?? a.updatedAt ?? '');
    }
  }

  const visibles = $derived((liste ?? []).filter((a) => a.albumName.toLowerCase().includes(filtre.trim().toLowerCase())).sort(trier));
  const miens = $derived(visibles.filter((a) => a.possede));
  const partages = $derived(visibles.filter((a) => !a.possede));

  const mois = new Intl.DateTimeFormat('fr-FR', { month: 'short', year: 'numeric' });
  function periode(a: Album) {
    if (!a.startDate || !a.endDate) return '';
    const debut = mois.format(new Date(a.startDate));
    const fin = mois.format(new Date(a.endDate));
    return debut === fin ? debut : `${debut} à ${fin}`;
  }

  async function creer(e: SubmitEvent) {
    e.preventDefault();
    try {
      const album = await creerAlbum(nom, description, []);
      creation = false;
      void goto(`/album?id=${album.id}`);
    } catch (raison) {
      notifier(`Création impossible : ${raison}`, 'erreur');
    }
  }

  onMount(() => {
    chargerAlbums()
      .then((a) => (liste = a))
      .catch((e) => (erreur = String(e)));
  });
</script>

{#snippet grille(albums: Album[])}
  <div class="grille">
    {#each albums as a (a.id)}
      <a href={`/album?id=${a.id}`} class="album">
        <span class="couverture">
          {#if a.albumThumbnailAssetId}
            <img src={urlImage('vignette', a.albumThumbnailAssetId, null)} alt="" loading="lazy" />
          {/if}
        </span>
        <span class="mt-2 block truncate font-medium">{a.albumName}</span>
        <span class="block text-xs opacity-60">
          {a.assetCount.toLocaleString('fr-FR')} élément{a.assetCount > 1 ? 's' : ''}{periode(a) ? ` · ${periode(a)}` : ''}{a.shared ? ' · partagé' : ''}
        </span>
      </a>
    {/each}
  </div>
{/snippet}

<svelte:head><title>Albums · Nicéphore</title></svelte:head>

<div class="h-full overflow-y-auto">
  <div class="space-y-8 p-6">
    <div class="flex flex-wrap items-center gap-3">
      <Heading size="small">Albums</Heading>
      {#if liste}<Text size="small" color="muted">{liste.length}</Text>{/if}
      <span class="flex-1"></span>
      <input class="tri w-56" type="search" bind:value={filtre} placeholder="Filtrer les albums" aria-label="Filtrer les albums" />
      <select
        class="tri"
        aria-label="Tri des albums"
        value={tri}
        onchange={(e) => {
          tri = e.currentTarget.value as Tri;
          try {
            localStorage.setItem('nicephore-tri-albums', tri);
          } catch {
            // Only a convenience.
          }
        }}
      >
        {#each TRIS as t (t.valeur)}<option value={t.valeur}>{t.libelle}</option>{/each}
      </select>
      <Button leadingIcon={mdiPlus} onclick={() => ((nom = ''), (description = ''), (creation = true))}>Nouvel album</Button>
    </div>

    {#if erreur}
      <Text color="danger">{erreur}</Text>
    {:else if !liste}
      <LoadingSpinner />
    {:else if liste.length === 0}
      <Text color="muted">Aucun album pour l’instant. Sélectionnez des photos puis « Ajouter à un album », ou créez-en un ici.</Text>
    {:else}
      {@render grille(miens)}
      {#if partages.length}
        <Heading size="small">Partagés avec moi</Heading>
        {@render grille(partages)}
      {/if}
    {/if}
  </div>
</div>

{#if creation}
  <Dialogue titre="Nouvel album" onfermer={() => (creation = false)}>
    <form class="space-y-3" onsubmit={creer}>
      <Input bind:value={nom} placeholder="Nom de l’album" />
      <textarea bind:value={description} placeholder="Description (facultative)" rows="3" class="zone"></textarea>
      <Button type="submit" fullWidth disabled={!nom.trim()}>Créer l’album</Button>
    </form>
  </Dialogue>
{/if}

<style>
  .grille {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 20px 16px;
  }
  .couverture {
    display: block;
    aspect-ratio: 1;
    overflow: hidden;
    border-radius: 14px;
    background: color-mix(in oklab, currentColor 10%, transparent);
  }
  .couverture img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    transition: transform 160ms;
  }
  .album:hover img {
    transform: scale(1.04);
  }
  .tri,
  .zone {
    border-radius: 8px;
    padding: 6px 10px;
    font-size: 14px;
    background: transparent;
    border: 1px solid color-mix(in oklab, currentColor 20%, transparent);
    color-scheme: light dark;
  }
  .zone {
    width: 100%;
    resize: vertical;
  }
</style>
