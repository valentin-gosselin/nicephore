<script lang="ts">
  // One memory: its photos, and a slideshow to replay them.
  import { Button, Text } from '@immich/ui';
  import { mdiPlay } from '@mdi/js';
  import { page } from '$app/state';
  import { souvenir as chargerSouvenir, type Souvenir } from '../../lib/api';
  import Chronologie from '../../lib/Chronologie.svelte';
  import { ChronologieListe } from '../../lib/modele-liste.svelte';
  import { locale, t } from '../../lib/i18n.svelte';
  import { notifier } from '../../lib/notifications.svelte';
  import { titreSouvenir } from '../../lib/souvenirs';
  import type { Vue } from '../../lib/vues';

  const id = $derived(page.url.searchParams.get('id') ?? '');
  let infos = $state<Souvenir | null>(null);
  let grille = $state<{ lancerDiaporama: () => void } | undefined>();

  $effect(() => {
    const demande = id;
    infos = null;
    chargerSouvenir(demande)
      .then((s) => demande === id && (infos = s))
      .catch((e) => notifier(t('Souvenir introuvable : {e}', { e: String(e) }), 'erreur'));
  });

  const date = $derived(
    infos?.date ? new Date(infos.date).toLocaleDateString(locale(), { day: 'numeric', month: 'long', year: 'numeric' }) : '',
  );
  const vue = $derived<Vue>({
    id: 'souvenir',
    titre: infos ? titreSouvenir(infos) : t('Souvenir'),
    chemin: '/souvenir',
    filtre: {},
    vide: t('Ce souvenir est vide.'),
  });

  function modele(s: Souvenir) {
    return new ChronologieListe(async () => ({ assets: s.assets, pageSuivante: null, total: s.assets.length }));
  }
</script>

<svelte:head><title>{vue.titre} · Nicéphore</title></svelte:head>

{#snippet actions()}
  {#if date}<Text size="small" color="muted">{date}</Text>{/if}
  <span class="flex-1"></span>
  <Button size="small" leadingIcon={mdiPlay} onclick={() => grille?.lancerDiaporama()}>{t('Diaporama')}</Button>
{/snippet}

{#if infos}
  {#key infos.id}
    <Chronologie bind:this={grille} {vue} modele={modele(infos)} entete={actions} />
  {/key}
{/if}
