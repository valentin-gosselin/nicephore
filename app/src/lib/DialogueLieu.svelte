<script lang="ts">
  // Picks a location on the map (click to place the pin) for one or many
  // assets, with the server's map style.
  import { Button, Text } from '@immich/ui';
  import { onMount } from 'svelte';
  import { stylesCarte } from './api';
  import Dialogue from './Dialogue.svelte';
  import { t } from './i18n.svelte';
  import { Map as Carte, Marker, NavigationControl } from './maplibre';
  import { theme } from './theme.svelte';

  let {
    nombre,
    latitude,
    longitude,
    onvalider,
    onfermer,
  }: {
    nombre: number;
    latitude: number | null;
    longitude: number | null;
    onvalider: (latitude: number, longitude: number) => void;
    onfermer: () => void;
  } = $props();

  let conteneur: HTMLDivElement;
  // svelte-ignore state_referenced_locally
  let choix = $state<{ lat: number; lon: number } | null>(latitude != null && longitude != null ? { lat: latitude, lon: longitude } : null);

  onMount(() => {
    let carte: Carte | undefined;
    const sombre = theme.choix === 'sombre' || (theme.choix === 'systeme' && theme.systeme === 'sombre');
    stylesCarte().then((styles) => {
      carte = new Carte({
        container: conteneur,
        style: sombre ? styles.mapDarkStyleUrl : styles.mapLightStyleUrl,
        center: choix ? [choix.lon, choix.lat] : [2.4, 46.6],
        zoom: choix ? 12 : 4.5,
        attributionControl: { compact: true },
      });
      carte.addControl(new NavigationControl({ showCompass: false }), 'top-left');
      const epingle = new Marker({ color: '#e8503f' });
      if (choix) epingle.setLngLat([choix.lon, choix.lat]).addTo(carte);
      carte.on('click', (e) => {
        choix = { lat: e.lngLat.lat, lon: e.lngLat.lng };
        epingle.setLngLat(e.lngLat).addTo(carte!);
      });
    });
    return () => carte?.remove();
  });
</script>

<Dialogue titre={nombre > 1 ? t('Lieu de {n} éléments', { n: nombre }) : t('Lieu de la prise de vue')} largeur={720} {onfermer}>
  <div class="space-y-3">
    <Text size="small" color="muted">{t('Cliquez sur la carte pour placer l’épingle.')}</Text>
    <div bind:this={conteneur} class="h-[420px] w-full overflow-hidden rounded-xl"></div>
    <div class="flex items-center justify-between gap-3">
      <Text size="small" color="muted">
        {choix ? `${choix.lat.toFixed(5)}, ${choix.lon.toFixed(5)}` : t('Aucun lieu choisi')}
      </Text>
      <Button disabled={!choix} onclick={() => choix && onvalider(choix.lat, choix.lon)}>{t('Enregistrer')}</Button>
    </div>
  </div>
</Dialogue>
