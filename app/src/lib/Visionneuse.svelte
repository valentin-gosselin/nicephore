<script lang="ts">
  import { IconButton, LoadingSpinner } from '@immich/ui';
  import { mdiChevronLeft, mdiChevronRight, mdiClose, mdiFullscreen, mdiInformationOutline } from '@mdi/js';
  import { onMount } from 'svelte';
  import { urlImage, urlVideo } from './api';
  import type { Chronologie, Position } from './modele-chronologie.svelte';
  import PanneauInfos from './PanneauInfos.svelte';

  let {
    chrono,
    depart,
    onfermer,
  }: { chrono: Chronologie; depart: Position; onfermer: (p: Position) => void } = $props();

  let racine: HTMLDivElement;
  // The viewer starts on `depart`, then navigates on its own.
  // svelte-ignore state_referenced_locally
  let position = $state<Position>(depart);
  let infos = $state(false);
  let video = $state<string | null>(null);
  let apercuCharge = $state(false);
  let pleineCharge = $state(false);
  // Zoom and pan state.
  let echelle = $state(1);
  let decalage = $state({ x: 0, y: 0 });
  let glisse: { x: number; y: number } | null = null;

  const asset = $derived(chrono.asset(position));
  // Two levels: a screen-sized preview first, full resolution only once zoomed.
  const veutPleine = $derived(echelle > 1.25);

  $effect(() => {
    const courant = asset;
    video = null;
    apercuCharge = false;
    pleineCharge = false;
    echelle = 1;
    decalage = { x: 0, y: 0 };
    if (courant && !courant.image) {
      urlVideo(courant.id).then((url) => {
        if (asset?.id === courant.id) video = url;
      });
    }
    // Preload the neighbours' previews.
    for (const sens of [1, -1] as const) {
      void chrono.voisin(position, sens).then((p) => {
        const voisin = p && chrono.asset(p);
        if (voisin?.image) new Image().src = urlImage('apercu', voisin.id, voisin.thumbhash);
      });
    }
  });

  async function aller(sens: 1 | -1) {
    const suivante = await chrono.voisin(position, sens);
    if (suivante) position = suivante;
  }

  function surTouche(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      if (document.fullscreenElement) return;
      onfermer(position);
    } else if (e.key === 'ArrowRight') void aller(1);
    else if (e.key === 'ArrowLeft') void aller(-1);
    else if (e.key === 'i') infos = !infos;
    else if (e.key === 'f') basculerPleinEcran();
    else return;
    e.preventDefault();
    e.stopPropagation();
  }

  function basculerPleinEcran() {
    if (document.fullscreenElement) void document.exitFullscreen();
    else void racine.requestFullscreen?.();
  }

  function surMolette(e: WheelEvent) {
    if (!asset?.image) return;
    e.preventDefault();
    const boite = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const facteur = e.deltaY < 0 ? 1.2 : 1 / 1.2;
    const nouvelle = Math.min(8, Math.max(1, echelle * facteur));
    // Zoom around the pointer.
    const px = e.clientX - boite.left - boite.width / 2;
    const py = e.clientY - boite.top - boite.height / 2;
    const k = nouvelle / echelle;
    decalage = nouvelle === 1 ? { x: 0, y: 0 } : { x: px - (px - decalage.x) * k, y: py - (py - decalage.y) * k };
    echelle = nouvelle;
  }

  function surDoubleClic() {
    if (echelle > 1) {
      echelle = 1;
      decalage = { x: 0, y: 0 };
    } else echelle = 2.5;
  }

  onMount(() => racine.focus());
</script>

<svelte:window onkeydown={surTouche} />

<div
  bind:this={racine}
  class="visionneuse fixed inset-0 z-50 flex bg-black text-white outline-none"
  role="dialog"
  aria-modal="true"
  tabindex="-1"
>
  <div class="relative flex min-w-0 flex-1 items-center justify-center overflow-hidden">
    {#if asset}
      {#key asset.id}
        {#if asset.image}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="scene absolute inset-0 flex items-center justify-center"
            class:zoomee={echelle > 1}
            onwheel={surMolette}
            ondblclick={surDoubleClic}
            onpointerdown={(e) => {
              if (echelle === 1) return;
              glisse = { x: e.clientX - decalage.x, y: e.clientY - decalage.y };
              (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
            }}
            onpointermove={(e) => glisse && (decalage = { x: e.clientX - glisse.x, y: e.clientY - glisse.y })}
            onpointerup={() => (glisse = null)}
          >
            <div
              class="calque"
              style:transform="translate({decalage.x}px, {decalage.y}px) scale({echelle})"
              style:aspect-ratio={asset.ratio}
            >
              <img class="image flou" src={urlImage('vignette', asset.id, asset.thumbhash)} alt="" draggable="false" />
              <img
                class="image"
                class:opacity-0={!apercuCharge}
                src={urlImage('apercu', asset.id, asset.thumbhash)}
                alt=""
                draggable="false"
                onload={() => (apercuCharge = true)}
              />
              {#if veutPleine}
                <img
                  class="image"
                  class:opacity-0={!pleineCharge}
                  src={urlImage('pleine', asset.id, asset.thumbhash)}
                  alt=""
                  draggable="false"
                  onload={() => (pleineCharge = true)}
                />
              {/if}
            </div>
          </div>
          {#if !apercuCharge || (veutPleine && !pleineCharge)}
            <div class="absolute bottom-6 left-1/2 -translate-x-1/2"><LoadingSpinner /></div>
          {/if}
        {:else if video}
          <!-- svelte-ignore a11y_media_has_caption -->
          <video class="max-h-full max-w-full" src={video} controls autoplay></video>
        {:else}
          <LoadingSpinner size="giant" />
        {/if}
      {/key}
    {/if}

    <div class="barre absolute left-0 right-0 top-0 flex justify-between p-3">
      <IconButton icon={mdiClose} aria-label="Fermer" variant="ghost" color="secondary" shape="round" onclick={() => onfermer(position)} />
      <div class="flex gap-1">
        <IconButton icon={mdiFullscreen} aria-label="Plein écran (F)" variant="ghost" color="secondary" shape="round" onclick={basculerPleinEcran} />
        <IconButton icon={mdiInformationOutline} aria-label="Infos (I)" variant="ghost" color="secondary" shape="round" onclick={() => (infos = !infos)} />
      </div>
    </div>
    <button class="fleche left-2" aria-label="Précédente" onclick={() => aller(-1)}>
      <svg viewBox="0 0 24 24" width="36" height="36"><path fill="currentColor" d={mdiChevronLeft} /></svg>
    </button>
    <button class="fleche right-2" aria-label="Suivante" onclick={() => aller(1)}>
      <svg viewBox="0 0 24 24" width="36" height="36"><path fill="currentColor" d={mdiChevronRight} /></svg>
    </button>
  </div>

  {#if infos && asset}
    <PanneauInfos id={asset.id} />
  {/if}
</div>

<style>
  .scene {
    cursor: zoom-in;
  }
  .scene.zoomee {
    cursor: grab;
  }
  .calque {
    position: relative;
    max-width: 100%;
    max-height: 100%;
    height: 100%;
    transform-origin: center;
    will-change: transform;
  }
  .image {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: contain;
    transition: opacity 150ms;
    user-select: none;
  }
  .flou {
    filter: blur(8px);
  }
  .barre {
    background: linear-gradient(to bottom, rgb(0 0 0 / 0.55), transparent);
  }
  .fleche {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    border-radius: 9999px;
    padding: 8px;
    color: white;
    background: rgb(0 0 0 / 0.25);
    opacity: 0.6;
    transition: opacity 120ms;
  }
  .fleche:hover {
    opacity: 1;
  }
</style>
