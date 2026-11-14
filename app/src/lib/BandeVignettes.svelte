<script lang="ts" module>
  import { vignetteLocale } from './api';

  const VIDEO = /\.(mp4|mov|m4v|webm|mkv|avi|3gp|mts|m2ts)$/i;
  /** Thumbnails already made, kept while the app runs (paths of small JPEGs). */
  const faites = new Map<string, string | null>();
  const images = $state<Record<string, string | null>>({});

  function charger(chemin: string) {
    if (chemin in images) return;
    if (faites.has(chemin)) {
      images[chemin] = faites.get(chemin) ?? null;
      return;
    }
    images[chemin] = null;
    if (VIDEO.test(chemin)) return;
    vignetteLocale(chemin)
      .then((v) => {
        faites.set(chemin, v);
        images[chemin] = v;
      })
      .catch(() => faites.set(chemin, null));
  }

  /** The photo changed (turned): its thumbnail made again, the old one shown meanwhile. */
  export async function oublierVignette(chemin: string) {
    faites.delete(chemin);
    const v = await vignetteLocale(chemin).catch(() => null);
    faites.set(chemin, v);
    if (chemin in images) images[chemin] = v;
  }
</script>

<script lang="ts">
  // The photos of the folder in small, under the local viewer: one click
  // jumps to one of them. Thumbnails are made only for the visible tiles.
  import { mdiImageOutline, mdiVideoOutline } from '@mdi/js';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { t } from './i18n.svelte';

  let { voisins, index, onchoisir }: { voisins: string[]; index: number; onchoisir: (i: number) => void } = $props();

  let bande: HTMLDivElement;
  const nom = (chemin: string) => chemin.split(/[\\/]/).pop() ?? chemin;

  /** Thumbnails made when their tile comes near the visible part. */
  function observer(tuile: HTMLElement, chemin: string) {
    const guet = new IntersectionObserver(
      (entrees) => {
        if (entrees.some((e) => e.isIntersecting)) {
          charger(chemin);
          guet.disconnect();
        }
      },
      { root: bande, rootMargin: '0px 400px' },
    );
    guet.observe(tuile);
    return { destroy: () => guet.disconnect() };
  }

  // The current photo stays in view.
  $effect(() => {
    const courante = bande?.children[index] as HTMLElement | undefined;
    courante?.scrollIntoView({ block: 'nearest', inline: 'center', behavior: 'smooth' });
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  bind:this={bande}
  class="bande flex h-full items-center gap-1.5 overflow-x-auto px-3"
  role="listbox"
  aria-label={t('Photos du dossier')}
  onwheel={(e) => {
    if (Math.abs(e.deltaY) > Math.abs(e.deltaX)) {
      e.preventDefault();
      bande.scrollLeft += e.deltaY;
    }
  }}
>
  {#each voisins as chemin, i (chemin)}
    <button
      type="button"
      class="tuile"
      class:courante={i === index}
      role="option"
      aria-selected={i === index}
      title={nom(chemin)}
      onclick={() => onchoisir(i)}
      use:observer={chemin}
    >
      {#if images[chemin]}
        <img src={convertFileSrc(images[chemin] ?? '')} alt="" draggable="false" />
      {:else}
        <svg viewBox="0 0 24 24" width="22" height="22" aria-hidden="true">
          <path fill="currentColor" d={VIDEO.test(chemin) ? mdiVideoOutline : mdiImageOutline} />
        </svg>
      {/if}
    </button>
  {/each}
</div>

<style>
  .bande {
    scrollbar-width: none;
  }
  .bande::-webkit-scrollbar {
    display: none;
  }
  .tuile {
    flex-shrink: 0;
    width: 58px;
    height: 58px;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    border-radius: 8px;
    background: rgb(255 255 255 / 0.06);
    color: rgb(255 255 255 / 0.5);
    opacity: 0.55;
    outline: 2px solid transparent;
    outline-offset: -2px;
    transition:
      opacity 120ms,
      outline-color 120ms;
  }
  .tuile:hover,
  .tuile:focus-visible {
    opacity: 0.9;
  }
  .tuile.courante {
    opacity: 1;
    outline-color: white;
  }
  .tuile img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
</style>
