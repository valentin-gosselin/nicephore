<script lang="ts">
  // Thin draggable strip with the window buttons, for screens without the
  // app header (loading, login, unlock).
  import BoutonsFenetre from './BoutonsFenetre.svelte';
</script>

<div class="flex h-11 shrink-0 items-center justify-end px-2" data-tauri-drag-region>
  <BoutonsFenetre />
</div>
