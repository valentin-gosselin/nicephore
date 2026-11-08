<script lang="ts">
  // Map of the geotagged photos, with the map styles configured on the
  // Immich server (Immich's own tile server by default). Markers are
  // clustered; a click on a cluster or a photo lists them in a side panel
  // with the usual grid, viewer and selection.
  import { IconButton, LoadingSpinner, Text } from '@immich/ui';
  import { mdiClose } from '@mdi/js';
  import { LngLatBounds, Map as Carte, NavigationControl, type GeoJSONSource } from '../../lib/maplibre';
  import { invoke } from '@tauri-apps/api/core';
  import { onMount, tick } from 'svelte';
  import { assetsParIds, marqueurs as chargerMarqueurs, stylesCarte, type Marqueur, type StylesCarte } from '../../lib/api';
  import Chronologie from '../../lib/Chronologie.svelte';
  import { ChronologieListe } from '../../lib/modele-liste.svelte';
  import { theme } from '../../lib/theme.svelte';
  import type { Vue } from '../../lib/vues';

  /** At most this many photos listed for one cluster. */
  const MAX_SELECTION = 300;

  let conteneur: HTMLDivElement;
  let carte: Carte | null = null;
  let styles: StylesCarte | null = null;
  let points: Marqueur[] = [];
  let chargement = $state(true);
  let erreur = $state<string | null>(null);
  let nombre = $state(0);
  let selection = $state<{ cle: number; ids: string[]; titre: string } | null>(null);
  let numero = 0;

  const sombre = $derived(theme.choix === 'sombre' || (theme.choix === 'systeme' && theme.systeme === 'sombre'));

  function geojson() {
    return {
      type: 'FeatureCollection' as const,
      features: points.map((m) => ({
        type: 'Feature' as const,
        geometry: { type: 'Point' as const, coordinates: [m.lon, m.lat] },
        properties: { id: m.id, ville: m.city ?? '' },
      })),
    };
  }

  /** Adds the photo layers; runs again after every style change. */
  function ajouterCalques() {
    if (!carte || carte.getSource('photos')) return;
    carte.addSource('photos', { type: 'geojson', data: geojson(), cluster: true, clusterRadius: 48, clusterMaxZoom: 17 });
    carte.addLayer({
      id: 'groupes',
      type: 'circle',
      source: 'photos',
      filter: ['has', 'point_count'],
      paint: {
        'circle-color': '#2f86e0',
        'circle-radius': ['step', ['get', 'point_count'], 14, 10, 18, 50, 24, 200, 30],
        'circle-stroke-width': 3,
        'circle-stroke-color': '#ffffff',
      },
    });
    // Count labels reuse a font the server style already ships.
    const police = carte
      .getStyle()
      .layers.map((l) => (l.layout as Record<string, unknown> | undefined)?.['text-font'])
      .find((f) => Array.isArray(f)) as string[] | undefined;
    if (police) {
      carte.addLayer({
        id: 'nombres',
        type: 'symbol',
        source: 'photos',
        filter: ['has', 'point_count'],
        layout: { 'text-field': ['get', 'point_count_abbreviated'], 'text-font': police, 'text-size': 13 },
        paint: { 'text-color': '#ffffff' },
      });
    }
    carte.addLayer({
      id: 'points',
      type: 'circle',
      source: 'photos',
      filter: ['!', ['has', 'point_count']],
      paint: { 'circle-color': '#e8503f', 'circle-radius': 8, 'circle-stroke-width': 3, 'circle-stroke-color': '#ffffff' },
    });
  }

  async function montrer(ids: string[], ville: string) {
    const titre = `${ids.length}${ids.length >= MAX_SELECTION ? '+' : ''} photo${ids.length > 1 ? 's' : ''}${ville ? ` · ${ville}` : ''}`;
    selection = { cle: ++numero, ids, titre };
    await tick();
    carte?.resize();
  }

  async function fermerSelection() {
    selection = null;
    await tick();
    carte?.resize();
  }

  function modele(ids: string[]) {
    return new ChronologieListe(async () => {
      const assets = await assetsParIds(ids);
      return { assets, pageSuivante: null, total: assets.length };
    });
  }

  const vue = (titre: string): Vue => ({ id: 'carte', titre, chemin: '/carte', filtre: {}, vide: 'Aucune photo ici.' });

  // Follow the light/dark theme.
  $effect(() => {
    const url = styles && (sombre ? styles.mapDarkStyleUrl : styles.mapLightStyleUrl);
    if (carte && url) carte.setStyle(url);
  });

  onMount(() => {
    Promise.all([stylesCarte(), chargerMarqueurs()])
      .then(([s, m]) => {
        styles = s;
        points = m;
        nombre = m.length;
        const c = new Carte({
          container: conteneur,
          style: sombre ? s.mapDarkStyleUrl : s.mapLightStyleUrl,
          center: [2.4, 46.6],
          zoom: 4.5,
          attributionControl: { compact: true },
        });
        carte = c;
        // The container may get its real size after the map was created.
        const observateur = new ResizeObserver(() => c.resize());
        observateur.observe(conteneur);
        c.on('remove', () => observateur.disconnect());
        c.on('error', (e) => {
          const message = e.error?.message ?? String(e.error ?? e);
          console.error('carte', message);
          void invoke('journal', { message: `erreur de carte : ${message}` }).catch(() => {});
          if (chargement) {
            erreur = message;
            chargement = false;
          }
        });
        c.addControl(new NavigationControl({ showCompass: false }), 'top-left');
        c.on('style.load', ajouterCalques);
        c.on('load', () => {
          chargement = false;
          if (m.length) {
            const bornes = new LngLatBounds();
            for (const p of m) bornes.extend([p.lon, p.lat]);
            c.fitBounds(bornes, { padding: 60, maxZoom: 12, duration: 0 });
          }
        });
        c.on('click', 'groupes', async (e) => {
          const objet = e.features?.[0];
          const source = c.getSource('photos') as GeoJSONSource | undefined;
          if (!objet || !source) return;
          const feuilles = await source.getClusterLeaves(objet.properties.cluster_id, MAX_SELECTION, 0);
          const ids = feuilles.map((f) => String(f.properties?.id));
          const villes = feuilles.map((f) => String(f.properties?.ville ?? '')).filter(Boolean);
          void montrer(ids, villes[0] ?? '');
        });
        c.on('click', 'points', (e) => {
          const objet = e.features?.[0];
          if (objet) void montrer([String(objet.properties.id)], String(objet.properties.ville ?? ''));
        });
        for (const calque of ['groupes', 'points']) {
          c.on('mouseenter', calque, () => (c.getCanvas().style.cursor = 'pointer'));
          c.on('mouseleave', calque, () => (c.getCanvas().style.cursor = ''));
        }
      })
      .catch((e) => {
        erreur = String(e);
        chargement = false;
      });
    return () => carte?.remove();
  });
</script>

<svelte:head><title>Carte · Nicéphore</title></svelte:head>

<div class="flex h-full">
  <div class="relative min-w-0 flex-1">
    <!-- Explicit size: maplibre-gl.css forces `position: relative` on the
         container, which would cancel an absolute inset. -->
    <div bind:this={conteneur} class="h-full w-full"></div>
    {#if chargement}
      <div class="absolute inset-0 flex items-center justify-center"><LoadingSpinner size="giant" /></div>
    {/if}
    {#if erreur}
      <div class="absolute inset-x-0 top-0 p-4"><Text color="danger">Carte indisponible : {erreur}</Text></div>
    {:else if !chargement}
      <div class="compteur absolute bottom-3 left-3 rounded-full px-3 py-1 text-xs">
        {nombre.toLocaleString('fr-FR')} photos géolocalisées
      </div>
    {/if}
  </div>
  {#if selection}
    <aside class="panneau flex w-[440px] shrink-0 flex-col border-l border-gray-200 dark:border-gray-800">
      <div class="flex justify-end px-2 pt-2">
        <IconButton icon={mdiClose} aria-label="Fermer la sélection" variant="ghost" color="secondary" shape="round" onclick={fermerSelection} />
      </div>
      <div class="min-h-0 flex-1">
        {#key selection.cle}
          <Chronologie vue={vue(selection.titre)} modele={modele(selection.ids)} />
        {/key}
      </div>
    </aside>
  {/if}
</div>

<style>
  .compteur {
    background: rgb(0 0 0 / 0.6);
    color: white;
  }
</style>
