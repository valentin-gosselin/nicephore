<script lang="ts">
  // Sets the capture date (and its time zone) of one or many assets.
  import { Button, Text } from '@immich/ui';
  import { fuseauLocal, versIso } from './dates';
  import Dialogue from './Dialogue.svelte';

  let {
    nombre,
    initiale,
    fuseauInitial,
    onvalider,
    onfermer,
  }: {
    nombre: number;
    /** "YYYY-MM-DDTHH:mm", in `fuseauInitial`. */
    initiale: string;
    fuseauInitial: string | null;
    onvalider: (date: string, fuseau: string) => void;
    onfermer: () => void;
  } = $props();

  // svelte-ignore state_referenced_locally
  let local = $state(initiale);
  // svelte-ignore state_referenced_locally
  let fuseau = $state(fuseauInitial || fuseauLocal());

  function valider(e: SubmitEvent) {
    e.preventDefault();
    if (local) onvalider(versIso(local, fuseau.trim()), fuseau.trim());
  }
</script>

<Dialogue titre={nombre > 1 ? `Date de ${nombre} éléments` : 'Date de la prise de vue'} {onfermer}>
  <form class="space-y-4" onsubmit={valider}>
    <label class="block space-y-1">
      <Text size="small">Date et heure</Text>
      <input class="champ w-full" type="datetime-local" bind:value={local} required />
    </label>
    <label class="block space-y-1">
      <Text size="small">Fuseau horaire</Text>
      <input class="champ w-full" bind:value={fuseau} placeholder="Europe/Paris" />
    </label>
    {#if nombre > 1}
      <Text size="small" color="muted">Tous les éléments sélectionnés prendront cette même date.</Text>
    {/if}
    <Button type="submit" fullWidth disabled={!local}>Enregistrer</Button>
  </form>
</Dialogue>

<style>
  .champ {
    border-radius: 8px;
    padding: 6px 10px;
    font-size: 14px;
    background: transparent;
    border: 1px solid color-mix(in oklab, currentColor 20%, transparent);
    color-scheme: light dark;
  }
</style>
