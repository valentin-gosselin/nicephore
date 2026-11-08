<script lang="ts">
  import { Icon } from '@immich/ui';
  import { mdiClose } from '@mdi/js';
  import { notifications, retirer } from './notifications.svelte';
</script>

<div class="pointer-events-none fixed bottom-5 left-1/2 z-[60] flex -translate-x-1/2 flex-col items-center gap-2">
  {#each notifications as n (n.id)}
    <div
      class="pointer-events-auto flex max-w-[640px] items-center gap-4 rounded-xl px-4 py-3 text-sm shadow-lg"
      class:bg-gray-900={n.type !== 'erreur'}
      class:text-white={true}
      class:erreur={n.type === 'erreur'}
      role={n.type === 'erreur' ? 'alert' : 'status'}
    >
      <span class="whitespace-pre-line">{n.texte}</span>
      {#if n.action}
        <button
          class="shrink-0 font-semibold text-primary-300 hover:underline"
          onclick={() => {
            n.action?.executer();
            retirer(n.id);
          }}
        >
          {n.action.libelle}
        </button>
      {/if}
      <button class="shrink-0 opacity-60 hover:opacity-100" aria-label="Fermer" onclick={() => retirer(n.id)}>
        <Icon icon={mdiClose} size="16" />
      </button>
    </div>
  {/each}
</div>

<style>
  .erreur {
    background: #8f1d1d;
  }
</style>
