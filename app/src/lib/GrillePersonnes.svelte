<script lang="ts">
  // Grid of people with their face, name and photo count, sortable by
  // number of photos or by name. Explorer shows the first ones; the People
  // page shows everyone, with a name filter and hidden people on request.
  import { Heading, LoadingSpinner, Text } from '@immich/ui';
  import { mdiEyeOff, mdiHeart } from '@mdi/js';
  import { onMount } from 'svelte';
  import { nombresPersonnes, personnes as chargerPersonnes, urlImage, type Personne } from './api';

  let { limite, complet = false }: { limite?: number; complet?: boolean } = $props();

  type Tri = 'nombre' | 'az' | 'za';
  const TRIS: { valeur: Tri; libelle: string }[] = [
    { valeur: 'nombre', libelle: 'Nombre de photos' },
    { valeur: 'az', libelle: 'Nom, de A à Z' },
    { valeur: 'za', libelle: 'Nom, de Z à A' },
  ];

  function lireTri(): Tri {
    try {
      const v = localStorage.getItem('nicephore-tri-personnes');
      if (v === 'nombre' || v === 'az' || v === 'za') return v;
    } catch {
      // Default below.
    }
    return 'nombre';
  }

  let personnes = $state<Personne[] | null>(null);
  let erreur = $state<string | null>(null);
  let sansNom = $state(false);
  let masquees = $state(false);
  let filtre = $state('');
  let tri = $state<Tri>(lireTri());
  let comptes = $state<Record<string, number>>({});
  let comptage = $state(false);

  const nombreNommees = $derived((personnes ?? []).filter((p) => p.name && !p.isHidden).length);
  const nombreAnonymes = $derived((personnes ?? []).filter((p) => !p.name && !p.isHidden).length);
  const nombreMasquees = $derived((personnes ?? []).filter((p) => p.isHidden).length);

  const triees = $derived.by(() => {
    const texte = filtre.trim().toLocaleLowerCase('fr');
    const liste = (personnes ?? []).filter(
      (p) =>
        (masquees || !p.isHidden) &&
        (sansNom || p.name || texte) &&
        (!texte || p.name.toLocaleLowerCase('fr').includes(texte)),
    );
    return liste.sort((a, b) => {
      // Unnamed faces after named people, whatever the order.
      if (!a.name !== !b.name) return a.name ? -1 : 1;
      if (tri === 'nombre') return (comptes[b.id] ?? -1) - (comptes[a.id] ?? -1) || a.name.localeCompare(b.name, 'fr');
      const ordre = a.name.localeCompare(b.name, 'fr', { sensitivity: 'base' });
      return tri === 'az' ? ordre : -ordre;
    });
  });
  const affichees = $derived(limite ? triees.slice(0, limite) : triees);

  // Count the photos of the people shown when sorting by number (all named
  // people are counted for the sort, unnamed ones only when displayed).
  $effect(() => {
    if (!personnes) return;
    const a_compter = (tri === 'nombre' ? triees : affichees).map((p) => p.id).filter((id) => !(id in comptes));
    if (!a_compter.length) return;
    comptage = true;
    nombresPersonnes(a_compter)
      .then((n) => (comptes = { ...comptes, ...n }))
      .finally(() => (comptage = false));
  });

  const lien = (p: Personne) => `/personne?id=${p.id}&nom=${encodeURIComponent(p.name)}`;
  const nombre = (n: number | undefined) => (n == null ? '' : `${n.toLocaleString('fr-FR')} photo${n > 1 ? 's' : ''}`);

  onMount(() => {
    chargerPersonnes(complet)
      .then((p) => (personnes = p))
      .catch((e) => (erreur = String(e)));
  });
</script>

<section class="space-y-4">
  <div class="flex flex-wrap items-center gap-3">
    <Heading size="small">Personnes</Heading>
    {#if personnes}<Text size="small" color="muted">{nombreNommees} nommées sur {nombreNommees + nombreAnonymes}</Text>{/if}
    {#if comptage}<LoadingSpinner size="small" />{/if}
    <span class="flex-1"></span>
    {#if limite}
      <a href="/personnes" class="text-sm text-primary hover:underline">Toutes les personnes</a>
    {:else}
      <input class="champ w-48" type="search" bind:value={filtre} placeholder="Chercher un nom" aria-label="Chercher un nom" />
    {/if}
    <label class="flex items-center gap-2 text-sm">
      <span class="opacity-70">Trier par</span>
      <select
        class="champ"
        value={tri}
        aria-label="Tri des personnes"
        onchange={(e) => {
          tri = e.currentTarget.value as Tri;
          try {
            localStorage.setItem('nicephore-tri-personnes', tri);
          } catch {
            // Only a convenience.
          }
        }}
      >
        {#each TRIS as t (t.valeur)}<option value={t.valeur}>{t.libelle}</option>{/each}
      </select>
    </label>
  </div>

  {#if erreur}
    <Text color="danger">{erreur}</Text>
  {:else if !personnes}
    <LoadingSpinner />
  {:else}
    <div class="grille">
      {#each affichees as p (p.id)}
        <a href={lien(p)} class="personne" class:masquee={p.isHidden} title={p.name || 'Sans nom'}>
          <span class="visage">
            <img src={urlImage('personne', p.id, p.updatedAt)} alt="" loading="lazy" />
            {#if p.isFavorite}
              <svg class="marque" viewBox="0 0 24 24" width="18" height="18" aria-label="Favori"><path fill="currentColor" d={mdiHeart} /></svg>
            {:else if p.isHidden}
              <svg class="marque" viewBox="0 0 24 24" width="18" height="18" aria-label="Masquée"><path fill="currentColor" d={mdiEyeOff} /></svg>
            {/if}
          </span>
          <span class="nom">{p.name || 'Sans nom'}</span>
          <span class="compte">{nombre(comptes[p.id])}</span>
        </a>
      {/each}
    </div>
    {#if !limite}
      <div class="flex flex-wrap gap-4">
        {#if nombreAnonymes}
          <button class="text-sm text-primary hover:underline" onclick={() => (sansNom = !sansNom)}>
            {sansNom ? 'Masquer les visages sans nom' : `Afficher aussi les ${nombreAnonymes} visages sans nom`}
          </button>
        {/if}
        {#if nombreMasquees}
          <button class="text-sm text-primary hover:underline" onclick={() => (masquees = !masquees)}>
            {masquees ? 'Cacher les personnes masquées' : `Afficher les ${nombreMasquees} personnes masquées`}
          </button>
        {/if}
      </div>
    {/if}
  {/if}
</section>

<style>
  .champ {
    border-radius: 8px;
    padding: 3px 8px;
    font-size: 13px;
    background: transparent;
    border: 1px solid color-mix(in oklab, currentColor 20%, transparent);
    color-scheme: light dark;
  }
  .grille {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(112px, 1fr));
    gap: 16px 12px;
  }
  .personne {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    font-size: 13px;
    text-align: center;
  }
  .masquee {
    opacity: 0.5;
  }
  .visage {
    position: relative;
    margin-bottom: 4px;
  }
  .visage img {
    width: 96px;
    height: 96px;
    border-radius: 9999px;
    object-fit: cover;
    background: color-mix(in oklab, currentColor 10%, transparent);
    transition: transform 120ms;
  }
  .personne:hover img {
    transform: scale(1.05);
  }
  .marque {
    position: absolute;
    right: 2px;
    bottom: 2px;
    color: white;
    filter: drop-shadow(0 1px 2px rgb(0 0 0 / 0.8));
  }
  .nom {
    max-width: 112px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .compte {
    min-height: 1.2em;
    font-size: 11px;
    opacity: 0.6;
  }
</style>
