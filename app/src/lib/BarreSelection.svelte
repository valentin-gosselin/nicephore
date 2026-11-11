<script lang="ts">
  // Action bar shown while assets are selected. Every action reports its
  // result in a toast; moving to the trash can be undone from the toast.
  import { IconButton, Text } from '@immich/ui';
  import {
    mdiArchiveArrowDownOutline,
    mdiArchiveArrowUpOutline,
    mdiClose,
    mdiDeleteRestore,
    mdiDownload,
    mdiHeart,
    mdiHeartOutline,
    mdiCalendarEditOutline,
    mdiImageMinusOutline,
    mdiImagePlusOutline,
    mdiMapMarkerOutline,
    mdiTagPlusOutline,
    mdiSelectAll,
    mdiTrashCanOutline,
  } from '@mdi/js';
  import { listen } from '@tauri-apps/api/event';
  import { pictureDir } from '@tauri-apps/api/path';
  import { open } from '@tauri-apps/plugin-dialog';
  import { goto } from '$app/navigation';
  import {
    ajouterALAlbum,
    creerAlbum,
    definirArchive,
    definirFavori,
    etiqueter,
    mettreCorbeille,
    modifierInfos,
    restaurerCorbeille,
    retirerDeLAlbum,
    telecharger,
    type Album,
  } from './api';
  import ChoixAlbum from './ChoixAlbum.svelte';
  import { fuseauLocal, versLocal } from './dates';
  import DialogueDate from './DialogueDate.svelte';
  import DialogueLieu from './DialogueLieu.svelte';
  import DialogueTag from './DialogueTag.svelte';
  import type { Chronologie } from './modele-chronologie.svelte';
  import { t, tp } from './i18n.svelte';
  import { notifier, remplacer } from './notifications.svelte';
  import type { Selection } from './selection.svelte';
  import type { Vue } from './vues';

  let { vue, selection, chrono }: { vue: Vue; selection: Selection; chrono: Chronologie } = $props();

  let enCours = $state(false);
  let choixAlbum = $state(false);
  let dialogue = $state<'date' | 'lieu' | 'tag' | null>(null);
  const nombre = $derived(selection.taille);

  /** Runs an action on the selection, then reloads the months it touched. */
  async function agir(
    action: (ids: string[]) => Promise<void>,
    message: ((n: number) => string) | null,
    retirees = true,
  ) {
    const ids = selection.ids();
    const touchees = chrono.sectionsDe(new Set(ids));
    enCours = true;
    try {
      await action(ids);
      selection.vider();
      if (retirees) await chrono.rafraichir(touchees);
      if (message) notifier(message(ids.length), 'succes');
      return ids;
    } catch (e) {
      notifier(t('L’action a échoué : {e}', { e: String(e) }), 'erreur');
      return null;
    } finally {
      enCours = false;
    }
  }

  async function basculerFavori() {
    const favori = !selection.tousFavoris();
    const ids = new Set(selection.ids());
    // In the favorites view, removing a favorite removes it from the view.
    const retirees = vue.id === 'favoris' && !favori;
    const fait = await agir(
      (liste) => definirFavori(liste, favori),
      (n) =>
        favori
          ? tp(n, '{n} élément ajouté aux favoris', '{n} éléments ajoutés aux favoris')
          : tp(n, '{n} élément retiré des favoris', '{n} éléments retirés des favoris'),
      retirees,
    );
    if (fait && !retirees) chrono.marquerFavoris(ids, favori);
  }

  const archiver = (archive: boolean) =>
    agir(
      (ids) => definirArchive(ids, archive),
      (n) =>
        archive
          ? tp(n, '{n} élément archivé', '{n} éléments archivés')
          : tp(n, '{n} élément désarchivé', '{n} éléments désarchivés'),
    );

  async function corbeille() {
    const ids = await agir(mettreCorbeille, null);
    if (!ids) return;
    notifier(tp(ids.length, '{n} élément mis à la corbeille', '{n} éléments mis à la corbeille'), 'info', {
      libelle: t('Annuler'),
      executer: async () => {
        try {
          await restaurerCorbeille(ids);
          await chrono.rafraichir(chrono.sections.map((_, i) => i).filter((i) => chrono.sections[i].etat === 'charge'));
          notifier(t('Restauration faite'), 'succes');
        } catch (e) {
          notifier(t('Restauration impossible : {e}', { e: String(e) }), 'erreur');
        }
      },
    });
  }

  async function ajouterA(album: Album) {
    choixAlbum = false;
    const ids = selection.ids();
    enCours = true;
    try {
      const bilan = await ajouterALAlbum(album.id, ids);
      selection.vider();
      const deja = bilan.dejaPresents ? t(' ({n} y étaient déjà)', { n: bilan.dejaPresents }) : '';
      const message = tp(bilan.ajoutes, '{n} élément ajouté à « {album} »', '{n} éléments ajoutés à « {album} »', { album: album.albumName });
      notifier(message + deja, 'succes', {
        libelle: t('Ouvrir'),
        executer: () => void goto(`/album?id=${album.id}`),
      });
    } catch (e) {
      notifier(t('Ajout impossible : {e}', { e: String(e) }), 'erreur');
    } finally {
      enCours = false;
    }
  }

  async function nouvelAlbum(nom: string) {
    choixAlbum = false;
    const ids = selection.ids();
    enCours = true;
    try {
      const album = await creerAlbum(nom, '', ids);
      selection.vider();
      notifier(tp(ids.length, 'Album « {album} » créé avec {n} élément', 'Album « {album} » créé avec {n} éléments', { album: album.albumName }), 'succes', {
        libelle: t('Ouvrir'),
        executer: () => void goto(`/album?id=${album.id}`),
      });
    } catch (e) {
      notifier(t('Création impossible : {e}', { e: String(e) }), 'erreur');
    } finally {
      enCours = false;
    }
  }

  const retirerDeLAlbumCourant = () =>
    agir(
      (ids) => retirerDeLAlbum(vue.filtre.album!, ids),
      (n) => tp(n, '{n} élément retiré de l’album (toujours dans la photothèque)', '{n} éléments retirés de l’album (toujours dans la photothèque)'),
    );

  async function changerDate(date: string, fuseau: string) {
    dialogue = null;
    const ids = selection.ids();
    enCours = true;
    try {
      await modifierInfos(ids, { date, fuseau });
      selection.vider();
      notifier(tp(ids.length, 'Date de {n} élément modifiée', 'Date de {n} éléments modifiée'), 'succes');
      // The photos move to another place in the timeline.
      setTimeout(() => chrono.actualiserMois().catch(() => {}), 1200);
    } catch (e) {
      notifier(t('Date impossible : {e}', { e: String(e) }), 'erreur');
    } finally {
      enCours = false;
    }
  }

  async function changerLieu(latitude: number, longitude: number) {
    dialogue = null;
    const ids = selection.ids();
    enCours = true;
    try {
      await modifierInfos(ids, { latitude, longitude });
      selection.vider();
      notifier(tp(ids.length, 'Lieu de {n} élément enregistré', 'Lieu de {n} éléments enregistré'), 'succes');
    } catch (e) {
      notifier(t('Lieu impossible : {e}', { e: String(e) }), 'erreur');
    } finally {
      enCours = false;
    }
  }

  async function ajouterTag(nom: string) {
    dialogue = null;
    const ids = selection.ids();
    enCours = true;
    try {
      await etiqueter(nom, ids);
      selection.vider();
      notifier(tp(ids.length, 'Tag « {nom} » ajouté à {n} élément', 'Tag « {nom} » ajouté à {n} éléments', { nom }), 'succes');
    } catch (e) {
      notifier(t('Tag impossible : {e}', { e: String(e) }), 'erreur');
    } finally {
      enCours = false;
    }
  }

  const restaurer = () => agir(restaurerCorbeille, (n) => tp(n, '{n} élément restauré', '{n} éléments restaurés'));

  async function telechargerSelection() {
    const dossier = await open({
      directory: true,
      title: t('Dossier où enregistrer les originaux'),
      defaultPath: await pictureDir().catch(() => undefined),
    });
    if (typeof dossier !== 'string') return;
    const ids = selection.ids();
    const suivi = notifier(tp(ids.length, 'Téléchargement de {n} élément…', 'Téléchargement de {n} éléments…'), 'info', undefined, 0);
    const arreter = await listen<{ fait: number; total: number; nom: string }>('telechargement', ({ payload }) =>
      remplacer(suivi, t('Téléchargement {fait} sur {total} : {nom}', { fait: payload.fait, total: payload.total, nom: payload.nom }), 'info', 0),
    );
    try {
      const bilan = await telecharger(ids, dossier);
      selection.vider();
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
</script>

<div class="barre flex h-14 items-center gap-2 px-3">
  <IconButton icon={mdiClose} aria-label={t('Annuler la sélection (Échap)')} variant="ghost" color="secondary" shape="round" onclick={() => selection.vider()} />
  <Text fontWeight="semi-bold">{tp(nombre, '{n} sélectionné', '{n} sélectionnés')}</Text>
  <IconButton
    icon={mdiSelectAll}
    aria-label={t('Tout sélectionner (Ctrl+A)')}
    variant="ghost"
    color="secondary"
    shape="round"
    onclick={() => selection.ajouter(chrono.chargees())}
  />
  <span class="flex-1"></span>
  {#if vue.id === 'corbeille'}
    <IconButton icon={mdiDeleteRestore} aria-label={t('Restaurer')} variant="ghost" shape="round" disabled={enCours} onclick={restaurer} />
  {:else}
    <IconButton
      icon={selection.tousFavoris() ? mdiHeart : mdiHeartOutline}
      aria-label={selection.tousFavoris() ? t('Retirer des favoris') : t('Ajouter aux favoris')}
      variant="ghost"
      shape="round"
      disabled={enCours}
      onclick={basculerFavori}
    />
    <IconButton icon={mdiImagePlusOutline} aria-label={t('Ajouter à un album')} variant="ghost" shape="round" disabled={enCours} onclick={() => (choixAlbum = true)} />
    {#if vue.id === 'album' && vue.filtre.album}
      <IconButton icon={mdiImageMinusOutline} aria-label={t('Retirer de l’album')} variant="ghost" shape="round" disabled={enCours} onclick={retirerDeLAlbumCourant} />
    {/if}
    <IconButton icon={mdiCalendarEditOutline} aria-label={t('Modifier la date')} variant="ghost" shape="round" disabled={enCours} onclick={() => (dialogue = 'date')} />
    <IconButton icon={mdiMapMarkerOutline} aria-label={t('Modifier le lieu')} variant="ghost" shape="round" disabled={enCours} onclick={() => (dialogue = 'lieu')} />
    <IconButton icon={mdiTagPlusOutline} aria-label={t('Ajouter un tag')} variant="ghost" shape="round" disabled={enCours} onclick={() => (dialogue = 'tag')} />
    <IconButton icon={mdiDownload} aria-label={t('Télécharger les originaux')} variant="ghost" shape="round" disabled={enCours} onclick={telechargerSelection} />
    {#if vue.id === 'archive'}
      <IconButton icon={mdiArchiveArrowUpOutline} aria-label={t('Désarchiver')} variant="ghost" shape="round" disabled={enCours} onclick={() => archiver(false)} />
    {:else}
      <IconButton icon={mdiArchiveArrowDownOutline} aria-label={t('Archiver')} variant="ghost" shape="round" disabled={enCours} onclick={() => archiver(true)} />
    {/if}
    <IconButton icon={mdiTrashCanOutline} aria-label={t('Mettre à la corbeille (Suppr)')} variant="ghost" color="danger" shape="round" disabled={enCours} onclick={corbeille} />
  {/if}
</div>

{#if dialogue === 'date'}
  <DialogueDate
    nombre={nombre}
    initiale={versLocal(new Date().toISOString(), fuseauLocal())}
    fuseauInitial={fuseauLocal()}
    onvalider={changerDate}
    onfermer={() => (dialogue = null)}
  />
{:else if dialogue === 'lieu'}
  <DialogueLieu nombre={nombre} latitude={null} longitude={null} onvalider={changerLieu} onfermer={() => (dialogue = null)} />
{:else if dialogue === 'tag'}
  <DialogueTag nombre={nombre} onvalider={ajouterTag} onfermer={() => (dialogue = null)} />
{/if}

{#if choixAlbum}
  <ChoixAlbum nombre={nombre} onchoisir={ajouterA} oncreer={nouvelAlbum} onfermer={() => (choixAlbum = false)} />
{/if}

<style>
  .barre {
    background: color-mix(in oklab, var(--immich-ui-primary-500) 14%, var(--immich-ui-light, white));
  }
</style>
