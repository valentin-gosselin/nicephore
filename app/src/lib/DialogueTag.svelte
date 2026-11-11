<script lang="ts">
  // Adds a tag (existing or new) to one or many assets.
  import { Button, Input, LoadingSpinner, Text } from '@immich/ui';
  import { onMount } from 'svelte';
  import { tags as chargerTags, type Tag } from './api';
  import Dialogue from './Dialogue.svelte';
  import { locale, t } from './i18n.svelte';

  let { nombre, onvalider, onfermer }: { nombre: number; onvalider: (nom: string) => void; onfermer: () => void } = $props();

  let tous = $state<Tag[] | null>(null);
  let nom = $state('');
  const proposes = $derived(
    (tous ?? []).filter((tag) => tag.value.toLocaleLowerCase(locale()).includes(nom.trim().toLocaleLowerCase(locale()))).slice(0, 30),
  );

  onMount(() => {
    chargerTags()
      .then((liste) => (tous = liste.sort((a, b) => a.value.localeCompare(b.value, locale()))))
      .catch(() => (tous = []));
  });
</script>

<Dialogue titre={nombre > 1 ? t('Tag pour {n} éléments', { n: nombre }) : t('Ajouter un tag')} {onfermer}>
  <form
    class="mb-3 flex gap-2"
    onsubmit={(e) => {
      e.preventDefault();
      if (nom.trim()) onvalider(nom.trim());
    }}
  >
    <Input bind:value={nom} placeholder={t('Nom du tag (Voyages/Italie pour un sous-tag)')} class="flex-1" />
    <Button type="submit" disabled={!nom.trim()}>{t('Ajouter')}</Button>
  </form>
  {#if !tous}
    <LoadingSpinner />
  {:else if proposes.length}
    <div class="flex flex-wrap gap-2">
      {#each proposes as tag (tag.id)}
        <button class="pastille" onclick={() => onvalider(tag.value)}>{tag.value}</button>
      {/each}
    </div>
  {:else}
    <Text size="small" color="muted">{t('Aucun tag existant de ce nom : « Ajouter » le créera.')}</Text>
  {/if}
</Dialogue>

<style>
  .pastille {
    border-radius: 9999px;
    padding: 4px 12px;
    font-size: 13px;
    border: 1px solid color-mix(in oklab, currentColor 20%, transparent);
  }
  .pastille:hover {
    background: color-mix(in oklab, currentColor 8%, transparent);
  }
</style>
