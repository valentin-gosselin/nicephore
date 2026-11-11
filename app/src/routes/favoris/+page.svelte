<script lang="ts">
  import { t } from '../../lib/i18n.svelte';
  import Chronologie from '../../lib/Chronologie.svelte';
  import { VUES } from '../../lib/vues';
</script>

<svelte:head><title>{t('Favoris')} · Nicéphore</title></svelte:head>
<Chronologie vue={VUES.favoris} />
