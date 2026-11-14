<script lang="ts">
  // One photo of the library in the viewer, by its id (from the local
  // viewer's "open in the library"): closing goes back where it came from.
  import { LoadingSpinner } from '@immich/ui';
  import { page } from '$app/state';
  import { assetsParIds } from '../../lib/api';
  import { t } from '../../lib/i18n.svelte';
  import { ChronologieListe } from '../../lib/modele-liste.svelte';
  import { notifier } from '../../lib/notifications.svelte';
  import Visionneuse from '../../lib/Visionneuse.svelte';

  const id = $derived(page.url.searchParams.get('id') ?? '');
  let chrono = $state<ChronologieListe | null>(null);

  $effect(() => {
    const demande = id;
    chrono = null;
    const modele = new ChronologieListe(async () => {
      const assets = await assetsParIds([demande]);
      return { assets, pageSuivante: null, total: assets.length };
    });
    modele
      .demarrer()
      .then(() => {
        if (demande !== id) return;
        if (modele.asset({ section: 0, index: 0 })) chrono = modele;
        else throw new Error(t('Photo introuvable'));
      })
      .catch((e) => {
        notifier(String(e), 'erreur');
        history.back();
      });
  });
</script>

<svelte:head><title>{t('Photo')} · Nicéphore</title></svelte:head>

{#if chrono}
  <Visionneuse {chrono} depart={{ section: 0, index: 0 }} onfermer={() => history.back()} />
{:else}
  <div class="flex h-full items-center justify-center"><LoadingSpinner size="giant" /></div>
{/if}
