<script lang="ts">
  // One album, with the buttons of Immich web: add photos, share (people
  // and public links), download, slideshow, and a menu (upload from this
  // computer, order of the photos, delete). Name and description edited in
  // place; deleting keeps the photos in the library.
  import { Button, IconButton, Text, type MenuItems } from '@immich/ui';
  import {
    mdiCheck,
    mdiDeleteOutline,
    mdiDotsVertical,
    mdiDownload,
    mdiFileImageOutline,
    mdiFolderOutline,
    mdiImagePlusOutline,
    mdiPresentationPlay,
    mdiShareVariantOutline,
    mdiSortCalendarAscending,
    mdiSortCalendarDescending,
  } from '@mdi/js';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { listen } from '@tauri-apps/api/event';
  import { pictureDir } from '@tauri-apps/api/path';
  import { open } from '@tauri-apps/plugin-dialog';
  import {
    ajouterALAlbum,
    album as chargerAlbum,
    albums as chargerAlbums,
    modifierAlbum,
    supprimerAlbum,
    telechargerAlbum,
    type Album,
  } from '../../lib/api';
  import DialoguePartage from '../../lib/DialoguePartage.svelte';
  import { lancerEnvoi } from '../../lib/envois.svelte';
  import { ouvrirMenu } from '../../lib/menu-contextuel';
  import { remplacer } from '../../lib/notifications.svelte';
  import { VUES } from '../../lib/vues';
  import TexteModifiable from '../../lib/TexteModifiable.svelte';
  import Chronologie from '../../lib/Chronologie.svelte';
  import Dialogue from '../../lib/Dialogue.svelte';
  import { locale, t, tp } from '../../lib/i18n.svelte';
  import { notifier } from '../../lib/notifications.svelte';
  import type { Vue } from '../../lib/vues';

  const id = $derived(page.url.searchParams.get('id') ?? '');
  let infos = $state<Album | null>(null);
  let suppression = $state(false);
  let version = $state(0);
  let partage = $state(false);
  /** Picking photos of the library to add. */
  let ajout = $state(false);
  let chronologie = $state<{ lancerDiaporama: () => void }>();

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
    filtre: { album: id, ordre: infos?.order === 'asc' ? 'asc' : 'desc' },
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

  async function recharger() {
    infos = await chargerAlbum(id).then((a) => ({ ...a, possede: infos?.possede ?? true }));
    version++;
  }

  async function ajouter(ids: string[]) {
    try {
      const bilan = await ajouterALAlbum(id, ids);
      const deja = bilan.dejaPresents ? t(' ({n} y étaient déjà)', { n: bilan.dejaPresents }) : '';
      notifier(tp(bilan.ajoutes, '{n} élément ajouté à « {album} »', '{n} éléments ajoutés à « {album} »', { album: infos?.albumName ?? '' }) + deja, 'succes');
      ajout = false;
      await recharger();
    } catch (e) {
      notifier(t('Ajout impossible : {e}', { e: String(e) }), 'erreur');
    }
  }

  async function telecharger() {
    if (!infos) return;
    const dossier = await open({ directory: true, title: t('Dossier où enregistrer l’album'), defaultPath: await pictureDir().catch(() => undefined) });
    if (typeof dossier !== 'string') return;
    const nom = infos.albumName;
    const suivi = notifier(tp(infos.assetCount, 'Téléchargement de {n} élément…', 'Téléchargement de {n} éléments…'), 'info', undefined, 0);
    const arreter = await listen<{ fait: number; total: number; nom: string }>('telechargement', ({ payload }) =>
      remplacer(suivi, t('Téléchargement {fait} sur {total} : {nom}', { fait: payload.fait, total: payload.total, nom: payload.nom }), 'info', 0),
    );
    try {
      const bilan = await telechargerAlbum(id, nom, dossier);
      remplacer(
        suivi,
        bilan.echecs.length
          ? t('{reussis} téléchargé(s), {echecs} en échec :\n{liste}', { reussis: bilan.reussis, echecs: bilan.echecs.length, liste: bilan.echecs.slice(0, 3).join('\n') })
          : tp(bilan.reussis, '{n} original enregistré dans {dossier}', '{n} originaux enregistrés dans {dossier}', { dossier: bilan.dossier }),
        bilan.echecs.length ? 'erreur' : 'succes',
        8000,
      );
    } catch (e) {
      remplacer(suivi, t('Téléchargement impossible : {e}', { e: String(e) }), 'erreur', 9000);
    } finally {
      arreter();
    }
  }

  async function trier(ordre: 'asc' | 'desc') {
    try {
      await modifierAlbum(id, { ordre });
      await recharger();
    } catch (e) {
      notifier(t('Modification impossible : {raison}', { raison: String(e) }), 'erreur');
    }
  }

  async function envoyerDepuisOrdinateur(dossier: boolean) {
    if (!infos) return;
    const choix = await open({ multiple: true, directory: dossier, title: t('Envoyer dans « {nom} »', { nom: infos.albumName }) });
    const chemins = choix == null ? [] : Array.isArray(choix) ? choix : [choix];
    if (chemins.length) await lancerEnvoi(chemins, { type: 'existant', id, nom: infos.albumName });
  }

  function menu(e: MouseEvent) {
    if (!infos) return;
    const recents = infos.order !== 'asc';
    const items: MenuItems = [
      { title: t('Envoyer des photos et vidéos…'), icon: mdiFileImageOutline, onAction: () => void envoyerDepuisOrdinateur(false) },
      { title: t('Envoyer un dossier…'), icon: mdiFolderOutline, onAction: () => void envoyerDepuisOrdinateur(true) },
      { title: t('Récents en premier'), icon: recents ? mdiCheck : mdiSortCalendarDescending, onAction: () => void trier('desc'), $if: () => !!infos?.possede },
      { title: t('Anciens en premier'), icon: recents ? mdiSortCalendarAscending : mdiCheck, onAction: () => void trier('asc'), $if: () => !!infos?.possede },
      { title: t('Supprimer l’album'), icon: mdiDeleteOutline, color: 'danger', onAction: () => (suppression = true), $if: () => !!infos?.possede },
    ];
    ouvrirMenu(e, items);
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
    <div class="actions-album flex items-center">
      <IconButton icon={mdiImagePlusOutline} aria-label={t('Ajouter des photos')} variant="ghost" color="secondary" shape="round" onclick={() => (ajout = true)} />
      <IconButton icon={mdiShareVariantOutline} aria-label={t('Partager')} variant="ghost" color="secondary" shape="round" onclick={() => (partage = true)} />
      <IconButton
        icon={mdiDownload}
        aria-label={t('Télécharger')}
        variant="ghost"
        color="secondary"
        shape="round"
        disabled={!infos.assetCount}
        onclick={() => void telecharger()}
      />
      <IconButton
        icon={mdiPresentationPlay}
        aria-label={t('Diaporama')}
        variant="ghost"
        color="secondary"
        shape="round"
        disabled={!infos.assetCount}
        onclick={() => chronologie?.lancerDiaporama()}
      />
      <IconButton icon={mdiDotsVertical} aria-label={t('Plus d’actions')} variant="ghost" color="secondary" shape="round" onclick={menu} />
    </div>
  {/if}
{/snippet}

{#if ajout && infos}
  <Chronologie
    vue={VUES.photos}
    choix={{
      titre: t('Ajouter à « {nom} »', { nom: infos.albumName }),
      action: t('Ajouter'),
      onvalider: ajouter,
      onannuler: () => (ajout = false),
    }}
  />
{:else}
  {#key `${id}:${version}`}
    <Chronologie bind:this={chronologie} {vue} entete={actions} titre={nomAlbum} />
  {/key}
{/if}

{#if partage && infos}
  <DialoguePartage album={{ id, nom: infos.albumName, possede: infos.possede }} onfermer={() => (partage = false)} />
{/if}

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
