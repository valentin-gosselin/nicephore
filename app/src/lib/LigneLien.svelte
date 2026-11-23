<script lang="ts">
  // One public link: what it shares, its address, its options at a glance,
  // and copy, change, delete.
  import { Button, IconButton } from '@immich/ui';
  import { mdiContentCopy, mdiDeleteOutline, mdiImageMultipleOutline, mdiLock, mdiPencilOutline } from '@mdi/js';
  import { copierTexte, urlImage, type LienPartage } from './api';
  import { locale, t, tp } from './i18n.svelte';
  import { notifier } from './notifications.svelte';

  let {
    lien,
    titre = true,
    onmodifier,
    onsupprimer,
  }: { lien: LienPartage; titre?: boolean; onmodifier: () => void; onsupprimer: () => void } = $props();

  const vignette = $derived(lien.album?.albumThumbnailAssetId ?? lien.assets[0]?.id ?? null);
  const expire = $derived(!!lien.expiresAt && new Date(lien.expiresAt).getTime() < Date.now());
  const date = (iso: string) => new Date(iso).toLocaleString(locale(), { dateStyle: 'medium', timeStyle: 'short' });
  const intitule = $derived(
    lien.album ? lien.album.albumName : tp(lien.assets.length, '{n} photo ou vidéo', '{n} photos ou vidéos'),
  );
  const details = $derived(
    [
      expire ? t('Expiré') : lien.expiresAt ? t('Expire le {date}', { date: date(lien.expiresAt) }) : null,
      lien.allowDownload ? t('Téléchargement permis') : t('Sans téléchargement'),
      lien.allowUpload ? t('Envoi permis') : null,
      lien.showMetadata ? null : t('Sans métadonnées'),
    ]
      .filter(Boolean)
      .join(' · '),
  );

  let confirmer = $state(false);

  async function copier() {
    try {
      await copierTexte(lien.url);
      notifier(t('Lien copié : {url}', { url: lien.url }), 'succes');
    } catch (e) {
      notifier(t('Copie impossible : {e}', { e: String(e) }), 'erreur');
    }
  }
</script>

{#if confirmer}
  <div class="confirmer flex items-center gap-3 rounded-xl p-3" role="alertdialog" aria-label={t('Supprimer le lien')}>
    <p class="min-w-0 flex-1 text-sm">{t('Supprimer ce lien ? Ceux qui l’ont reçu ne pourront plus l’ouvrir.')}</p>
    <Button size="small" variant="ghost" color="secondary" onclick={() => (confirmer = false)}>{t('Annuler')}</Button>
    <Button size="small" color="danger" onclick={onsupprimer}>{t('Supprimer')}</Button>
  </div>
{:else}
<div class="ligne-lien flex items-center gap-3 rounded-xl p-2" class:expire>
  {#if titre}
    {#if vignette}
      <img src={urlImage('vignette', vignette, null)} alt="" class="h-14 w-14 shrink-0 rounded-lg object-cover" />
    {:else}
      <span class="flex h-14 w-14 shrink-0 items-center justify-center rounded-lg bg-gray-500/20">
        <svg viewBox="0 0 24 24" width="24" height="24" aria-hidden="true"><path fill="currentColor" d={mdiImageMultipleOutline} /></svg>
      </span>
    {/if}
  {/if}
  <div class="min-w-0 flex-1">
    {#if titre}<p class="truncate font-medium">{intitule}</p>{/if}
    {#if lien.description}<p class="truncate text-sm" class:opacity-80={titre}>{lien.description}</p>{/if}
    <p class="url flex items-center gap-1 truncate text-xs opacity-70">
      {#if lien.password}<svg viewBox="0 0 24 24" width="12" height="12" aria-label={t('Protégé par un mot de passe')}><path fill="currentColor" d={mdiLock} /></svg>{/if}
      <span class="truncate">{lien.url}</span>
    </p>
    <p class="truncate text-xs opacity-60">{details}</p>
  </div>
  <IconButton icon={mdiContentCopy} aria-label={t('Copier le lien')} variant="ghost" color="secondary" shape="round" onclick={copier} />
  <IconButton icon={mdiPencilOutline} aria-label={t('Modifier le lien')} variant="ghost" color="secondary" shape="round" onclick={onmodifier} />
  <IconButton icon={mdiDeleteOutline} aria-label={t('Supprimer le lien')} variant="ghost" color="danger" shape="round" onclick={() => (confirmer = true)} />
</div>
{/if}

<style>
  .ligne-lien:hover {
    background: color-mix(in oklab, currentColor 6%, transparent);
  }
  .confirmer {
    background: color-mix(in oklab, #e64132 12%, transparent);
  }
  .expire {
    opacity: 0.6;
  }
</style>
