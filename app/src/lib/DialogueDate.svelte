<script lang="ts">
  // Sets the capture date (and its time zone) of one or many assets.
  import { Time } from '@internationalized/date';
  import { Button, Text, TimeInput } from '@immich/ui';
  import ChampDate from './ChampDate.svelte';
  import { fuseauLocal, versIso } from './dates';
  import Dialogue from './Dialogue.svelte';
  import { t } from './i18n.svelte';

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
  let jour = $state<string | undefined>(initiale.slice(0, 10) || undefined);
  // svelte-ignore state_referenced_locally
  let heure = $state(new Time(Number(initiale.slice(11, 13)) || 0, Number(initiale.slice(14, 16)) || 0));
  const deux = (n: number) => String(n).padStart(2, '0');
  const local = $derived(jour ? `${jour}T${deux(heure.hour)}:${deux(heure.minute)}` : '');
  // svelte-ignore state_referenced_locally
  let fuseau = $state(fuseauInitial || fuseauLocal());

  function valider(e: SubmitEvent) {
    e.preventDefault();
    if (local) onvalider(versIso(local, fuseau.trim()), fuseau.trim());
  }
</script>

<Dialogue titre={nombre > 1 ? t('Date de {n} éléments', { n: nombre }) : t('Date de la prise de vue')} {onfermer}>
  <form class="space-y-4" onsubmit={valider}>
    <div class="space-y-1">
      <Text size="small">{t('Date et heure')}</Text>
      <div class="flex items-center gap-2">
        <ChampDate libelle={t('Date')} class="flex-1" bind:valeur={jour} />
        <div class="w-28" role="group" aria-label={t('Heure')}>
          <TimeInput size="small" value={heure} onChange={(h) => h && (heure = new Time(h.hour, h.minute))} />
        </div>
      </div>
    </div>
    <label class="block space-y-1">
      <Text size="small">{t('Fuseau horaire')}</Text>
      <input class="champ w-full" bind:value={fuseau} placeholder="Europe/Paris" />
    </label>
    {#if nombre > 1}
      <Text size="small" color="muted">{t('Tous les éléments sélectionnés prendront cette même date.')}</Text>
    {/if}
    <Button type="submit" fullWidth disabled={!local}>{t('Enregistrer')}</Button>
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
