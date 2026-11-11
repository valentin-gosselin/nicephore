<script lang="ts">
  // Explore: the most photographed people (all of them on the People page)
  // and the places of the library, sortable by number of photos or by name.
  import { Heading, LoadingSpinner, Select, Text } from '@immich/ui';
  import { onMount } from 'svelte';
  import { lieux as chargerLieux, nombresVilles, urlImage, type Lieu } from '../../lib/api';
  import { urlRecherche } from '../../lib/criteres-url';
  import GrillePersonnes from '../../lib/GrillePersonnes.svelte';
  import { locale, t, tp } from '../../lib/i18n.svelte';
  import { apercuFlou } from '../../lib/miniatures';

  type Tri = 'nombre' | 'az' | 'za';
  const TRIS: { valeur: Tri; libelle: string }[] = $derived([
    { valeur: 'nombre', libelle: t('Nombre de photos') },
    { valeur: 'az', libelle: t('Nom, de A à Z') },
    { valeur: 'za', libelle: t('Nom, de Z à A') },
  ]);

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
      if (tri === 'nombre') return (comptes[b.ville] ?? -1) - (comptes[a.ville] ?? -1) || a.ville.localeCompare(b.ville, locale());
      const ordre = a.ville.localeCompare(b.ville, locale(), { sensitivity: 'base' });
      return tri === 'az' ? ordre : -ordre;
    }),
  );

  const nombre = (n: number | undefined) => (n == null ? '' : tp(n, '{n} photo', '{n} photos'));

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

<svelte:head><title>{t('Explorer')} · Nicéphore</title></svelte:head>

<div class="h-full overflow-y-auto">
  <div class="space-y-10 p-6">
    <GrillePersonnes limite={16} />

    <section class="space-y-4">
      <div class="flex flex-wrap items-center gap-3">
        <Heading size="small">{t('Lieux')}</Heading>
        {#if lieux}<Text size="small" color="muted">{tp(lieux.length, '{n} ville', '{n} villes')}</Text>{/if}
        <span class="flex-1"></span>
        <label class="flex items-center gap-2 text-sm">
          <span class="opacity-70">{t('Trier par')}</span>
          <Select
            class="w-52"
            size="tiny"
            title={t('Tri des lieux')}
            options={TRIS.map((o) => ({ value: o.valeur, label: o.libelle }))}
            value={tri}
            onChange={(valeur) => {
              tri = valeur;
              try {
                localStorage.setItem('nicephore-tri-lieux', tri);
              } catch {
                // Only a convenience.
              }
            }}
          />
        </label>
      </div>
      {#if erreur}
        <Text color="danger">{erreur}</Text>
      {:else if !lieux}
        <LoadingSpinner />
      {:else if lieux.length === 0}
        <Text color="muted">{t('Aucun lieu : les photos n’ont pas de position GPS.')}</Text>
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
