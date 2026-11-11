<script lang="ts">
  import '../app.css';
  // Interface language, also for the @immich/ui components (applied on import).
  import '../lib/i18n.svelte';
  import { TooltipProvider } from '@immich/ui';
  import { onMount } from 'svelte';
  import Coquille from '../lib/Coquille.svelte';
  import { initialiserTheme } from '../lib/theme.svelte';

  let { children } = $props();

  onMount(() => void initialiserTheme());

  /**
   * No web browser menu on right click (reload, inspect…): it has nothing to
   * do in an app. Text fields keep it, for copy and paste.
   */
  function clicDroit(e: MouseEvent) {
    const cible = e.target as HTMLElement | null;
    if (cible?.closest('input, textarea, [contenteditable="true"]')) return;
    e.preventDefault();
  }
</script>

<svelte:window oncontextmenu={clicDroit} />

<!-- Buttons from @immich/ui show tooltips, which need this provider. -->
<TooltipProvider>
  <Coquille>
    {@render children()}
  </Coquille>
</TooltipProvider>
