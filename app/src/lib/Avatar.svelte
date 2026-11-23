<script lang="ts">
  // Round avatar of an Immich user: the profile picture, or the initials on
  // the user's Immich color.
  import { urlImage, type Utilisateur } from './api';
  import { locale } from './i18n.svelte';

  let { utilisateur, taille = 36 }: { utilisateur: Utilisateur; taille?: number } = $props();

  const COULEURS: Record<string, string> = {
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
    utilisateur.name
      .split(/\s+/)
      .filter(Boolean)
      .slice(0, 2)
      .map((m) => m[0]?.toLocaleUpperCase(locale()))
      .join('') || '?',
  );
</script>

<span
  class="avatar"
  style:width="{taille}px"
  style:height="{taille}px"
  style:font-size="{Math.round(taille * 0.4)}px"
  style:background={utilisateur.profileImagePath ? 'transparent' : (COULEURS[utilisateur.avatarColor ?? 'primary'] ?? COULEURS.primary)}
  aria-hidden="true"
>
  {#if utilisateur.profileImagePath}
    <img src={urlImage('profil', utilisateur.id, utilisateur.profileChangedAt)} alt="" />
  {:else}
    {initiales}
  {/if}
</span>

<style>
  .avatar {
    display: inline-flex;
    flex-shrink: 0;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    border-radius: 9999px;
    color: white;
    font-weight: 600;
  }
  .avatar img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
</style>
