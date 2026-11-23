<script lang="ts">
  // Picks the album that receives the selection, or creates a new one. For
  // photos being sent, it may also offer « the library only ».
  import { Button, Input, LoadingSpinner, Text } from '@immich/ui';
  import { onMount } from 'svelte';
  import { albums as chargerAlbums, urlImage, type Album } from './api';
  import Dialogue from './Dialogue.svelte';
  import { t, tp } from './i18n.svelte';

  let {
    nombre,
    onchoisir,
    oncreer,
    onfermer,
    titre,
    nomPropose = '',
    onbibliotheque,
  }: {
    nombre: number;
    onchoisir: (album: Album) => void;
    oncreer: (nom: string) => void;
    onfermer: () => void;
    titre?: string;
    /** Name filled in for a new album (a dropped folder's). */
    nomPropose?: string;
    /** Offers sending without album. */
    onbibliotheque?: () => void;
  } = $props();

  let liste = $state<Album[] | null>(null);
  let erreur = $state<string | null>(null);
  // svelte-ignore state_referenced_locally
  let filtre = $state(nomPropose);

  const filtres = $derived(
    (liste ?? [])
      // Only albums the user owns can receive photos for sure.
      .filter((a) => a.possede && a.albumName.toLowerCase().includes(filtre.trim().toLowerCase()))
      .sort((a, b) => (b.lastModifiedAssetTimestamp ?? b.updatedAt ?? '').localeCompare(a.lastModifiedAssetTimestamp ?? a.updatedAt ?? '')),
  );

  onMount(() => {
    chargerAlbums()
      .then((a) => (liste = a))
      .catch((e) => (erreur = String(e)));
  });
</script>

<Dialogue titre={titre ?? tp(nombre, 'Ajouter {n} élément à un album', 'Ajouter {n} éléments à un album')} {onfermer}>
  {#if onbibliotheque}
    <Button class="mb-4 w-full" variant="outline" color="secondary" onclick={onbibliotheque}>{t('Dans la bibliothèque seulement, sans album')}</Button>
    <Text size="small" color="muted" class="mb-2">{t('Ou dans un album :')}</Text>
  {/if}
  <form
    class="mb-4 flex gap-2"
    onsubmit={(e) => {
      e.preventDefault();
      if (filtre.trim()) oncreer(filtre.trim());
    }}
  >
    <Input bind:value={filtre} placeholder={t('Chercher un album ou nommer un nouvel album')} class="flex-1" />
    <Button type="submit" disabled={!filtre.trim()}>{t('Créer')}</Button>
  </form>
  {#if erreur}
    <Text color="danger">{erreur}</Text>
  {:else if !liste}
    <LoadingSpinner />
  {:else}
    <ul class="space-y-1">
      {#each filtres as a (a.id)}
        <li>
          <button class="choix flex w-full items-center gap-3 rounded-lg p-2 text-left" onclick={() => onchoisir(a)}>
            {#if a.albumThumbnailAssetId}
              <img src={urlImage('vignette', a.albumThumbnailAssetId, null)} alt="" class="h-12 w-12 rounded-md object-cover" />
            {:else}
              <span class="h-12 w-12 rounded-md bg-gray-500/20"></span>
            {/if}
            <span class="min-w-0">
              <span class="block truncate font-medium">{a.albumName}</span>
              <span class="block text-xs opacity-60">{tp(a.assetCount, '{n} élément', '{n} éléments')}</span>
            </span>
          </button>
        </li>
      {:else}
        <Text color="muted">{t('Aucun album à ce nom : « Créer » en fera un nouveau.')}</Text>
      {/each}
    </ul>
  {/if}
</Dialogue>

<style>
  .choix:hover {
    background: color-mix(in oklab, currentColor 8%, transparent);
  }
</style>
