<script lang="ts">
  // One person: their timeline, plus naming, favorite, hiding and merging
  // with the faces Immich finds most similar.
  import { Button, IconButton, Input, LoadingSpinner, Text } from '@immich/ui';
  import { mdiCallMerge, mdiEyeOffOutline, mdiEyeOutline, mdiHeart, mdiHeartOutline } from '@mdi/js';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import {
    fusionnerPersonnes,
    modifierPersonne,
    personne as chargerPersonne,
    personnes as chargerPersonnes,
    personnesProches,
    urlImage,
    type Personne,
  } from '../../lib/api';
  import Chronologie from '../../lib/Chronologie.svelte';
  import Dialogue from '../../lib/Dialogue.svelte';
  import { remplacerProchaineNavigation } from '../../lib/navigation.svelte';
  import { notifier } from '../../lib/notifications.svelte';
  import TexteModifiable from '../../lib/TexteModifiable.svelte';
  import type { Vue } from '../../lib/vues';

  const id = $derived(page.url.searchParams.get('id') ?? '');
  let infos = $state<Personne | null>(null);
  let version = $state(0);

  // Merge dialog.
  let fusion = $state(false);
  let proches = $state<Personne[] | null>(null);
  let toutes = $state<Personne[] | null>(null);
  let recherche = $state('');
  let choisies = $state<string[]>([]);
  let confirmation = $state(false);
  let fusionEnCours = $state(false);

  $effect(() => {
    const demande = id;
    infos = null;
    chargerPersonne(demande)
      .then((p) => demande === id && (infos = p))
      .catch((e) => notifier(`Personne introuvable : ${e}`, 'erreur'));
  });

  const titre = $derived(infos ? infos.name || 'Sans nom' : page.url.searchParams.get('nom') || 'Sans nom');
  const vue = $derived<Vue>({
    id: 'personne',
    titre,
    chemin: '/personne',
    filtre: { personne: id },
    vide: 'Aucune photo de cette personne.',
  });

  async function modifier(modification: Parameters<typeof modifierPersonne>[1], message: string) {
    try {
      infos = await modifierPersonne(id, modification);
      notifier(message, 'succes');
      // Keep the URL (and so the back history) in step with the name.
      remplacerProchaineNavigation();
      void goto(`/personne?id=${id}&nom=${encodeURIComponent(infos.name)}`, { replace: true });
    } catch (e) {
      notifier(`Modification impossible : ${e}`, 'erreur');
    }
  }

  async function ouvrirFusion() {
    fusion = true;
    choisies = [];
    recherche = '';
    proches = null;
    personnesProches(id)
      .then((p) => (proches = p))
      .catch((e) => notifier(`Suggestions indisponibles : ${e}`, 'erreur'));
    if (!toutes) chargerPersonnes().then((p) => (toutes = p.filter((x) => x.id !== id)));
  }

  // Similar faces first; typing a name searches every named person.
  const candidates = $derived.by(() => {
    const texte = recherche.trim().toLocaleLowerCase('fr');
    if (!texte) return proches ?? [];
    return (toutes ?? []).filter((p) => p.name.toLocaleLowerCase('fr').includes(texte)).slice(0, 60);
  });

  function basculer(autre: string) {
    choisies = choisies.includes(autre) ? choisies.filter((x) => x !== autre) : [...choisies, autre];
  }

  async function fusionner() {
    fusionEnCours = true;
    try {
      const n = await fusionnerPersonnes(id, choisies);
      notifier(`${n} fiche${n > 1 ? 's' : ''} fusionnée${n > 1 ? 's' : ''} dans « ${titre} »`, 'succes');
      confirmation = false;
      fusion = false;
      infos = await chargerPersonne(id);
      version++;
    } catch (e) {
      notifier(`Fusion impossible : ${e}`, 'erreur');
    } finally {
      fusionEnCours = false;
    }
  }

  const nomDe = (autre: string) =>
    [...(proches ?? []), ...(toutes ?? [])].find((p) => p.id === autre)?.name || 'Sans nom';
</script>

<svelte:head><title>{titre} · Nicéphore</title></svelte:head>

{#snippet nomPersonne()}
  <TexteModifiable
    titre
    valeur={infos?.name ?? ''}
    vide="Nommer cette personne"
    modifiable={!!infos}
    onvalider={(v) => modifier({ name: v }, v ? `Nommée « ${v} »` : 'Nom retiré')}
  />
{/snippet}

{#snippet actions()}
  {#if infos}
    <span class="flex-1"></span>
    <IconButton
      icon={infos.isFavorite ? mdiHeart : mdiHeartOutline}
      aria-label={infos.isFavorite ? 'Retirer des favoris' : 'Mettre en favori'}
      variant="ghost"
      shape="round"
      onclick={() => modifier({ isFavorite: !infos!.isFavorite }, infos!.isFavorite ? 'Retirée des favoris' : 'Mise en favori')}
    />
    <IconButton
      icon={infos.isHidden ? mdiEyeOutline : mdiEyeOffOutline}
      aria-label={infos.isHidden ? 'Ne plus masquer' : 'Masquer cette personne'}
      variant="ghost"
      shape="round"
      onclick={() =>
        modifier(
          { isHidden: !infos!.isHidden },
          infos!.isHidden ? 'De nouveau visible' : 'Masquée : elle n’apparaît plus dans Explorer ni Personnes',
        )}
    />
    <IconButton icon={mdiCallMerge} aria-label="Fusionner avec d’autres fiches" variant="ghost" shape="round" onclick={ouvrirFusion} />
  {/if}
{/snippet}

{#key `${id}:${version}`}
  <Chronologie {vue} entete={actions} titre={nomPersonne} />
{/key}

{#if fusion && !confirmation}
  <Dialogue titre={`Fusionner avec « ${titre} »`} largeur={640} onfermer={() => (fusion = false)}>
    <div class="space-y-4">
      <Text size="small" color="muted">
        Choisissez les fiches qui sont la même personne. Les visages les plus ressemblants viennent en premier ; tapez un
        nom pour chercher parmi toutes les personnes.
      </Text>
      <Input bind:value={recherche} placeholder="Chercher un nom" />
      {#if !proches && !recherche}
        <LoadingSpinner />
      {:else}
        <div class="grille">
          {#each candidates as p (p.id)}
            <button class="candidat" class:choisie={choisies.includes(p.id)} onclick={() => basculer(p.id)} aria-pressed={choisies.includes(p.id)}>
              <img src={urlImage('personne', p.id, p.updatedAt)} alt="" loading="lazy" />
              <span>{p.name || 'Sans nom'}</span>
            </button>
          {:else}
            <Text color="muted">Aucune fiche trouvée.</Text>
          {/each}
        </div>
      {/if}
      <div class="flex justify-end gap-2">
        <Button variant="ghost" color="secondary" onclick={() => (fusion = false)}>Annuler</Button>
        <Button disabled={!choisies.length} onclick={() => (confirmation = true)}>
          Fusionner {choisies.length || ''} fiche{choisies.length > 1 ? 's' : ''}
        </Button>
      </div>
    </div>
  </Dialogue>
{/if}

{#if confirmation}
  <Dialogue titre="Confirmer la fusion" onfermer={() => (confirmation = false)}>
    <div class="space-y-4">
      <Text>
        {choisies.map(nomDe).join(', ')} {choisies.length > 1 ? 'rejoignent' : 'rejoint'} « {titre} » : leurs visages
        passent sur cette fiche, et {choisies.length > 1 ? 'leurs fiches disparaissent' : 'sa fiche disparaît'}.
      </Text>
      <Text color="danger">Immich ne permet pas d’annuler une fusion.</Text>
      <div class="flex justify-end gap-2">
        <Button variant="ghost" color="secondary" onclick={() => (confirmation = false)}>Revenir</Button>
        <Button color="danger" loading={fusionEnCours} onclick={fusionner}>Fusionner</Button>
      </div>
    </div>
  </Dialogue>
{/if}

<style>
  .grille {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(96px, 1fr));
    gap: 12px;
  }
  .candidat {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 6px;
    border-radius: 12px;
    font-size: 12px;
    border: 2px solid transparent;
  }
  .candidat img {
    width: 72px;
    height: 72px;
    border-radius: 9999px;
    object-fit: cover;
  }
  .candidat span {
    max-width: 90px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .candidat:hover {
    background: color-mix(in oklab, currentColor 8%, transparent);
  }
  .candidat.choisie {
    border-color: var(--immich-ui-primary-500);
    background: color-mix(in oklab, var(--immich-ui-primary-500) 14%, transparent);
  }
</style>
