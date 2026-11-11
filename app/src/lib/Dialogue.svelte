<script lang="ts">
  // Modal dialog: dimmed backdrop, Escape or a click outside closes it.
  import { IconButton } from '@immich/ui';
  import { mdiClose } from '@mdi/js';
  import type { Snippet } from 'svelte';
  import { onMount } from 'svelte';
  import { t } from './i18n.svelte';

  let {
    titre,
    onfermer,
    children,
    largeur = 460,
  }: { titre: string; onfermer: () => void; children: Snippet; largeur?: number } = $props();

  let boite: HTMLDivElement;

  function surTouche(e: KeyboardEvent) {
    if (e.key !== 'Escape') return;
    e.stopImmediatePropagation();
    e.preventDefault();
    onfermer();
  }

  onMount(() => {
    // First field of the dialog gets the focus.
    boite.querySelector<HTMLElement>(`input, textarea, button:not([aria-label="${t('Fermer')}"])`)?.focus();
  });
</script>

<svelte:window onkeydowncapture={surTouche} />

<div class="fond fixed inset-0 z-[70] flex items-center justify-center p-6" role="presentation" onclick={(e) => e.target === e.currentTarget && onfermer()}>
  <div
    bind:this={boite}
    class="boite flex max-h-full w-full flex-col overflow-hidden rounded-2xl shadow-2xl"
    style:max-width="{largeur}px"
    role="dialog"
    aria-modal="true"
    aria-label={titre}
  >
    <div class="flex items-center justify-between px-5 pt-4">
      <h2 class="text-lg font-semibold">{titre}</h2>
      <IconButton icon={mdiClose} aria-label={t('Fermer')} variant="ghost" color="secondary" shape="round" onclick={onfermer} />
    </div>
    <div class="min-h-0 overflow-y-auto px-5 pb-5 pt-2">
      {@render children()}
    </div>
  </div>
</div>

<style>
  .fond {
    background: rgb(0 0 0 / 0.55);
  }
  .boite {
    background: var(--immich-ui-light, white);
    color: var(--immich-ui-dark, black);
  }
  :global(.dark) .boite {
    background: #1c1f24;
    color: #e8e8e8;
  }
</style>
