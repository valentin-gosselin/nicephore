<script lang="ts">
  import '../app.css';
  // Importing the theme manager applies the light/dark class, following the
  // system preference unless the user picks one.
  import { themeManager, TooltipProvider } from '@immich/ui';

  let { children } = $props();
  void themeManager;
</script>

<!-- Buttons from @immich/ui show tooltips, which need this provider. -->
<TooltipProvider>
  {@render children()}
</TooltipProvider>
