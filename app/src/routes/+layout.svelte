<script lang="ts">
  import '../app.css';
  import { TooltipProvider } from '@immich/ui';
  import { onMount } from 'svelte';
  import Coquille from '../lib/Coquille.svelte';
  import { initialiserTheme } from '../lib/theme.svelte';

  let { children } = $props();

  onMount(() => void initialiserTheme());
</script>

<!-- Buttons from @immich/ui show tooltips, which need this provider. -->
<TooltipProvider>
  <Coquille>
    {@render children()}
  </Coquille>
</TooltipProvider>
