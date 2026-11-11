<script lang="ts">
  import { t } from '../../lib/i18n.svelte';
  import Chronologie from '../../lib/Chronologie.svelte';
  import { VUES } from '../../lib/vues';
</script>

<svelte:head><title>{t('Corbeille')} · Nicéphore</title></svelte:head>
<Chronologie vue={VUES.corbeille} />
