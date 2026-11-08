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
      .catch((e) => notifier(`Album introuvable : ${e}`, 'erreur'));
  });

  const vue = $derived<Vue>({
    id: 'album',
    titre: infos?.albumName ?? 'Album',
    chemin: '/album',
    filtre: { album: id },
    vide: 'Cet album est vide. Sélectionnez des photos ailleurs puis « Ajouter à un album ».',
  });

  async function modifier(modification: { nom?: string; description?: string }) {
    try {
      await modifierAlbum(id, modification);
      infos = await chargerAlbum(id).then((a) => ({ ...a, possede: true }));
      notifier(modification.nom !== undefined ? 'Album renommé' : 'Description enregistrée', 'succes');
    } catch (raison) {
      notifier(`Modification impossible : ${raison}`, 'erreur');
    }
  }

  async function supprimer() {
    try {
      await supprimerAlbum(id);
      suppression = false;
      notifier(`Album « ${infos?.albumName} » supprimé. Ses photos restent dans la photothèque.`, 'succes');
      void goto('/albums');
    } catch (raison) {
      notifier(`Suppression impossible : ${raison}`, 'erreur');
    }
  }
</script>

<svelte:head><title>{infos?.albumName ?? 'Album'} · Nicéphore</title></svelte:head>

{#snippet nomAlbum()}
  {#if infos}
    <TexteModifiable
      titre
      valeur={infos.albumName}
      vide="Album sans nom"
      modifiable={infos.possede}
      onvalider={(v) => v && modifier({ nom: v })}
    />
  {:else}
    <span class="text-lg font-semibold">Album</span>
  {/if}
{/snippet}

{#snippet actions()}
  {#if infos}
    {#if infos.possede || infos.description}
      <TexteModifiable
        valeur={infos.description}
        vide="Ajouter une description"
        modifiable={infos.possede}
        onvalider={(v) => modifier({ description: v })}
      />
    {/if}
    <span class="flex-1"></span>
    {#if infos.possede}
      <IconButton icon={mdiDeleteOutline} aria-label="Supprimer l’album" variant="ghost" color="danger" shape="round" onclick={() => (suppression = true)} />
    {/if}
  {/if}
{/snippet}

{#key `${id}:${version}`}
  <Chronologie {vue} entete={actions} titre={nomAlbum} />
{/key}

{#if suppression && infos}
  <Dialogue titre="Supprimer l’album ?" onfermer={() => (suppression = false)}>
    <div class="space-y-4">
      <Text>
        L’album « {infos.albumName} » sera supprimé. Ses {infos.assetCount.toLocaleString('fr-FR')} photos et vidéos
        restent dans votre photothèque.
      </Text>
      <div class="flex justify-end gap-2">
        <Button variant="ghost" color="secondary" onclick={() => (suppression = false)}>Annuler</Button>
        <Button color="danger" onclick={supprimer}>Supprimer l’album</Button>
      </div>
    </div>
  </Dialogue>
{/if}
