<script lang="ts">
  import { LoadingSpinner, Text } from '@immich/ui';
  import { onMount, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listeMois } from './api';
  import { lancerBanc } from './banc';
  import { Chronologie, HAUTEUR_TITRE_JOUR, HAUTEUR_TITRE_MOIS, type Position, type TuileVisible } from './modele-chronologie.svelte';
  import { defilement as etatDefilement } from './defilement.svelte';
  import Frise from './Frise.svelte';
  import Tuile from './Tuile.svelte';
  import Visionneuse from './Visionneuse.svelte';

  let { onnombre }: { onnombre?: (n: number) => void } = $props();

  const MARGE = 16;
  const chrono = new Chronologie();

  let conteneur: HTMLDivElement;
  let defilement = $state(0);
  let hauteurVue = $state(800);
  let largeurVue = $state(0);
  let hauteurCible = $state(220);
  let erreur = $state<string | null>(null);
  let pret = $state(false);
  let ouverte = $state<Position | null>(null);

  // Rendering window: half a screen behind, one and a half ahead.
  const fenetre = $derived.by(() => {
    chrono.revision;
    return chrono.visibles(defilement - hauteurVue * 0.5, defilement + hauteurVue * 1.5);
  });

  /** Runs a layout change while keeping the content under the top edge still. */
  async function garderAncre(changement: () => void) {
    const ancre = chrono.ancre(conteneur.scrollTop);
    changement();
    await tick();
    const cible = chrono.positionAncre(ancre);
    if (Math.abs(cible - conteneur.scrollTop) > 0.5) {
      conteneur.scrollTop = cible;
      defilement = cible;
    }
  }

  chrono.surChangement = () => {
    // A month just got its real height: keep the anchor.
    const ancre = chrono.ancre(defilement);
    void tick().then(() => {
      const cible = chrono.positionAncre(ancre);
      if (conteneur && Math.abs(cible - conteneur.scrollTop) > 0.5) {
        conteneur.scrollTop = cible;
        defilement = cible;
      }
    });
  };

  // Load the months around the viewport, unless the user is flinging.
  $effect(() => {
    if (!pret || etatDefilement.rapide) return;
    chrono.demander(defilement - hauteurVue, defilement + hauteurVue * 2, defilement + hauteurVue / 2);
  });

  // Fast-scroll detection: more than three screens per second, measured over
  // at least 50 ms (scroll events often come in bursts a few ms apart).
  let reference = { y: 0, t: 0 };
  let minuteurArret: ReturnType<typeof setTimeout> | undefined;
  function surDefilement() {
    const y = conteneur.scrollTop;
    const t = performance.now();
    defilement = y;
    const ecoule = t - reference.t;
    if (ecoule >= 50) {
      const ecransParSeconde = (Math.abs(y - reference.y) / ecoule) * (1000 / hauteurVue);
      etatDefilement.rapide = ecransParSeconde > 3;
      reference = { y, t };
    }
    clearTimeout(minuteurArret);
    minuteurArret = setTimeout(() => {
      etatDefilement.rapide = false;
      reference = { y: conteneur.scrollTop, t: performance.now() };
    }, 150);
  }

  function allerA(y: number, glisse: boolean) {
    etatDefilement.rapide = glisse;
    conteneur.scrollTop = y;
    defilement = y;
    if (!glisse) etatDefilement.rapide = false;
  }

  function zoomer(facteur: number) {
    const nouvelle = Math.round(Math.min(480, Math.max(96, hauteurCible * facteur)));
    if (nouvelle === hauteurCible) return;
    hauteurCible = nouvelle;
    void garderAncre(() => chrono.redisposer(largeurVue - MARGE * 2, hauteurCible));
  }

  function surMolette(e: WheelEvent) {
    if (!e.ctrlKey) return;
    e.preventDefault();
    zoomer(e.deltaY < 0 ? 1.1 : 1 / 1.1);
  }

  function surTouche(e: KeyboardEvent) {
    if (ouverte) return;
    if (e.ctrlKey && (e.key === '+' || e.key === '=')) zoomer(1.15);
    else if (e.ctrlKey && e.key === '-') zoomer(1 / 1.15);
    else if (e.ctrlKey && e.key === '0') {
      hauteurCible = 220;
      void garderAncre(() => chrono.redisposer(largeurVue - MARGE * 2, hauteurCible));
    } else if (e.key === 'Home') allerA(0, false);
    else if (e.key === 'End') allerA(chrono.hauteurTotale, false);
    else return;
    e.preventDefault();
  }

  function ouvrir(tuile: TuileVisible) {
    ouverte = { section: tuile.section, index: tuile.index };
  }

  function fermer(position: Position) {
    ouverte = null;
    // Bring the last viewed photo into view if it is off screen.
    const y = chrono.hautAsset(position);
    if (y != null && (y < defilement || y > defilement + hauteurVue - 100)) {
      allerA(Math.max(0, y - hauteurVue / 3), false);
    }
    conteneur.focus();
  }

  onMount(() => {
    const observateur = new ResizeObserver(() => {
      const largeur = conteneur.clientWidth;
      hauteurVue = conteneur.clientHeight;
      if (largeur === largeurVue) return;
      largeurVue = largeur;
      if (pret) void garderAncre(() => chrono.redisposer(largeurVue - MARGE * 2, hauteurCible));
    });
    observateur.observe(conteneur);

    const debutChargement = performance.now();
    listeMois()
      .then(async (mois) => {
        largeurVue = conteneur.clientWidth;
        chrono.largeur = largeurVue - MARGE * 2;
        chrono.hauteurCible = hauteurCible;
        chrono.initialiser(mois);
        onnombre?.(chrono.nombreTotal);
        pret = true;
        await tick();
        if (await invoke<boolean>('mode_banc')) {
          const touche = (key: string) => window.dispatchEvent(new KeyboardEvent('keydown', { key }));
          void lancerBanc(
            {
              conteneur,
              hauteurTotale: () => chrono.hauteurTotale,
              allerA: (y) => allerA(y, false),
              ouvrirPremiere: () => {
                const premiere = fenetre.tuiles.find((t) => t.y >= defilement);
                if (premiere) ouvrir(premiere);
                return !!premiere;
              },
              ouvrirPremiereVideo: () => {
                const video = fenetre.tuiles.find((t) => t.y >= defilement && !t.asset.image);
                if (video) ouvrir(video);
                return !!video;
              },
              basculerInfos: () => touche('i'),
              fermer: () => touche('Escape'),
            },
            debutChargement,
          );
        }
      })
      .catch((e) => (erreur = String(e)));

    return () => observateur.disconnect();
  });
</script>

<svelte:window onkeydown={surTouche} />

<div class="flex h-full min-h-0">
  <div
    id="chronologie"
    bind:this={conteneur}
    class="chronologie relative h-full min-w-0 flex-1 overflow-y-auto outline-none"
    onscroll={surDefilement}
    onwheel={surMolette}
    tabindex="-1"
  >
    {#if erreur}
      <div class="p-8"><Text color="danger">Impossible de charger la chronologie : {erreur}</Text></div>
    {:else if !pret}
      <div class="flex h-full items-center justify-center"><LoadingSpinner size="giant" /></div>
    {/if}

    <div class="relative" style:height="{chrono.hauteurTotale}px" style:margin="0 {MARGE}px">
      {#each fenetre.attentes as a (a.cle)}
        <div
          class="attente absolute left-0 right-0 rounded-lg"
          style:transform="translateY({a.y}px)"
          style:height="{a.hauteur}px"
        ></div>
      {/each}

      {#each fenetre.titres as titre (titre.cle)}
        <div
          class="absolute left-0 right-0 flex items-end"
          class:titre-mois={titre.mois}
          class:titre-jour={!titre.mois}
          style:transform="translateY({titre.y}px)"
          style:height="{titre.mois ? HAUTEUR_TITRE_MOIS : HAUTEUR_TITRE_JOUR}px"
        >
          {titre.libelle}
        </div>
      {/each}

      {#each fenetre.tuiles as tuile (`${tuile.section}:${tuile.asset.id}`)}
        <Tuile {tuile} onouvrir={ouvrir} />
      {/each}
    </div>
  </div>

  {#if pret}
    <Frise {chrono} {defilement} {hauteurVue} onaller={allerA} />
  {/if}
</div>

{#if ouverte}
  <Visionneuse {chrono} depart={ouverte} onfermer={fermer} />
{/if}

<style>
  .chronologie {
    scrollbar-width: none;
  }
  .chronologie::-webkit-scrollbar {
    display: none;
  }
  .titre-mois {
    padding-bottom: 10px;
    font-size: 20px;
    font-weight: 600;
  }
  .titre-jour {
    padding-bottom: 8px;
    font-size: 13px;
    font-weight: 500;
    opacity: 0.75;
  }
  .attente {
    background: repeating-linear-gradient(
      90deg,
      color-mix(in oklab, currentColor 6%, transparent) 0 180px,
      transparent 180px 184px
    );
  }
</style>
