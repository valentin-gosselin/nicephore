<script lang="ts">
  // Adds a folder to send: the folder, then where its photos go.
  import { Button, LoadingSpinner, Text } from '@immich/ui';
  import { open } from '@tauri-apps/plugin-dialog';
  import { onMount } from 'svelte';
  import { albums as chargerAlbums, estimerDossier, type Album, type Cible, type EstimationDossier } from './api';
  import Dialogue from './Dialogue.svelte';

  let {
    onvalider,
    onfermer,
  }: { onvalider: (dossier: string, cible: Cible) => Promise<boolean>; onfermer: () => void } = $props();

  let dossier = $state<string | null>(null);
  let genre = $state<Cible['genre']>('albumParDossier');
  let albumChoisi = $state('');
  let tous = $state<Album[] | null>(null);
  let filtre = $state('');
  let enCours = $state(false);
  let estimation = $state<EstimationDossier | null>(null);
  let estimationEnCours = $state(false);
  let erreurEstimation = $state<string | null>(null);

  /** Beyond this, the size is said plainly before anything starts. */
  const GROS_OCTETS = 10 * 1024 ** 3;
  const GROS_FICHIERS = 5000;
  const gros = $derived(!!estimation && (estimation.octets > GROS_OCTETS || estimation.fichiers > GROS_FICHIERS));
  const volume = (o: number) =>
    o >= 1024 ** 3
      ? `${(o / 1024 ** 3).toLocaleString('fr-FR', { maximumFractionDigits: 1 })} Go`
      : `${Math.max(1, Math.round(o / 1024 ** 2)).toLocaleString('fr-FR')} Mo`;

  $effect(() => {
    const choisi = dossier;
    estimation = null;
    erreurEstimation = null;
    if (!choisi) return;
    estimationEnCours = true;
    estimerDossier(choisi)
      .then((e) => choisi === dossier && (estimation = e))
      .catch((e) => choisi === dossier && (erreurEstimation = String(e)))
      .finally(() => choisi === dossier && (estimationEnCours = false));
  });

  const proposes = $derived(
    (tous ?? []).filter((a) => a.possede && a.albumName.toLocaleLowerCase('fr').includes(filtre.trim().toLocaleLowerCase('fr'))),
  );
  const pret = $derived(!!dossier && !!estimation && estimation.fichiers > 0 && (genre !== 'album' || !!albumChoisi));
  const nomDossier = $derived(dossier?.split(/[\\/]/).filter(Boolean).pop() ?? '');

  async function choisir() {
    const choix = await open({ directory: true, title: 'Dossier à envoyer vers Immich' });
    if (typeof choix === 'string') dossier = choix;
  }

  async function valider(e: SubmitEvent) {
    e.preventDefault();
    if (!dossier || !pret) return;
    let cible: Cible;
    if (genre === 'album') {
      const album = tous?.find((a) => a.id === albumChoisi);
      if (!album) return;
      cible = { genre: 'album', id: album.id, nom: album.albumName };
    } else {
      cible = { genre };
    }
    enCours = true;
    const ok = await onvalider(dossier, cible);
    enCours = false;
    if (ok) onfermer();
  }

  onMount(() => {
    void choisir();
    chargerAlbums()
      .then((a) => (tous = a.sort((x, y) => x.albumName.localeCompare(y.albumName, 'fr'))))
      .catch(() => (tous = []));
  });

  const options: { genre: Cible['genre']; titre: string; detail: string }[] = [
    {
      genre: 'albumParDossier',
      titre: 'Un album par dossier',
      detail: 'Chaque dossier donne un album à son nom, créé au premier envoi.',
    },
    { genre: 'album', titre: 'Un album existant', detail: 'Toutes les photos du dossier rejoignent cet album.' },
    { genre: 'bibliotheque', titre: 'La photothèque seulement', detail: 'Sans album.' },
  ];
</script>

<Dialogue titre="Envoyer un dossier vers Immich" {onfermer} largeur={520}>
  <form class="space-y-4" onsubmit={valider}>
    <div class="space-y-1">
      <Text size="small" color="muted">Dossier</Text>
      <div class="flex items-center gap-3">
        <span class="chemin min-w-0 flex-1 truncate" title={dossier ?? ''}>{dossier ?? 'Aucun dossier choisi'}</span>
        <Button type="button" variant="outline" size="small" onclick={choisir}>Choisir…</Button>
      </div>
    </div>

    {#if estimationEnCours}
      <div class="flex items-center gap-2 text-sm opacity-70"><LoadingSpinner size="small" /> Analyse du dossier…</div>
    {:else if erreurEstimation}
      <Text size="small" color="danger">{erreurEstimation}</Text>
    {:else if estimation}
      <div class="estimation rounded-xl p-3 text-sm" class:gros>
        {#if estimation.fichiers === 0}
          Aucune photo ni vidéo dans ce dossier.
        {:else}
          <strong>{estimation.fichiers.toLocaleString('fr-FR')} photo{estimation.fichiers > 1 ? 's et vidéos' : ''}, {volume(estimation.octets)}</strong>
          à envoyer, sous-dossiers compris (moins celles déjà sur le serveur, reconnues avant l’envoi).
          {#if gros}
            <span class="mt-1 block">
              C’est un gros envoi : il occupera ce volume sur le serveur et prendra du temps. La pause et le retrait du
              dossier l’arrêtent aussitôt.
            </span>
          {/if}
        {/if}
        {#if estimation.illisibles}<span class="mt-1 block opacity-70">{estimation.illisibles} élément(s) illisible(s) ignoré(s).</span>{/if}
      </div>
    {/if}

    <fieldset class="space-y-2">
      <legend class="mb-1 text-sm opacity-60">Où vont les photos</legend>
      {#each options as o (o.genre)}
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
      {#if !tous}
        <LoadingSpinner />
      {:else}
        <input class="champ w-full" placeholder="Chercher un album" bind:value={filtre} />
        <div class="liste">
          {#each proposes as a (a.id)}
            <label class="ligne" class:choisie={albumChoisi === a.id}>
              <input type="radio" name="album" value={a.id} bind:group={albumChoisi} />
              <span class="truncate">{a.albumName}</span>
              <span class="ml-auto text-xs opacity-60">{a.assetCount}</span>
            </label>
          {:else}
            <Text size="small" color="muted">Aucun album de ce nom.</Text>
          {/each}
        </div>
      {/if}
    {/if}

    <Text size="small" color="muted">
      Rien n’est jamais supprimé : ni les fichiers de ce dossier, ni les photos sur le serveur. Les photos déjà
      présentes sur le serveur ne sont pas renvoyées.
    </Text>
    <div class="flex justify-end gap-2">
      <Button type="button" variant="ghost" color="secondary" onclick={onfermer}>Annuler</Button>
      <Button type="submit" disabled={!pret} loading={enCours}>
        {estimation && estimation.fichiers > 0 ? `Envoyer ${volume(estimation.octets)}` : 'Ajouter'}
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
  .choisie {
    border-color: var(--immich-ui-primary-500);
    background: color-mix(in oklab, var(--immich-ui-primary-500) 8%, transparent);
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
