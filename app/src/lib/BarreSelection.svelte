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
    mdiSelectAll,
    mdiTrashCanOutline,
  } from '@mdi/js';
  import { listen } from '@tauri-apps/api/event';
  import { pictureDir } from '@tauri-apps/api/path';
  import { open } from '@tauri-apps/plugin-dialog';
  import { definirArchive, definirFavori, mettreCorbeille, restaurerCorbeille, telecharger } from './api';
  import type { Chronologie } from './modele-chronologie.svelte';
  import { notifier, remplacer } from './notifications.svelte';
  import type { Selection } from './selection.svelte';
  import type { Vue } from './vues';

  let { vue, selection, chrono }: { vue: Vue; selection: Selection; chrono: Chronologie } = $props();

  let enCours = $state(false);
  const nombre = $derived(selection.taille);
  const pluriel = (n: number, mot: string) => `${n.toLocaleString('fr-FR')} ${mot}${n > 1 ? 's' : ''}`;

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
      notifier(`L’action a échoué : ${e}`, 'erreur');
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
      (n) => (favori ? `${pluriel(n, 'élément')} ajouté${n > 1 ? 's' : ''} aux favoris` : `${pluriel(n, 'élément')} retiré${n > 1 ? 's' : ''} des favoris`),
      retirees,
    );
    if (fait && !retirees) chrono.marquerFavoris(ids, favori);
  }

  const archiver = (archive: boolean) =>
    agir(
      (ids) => definirArchive(ids, archive),
      (n) => `${pluriel(n, 'élément')} ${archive ? 'archivé' : 'désarchivé'}${n > 1 ? 's' : ''}`,
    );

  async function corbeille() {
    const ids = await agir(mettreCorbeille, null);
    if (!ids) return;
    notifier(`${pluriel(ids.length, 'élément')} mis à la corbeille`, 'info', {
      libelle: 'Annuler',
      executer: async () => {
        try {
          await restaurerCorbeille(ids);
          await chrono.rafraichir(chrono.sections.map((_, i) => i).filter((i) => chrono.sections[i].etat === 'charge'));
          notifier('Restauration faite', 'succes');
        } catch (e) {
          notifier(`Restauration impossible : ${e}`, 'erreur');
        }
      },
    });
  }

  const restaurer = () => agir(restaurerCorbeille, (n) => `${pluriel(n, 'élément')} restauré${n > 1 ? 's' : ''}`);

  async function telechargerSelection() {
    const dossier = await open({
      directory: true,
      title: 'Dossier où enregistrer les originaux',
      defaultPath: await pictureDir().catch(() => undefined),
    });
    if (typeof dossier !== 'string') return;
    const ids = selection.ids();
    const suivi = notifier(`Téléchargement de ${pluriel(ids.length, 'élément')}…`, 'info', undefined, 0);
    const arreter = await listen<{ fait: number; total: number; nom: string }>('telechargement', ({ payload }) =>
      remplacer(suivi, `Téléchargement ${payload.fait} sur ${payload.total} : ${payload.nom}`, 'info', 0),
    );
    try {
      const bilan = await telecharger(ids, dossier);
      selection.vider();
      remplacer(
        suivi,
        bilan.echecs.length
          ? `${bilan.reussis} téléchargé(s), ${bilan.echecs.length} en échec :\n${bilan.echecs.slice(0, 3).join('\n')}`
          : `${pluriel(bilan.reussis, 'original')} enregistré${bilan.reussis > 1 ? 's' : ''} dans ${bilan.dossier}`,
        bilan.echecs.length ? 'erreur' : 'succes',
        8000,
      );
    } catch (e) {
      remplacer(suivi, `Téléchargement impossible : ${e}`, 'erreur', 9000);
    } finally {
      arreter();
    }
  }
</script>

<div class="barre flex h-14 items-center gap-2 px-3">
  <IconButton icon={mdiClose} aria-label="Annuler la sélection (Échap)" variant="ghost" color="secondary" shape="round" onclick={() => selection.vider()} />
  <Text fontWeight="semi-bold">{pluriel(nombre, 'sélectionné')}</Text>
  <IconButton
    icon={mdiSelectAll}
    aria-label="Tout sélectionner (Ctrl+A)"
    variant="ghost"
    color="secondary"
    shape="round"
    onclick={() => selection.ajouter(chrono.chargees())}
  />
  <span class="flex-1"></span>
  {#if vue.id === 'corbeille'}
    <IconButton icon={mdiDeleteRestore} aria-label="Restaurer" variant="ghost" shape="round" disabled={enCours} onclick={restaurer} />
  {:else}
    <IconButton
      icon={selection.tousFavoris() ? mdiHeart : mdiHeartOutline}
      aria-label={selection.tousFavoris() ? 'Retirer des favoris' : 'Ajouter aux favoris'}
      variant="ghost"
      shape="round"
      disabled={enCours}
      onclick={basculerFavori}
    />
    <IconButton icon={mdiDownload} aria-label="Télécharger les originaux" variant="ghost" shape="round" disabled={enCours} onclick={telechargerSelection} />
    {#if vue.id === 'archive'}
      <IconButton icon={mdiArchiveArrowUpOutline} aria-label="Désarchiver" variant="ghost" shape="round" disabled={enCours} onclick={() => archiver(false)} />
    {:else}
      <IconButton icon={mdiArchiveArrowDownOutline} aria-label="Archiver" variant="ghost" shape="round" disabled={enCours} onclick={() => archiver(true)} />
    {/if}
    <IconButton icon={mdiTrashCanOutline} aria-label="Mettre à la corbeille (Suppr)" variant="ghost" color="danger" shape="round" disabled={enCours} onclick={corbeille} />
  {/if}
</div>

<style>
  .barre {
    background: color-mix(in oklab, var(--immich-ui-primary-500) 14%, var(--immich-ui-light, white));
  }
</style>
