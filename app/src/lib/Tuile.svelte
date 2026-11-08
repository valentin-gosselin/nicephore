<script lang="ts">
  import { mdiCheckCircle, mdiCheckboxBlankCircleOutline, mdiHeart, mdiPlay } from '@mdi/js';
  import { urlImage } from './api';
  import type { TuileVisible } from './modele-chronologie.svelte';
  import { defilement } from './defilement.svelte';
  import { apercuFlou, dejaAffichees } from './miniatures';

  let {
    tuile,
    selectionnee,
    modeSelection,
    onouvrir,
    onbasculer,
  }: {
    tuile: TuileVisible;
    selectionnee: boolean;
    /** At least one asset is selected: a click selects instead of opening. */
    modeSelection: boolean;
    onouvrir: (t: TuileVisible) => void;
    onbasculer: (t: TuileVisible, plage: boolean) => void;
  } = $props();

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
    if (dejaAffichees.has(voulue) || (taille === 'vignette' && defilement.vignettesLocales)) {
      src = voulue;
      return;
    }
    if (defilement.rapide) return;
    const minuteur = setTimeout(() => (src = voulue), 120);
    return () => clearTimeout(minuteur);
  });

  function surClic(e: MouseEvent) {
    if (modeSelection || e.ctrlKey || e.metaKey) onbasculer(tuile, e.shiftKey);
    else onouvrir(tuile);
  }
</script>

<div
  class="tuile absolute overflow-hidden"
  class:selectionnee
  class:mode-selection={modeSelection}
  style:transform="translate({tuile.x}px, {tuile.y}px)"
  style:width="{tuile.largeur}px"
  style:height="{tuile.hauteur}px"
>
  <button
    type="button"
    class="image-tuile bg-gray-200 dark:bg-gray-800"
    style:background-image={flou ? `url(${flou})` : undefined}
    onclick={surClic}
    aria-label={asset.image ? 'Photo' : 'Vidéo'}
    aria-pressed={modeSelection ? selectionnee : undefined}
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
        <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true"><path fill="currentColor" d={mdiPlay} /></svg>
      </span>
    {/if}
    {#if asset.favori}
      <span class="badge bottom-1.5 left-1.5">
        <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true"><path fill="currentColor" d={mdiHeart} /></svg>
      </span>
    {/if}
  </button>
  <button
    type="button"
    class="coche"
    aria-label={selectionnee ? 'Désélectionner' : 'Sélectionner'}
    onclick={(e) => onbasculer(tuile, e.shiftKey)}
  >
    <!-- Plain SVG: one icon component per tile costs frames while scrolling. -->
    <svg viewBox="0 0 24 24" width="24" height="24" aria-hidden="true">
      <path fill="currentColor" d={selectionnee ? mdiCheckCircle : mdiCheckboxBlankCircleOutline} />
    </svg>
  </button>
</div>

<style>
  .tuile {
    left: 0;
    top: 0;
    contain: strict;
  }
  .image-tuile {
    display: block;
    width: 100%;
    height: 100%;
    background-size: cover;
    border: 0;
    padding: 0;
    cursor: pointer;
    transition:
      transform 120ms ease,
      border-radius 120ms ease;
  }
  .tuile:hover .image-tuile img {
    filter: brightness(1.08);
  }
  .image-tuile:focus-visible {
    outline: 3px solid var(--immich-ui-primary-500);
    outline-offset: -3px;
  }
  .selectionnee .image-tuile {
    transform: scale(0.88);
    border-radius: 10px;
  }
  .selectionnee {
    background: color-mix(in oklab, var(--immich-ui-primary-500) 18%, transparent);
  }
  .coche {
    position: absolute;
    top: 6px;
    left: 6px;
    display: flex;
    color: white;
    opacity: 0;
    filter: drop-shadow(0 1px 2px rgb(0 0 0 / 0.7));
    transition: opacity 120ms;
  }
  .tuile:hover .coche,
  .mode-selection .coche,
  .coche:focus-visible {
    opacity: 1;
  }
  .selectionnee .coche {
    color: var(--immich-ui-primary-500);
    filter: drop-shadow(0 0 2px white);
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
