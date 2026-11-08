<script lang="ts">
  // Explore: the people recognised by Immich and the places of the library,
  // each sortable by number of photos or by name.
  import { Heading, LoadingSpinner, Text } from '@immich/ui';
  import { onMount } from 'svelte';
  import {
    lieux as chargerLieux,
    nombresPersonnes,
    nombresVilles,
    personnes as chargerPersonnes,
    urlImage,
    type Lieu,
    type Personne,
  } from '../../lib/api';
  import { urlRecherche } from '../../lib/criteres-url';
  import { apercuFlou } from '../../lib/miniatures';

  type Tri = 'nombre' | 'az' | 'za';
  const TRIS: { valeur: Tri; libelle: string }[] = [
    { valeur: 'nombre', libelle: 'Nombre de photos' },
    { valeur: 'az', libelle: 'Nom, de A à Z' },
    { valeur: 'za', libelle: 'Nom, de Z à A' },
  ];

  function lireTri(cle: string): Tri {
    try {
      const v = localStorage.getItem(cle);
      if (v === 'nombre' || v === 'az' || v === 'za') return v;
    } catch {
      // Default below.
    }
    return 'nombre';
  }
  function retenirTri(cle: string, tri: Tri) {
    try {
      localStorage.setItem(cle, tri);
    } catch {
      // Only a convenience.
    }
  }

  let personnes = $state<Personne[] | null>(null);
  let lieux = $state<Lieu[] | null>(null);
  let erreur = $state<string | null>(null);
  let sansNom = $state(false);
  let triPersonnes = $state<Tri>(lireTri('nicephore-tri-personnes'));
  let triLieux = $state<Tri>(lireTri('nicephore-tri-lieux'));
  // Photo counts, fetched once per person or city (8 requests at a time).
  let comptesPersonnes = $state<Record<string, number>>({});
  let comptesLieux = $state<Record<string, number>>({});
  let comptageEnCours = $state(false);

  const compare = (tri: Tri, comptes: Record<string, number>) => (a: { nom: string; cle: string }, b: { nom: string; cle: string }) => {
    if (tri === 'nombre') return (comptes[b.cle] ?? -1) - (comptes[a.cle] ?? -1) || a.nom.localeCompare(b.nom, 'fr');
    const ordre = a.nom.localeCompare(b.nom, 'fr', { sensitivity: 'base' });
    return tri === 'az' ? ordre : -ordre;
  };

  const affichees = $derived.by(() => {
    const liste = (personnes ?? []).filter((p) => sansNom || p.name);
    const cmp = compare(triPersonnes, comptesPersonnes);
    return liste.sort((a, b) => {
      // Unnamed faces stay after the named ones, whatever the order.
      if (!a.name !== !b.name) return a.name ? -1 : 1;
      return cmp({ nom: a.name, cle: a.id }, { nom: b.name, cle: b.id });
    });
  });
  const nombreNommees = $derived((personnes ?? []).filter((p) => p.name).length);
  const nombreAnonymes = $derived((personnes ?? []).length - nombreNommees);

  const lieuxTries = $derived(
    [...(lieux ?? [])].sort((a, b) => compare(triLieux, comptesLieux)({ nom: a.ville, cle: a.ville }, { nom: b.ville, cle: b.ville })),
  );

  // Count the photos of the people shown when sorting by number.
  $effect(() => {
    if (triPersonnes !== 'nombre' || !personnes) return;
    const manquants = affichees.map((p) => p.id).filter((id) => !(id in comptesPersonnes));
    if (!manquants.length) return;
    comptageEnCours = true;
    nombresPersonnes(manquants)
      .then((n) => (comptesPersonnes = { ...comptesPersonnes, ...n }))
      .finally(() => (comptageEnCours = false));
  });

  $effect(() => {
    if (!lieux) return;
    const manquants = lieux.map((l) => l.ville).filter((v) => !(v in comptesLieux));
    if (manquants.length) nombresVilles(manquants).then((n) => (comptesLieux = { ...comptesLieux, ...n }));
  });

  const lienPersonne = (p: Personne) => `/personne?id=${p.id}&nom=${encodeURIComponent(p.name)}`;
  const nombre = (n: number | undefined) =>
    n == null ? '' : `${n.toLocaleString('fr-FR')} photo${n > 1 ? 's' : ''}`;

  onMount(() => {
    chargerPersonnes()
      .then((p) => (personnes = p))
      .catch((e) => (erreur = String(e)));
    chargerLieux()
      .then((l) => (lieux = l))
      .catch((e) => (erreur = String(e)));
  });
</script>

{#snippet choixTri(valeur: Tri, changer: (t: Tri) => void, libelle: string)}
  <label class="ml-auto flex items-center gap-2 text-sm">
    <span class="opacity-70">Trier par</span>
    <select class="tri" aria-label={libelle} value={valeur} onchange={(e) => changer(e.currentTarget.value as Tri)}>
      {#each TRIS as t (t.valeur)}<option value={t.valeur}>{t.libelle}</option>{/each}
    </select>
  </label>
{/snippet}

<svelte:head><title>Explorer · Nicéphore</title></svelte:head>

<div class="h-full overflow-y-auto">
  <div class="space-y-10 p-6">
    {#if erreur}<Text color="danger">{erreur}</Text>{/if}

    <section class="space-y-4">
      <div class="flex flex-wrap items-center gap-3">
        <Heading size="small">Personnes</Heading>
        {#if personnes}<Text size="small" color="muted">{nombreNommees} nommées sur {personnes.length}</Text>{/if}
        {#if comptageEnCours}<LoadingSpinner size="small" />{/if}
        {@render choixTri(triPersonnes, (t) => retenirTri('nicephore-tri-personnes', (triPersonnes = t)), 'Tri des personnes')}
      </div>
      {#if !personnes}
        <LoadingSpinner />
      {:else}
        <div class="grille-personnes">
          {#each affichees as p (p.id)}
            <a href={lienPersonne(p)} class="personne" title={p.name || 'Sans nom'}>
              <img src={urlImage('personne', p.id, p.updatedAt)} alt="" loading="lazy" />
              <span class="nom">{p.name || 'Sans nom'}</span>
              <span class="compte">{nombre(comptesPersonnes[p.id])}</span>
            </a>
          {/each}
        </div>
        {#if nombreAnonymes}
          <button class="text-sm text-primary hover:underline" onclick={() => (sansNom = !sansNom)}>
            {sansNom ? 'Masquer les visages sans nom' : `Afficher aussi les ${nombreAnonymes} visages sans nom`}
          </button>
        {/if}
      {/if}
    </section>

    <section class="space-y-4">
      <div class="flex flex-wrap items-center gap-3">
        <Heading size="small">Lieux</Heading>
        {#if lieux}<Text size="small" color="muted">{lieux.length} villes</Text>{/if}
        {@render choixTri(triLieux, (t) => retenirTri('nicephore-tri-lieux', (triLieux = t)), 'Tri des lieux')}
      </div>
      {#if !lieux}
        <LoadingSpinner />
      {:else if lieux.length === 0}
        <Text color="muted">Aucun lieu : les photos n’ont pas de position GPS.</Text>
      {:else}
        <div class="grille-lieux">
          {#each lieuxTries as l (l.ville)}
            <a
              href={urlRecherche({ ville: l.ville })}
              class="lieu"
              style:background-image={apercuFlou(l.asset.thumbhash) ? `url(${apercuFlou(l.asset.thumbhash)})` : undefined}
            >
              <img src={urlImage('vignette', l.asset.id, l.asset.thumbhash)} alt="" loading="lazy" />
              <span class="legende">
                <span class="font-semibold">{l.ville}</span>
                <span class="compte-lieu">{nombre(comptesLieux[l.ville])}</span>
              </span>
            </a>
          {/each}
        </div>
      {/if}
    </section>
  </div>
</div>

<style>
  .tri {
    border-radius: 8px;
    padding: 3px 8px;
    background: transparent;
    border: 1px solid color-mix(in oklab, currentColor 20%, transparent);
    color-scheme: light dark;
  }
  .grille-personnes {
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
  .personne img {
    width: 96px;
    height: 96px;
    margin-bottom: 4px;
    border-radius: 9999px;
    object-fit: cover;
    background: color-mix(in oklab, currentColor 10%, transparent);
    transition: transform 120ms;
  }
  .personne:hover img {
    transform: scale(1.05);
  }
  .personne .nom {
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
  .grille-lieux {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 10px;
  }
  .lieu {
    position: relative;
    aspect-ratio: 1;
    overflow: hidden;
    border-radius: 12px;
    background-size: cover;
  }
  .lieu img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    transition: transform 160ms;
  }
  .lieu:hover img {
    transform: scale(1.04);
  }
  .legende {
    position: absolute;
    inset: auto 0 0 0;
    display: flex;
    flex-direction: column;
    padding: 24px 10px 8px;
    color: white;
    background: linear-gradient(to top, rgb(0 0 0 / 0.75), transparent);
  }
  .compte-lieu {
    font-size: 12px;
    opacity: 0.8;
  }
</style>
