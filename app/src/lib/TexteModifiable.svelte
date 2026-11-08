<script lang="ts">
  // Text edited in place, like album titles in Immich: a click turns it into
  // a field; Enter or leaving the field saves, Escape cancels.
  import { mdiPencilOutline } from '@mdi/js';
  import { tick } from 'svelte';

  let {
    valeur,
    vide,
    modifiable = true,
    titre = false,
    onvalider,
  }: {
    valeur: string;
    /** Shown (and used as placeholder) when the value is empty. */
    vide: string;
    modifiable?: boolean;
    /** Large heading style, otherwise small muted text. */
    titre?: boolean;
    onvalider: (nouvelle: string) => void;
  } = $props();

  let edition = $state(false);
  let brouillon = $state('');
  let champ = $state<HTMLInputElement>();

  async function commencer() {
    if (!modifiable) return;
    brouillon = valeur;
    edition = true;
    await tick();
    champ?.select();
  }

  function terminer(enregistrer: boolean) {
    if (!edition) return;
    edition = false;
    if (enregistrer && brouillon.trim() !== valeur.trim()) onvalider(brouillon.trim());
  }
</script>

{#if edition}
  <input
    bind:this={champ}
    bind:value={brouillon}
    class="champ"
    class:grand={titre}
    placeholder={vide}
    aria-label={vide}
    onkeydown={(e) => {
      if (e.key === 'Enter') terminer(true);
      else if (e.key === 'Escape') {
        e.stopPropagation();
        terminer(false);
      }
    }}
    onblur={() => terminer(true)}
  />
{:else}
  <button
    type="button"
    class="texte"
    class:grand={titre}
    class:absent={!valeur}
    class:fixe={!modifiable}
    title={modifiable ? 'Cliquer pour modifier' : undefined}
    onclick={commencer}
  >
    <span class="truncate">{valeur || vide}</span>
    {#if modifiable}
      <svg class="crayon" viewBox="0 0 24 24" width="16" height="16" aria-hidden="true"><path fill="currentColor" d={mdiPencilOutline} /></svg>
    {/if}
  </button>
{/if}

<style>
  .texte {
    display: inline-flex;
    min-width: 0;
    align-items: center;
    gap: 6px;
    border-radius: 8px;
    padding: 2px 6px;
    margin-left: -6px;
    font-size: 13px;
    text-align: left;
  }
  .texte.grand {
    font-size: 18px;
    font-weight: 600;
  }
  .texte:not(.grand) {
    opacity: 0.7;
  }
  .texte:not(.fixe):hover {
    background: color-mix(in oklab, currentColor 8%, transparent);
  }
  .fixe {
    cursor: default;
  }
  .absent {
    font-style: italic;
    opacity: 0.5;
  }
  .crayon {
    flex-shrink: 0;
    opacity: 0;
    transition: opacity 120ms;
  }
  .texte:hover .crayon,
  .texte:focus-visible .crayon {
    opacity: 0.6;
  }
  .champ {
    min-width: 12rem;
    border-radius: 8px;
    padding: 2px 6px;
    margin-left: -6px;
    font-size: 13px;
    background: transparent;
    border: 1px solid var(--immich-ui-primary-500);
    outline: none;
  }
  .champ.grand {
    min-width: 18rem;
    font-size: 18px;
    font-weight: 600;
  }
</style>
