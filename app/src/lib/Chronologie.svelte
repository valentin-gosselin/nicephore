<script lang="ts">
  import { Heading, LoadingSpinner, Text } from '@immich/ui';
  import { mdiCheckCircle, mdiCheckboxBlankCircleOutline } from '@mdi/js';
  import { onMount, tick, type Snippet } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { etatPrechargement, signalerDefilement, type EtatPrechargement } from './api';
  import { listen } from '@tauri-apps/api/event';
  import { lancerBanc } from './banc';
  import { lancerScenarioEcriture } from './scenario-ecriture';

  // Benchmarks start once per app run, not on every visit of Photos.
  const lancements = globalThis as { __bancLance?: boolean; __scenarioLance?: boolean };
  import BarreSelection from './BarreSelection.svelte';
  import {
    Chronologie,
    HAUTEUR_TITRE_JOUR,
    HAUTEUR_TITRE_MOIS,
    type Position,
    type TitreVisible,
    type TuileVisible,
  } from './modele-chronologie.svelte';
  import { defilement as etatDefilement } from './defilement.svelte';
  import Frise from './Frise.svelte';
  import { ChronologieListe } from './modele-liste.svelte';
  import { Selection } from './selection.svelte';
  import Tuile from './Tuile.svelte';
  import Visionneuse from './Visionneuse.svelte';
  import type { Vue } from './vues';

  let {
    vue,
    modele,
    entete,
    titre,
  }: { vue: Vue; modele?: Chronologie; entete?: Snippet; titre?: Snippet } = $props();

  const MARGE = 16;
  // A ready-made model (search results...) or the timeline of the view.
  // svelte-ignore state_referenced_locally
  const chrono = modele ?? new Chronologie();
  // svelte-ignore state_referenced_locally
  if (!modele) chrono.filtre = vue.filtre;
  const liste = chrono instanceof ChronologieListe ? chrono : null;
  const selection = new Selection();

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

  // Load the months around the viewport. Month descriptions are small and the
  // queue only keeps the current window, so this also runs while flinging
  // (thumbnails are the ones held back, unless they are all on disk).
  $effect(() => {
    if (!pret) return;
    chrono.demander(defilement - hauteurVue, defilement + hauteurVue * 2, defilement + hauteurVue / 2);
  });

  // Fast-scroll detection: more than three screens per second, measured over
  // at least 50 ms (scroll events often come in bursts a few ms apart).
  let reference = { y: 0, t: 0 };
  let minuteurArret: ReturnType<typeof setTimeout> | undefined;
  let dernierSignal = 0;
  function surDefilement() {
    const y = conteneur.scrollTop;
    const t = performance.now();
    defilement = y;
    // Background thumbnail preloading pauses while the user scrolls.
    if (t - dernierSignal > 500) {
      dernierSignal = t;
      void signalerDefilement().catch(() => {});
    }
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
    const champ = e.target instanceof HTMLElement && e.target.closest('input, textarea');
    if (champ) return;
    if (e.ctrlKey && (e.key === '+' || e.key === '=')) zoomer(1.15);
    else if (e.ctrlKey && e.key === '-') zoomer(1 / 1.15);
    else if (e.ctrlKey && e.key === '0') {
      hauteurCible = 220;
      void garderAncre(() => chrono.redisposer(largeurVue - MARGE * 2, hauteurCible));
    } else if (e.ctrlKey && e.key.toLowerCase() === 'a') selection.ajouter(chrono.chargees());
    else if (e.key === 'Escape' && selection.active) selection.vider();
    else if (e.key === 'Delete' && selection.active && vue.id !== 'corbeille') {
      document.querySelector<HTMLButtonElement>('[aria-label^="Mettre à la corbeille"]')?.click();
    } else if (e.key === 'Home') allerA(0, false);
    else if (e.key === 'End') allerA(chrono.hauteurTotale, false);
    else return;
    e.preventDefault();
  }

  let diaporama = $state(false);

  function ouvrir(tuile: TuileVisible) {
    diaporama = false;
    ouverte = { section: tuile.section, index: tuile.index };
  }

  /** Opens the viewer on the first item, as a slideshow (memories). */
  export function lancerDiaporama() {
    const premiere = chrono.sections.findIndex((s) => s.assets.length > 0);
    if (premiere < 0) return;
    diaporama = true;
    ouverte = { section: premiere, index: 0 };
  }

  /** Toggles one asset; with shift, selects the whole range from the last click. */
  function basculer(tuile: TuileVisible, plage: boolean) {
    const position = { section: tuile.section, index: tuile.index };
    if (plage && selection.ancre) {
      selection.ajouter(chrono.entre(selection.ancre, position));
      selection.ancre = position;
    } else {
      selection.basculer(tuile.asset, position);
    }
  }

  /** Day title checkbox: selects (or deselects) every asset of that day. */
  function jourSelectionne(titre: TitreVisible) {
    const assets = chrono.sections[titre.section]?.assets.slice(titre.debut, titre.fin) ?? [];
    return assets.length > 0 && assets.every((a) => selection.contient(a.id));
  }

  function basculerJour(titre: TitreVisible) {
    const assets = chrono.sections[titre.section].assets.slice(titre.debut, titre.fin);
    if (jourSelectionne(titre)) selection.retirerTous(assets);
    else selection.ajouter(assets);
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

    // All thumbnails on disk: no need to hold them back while flinging.
    const suivre = (e: EtatPrechargement) => {
      etatDefilement.vignettesLocales = e.actif && e.complet;
    };
    void etatPrechargement().then(suivre).catch(() => {});
    const arretPrechargement = listen<EtatPrechargement>('prechargement', ({ payload }) => suivre(payload));

    const debutChargement = performance.now();
    largeurVue = conteneur.clientWidth;
    chrono.largeur = largeurVue - MARGE * 2;
    chrono.hauteurCible = hauteurCible;
    chrono
      .demarrer()
      .then(async () => {
        pret = true;
        await tick();
        const banc = vue.id === 'photos' && (await invoke<boolean>('mode_banc'));
        if (banc && (await invoke<string>('scenario_banc')) === 'ecriture') {
          if (!lancements.__scenarioLance) {
            lancements.__scenarioLance = true;
            void lancerScenarioEcriture();
          }
        } else if (banc && !lancements.__bancLance) {
          lancements.__bancLance = true;
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
              nombreSelectionnes: () => selection.taille,
              etatMois: () => {
                const compte: Record<string, number> = {};
                for (const m of chrono.sections) compte[m.etat] = (compte[m.etat] ?? 0) + 1;
                const visible = chrono.sections[chrono.sectionA(defilement)];
                return `${JSON.stringify(compte)} ; à l'écran ${visible?.cle} (${visible?.etat}), local ${etatDefilement.vignettesLocales}, rapide ${etatDefilement.rapide}`;
              },
            },
            debutChargement,
          );
        }
      })
      .catch((e) => (erreur = String(e)));

    return () => {
      observateur.disconnect();
      void arretPrechargement.then((f) => f());
    };
  });
</script>

<svelte:window onkeydown={surTouche} />

<div class="flex h-full min-h-0 flex-col">
  <div class="entete shrink-0 border-b border-gray-200 dark:border-gray-800">
    {#if selection.active}
      <BarreSelection {vue} {selection} {chrono} />
    {:else}
      <div class="flex h-14 items-center gap-3 px-5">
        {#if titre}{@render titre()}{:else}<Heading size="small">{vue.titre}</Heading>{/if}
        {#if pret}
          <Text size="small" color="muted">
            {chrono.nombreTotal.toLocaleString('fr-FR')}{liste?.plusDeResultats ? '+' : ''}
            {liste && vue.id === 'recherche' ? 'résultat' : 'élément'}{chrono.nombreTotal > 1 ? 's' : ''}
          </Text>
        {/if}
        {#if entete}{@render entete()}{/if}
      </div>
    {/if}
  </div>

  <div class="flex min-h-0 flex-1">
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
      {:else if chrono.nombreTotal === 0}
        <div class="flex h-full items-center justify-center p-8"><Text color="muted">{vue.vide}</Text></div>
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
            class="titre absolute left-0 right-0 flex items-end gap-1.5"
            class:titre-mois={titre.mois}
            class:titre-jour={!titre.mois}
            class:mode-selection={selection.active}
            style:transform="translateY({titre.y}px)"
            style:height="{titre.mois ? HAUTEUR_TITRE_MOIS : HAUTEUR_TITRE_JOUR}px"
          >
            {#if !titre.mois}
              <button
                type="button"
                class="coche-jour"
                class:cochee={jourSelectionne(titre)}
                aria-label="Sélectionner la journée"
                onclick={() => basculerJour(titre)}
              >
                <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
                  <path fill="currentColor" d={jourSelectionne(titre) ? mdiCheckCircle : mdiCheckboxBlankCircleOutline} />
                </svg>
              </button>
            {/if}
            {titre.libelle}
          </div>
        {/each}

        {#each fenetre.tuiles as tuile (`${tuile.section}:${tuile.asset.id}`)}
          <Tuile
            {tuile}
            selectionnee={selection.contient(tuile.asset.id)}
            modeSelection={selection.active}
            onouvrir={ouvrir}
            onbasculer={basculer}
          />
        {/each}
      </div>
    </div>

    {#if pret && chrono.nombreTotal > 0 && chrono.afficherFrise}
      <Frise {chrono} {defilement} {hauteurVue} onaller={allerA} />
    {/if}
  </div>
</div>

{#if ouverte}
  <Visionneuse {chrono} depart={ouverte} onfermer={fermer} {diaporama} />
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
  }
  .titre-jour {
    color: color-mix(in oklab, currentColor 75%, transparent);
  }
  .coche-jour {
    display: flex;
    width: 0;
    overflow: hidden;
    opacity: 0;
    transition:
      width 120ms,
      opacity 120ms;
  }
  .titre-jour:hover .coche-jour,
  .mode-selection .coche-jour,
  .coche-jour:focus-visible {
    width: 18px;
    opacity: 1;
  }
  .coche-jour.cochee {
    color: var(--immich-ui-primary-500);
  }
  .attente {
    background: repeating-linear-gradient(
      90deg,
      color-mix(in oklab, currentColor 6%, transparent) 0 180px,
      transparent 180px 184px
    );
  }
</style>
