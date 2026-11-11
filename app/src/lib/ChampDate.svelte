<script lang="ts">
  /**
   * A date field with the calendar of the Immich web app, instead of the
   * system's date picker. The value is a "YYYY-MM-DD" string, or empty.
   */
  import { DatePicker } from '@immich/ui';
  import { DateTime } from 'luxon';

  let {
    valeur = $bindable(),
    onchange,
    libelle,
    taille = 'small',
    class: classe = '',
  }: {
    valeur?: string;
    onchange?: (valeur: string | undefined) => void;
    /** Read aloud by screen readers, and used by the test bench. */
    libelle: string;
    taille?: 'tiny' | 'small';
    class?: string;
  } = $props();

  const date = $derived(valeur ? DateTime.fromISO(valeur) : undefined);

  function changer(d: DateTime | undefined) {
    const nouvelle = d?.isValid ? (d.toISODate() ?? undefined) : undefined;
    if (nouvelle === valeur) return;
    valeur = nouvelle;
    onchange?.(nouvelle);
  }
</script>

<div class="champ-date {classe}" role="group" aria-label={libelle} data-valeur={valeur ?? ''}>
  <DatePicker value={date} onChange={changer} size={taille} />
</div>

<style>
  .champ-date {
    display: inline-block;
    min-width: 11rem;
  }
</style>
