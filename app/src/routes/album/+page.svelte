<script lang="ts">
  // One album: its photos as a timeline, plus rename, description and
  // deletion (the photos stay in the library).
  import { Button, IconButton, Text } from '@immich/ui';
  import { mdiDeleteOutline } from '@mdi/js';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { album as chargerAlbum, albums as chargerAlbums, modifierAlbum, supprimerAlbum, type Album } from '../../lib/api';
  import TexteModifiable from '../../lib/TexteModifiable.svelte';
  import Chronologie from '../../lib/Chronologie.svelte';
  import Dialogue from '../../lib/Dialogue.svelte';
  import { locale, t } from '../../lib/i18n.svelte';
  import { notifier } from '../../lib/notifications.svelte';
  import type { Vue } from '../../lib/vues';

  const id = $derived(page.url.searchParams.get('id') ?? '');
  let infos = $state<Album | null>(null);
  let suppression = $state(false);
  let version = $state(0);

  $effect(() => {
    const demande = id;
    infos = null;
    Promise.all([chargerAlbum(demande), chargerAlbums()])
      .then(([a, tous]) => {
        if (demande === id) infos = { ...a, possede: tous.some((x) => x.id === a.id && x.possede) };
      })
      .catch((e) => notifier(t('Album introuvable : {e}', { e: String(e) }), 'erreur'));
  });

  const vue = $derived<Vue>({
    id: 'album',
    titre: infos?.albumName ?? t('Album'),
    chemin: '/album',
    filtre: { album: id },
    vide: 'Cet album est vide. Sélectionnez des photos ailleurs puis « Ajouter à un album ».',
  });

  async function modifier(modification: { nom?: string; description?: string }) {
    try {
      await modifierAlbum(id, modification);
      infos = await chargerAlbum(id).then((a) => ({ ...a, possede: true }));
      notifier(modification.nom !== undefined ? t('Album renommé') : t('Description enregistrée'), 'succes');
    } catch (raison) {
      notifier(t('Modification impossible : {raison}', { raison: String(raison) }), 'erreur');
    }
  }

  async function supprimer() {
    try {
      await supprimerAlbum(id);
      suppression = false;
      notifier(t('Album « {nom} » supprimé. Ses photos restent dans la photothèque.', { nom: infos?.albumName ?? '' }), 'succes');
      void goto('/albums');
    } catch (raison) {
      notifier(t('Suppression impossible : {raison}', { raison: String(raison) }), 'erreur');
    }
  }
</script>

<svelte:head><title>{infos?.albumName ?? t('Album')} · Nicéphore</title></svelte:head>

{#snippet nomAlbum()}
  {#if infos}
    <TexteModifiable
      titre
      valeur={infos.albumName}
      vide={t('Album sans nom')}
      modifiable={infos.possede}
      onvalider={(v) => v && modifier({ nom: v })}
    />
  {:else}
    <span class="text-lg font-semibold">{t('Album')}</span>
  {/if}
{/snippet}

{#snippet actions()}
  {#if infos}
    {#if infos.possede || infos.description}
      <TexteModifiable
        valeur={infos.description}
        vide={t('Ajouter une description')}
        modifiable={infos.possede}
        onvalider={(v) => modifier({ description: v })}
      />
    {/if}
    <span class="flex-1"></span>
    {#if infos.possede}
      <IconButton icon={mdiDeleteOutline} aria-label={t('Supprimer l’album')} variant="ghost" color="danger" shape="round" onclick={() => (suppression = true)} />
    {/if}
  {/if}
{/snippet}

{#key `${id}:${version}`}
  <Chronologie {vue} entete={actions} titre={nomAlbum} />
{/key}

{#if suppression && infos}
  <Dialogue titre={t('Supprimer l’album ?')} onfermer={() => (suppression = false)}>
    <div class="space-y-4">
      <Text>
        {t('L’album « {nom} » sera supprimé. Ses {n} photos et vidéos restent dans votre photothèque.', {
          nom: infos.albumName,
          n: infos.assetCount.toLocaleString(locale()),
        })}
      </Text>
      <div class="flex justify-end gap-2">
        <Button variant="ghost" color="secondary" onclick={() => (suppression = false)}>{t('Annuler')}</Button>
        <Button color="danger" onclick={supprimer}>{t('Supprimer l’album')}</Button>
      </div>
    </div>
  </Dialogue>
{/if}
