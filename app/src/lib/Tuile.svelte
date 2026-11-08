<script lang="ts">
  import { mdiHeart, mdiPlay } from '@mdi/js';
  import { Icon } from '@immich/ui';
  import { urlImage } from './api';
  import type { TuileVisible } from './modele-chronologie.svelte';
  import { defilement } from './defilement.svelte';
  import { apercuFlou, dejaAffichees } from './miniatures';

  let { tuile, onouvrir }: { tuile: TuileVisible; onouvrir: (t: TuileVisible) => void } = $props();

  const asset = $derived(tuile.asset);
  // Large tiles (zoomed grid) use the preview rendering to stay sharp.
  const taille = $derived(tuile.hauteur * devicePixelRatio > 320 ? 'apercu' : 'vignette');
  const url = $derived(urlImage(taille, asset.id, asset.thumbhash));
  const flou = $derived(apercuFlou(asset.thumbhash));

  let src = $state<string | null>(null);
  let chargee = $state(false);

  // Dwell delay: only request the image once the tile stayed on screen a
  // little and the user is not flinging through the timeline.
  $effect(() => {
    const voulue = url;
    if (dejaAffichees.has(voulue)) {
      src = voulue;
      return;
    }
    if (defilement.rapide) return;
    const minuteur = setTimeout(() => (src = voulue), 120);
    return () => clearTimeout(minuteur);
  });
</script>

<button
  type="button"
  class="tuile absolute overflow-hidden bg-gray-200 dark:bg-gray-800"
  style:transform="translate({tuile.x}px, {tuile.y}px)"
  style:width="{tuile.largeur}px"
  style:height="{tuile.hauteur}px"
  style:background-image={flou ? `url(${flou})` : undefined}
  onclick={() => onouvrir(tuile)}
  aria-label={asset.image ? 'Photo' : 'Vidéo'}
>
  {#if src}
    <img
      {src}
      alt=""
      draggable="false"
      decoding="async"
      class="h-full w-full object-cover transition-opacity duration-200"
      class:opacity-0={!chargee}
      onload={() => {
        chargee = true;
        dejaAffichees.add(src!);
      }}
    />
  {/if}
  {#if !asset.image}
    <span class="badge right-1.5 top-1.5">
      {#if asset.duree}{asset.duree}{/if}
      <Icon icon={mdiPlay} size="16" />
    </span>
  {/if}
  {#if asset.favori}
    <span class="badge bottom-1.5 left-1.5"><Icon icon={mdiHeart} size="14" /></span>
  {/if}
</button>

<style>
  .tuile {
    left: 0;
    top: 0;
    background-size: cover;
    border: 0;
    padding: 0;
    cursor: pointer;
    contain: strict;
  }
  .tuile:hover img {
    filter: brightness(1.08);
  }
  .tuile:focus-visible {
    outline: 3px solid var(--immich-ui-primary-500);
    outline-offset: -3px;
  }
  .badge {
    position: absolute;
    display: flex;
    align-items: center;
    gap: 2px;
    color: white;
    font-size: 12px;
    font-weight: 600;
    filter: drop-shadow(0 1px 2px rgb(0 0 0 / 0.8));
  }
</style>
