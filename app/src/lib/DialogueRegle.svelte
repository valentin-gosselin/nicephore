<script lang="ts" module>
  import type { Cible as CibleEnvoi, Rangement as RangementReception, Selection as SelectionServeur } from './api';
  export type Demande = (
    | { sens: 'montante'; dossier: string; cible: CibleEnvoi }
    | { sens: 'descendante'; dossier: string; selection: SelectionServeur; rangement: RangementReception }
    | { sens: 'double'; dossier: string; album: string | null; nouvelAlbum: string | null; miroir: boolean }
    | { sens: 'virtuel'; dossier: string; selection: SelectionServeur; rangement: RangementReception }
  ) & { sansRaw: boolean };
</script>

<script lang="ts">
  // Adds a synced folder: which way first (send a folder, or receive a
  // selection of the server), then the folder, then the details, with what
  // would be sent or received before anything starts.
  import { Button, LoadingSpinner, Switch, Text } from '@immich/ui';
  import { open } from '@tauri-apps/plugin-dialog';
  import { onMount } from 'svelte';
  import {
    albums as chargerAlbums,
    estimerDossier,
    estimerReception,
    personnes as chargerPersonnes,
    type Album,
    type Cible,
    type EstimationDossier,
    type EstimationReception,
    type Personne,
    type Rangement,
    type Selection,
  } from './api';
  import Dialogue from './Dialogue.svelte';

  let { onvalider, onfermer }: { onvalider: (d: Demande) => Promise<boolean>; onfermer: () => void } = $props();

  let sens = $state<Demande['sens']>('montante');
  let dossier = $state<string | null>(null);
  let enCours = $state(false);

  // Sending.
  let genre = $state<Cible['genre']>('albumParDossier');
  let albumChoisi = $state('');
  // Receiving.
  let quoi = $state<Selection['genre']>('album');
  let personneChoisie = $state('');
  let du = $state('');
  let au = $state('');
  let rangement = $state<Rangement>('plat');
  // Two-way.
  let albumDouble = $state<string>('nouveau');
  let miroir = $state(false);
  let avecRaw = $state(true);

  let albums = $state<Album[] | null>(null);
  let gens = $state<Personne[] | null>(null);
  let filtre = $state('');

  let estimationEnvoi = $state<EstimationDossier | null>(null);
  let estimationReception = $state<EstimationReception | null>(null);
  let estimationEnCours = $state(false);
  let erreurEstimation = $state<string | null>(null);

  /** Beyond this, the size is said plainly before anything starts. */
  const GROS_OCTETS = 10 * 1024 ** 3;
  const GROS_FICHIERS = 5000;
  const MARGE = 1024 ** 3;
  const volume = (o: number) =>
    o >= 1024 ** 3
      ? `${(o / 1024 ** 3).toLocaleString('fr-FR', { maximumFractionDigits: 1 })} Go`
      : `${Math.max(1, Math.round(o / 1024 ** 2)).toLocaleString('fr-FR')} Mo`;
  const elements = (n: number) => `${n.toLocaleString('fr-FR')} photo${n > 1 ? 's et vidéos' : ''}`;

  const albumsProposes = $derived(
    (albums ?? []).filter((a) => a.albumName.toLocaleLowerCase('fr').includes(filtre.trim().toLocaleLowerCase('fr'))),
  );
  const gensProposes = $derived(
    (gens ?? []).filter((p) => p.name && p.name.toLocaleLowerCase('fr').includes(filtre.trim().toLocaleLowerCase('fr'))),
  );

  /** The selection being described, once complete. */
  const selection = $derived.by((): Selection | null => {
    if (quoi === 'album') {
      const a = albums?.find((x) => x.id === albumChoisi);
      return a ? { genre: 'album', id: a.id, nom: a.albumName } : null;
    }
    if (quoi === 'personne') {
      const p = gens?.find((x) => x.id === personneChoisie);
      return p ? { genre: 'personne', id: p.id, nom: p.name } : null;
    }
    if (quoi === 'periode') return du || au ? { genre: 'periode', du: du || null, au: au || null } : null;
    return { genre: quoi } as Selection;
  });

  // What would be sent from the folder.
  $effect(() => {
    if (sens === 'descendante' || sens === 'virtuel') return;
    const choisi = dossier;
    estimationEnvoi = null;
    erreurEstimation = null;
    if (!choisi) return;
    estimationEnCours = true;
    estimerDossier(choisi)
      .then((e) => choisi === dossier && (estimationEnvoi = e))
      .catch((e) => choisi === dossier && (erreurEstimation = String(e)))
      .finally(() => choisi === dossier && (estimationEnCours = false));
  });

  // What would be received, and the room for it.
  $effect(() => {
    if (sens !== 'descendante' && sens !== 'virtuel') return;
    const s = selection;
    const d = dossier;
    const cle = JSON.stringify([s, d]);
    estimationReception = null;
    erreurEstimation = null;
    if (!s) return;
    estimationEnCours = true;
    estimerReception(s, d)
      .then((e) => cle === JSON.stringify([selection, dossier]) && (estimationReception = e))
      .catch((e) => (erreurEstimation = String(e)))
      .finally(() => (estimationEnCours = false));
  });

  // Year and month folders suit big selections, a flat folder an album.
  $effect(() => {
    rangement = quoi === 'periode' || quoi === 'bibliotheque' ? 'anneeMois' : 'plat';
  });

  const grosEnvoi = $derived(!!estimationEnvoi && (estimationEnvoi.octets > GROS_OCTETS || estimationEnvoi.fichiers > GROS_FICHIERS));
  const grosseReception = $derived(
    !!estimationReception && (estimationReception.octets > GROS_OCTETS || estimationReception.fichiers > GROS_FICHIERS),
  );
  const placeManque = $derived(
    !!estimationReception && estimationReception.libre != null && estimationReception.octets + MARGE > estimationReception.libre,
  );

  const pret = $derived(
    !!dossier &&
      (sens === 'montante'
        ? !!estimationEnvoi && estimationEnvoi.fichiers > 0 && (genre !== 'album' || !!albumChoisi)
        : sens === 'double'
          ? !!estimationEnvoi && (albumDouble === 'nouveau' || !!albums?.some((a) => a.id === albumDouble))
          : sens === 'virtuel'
            ? !!selection && !!estimationReception && estimationReception.fichiers > 0
            : !!selection && !!estimationReception && estimationReception.fichiers > 0 && !placeManque),
  );
  const nomDossier = $derived(dossier?.split(/[\\/]/).filter(Boolean).pop() ?? '');

  async function choisir() {
    const titre = sens === 'montante' ? 'Dossier à envoyer vers Immich' : 'Dossier où recevoir les photos';
    const choix = await open({ directory: true, title: titre });
    if (typeof choix === 'string') dossier = choix;
  }

  async function valider(e: SubmitEvent) {
    e.preventDefault();
    if (!dossier || !pret) return;
    let demande: Demande;
    if (sens === 'montante') {
      let cible: Cible;
      if (genre === 'album') {
        const album = albums?.find((a) => a.id === albumChoisi);
        if (!album) return;
        cible = { genre: 'album', id: album.id, nom: album.albumName };
      } else {
        cible = { genre };
      }
      demande = { sens, dossier, cible, sansRaw: false };
    } else if (sens === 'double') {
      demande =
        albumDouble === 'nouveau'
          ? { sens, dossier, album: null, nouvelAlbum: nomDossier, miroir, sansRaw: false }
          : { sens, dossier, album: albumDouble, nouvelAlbum: null, miroir, sansRaw: false };
    } else {
      if (!selection) return;
      demande =
        sens === 'virtuel'
          ? { sens, dossier, selection, rangement, sansRaw: false }
          : { sens, dossier, selection, rangement, sansRaw: false };
    }
    demande = { ...demande, sansRaw: !avecRaw };
    enCours = true;
    const ok = await onvalider(demande);
    enCours = false;
    if (ok) onfermer();
  }

  onMount(() => {
    chargerAlbums()
      .then((a) => (albums = a.sort((x, y) => x.albumName.localeCompare(y.albumName, 'fr'))))
      .catch(() => (albums = []));
    chargerPersonnes()
      .then((p) => (gens = p.filter((x) => x.name).sort((x, y) => x.name.localeCompare(y.name, 'fr'))))
      .catch(() => (gens = []));
  });

  const sensPossibles: { sens: Demande['sens']; titre: string; detail: string }[] = [
    { sens: 'montante', titre: 'Envoyer un dossier vers Immich', detail: 'Ses photos et vidéos partent sur le serveur.' },
    {
      sens: 'descendante',
      titre: 'Recevoir des photos d’Immich dans un dossier',
      detail: 'Un album, une personne, les favoris, une période ou toute la photothèque, copiés sur cet ordinateur.',
    },
    {
      sens: 'virtuel',
      titre: 'Voir une sélection sans la télécharger',
      detail: 'Fichiers virtuels : les photos apparaissent dans un dossier vide et ne sont téléchargées qu’à l’ouverture.',
    },
    {
      sens: 'double',
      titre: 'Garder un dossier et un album identiques',
      detail: 'Ce qui arrive dans le dossier part dans l’album, ce qui arrive dans l’album vient dans le dossier.',
    },
  ];
  const ciblesEnvoi: { genre: Cible['genre']; titre: string; detail: string }[] = [
    { genre: 'albumParDossier', titre: 'Un album par dossier', detail: 'Chaque dossier donne un album à son nom, créé au premier envoi.' },
    { genre: 'album', titre: 'Un album existant', detail: 'Toutes les photos du dossier rejoignent cet album.' },
    { genre: 'bibliotheque', titre: 'La photothèque seulement', detail: 'Sans album.' },
  ];
  const selections: { genre: Selection['genre']; titre: string }[] = [
    { genre: 'album', titre: 'Un album' },
    { genre: 'personne', titre: 'Une personne' },
    { genre: 'favoris', titre: 'Les favoris' },
    { genre: 'periode', titre: 'Une période' },
    { genre: 'bibliotheque', titre: 'Toute la photothèque' },
  ];
</script>

{#snippet listeAlbums(nom: string, choisi: string, choisir: (id: string) => void, seulementMiens: boolean)}
  {#if !albums}
    <LoadingSpinner />
  {:else}
    <input class="champ w-full" placeholder="Chercher un album" bind:value={filtre} />
    <div class="liste">
      {#each albumsProposes.filter((a) => !seulementMiens || a.possede) as a (a.id)}
        <label class="ligne" class:choisie={choisi === a.id}>
          <input type="radio" name={nom} value={a.id} checked={choisi === a.id} onchange={() => choisir(a.id)} />
          <span class="truncate">{a.albumName}</span>
          <span class="ml-auto text-xs opacity-60">{a.assetCount}</span>
        </label>
      {:else}
        <Text size="small" color="muted">Aucun album de ce nom.</Text>
      {/each}
    </div>
  {/if}
{/snippet}

<Dialogue titre="Ajouter un dossier synchronisé" {onfermer} largeur={560}>
  <form class="space-y-4" onsubmit={valider}>
    <fieldset class="space-y-2">
      <legend class="mb-1 text-sm opacity-60">Dans quel sens</legend>
      {#each sensPossibles as s (s.sens)}
        <label class="option" class:choisie={sens === s.sens}>
          <input type="radio" name="sens" value={s.sens} bind:group={sens} />
          <span>
            <span class="block font-medium">{s.titre}</span>
            <span class="block text-sm opacity-70">{s.detail}</span>
          </span>
        </label>
      {/each}
    </fieldset>

    <div class="space-y-1">
      <Text size="small" color="muted">
        {sens === 'montante'
          ? 'Dossier à envoyer'
          : sens === 'double'
            ? 'Dossier à garder identique à l’album'
            : sens === 'virtuel'
              ? 'Dossier vide où voir les photos'
              : 'Dossier où recevoir'}
      </Text>
      <div class="flex items-center gap-3">
        <span class="chemin min-w-0 flex-1 truncate" title={dossier ?? ''}>{dossier ?? 'Aucun dossier choisi'}</span>
        <Button type="button" variant="outline" size="small" onclick={choisir}>Choisir…</Button>
      </div>
    </div>

    {#if sens === 'double'}
      <fieldset class="space-y-2">
        <legend class="mb-1 text-sm opacity-60">Album</legend>
        <label class="ligne" class:choisie={albumDouble === 'nouveau'}>
          <input type="radio" name="album-double" value="nouveau" bind:group={albumDouble} />
          <span>Un nouvel album{nomDossier ? ` « ${nomDossier} »` : ' au nom du dossier'}</span>
        </label>
        {@render listeAlbums('album-double-liste', albumDouble, (id) => (albumDouble = id), true)}
      </fieldset>
      <label class="flex items-start justify-between gap-4 rounded-xl p-3 option">
        <span>
          <span class="block font-medium">Répercuter les suppressions</span>
          <span class="block text-sm opacity-70">
            Une photo effacée ici quitte l’album (elle reste dans la photothèque) ; une photo retirée de l’album va dans la
            corbeille de l’ordinateur. Après 10 minutes, avec votre accord au-delà de 20 fichiers, et jamais si le
            dossier change de disque.
          </span>
        </span>
        <Switch bind:checked={miroir} />
      </label>
    {:else if sens === 'montante'}
      <fieldset class="space-y-2">
        <legend class="mb-1 text-sm opacity-60">Où vont les photos</legend>
        {#each ciblesEnvoi as o (o.genre)}
          <label class="option" class:choisie={genre === o.genre}>
            <input type="radio" name="cible" value={o.genre} bind:group={genre} />
            <span>
              <span class="block font-medium">{o.titre}</span>
              <span class="block text-sm opacity-70">
                {o.genre === 'albumParDossier' && nomDossier ? `${o.detail} Les photos à la racine vont dans « ${nomDossier} ».` : o.detail}
              </span>
            </span>
          </label>
        {/each}
      </fieldset>
      {#if genre === 'album'}
        {@render listeAlbums('album-envoi', albumChoisi, (id) => (albumChoisi = id), true)}
      {/if}
    {:else}
      <fieldset class="flex flex-wrap gap-2">
        <legend class="mb-1 text-sm opacity-60">{sens === 'virtuel' ? 'Quoi montrer' : 'Quoi recevoir'}</legend>
        {#each selections as o (o.genre)}
          <label class="pastille" class:choisie={quoi === o.genre}>
            <input type="radio" name="quoi" value={o.genre} bind:group={quoi} class="sr-only" />
            {o.titre}
          </label>
        {/each}
      </fieldset>
      {#if quoi === 'album'}
        {@render listeAlbums('album-reception', albumChoisi, (id) => (albumChoisi = id), false)}
      {:else if quoi === 'personne'}
        {#if !gens}
          <LoadingSpinner />
        {:else}
          <input class="champ w-full" placeholder="Chercher une personne" bind:value={filtre} />
          <div class="liste">
            {#each gensProposes as p (p.id)}
              <label class="ligne" class:choisie={personneChoisie === p.id}>
                <input type="radio" name="personne" value={p.id} bind:group={personneChoisie} />
                <span class="truncate">{p.name}</span>
              </label>
            {:else}
              <Text size="small" color="muted">Aucune personne nommée ainsi.</Text>
            {/each}
          </div>
        {/if}
      {:else if quoi === 'periode'}
        <div class="flex flex-wrap items-center gap-3 text-sm">
          <label class="flex items-center gap-2">Du <input type="date" class="champ" bind:value={du} aria-label="Du" /></label>
          <label class="flex items-center gap-2">au <input type="date" class="champ" bind:value={au} aria-label="Au" /></label>
        </div>
      {/if}
      <fieldset class="flex flex-wrap gap-2">
        <legend class="mb-1 text-sm opacity-60">Rangement dans le dossier</legend>
        <label class="pastille" class:choisie={rangement === 'plat'}>
          <input type="radio" name="rangement" value="plat" bind:group={rangement} class="sr-only" /> Tout dans le dossier
        </label>
        <label class="pastille" class:choisie={rangement === 'anneeMois'}>
          <input type="radio" name="rangement" value="anneeMois" bind:group={rangement} class="sr-only" /> Par année et mois
        </label>
      </fieldset>
    {/if}

    {#if estimationEnCours}
      <div class="flex items-center gap-2 text-sm opacity-70">
        <LoadingSpinner size="small" />
        {sens === 'montante' ? 'Analyse du dossier…' : 'Compte des photos sur le serveur…'}
      </div>
    {:else if erreurEstimation}
      <Text size="small" color="danger">{erreurEstimation}</Text>
    {:else if sens === 'double' && estimationEnvoi}
      <div class="estimation rounded-xl p-3 text-sm" class:gros={grosEnvoi}>
        Dans le dossier : <strong>{elements(estimationEnvoi.fichiers)}, {volume(estimationEnvoi.octets)}</strong>, envoyées
        dans l’album (sauf celles déjà sur le serveur). Les photos de l’album absentes du dossier y seront copiées.
      </div>
    {:else if sens === 'montante' && estimationEnvoi}
      <div class="estimation rounded-xl p-3 text-sm" class:gros={grosEnvoi}>
        {#if estimationEnvoi.fichiers === 0}
          Aucune photo ni vidéo dans ce dossier.
        {:else}
          <strong>{elements(estimationEnvoi.fichiers)}, {volume(estimationEnvoi.octets)}</strong>
          à envoyer, sous-dossiers compris (moins celles déjà sur le serveur, reconnues avant l’envoi).
          {#if grosEnvoi}
            <span class="mt-1 block">
              C’est un gros envoi : il occupera ce volume sur le serveur et prendra du temps. La pause et le retrait du
              dossier l’arrêtent aussitôt.
            </span>
          {/if}
        {/if}
        {#if estimationEnvoi.illisibles}<span class="mt-1 block opacity-70">{estimationEnvoi.illisibles} élément(s) illisible(s) ignoré(s).</span>{/if}
      </div>
    {:else if sens === 'virtuel' && estimationReception}
      <div class="estimation rounded-xl p-3 text-sm">
        <strong>{elements(estimationReception.fichiers)}, {volume(estimationReception.octets)}</strong> visibles dans le dossier,
        sans rien télécharger : un fichier n’arrive qu’au moment où un programme l’ouvre.
      </div>
    {:else if sens === 'descendante' && estimationReception}
      <div class="estimation rounded-xl p-3 text-sm" class:gros={grosseReception || placeManque}>
        {#if estimationReception.fichiers === 0}
          Rien à recevoir dans cette sélection.
        {:else}
          <strong>{elements(estimationReception.fichiers)}, {volume(estimationReception.octets)}</strong> à recevoir{estimationReception.libre != null
            ? ` (${volume(estimationReception.libre)} libres sur ce disque)`
            : ''}.
          {#if placeManque}
            <span class="mt-1 block">Pas assez de place sur ce disque : choisissez un autre dossier ou une sélection plus petite.</span>
          {:else if grosseReception}
            <span class="mt-1 block">C’est une grosse réception : elle prendra ce volume ici et du temps. La pause l’arrête aussitôt.</span>
          {/if}
        {/if}
      </div>
    {/if}

    <label class="flex items-center gap-2 text-sm">
      <input type="checkbox" bind:checked={avecRaw} />
      Inclure les fichiers RAW (DNG, ARW, CR2, NEF…), souvent bien plus lourds que le JPEG du même déclenchement
    </label>

    <Text size="small" color="muted">
      {#if sens === 'montante'}
        Rien n’est jamais supprimé : ni les fichiers de ce dossier, ni les photos sur le serveur. Les photos déjà présentes
        sur le serveur ne sont pas renvoyées.
      {:else if sens === 'virtuel'}
        Le dossier reste accessible même Nicéphore fermé (un petit service sans fenêtre le sert, lancé avec la session).
        Lecture seule : rien ne peut être modifié ni effacé dans ce dossier. Les photos ouvertes restent dans un cache
        (20 Go au plus), à vider dans les Réglages.
      {:else if sens === 'double'}
        Rien n’est jamais effacé définitivement : au pire une photo quitte l’album ou va dans la corbeille. Un fichier
        modifié d’un côté est gardé en double plutôt qu’écrasé.
      {:else}
        Rien n’est jamais supprimé ni écrasé ici. Une photo que vous effacez de ce dossier n’est pas retéléchargée ; une photo
        retirée sur le serveur reste ici. Les fichiers déjà présents à l’identique sont reconnus.
      {/if}
    </Text>
    <div class="flex justify-end gap-2">
      <Button type="button" variant="ghost" color="secondary" onclick={onfermer}>Annuler</Button>
      <Button type="submit" disabled={!pret} loading={enCours}>
        {#if sens === 'montante'}
          {estimationEnvoi && estimationEnvoi.fichiers > 0 ? `Envoyer ${volume(estimationEnvoi.octets)}` : 'Envoyer'}
        {:else if sens === 'double'}
          Synchroniser
        {:else if sens === 'virtuel'}
          Afficher dans le dossier
        {:else}
          {estimationReception && estimationReception.fichiers > 0 ? `Recevoir ${volume(estimationReception.octets)}` : 'Recevoir'}
        {/if}
      </Button>
    </div>
  </form>
</Dialogue>

<style>
  .estimation {
    background: color-mix(in oklab, currentColor 6%, transparent);
  }
  .estimation.gros {
    color: #b45309;
    background: color-mix(in oklab, #f59e0b 14%, transparent);
  }
  :global(.dark) .estimation.gros {
    color: #fbbf24;
  }
  .chemin {
    font-family: ui-monospace, monospace;
    font-size: 13px;
  }
  .option {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    padding: 10px 12px;
    border-radius: 12px;
    border: 1px solid color-mix(in oklab, currentColor 15%, transparent);
    cursor: pointer;
  }
  .option input {
    margin-top: 4px;
  }
  .pastille {
    padding: 5px 12px;
    border-radius: 9999px;
    font-size: 13px;
    cursor: pointer;
    border: 1px solid color-mix(in oklab, currentColor 18%, transparent);
  }
  .choisie {
    border-color: var(--immich-ui-primary-500);
    background: color-mix(in oklab, var(--immich-ui-primary-500) 10%, transparent);
  }
  .champ {
    border-radius: 10px;
    padding: 6px 10px;
    background: color-mix(in oklab, currentColor 6%, transparent);
  }
  .liste {
    max-height: 200px;
    overflow-y: auto;
  }
  .ligne {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 8px;
    border-radius: 8px;
    cursor: pointer;
  }
  .ligne:hover {
    background: color-mix(in oklab, currentColor 6%, transparent);
  }
</style>
