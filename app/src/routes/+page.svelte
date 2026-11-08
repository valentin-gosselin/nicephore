<script lang="ts">
  import Chronologie from '../lib/Chronologie.svelte';
  import { VUES } from '../lib/vues';
</script>

<svelte:head><title>Photos · Nicéphore</title></svelte:head>
<Chronologie vue={VUES.photos} />
