<script lang="ts">
  // Minimize, maximize and close, drawn by the app since the window has no
  // system title bar. Closing goes through the usual close request (the
  // window may stay in the tray while folders are sent).
  import { mdiClose, mdiWindowMaximize, mdiWindowMinimize, mdiWindowRestore } from '@mdi/js';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { onMount } from 'svelte';

  const fenetre = getCurrentWindow();
  let agrandie = $state(false);

  onMount(() => {
    const suivre = () => fenetre.isMaximized().then((m) => (agrandie = m));
    void suivre();
    const arret = fenetre.onResized(suivre);
    return () => void arret.then((f) => f());
  });
</script>

{#snippet bouton(libelle: string, icone: string, action: () => void, fermer = false)}
  <button type="button" class="bouton" class:fermer aria-label={libelle} title={libelle} onclick={action}>
    <svg viewBox="0 0 24 24" width="17" height="17" aria-hidden="true"><path fill="currentColor" d={icone} /></svg>
  </button>
{/snippet}

<div class="flex shrink-0 items-center gap-0.5">
  {@render bouton('Réduire la fenêtre', mdiWindowMinimize, () => void fenetre.minimize())}
  {@render bouton(agrandie ? 'Restaurer la fenêtre' : 'Agrandir la fenêtre', agrandie ? mdiWindowRestore : mdiWindowMaximize, () => void fenetre.toggleMaximize())}
  {@render bouton('Fermer la fenêtre', mdiClose, () => void fenetre.close(), true)}
</div>

<style>
  .bouton {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 30px;
    border-radius: 8px;
    opacity: 0.75;
    transition:
      background 120ms,
      opacity 120ms;
  }
  .bouton:hover {
    opacity: 1;
    background: color-mix(in oklab, currentColor 12%, transparent);
  }
  .bouton.fermer:hover {
    color: white;
    background: #d9363e;
  }
</style>
