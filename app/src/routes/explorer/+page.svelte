<script lang="ts">
  // Explore: the most photographed people (all of them on the People page)
  // and the places of the library, sortable by number of photos or by name.
  import { Heading, LoadingSpinner, Text } from '@immich/ui';
  import { onMount } from 'svelte';
  import { lieux as chargerLieux, nombresVilles, urlImage, type Lieu } from '../../lib/api';
  import { urlRecherche } from '../../lib/criteres-url';
  import GrillePersonnes from '../../lib/GrillePersonnes.svelte';
  import { apercuFlou } from '../../lib/miniatures';

  type Tri = 'nombre' | 'az' | 'za';
  const TRIS: { valeur: Tri; libelle: string }[] = [
    { valeur: 'nombre', libelle: 'Nombre de photos' },
    { valeur: 'az', libelle: 'Nom, de A à Z' },
    { valeur: 'za', libelle: 'Nom, de Z à A' },
  ];

  function lireTri(): Tri {
    try {
      const v = localStorage.getItem('nicephore-tri-lieux');
      if (v === 'nombre' || v === 'az' || v === 'za') return v;
    } catch {
      // Default below.
    }
    return 'nombre';
  }

  let lieux = $state<Lieu[] | null>(null);
  let erreur = $state<string | null>(null);
  let tri = $state<Tri>(lireTri());
  let comptes = $state<Record<string, number>>({});

  const tries = $derived(
    [...(lieux ?? [])].sort((a, b) => {
      if (tri === 'nombre') return (comptes[b.ville] ?? -1) - (comptes[a.ville] ?? -1) || a.ville.localeCompare(b.ville, 'fr');
      const ordre = a.ville.localeCompare(b.ville, 'fr', { sensitivity: 'base' });
      return tri === 'az' ? ordre : -ordre;
    }),
  );

  const nombre = (n: number | undefined) => (n == null ? '' : `${n.toLocaleString('fr-FR')} photo${n > 1 ? 's' : ''}`);

  onMount(() => {
    chargerLieux()
      .then((l) => {
        lieux = l;
        return nombresVilles(l.map((x) => x.ville));
      })
      .then((n) => (comptes = n))
      .catch((e) => (erreur = String(e)));
  });
</script>

<svelte:head><title>Explorer · Nicéphore</title></svelte:head>

<div class="h-full overflow-y-auto">
  <div class="space-y-10 p-6">
    <GrillePersonnes limite={16} />

    <section class="space-y-4">
      <div class="flex flex-wrap items-center gap-3">
        <Heading size="small">Lieux</Heading>
        {#if lieux}<Text size="small" color="muted">{lieux.length} villes</Text>{/if}
        <span class="flex-1"></span>
        <label class="flex items-center gap-2 text-sm">
          <span class="opacity-70">Trier par</span>
          <select
            class="tri"
            aria-label="Tri des lieux"
            value={tri}
            onchange={(e) => {
              tri = e.currentTarget.value as Tri;
              try {
                localStorage.setItem('nicephore-tri-lieux', tri);
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
      {:else if !lieux}
        <LoadingSpinner />
      {:else if lieux.length === 0}
        <Text color="muted">Aucun lieu : les photos n’ont pas de position GPS.</Text>
      {:else}
        <div class="grille-lieux">
          {#each tries as l (l.ville)}
            <a
              href={urlRecherche({ ville: l.ville })}
              class="lieu"
              style:background-image={apercuFlou(l.asset.thumbhash) ? `url(${apercuFlou(l.asset.thumbhash)})` : undefined}
            >
              <img src={urlImage('vignette', l.asset.id, l.asset.thumbhash)} alt="" loading="lazy" />
              <span class="legende">
                <span class="font-semibold">{l.ville}</span>
                <span class="compte-lieu">{nombre(comptes[l.ville])}</span>
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
    font-size: 13px;
    background: transparent;
    border: 1px solid color-mix(in oklab, currentColor 20%, transparent);
    color-scheme: light dark;
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
