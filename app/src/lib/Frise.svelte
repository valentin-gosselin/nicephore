<script lang="ts">
  // Scrubber: a vertical rail with the years, proportional to the timeline.
  // Hover shows the month under the pointer; click or drag jumps there.
  import type { Chronologie } from './modele-chronologie.svelte';

  let {
    chrono,
    defilement,
    hauteurVue,
    onaller,
  }: {
    chrono: Chronologie;
    defilement: number;
    hauteurVue: number;
    onaller: (y: number, glisse: boolean) => void;
  } = $props();

  let rail: HTMLDivElement;
  let hauteurRail = $state(0);
  let survol = $state<{ y: number; libelle: string } | null>(null);
  let glisse = $state(false);

  const total = $derived(Math.max(1, chrono.hauteurTotale - hauteurVue));
  const marques = $derived.by(() => {
    chrono.revision;
    const brutes = chrono.annees();
    // Keep labels at least 16 px apart.
    const gardees: { annee: number; y: number }[] = [];
    let derniere = -Infinity;
    for (const m of brutes) {
      const y = (m.haut / Math.max(1, chrono.hauteurTotale)) * hauteurRail;
      if (y - derniere >= 16) {
        gardees.push({ annee: m.annee, y });
        derniere = y;
      }
    }
    return gardees;
  });
  const curseur = $derived((defilement / total) * hauteurRail);

  function versPosition(clientY: number) {
    const boite = rail.getBoundingClientRect();
    const ratio = Math.min(1, Math.max(0, (clientY - boite.top) / boite.height));
    return ratio * chrono.hauteurTotale;
  }

  function libelleA(position: number) {
    const section = chrono.sections[chrono.sectionA(position)];
    return section?.libelle ?? '';
  }

  function deplacer(e: PointerEvent) {
    const position = versPosition(e.clientY);
    survol = { y: e.clientY - rail.getBoundingClientRect().top, libelle: libelleA(position) };
    if (glisse) onaller(Math.min(position, total), true);
  }
</script>

<div
  bind:this={rail}
  bind:clientHeight={hauteurRail}
  class="frise relative h-full w-14 shrink-0 select-none"
  role="scrollbar"
  aria-controls="chronologie"
  aria-valuenow={Math.round((defilement / total) * 100)}
  aria-orientation="vertical"
  tabindex="-1"
  onpointerdown={(e) => {
    glisse = true;
    rail.setPointerCapture(e.pointerId);
    onaller(Math.min(versPosition(e.clientY), total), true);
  }}
  onpointermove={deplacer}
  onpointerup={(e) => {
    glisse = false;
    rail.releasePointerCapture(e.pointerId);
    onaller(Math.min(versPosition(e.clientY), total), false);
  }}
  onpointerleave={() => !glisse && (survol = null)}
>
  {#each marques as m (m.annee)}
    <span class="annee absolute right-2 text-[11px] text-gray-500 dark:text-gray-400" style:top="{m.y}px">
      {m.annee}
    </span>
  {/each}
  <span class="absolute right-0 h-0.5 w-6 bg-primary" style:top="{curseur}px"></span>
  {#if survol}
    <span
      class="bulle pointer-events-none absolute right-14 whitespace-nowrap rounded-md bg-primary px-2 py-1 text-xs font-medium text-light"
      style:top="{survol.y - 12}px"
    >
      {survol.libelle}
    </span>
  {/if}
</div>

<style>
  .frise {
    cursor: ns-resize;
    touch-action: none;
  }
  .annee {
    transform: translateY(-50%);
  }
</style>
