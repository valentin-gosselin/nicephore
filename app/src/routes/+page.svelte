<script lang="ts">
  import { t } from '../lib/i18n.svelte';
  import BandeauSouvenirs from '../lib/BandeauSouvenirs.svelte';
  import Chronologie from '../lib/Chronologie.svelte';
  import { VUES } from '../lib/vues';
</script>

<svelte:head><title>{t('Photos')} · Nicéphore</title></svelte:head>
<div class="flex h-full flex-col">
  <BandeauSouvenirs />
  <div class="min-h-0 flex-1">
    <Chronologie vue={VUES.photos} />
  </div>
</div>
