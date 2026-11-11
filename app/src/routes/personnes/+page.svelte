<script lang="ts">
  import { t } from '../../lib/i18n.svelte';
  import GrillePersonnes from '../../lib/GrillePersonnes.svelte';
</script>

<svelte:head><title>{t('Personnes')} · Nicéphore</title></svelte:head>

<div class="h-full overflow-y-auto p-6">
  <GrillePersonnes complet />
</div>
