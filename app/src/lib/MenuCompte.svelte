<script lang="ts">
  // Account avatar at the top right, like the web app: the profile picture
  // (or initials on the account color), opening a small menu.
  import { mdiCogOutline, mdiLogout } from '@mdi/js';
  import { goto } from '$app/navigation';
  import { urlImage, type Connexion } from './api';
  import { deconnecter } from './session.svelte';

  let { connexion }: { connexion: Connexion } = $props();

  let ouvert = $state(false);
  let racine: HTMLDivElement;
  const u = $derived(connexion.utilisateur);

  // Immich avatar colors.
  const couleurs: Record<string, string> = {
    primary: '#4250af',
    pink: '#de7fb3',
    red: '#e64132',
    yellow: '#e2d25a',
    blue: '#2e7cd3',
    green: '#14a85a',
    purple: '#9c27b0',
    orange: '#f46d1b',
    gray: '#616161',
    amber: '#ffc107',
  };
  const initiales = $derived(
    u.name
      .split(/\s+/)
      .filter(Boolean)
      .slice(0, 2)
      .map((m) => m[0]?.toLocaleUpperCase('fr'))
      .join('') || '?',
  );

  function dehors(e: MouseEvent) {
    if (ouvert && !racine.contains(e.target as Node)) ouvert = false;
  }

  function aller(chemin: string) {
    ouvert = false;
    void goto(chemin);
  }
</script>

<svelte:window onmousedown={dehors} onkeydown={(e) => e.key === 'Escape' && (ouvert = false)} />

<div class="relative" bind:this={racine}>
  <button
    type="button"
    class="avatar"
    style:background={u.profileImagePath ? 'transparent' : (couleurs[u.avatarColor ?? 'primary'] ?? couleurs.primary)}
    aria-label={`Compte de ${u.name}`}
    aria-haspopup="menu"
    aria-expanded={ouvert}
    title={u.name}
    onclick={() => (ouvert = !ouvert)}
  >
    {#if u.profileImagePath}
      <img src={urlImage('profil', u.id, u.profileChangedAt)} alt="" />
    {:else}
      {initiales}
    {/if}
  </button>

  {#if ouvert}
    <div class="menu absolute right-0 top-full z-[60] mt-2 w-72 rounded-2xl p-2 shadow-2xl" role="menu">
      <div class="px-3 pb-3 pt-2">
        <div class="font-semibold">{u.name}</div>
        <div class="truncate text-sm opacity-70">{u.email}</div>
        <div class="mt-1 truncate text-xs opacity-50" title={connexion.urlServeur}>{connexion.urlServeur} · Immich {connexion.version}</div>
      </div>
      <div class="separateur"></div>
      <button type="button" class="entree" role="menuitem" onclick={() => aller('/reglages')}>
        <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true"><path fill="currentColor" d={mdiCogOutline} /></svg>
        Réglages
      </button>
      {#if connexion.origine !== 'environnement'}
        <button
          type="button"
          class="entree"
          role="menuitem"
          onclick={() => {
            ouvert = false;
            void deconnecter();
          }}
        >
          <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true"><path fill="currentColor" d={mdiLogout} /></svg>
          Se déconnecter
        </button>
      {/if}
    </div>
  {/if}
</div>

<style>
  .avatar {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    overflow: hidden;
    border-radius: 9999px;
    color: white;
    font-size: 13px;
    font-weight: 600;
    transition: box-shadow 120ms;
  }
  .avatar:hover,
  .avatar[aria-expanded='true'] {
    box-shadow: 0 0 0 3px color-mix(in oklab, var(--immich-ui-primary-500) 45%, transparent);
  }
  .avatar img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .menu {
    background: white;
    color: #1c1d20;
    border: 1px solid color-mix(in oklab, currentColor 10%, transparent);
  }
  :global(.dark) .menu {
    background: #1c1f24;
    color: #e8e8e8;
  }
  .separateur {
    height: 1px;
    margin: 0 4px 4px;
    background: color-mix(in oklab, currentColor 10%, transparent);
  }
  .entree {
    display: flex;
    width: 100%;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
    border-radius: 10px;
    font-size: 14px;
    text-align: left;
  }
  .entree:hover {
    background: color-mix(in oklab, currentColor 8%, transparent);
  }
</style>
