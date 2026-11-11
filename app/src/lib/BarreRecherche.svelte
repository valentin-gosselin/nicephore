<script lang="ts">
  /**
   * The search bar of the header, with the filter panel of the Immich web
   * app opening under it: recent searches, "Filtrer par" (search type,
   * people, date, places, tags, media), advanced filters (camera, rating,
   * library), "Effacer tout" and "Recherche". The panel edits a draft of the
   * criteria; searching turns it into the /recherche address.
   */
  import { Button, DatePicker, Icon, IconButton, LoadingSpinner, Text } from '@immich/ui';
  import {
    mdiAccount,
    mdiArrowRight,
    mdiCalendarBlank,
    mdiCheck,
    mdiChevronDown,
    mdiChevronUp,
    mdiClockOutline,
    mdiClose,
    mdiImage,
    mdiMagnify,
    mdiMapMarker,
    mdiTagMultiple,
    mdiTune,
  } from '@mdi/js';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { DateTime } from 'luxon';
  import { fly } from 'svelte/transition';
  import { locale, t, tp } from './i18n.svelte';
  import {
    optionsRecherche,
    personnes as chargerPersonnes,
    suggestionsRecherche,
    tags as chargerEtiquettes,
    urlImage,
    type CriteresRecherche,
    type GenreSuggestion,
    type ModeRecherche,
    type OptionsRecherche,
    type Personne,
    type Tag,
  } from './api';
  import ChampChoix from './ChampChoix.svelte';
  import { lireCriteres, nomsPersonnes, urlRecherche } from './criteres-url';

  let champ = $state<HTMLInputElement>();
  let racine = $state<HTMLDivElement>();
  let ouvert = $state(false);
  // Focus is in the bar or its panel ("isSearchEnabled" in Immich).
  let actif = $state(false);
  // Filters offered as Immich does: tags and ratings by the user's
  // preferences, context and OCR by the server's features.
  let options = $state<OptionsRecherche>({ etiquettes: false, notes: false, contexte: true, ocr: true });
  let optionsChargees = false;
  let avances = $state(false);
  let filtre = $state<'type' | 'personnes' | 'date' | 'lieux' | 'etiquettes' | 'media'>('type');

  // Search type chosen last, remembered as Immich does.
  function modeParDefaut(): ModeRecherche {
    try {
      const m = localStorage.getItem('nicephore-mode-recherche');
      if (m === 'contenu' || m === 'fichierNom' || m === 'description' || m === 'texte' || m === 'chemin') return m;
    } catch {
      // Default below.
    }
    return 'contenu';
  }

  // Draft of the criteria, taken from the search shown when the panel opens.
  let brouillon = $state<CriteresRecherche>({ texte: '', mode: modeParDefaut() });
  let noms = $state<Record<string, string>>({});

  function reprendre() {
    if (page.url.pathname === '/recherche') {
      brouillon = lireCriteres(page.url.searchParams);
      noms = nomsPersonnes(page.url.searchParams);
    } else {
      brouillon = { texte: '', mode: modeParDefaut() };
      noms = {};
    }
  }
  // The field follows the search being shown.
  $effect(() => {
    void page.url.search;
    if (!ouvert) reprendre();
  });

  function ouvrir() {
    if (ouvert) return;
    reprendre();
    ouvert = true;
    if (!optionsChargees) {
      optionsChargees = true;
      optionsRecherche()
        .then((o) => (options = o))
        .catch(() => (optionsChargees = false));
    }
  }
  function fermer() {
    ouvert = false;
  }

  /** Focuses the field (Ctrl+K). */
  export function focaliser() {
    champ?.focus();
    champ?.select();
  }

  // Recent searches (5 at most), as in Immich.
  function lireHistorique(): string[] {
    try {
      const h = JSON.parse(localStorage.getItem('nicephore-recherches-recentes') ?? '[]');
      return Array.isArray(h) ? h.filter((x) => typeof x === 'string') : [];
    } catch {
      return [];
    }
  }
  let historique = $state<string[]>(lireHistorique());
  $effect(() => {
    try {
      localStorage.setItem('nicephore-recherches-recentes', JSON.stringify(historique));
    } catch {
      // Only a convenience.
    }
  });
  const historiqueFiltre = $derived(
    historique.filter((h) => h.toLowerCase().includes((brouillon.texte ?? '').trim().toLowerCase())),
  );

  function chercher() {
    const texte = (brouillon.texte ?? '').trim();
    if (texte) historique = [texte, ...historique.filter((h) => h.toLowerCase() !== texte.toLowerCase())].slice(0, 5);
    fermer();
    champ?.blur();
    void goto(urlRecherche({ ...brouillon, texte }, noms));
  }

  function toutEffacer() {
    brouillon = { texte: brouillon.texte, mode: brouillon.mode };
    noms = {};
  }

  function changerMode(mode: ModeRecherche) {
    brouillon.mode = mode;
    try {
      localStorage.setItem('nicephore-mode-recherche', mode);
    } catch {
      // Only a convenience.
    }
  }

  const tousLesModes: { valeur: ModeRecherche; libelle: string; exemple: string }[] = $derived([
    {
      valeur: 'contenu',
      libelle: t('Contexte'),
      exemple: t('Décrivez la photo que vous recherchez, ex : "Enfants qui sautent sur un trampoline"'),
    },
    { valeur: 'fichierNom', libelle: t('Nom du fichier'), exemple: t('Exemple : IMG_1234.JPG ou PNG') },
    { valeur: 'description', libelle: t('Description'), exemple: t('Randonnée à Sapa') },
    {
      valeur: 'chemin',
      libelle: t('Chemin complet ou dossier'),
      exemple: t('/John/Projects/3D_Printing/2026-07-01 - vous pouvez rechercher Projets, 3D, Impression, 2026, etc.'),
    },
    { valeur: 'texte', libelle: t('Reconnaissance Optique de Caractères OCR'), exemple: t('café latte') },
  ]);
  const modes = $derived(tousLesModes.filter((m) => (m.valeur !== 'contenu' || options.contexte) && (m.valeur !== 'texte' || options.ocr)));
  const modeActuel = $derived(tousLesModes.find((m) => m.valeur === (brouillon.mode ?? 'contenu')) ?? tousLesModes[0]);

  // People, loaded on first use.
  let gens = $state<Personne[] | null>(null);
  let filtreGens = $state('');
  let tousLesGens = $state(false);
  // One row of faces until "Voir toutes les personnes", as Immich does
  // (columns of at least 5rem with 1.25rem gaps).
  let largeurGens = $state(0);
  const parRangee = $derived(Math.max(1, Math.floor((largeurGens + 20) / 100)));
  const nombreGens = $derived((gens ?? []).filter((p) => !filtreGens.trim() || p.name.toLowerCase().includes(filtreGens.trim().toLowerCase())).length);
  $effect(() => {
    if (ouvert && filtre === 'personnes' && !gens)
      chargerPersonnes()
        .then((p) => (gens = p.filter((x) => x.name)))
        .catch(() => (gens = []));
  });
  const gensFiltres = $derived.by(() => {
    const q = filtreGens.trim().toLowerCase();
    const liste = (gens ?? []).filter((p) => !q || p.name.toLowerCase().includes(q));
    const choisis = new Set(brouillon.personnes ?? []);
    liste.sort((a, b) => Number(choisis.has(b.id)) - Number(choisis.has(a.id)));
    return tousLesGens ? liste : liste.slice(0, parRangee);
  });
  function basculerPersonne(p: Personne) {
    const ids = brouillon.personnes ?? [];
    brouillon.personnes = ids.includes(p.id) ? ids.filter((i) => i !== p.id) : [...ids, p.id];
    noms = { ...noms, [p.id]: p.name };
  }

  // Dates: presets, or a custom range.
  const iso = (d: DateTime) => d.toISODate() ?? undefined;
  const presets = {
    cetteAnnee: () => [iso(DateTime.now().startOf('year')), iso(DateTime.now().endOf('year'))],
    anneeDerniere: () => [iso(DateTime.now().minus({ years: 1 }).startOf('year')), iso(DateTime.now().minus({ years: 1 }).endOf('year'))],
    trenteJours: () => [iso(DateTime.now().minus({ days: 30 })), iso(DateTime.now())],
  } as const;
  type Preset = keyof typeof presets | 'perso';
  let presetPerso = $state(false);
  const presetActuel = $derived.by((): Preset | undefined => {
    if (!brouillon.du && !brouillon.au) return presetPerso ? 'perso' : undefined;
    for (const [nom, f] of Object.entries(presets)) {
      const [du, au] = f();
      if (brouillon.du === du && brouillon.au === au) return nom as Preset;
    }
    return 'perso';
  });
  function choisirPreset(p: Preset) {
    if (presetActuel === p) {
      brouillon.du = brouillon.au = undefined;
      presetPerso = false;
      return;
    }
    presetPerso = p === 'perso';
    if (p === 'perso') {
      brouillon.du = brouillon.au = undefined;
    } else {
      [brouillon.du, brouillon.au] = presets[p]();
    }
  }
  const dateInvalide = $derived(!!brouillon.du && !!brouillon.au && brouillon.du > brouillon.au);
  const enDate = (v: string | undefined) => (v ? DateTime.fromISO(v) : undefined);
  const libelleDate = (v: string) => DateTime.fromISO(v).setLocale(locale()).toLocaleString(DateTime.DATE_MED);

  // Places and camera: the values known to the server, narrowed step by step.
  let suggestions = $state<Record<GenreSuggestion, string[]>>({ pays: [], region: [], ville: [], marque: [], modele: [], objectif: [] });
  function charger(genre: GenreSuggestion, filtres: Parameters<typeof suggestionsRecherche>[1]) {
    suggestionsRecherche(genre, filtres)
      .then((v) => (suggestions[genre] = v))
      .catch(() => (suggestions[genre] = []));
  }
  $effect(() => {
    if (!ouvert || filtre !== 'lieux') return;
    const pays = brouillon.pays;
    const region = brouillon.region;
    charger('pays', {});
    charger('region', { pays });
    charger('ville', { pays, region });
  });
  $effect(() => {
    if (!ouvert || !avances) return;
    const marque = brouillon.marque;
    const modele = brouillon.modele;
    charger('marque', {});
    charger('modele', { marque });
    charger('objectif', { marque, modele });
  });

  // Tags.
  let etiquettes = $state<Tag[] | null>(null);
  $effect(() => {
    if (ouvert && (filtre === 'etiquettes' || brouillon.etiquettes?.length) && !etiquettes)
      chargerEtiquettes()
        .then((liste) => (etiquettes = liste.sort((a, b) => a.value.localeCompare(b.value, locale()))))
        .catch(() => (etiquettes = []));
  });
  const nomEtiquette = (id: string) => etiquettes?.find((e) => e.id === id)?.value ?? '';

  // Titles of the "Filtrer par" buttons once a filter is set.
  const titres = $derived({
    type: brouillon.mode && brouillon.mode !== 'contenu' ? modeActuel.libelle : undefined,
    personnes: (() => {
      const ids = brouillon.personnes ?? [];
      if (!ids.length) return undefined;
      const nom = ids.map((i) => noms[i] || gens?.find((p) => p.id === i)?.name).find(Boolean);
      const autres = ids.length - 1;
      if (!nom) return tp(ids.length, '{n} personne', '{n} personnes');
      return autres ? tp(autres, '{nom} + {n} personne', '{nom} + {n} personnes', { nom }) : nom;
    })(),
    date: (() => {
      switch (presetActuel) {
        case 'cetteAnnee':
          return t('Cette année');
        case 'anneeDerniere':
          return t('Année dernière');
        case 'trenteJours':
          return t('30 derniers jours');
        case 'perso':
          return brouillon.du && brouillon.au
            ? t('{du} à {au}', { du: libelleDate(brouillon.du), au: libelleDate(brouillon.au) })
            : brouillon.du
              ? libelleDate(brouillon.du)
              : brouillon.au
                ? libelleDate(brouillon.au)
                : undefined;
      }
      return undefined;
    })(),
    lieux: [brouillon.ville, brouillon.region, brouillon.pays].filter(Boolean).join(', ') || undefined,
    etiquettes: (() => {
      const ids = brouillon.etiquettes ?? [];
      if (!ids.length) return undefined;
      const nom = nomEtiquette(ids[0]);
      if (!nom) return undefined;
      return ids.length > 1 ? tp(ids.length - 1, '{nom} + {n} étiquette', '{nom} + {n} étiquettes', { nom }) : nom;
    })(),
    media: brouillon.typeMedia === 'image' ? t('Image') : brouillon.typeMedia === 'video' ? t('Vidéo') : undefined,
  });

  const tousLesFiltres = $derived([
    { nom: 'type', icone: mdiMagnify, titre: t('Rechercher par type') },
    { nom: 'personnes', icone: mdiAccount, titre: t('Personnes') },
    { nom: 'date', icone: mdiCalendarBlank, titre: t('Date') },
    { nom: 'lieux', icone: mdiMapMarker, titre: t('Lieux') },
    { nom: 'etiquettes', icone: mdiTagMultiple, titre: t('Étiquettes') },
    { nom: 'media', icone: mdiImage, titre: t('Média') },
  ] as const);
  const filtres = $derived(tousLesFiltres.filter((f) => f.nom !== 'etiquettes' || options.etiquettes));

  const avancesActifs = $derived(!!(brouillon.favoris || brouillon.archive || brouillon.pasDansAlbum || brouillon.note));

  // Clicks and focus outside close the panel.
  function surFocusSortant(e: FocusEvent) {
    const cible = e.relatedTarget;
    if (cible instanceof Node && racine?.contains(cible)) return;
    actif = false;
    // Pickers (calendar) render outside: keep the panel open for them.
    if (cible instanceof Element && cible.closest('[data-bits-popover-content], [data-popover-content], [role="dialog"]')) return;
    fermer();
  }
  function surClicFenetre(e: MouseEvent) {
    if (!ouvert) return;
    const cible = e.target;
    if (cible instanceof Node && racine?.contains(cible)) return;
    if (cible instanceof Element && cible.closest('[data-bits-popover-content], [data-popover-content], [data-bits-calendar-root], [role="dialog"]'))
      return;
    actif = false;
    fermer();
  }

  function surTouche(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      fermer();
      champ?.blur();
    }
  }
</script>

<svelte:window onmousedown={surClicFenetre} />

<!-- Classes copied from the Immich web app (SearchBar, SearchFilters and
     their sections), for the same look to the pixel. -->
{#snippet pastille(allume: boolean, texte: string, onclick: () => void, icone?: string, coche = false, classe = '')}
  <Button
    size="small"
    shape="round"
    color={allume ? 'primary' : 'secondary'}
    variant="outline"
    class="{allume ? '' : 'bg-transparent'} {classe}"
    leadingIcon={coche && allume ? mdiCheck : icone}
    {onclick}>{texte}</Button
  >
{/snippet}

<div
  class="relative z-auto w-full outline-none"
  bind:this={racine}
  onfocusin={() => (actif = true)}
  onfocusout={surFocusSortant}
  role="presentation"
  tabindex="-1"
>
  <form
    draggable="false"
    autocomplete="off"
    class="text-sm select-text"
    role="search"
    onsubmit={(e) => {
      e.preventDefault();
      chercher();
    }}
  >
    <!-- Focusable like Immich's wrapper: a click on the panel's text keeps
         the focus inside, so the panel stays open. -->
    <div tabindex="-1" class="outline-none">
      <input
        bind:this={champ}
        bind:value={brouillon.texte}
        type="text"
        class="champ-recherche w-full border-b py-4 ps-14 text-immich-fg/75 transition-all dark:text-immich-dark-fg
          {brouillon.texte ? 'pe-22.5' : 'pe-14'}
          dark:bg-immich-dark-gray
          {ouvert ? 'rounded-t-3xl shadow-[0_8px_20px_rgba(0,0,0,0.12)]' : 'rounded-3xl bg-gray-200'}
          {actif ? 'border-light-200 bg-white dark:border-dark-600' : 'border-transparent'}"
        placeholder={actif ? modeActuel.exemple : t('Rechercher vos photos')}
        aria-label={t('Rechercher')}
        role="combobox"
        aria-expanded={ouvert}
        aria-controls="panneau-recherche"
        onfocus={ouvrir}
        onclick={ouvrir}
        oninput={ouvrir}
        onkeydown={surTouche}
      />

      {#if ouvert}
        <div
          id="panneau-recherche"
          role="listbox"
          transition:fly={{ y: 25, duration: 250 }}
          class="absolute z-[65] max-h-[80svh] w-full overflow-y-auto rounded-b-3xl bg-white shadow-[0_8px_20px_rgba(0,0,0,0.12)] transition-all dark:bg-immich-dark-gray dark:text-gray-300"
        >
          {#if historiqueFiltre.length}
            <div id="history-section">
              <div class="flex items-center justify-between px-5 pt-5">
                <Text fontWeight="medium" class="pb-5">{t('Recherches récentes')}</Text>
                {#if !(brouillon.texte ?? '').trim()}
                  <button
                    type="button"
                    class="rounded-lg p-2 font-semibold text-primary hover:bg-immich-primary/25"
                    aria-label={t('Supprimer les recherches récentes')}
                    onclick={() => (historique = [])}>{t('Effacer tout')}</button
                  >
                {/if}
              </div>
              {#each historiqueFiltre as terme (terme)}
                <div class="flex w-full items-center justify-between text-black dark:text-gray-300">
                  <div class="relative w-full items-center">
                    <button
                      type="button"
                      class="relative flex w-full cursor-pointer gap-3 py-3 ps-5 text-start hover:bg-gray-100 dark:hover:bg-gray-500/30"
                      role="option"
                      aria-selected="false"
                      onclick={() => {
                        brouillon.texte = terme;
                        chercher();
                      }}
                    >
                      <Icon icon={mdiClockOutline} size="1.5em" aria-hidden />
                      {terme}
                    </button>
                    <div class="absolute inset-e-5 top-0 items-center justify-center py-1">
                      <IconButton
                        shape="round"
                        color="secondary"
                        variant="ghost"
                        icon={mdiClose}
                        aria-label={t('Retirer')}
                        size="medium"
                        onclick={() => (historique = historique.filter((h) => h !== terme))}
                      />
                    </div>
                  </div>
                </div>
              {/each}
            </div>
          {/if}

          <div class="px-5">
            <Text class="py-5" fontWeight="medium">{t('Filtrer par')}</Text>
            <div class="flex flex-wrap gap-2">
              {#each filtres as f (f.nom)}
                {@render pastille(
                  filtre === f.nom || !!titres[f.nom],
                  titres[f.nom] ?? f.titre,
                  () => (filtre = f.nom),
                  f.icone,
                  false,
                  filtre === f.nom ? 'border-2' : '',
                )}
              {/each}
            </div>
          </div>

          <div class="px-5 pt-5" data-filtre={filtre}>
            {#if filtre === 'type'}
              <section>
                <fieldset>
                  <Text class="mb-5">{t('Choisir comment Immich interprète votre recherche : par contenu visuel, détails des fichiers, ou texte incrusté.')}</Text>
                  <div class="flex flex-wrap gap-2">
                    {#each modes as m (m.valeur)}
                      {@render pastille((brouillon.mode ?? 'contenu') === m.valeur, m.libelle, () => changerMode(m.valeur), undefined, true)}
                    {/each}
                  </div>
                </fieldset>
              </section>
            {:else if filtre === 'personnes'}
              {#if !gens}
                <div class="-mb-4 flex h-54 items-center justify-center"><LoadingSpinner size="large" /></div>
              {:else if gens.length}
                <div id="people-selection" class="max-h-80 overflow-y-auto">
                  <Text class="pb-5">{t('Sélectionnez une personne ou plus pour trouver les photos dans lesquelles elles apparaissent.')}</Text>
                  <div class="flex h-full place-items-center items-center gap-2 rounded-2xl bg-gray-200 p-2 text-sm dark:bg-gray-800">
                    <IconButton shape="round" color="secondary" variant="ghost" icon={mdiMagnify} aria-label={t('Recherche')} size="small" />
                    <input
                      class="w-full gap-2 bg-gray-200 outline-none dark:bg-gray-800 dark:text-white"
                      type="text"
                      placeholder={t('Filtrer les personnes')}
                      aria-label={t('Filtrer les personnes')}
                      bind:value={filtreGens}
                    />
                    {#if filtreGens}
                      <IconButton shape="round" color="secondary" variant="ghost" icon={mdiClose} aria-label={t('Effacer')} size="small" onclick={() => (filtreGens = '')} />
                    {/if}
                  </div>
                  <div class="grille-gens grid gap-5 overflow-y-auto pt-5" bind:clientWidth={largeurGens}>
                    {#each gensFiltres as p (p.id)}
                      {@const choisi = (brouillon.personnes ?? []).includes(p.id)}
                      <button
                        type="button"
                        class="flex flex-col items-center rounded-3xl border-none p-0 transition-all"
                        aria-pressed={choisi}
                        onclick={() => basculerPersonne(p)}
                      >
                        <div class="relative w-full">
                          <img
                            class="aspect-square w-full rounded-full object-cover shadow-lg transition-shadow duration-150"
                            src={urlImage('personne', p.id, p.updatedAt)}
                            alt={p.name}
                            loading="lazy"
                          />
                          {#if choisi}
                            <div class="absolute top-0 flex size-full items-center justify-center rounded-full bg-primary opacity-75">
                              <Icon icon={mdiCheck} size="32" color="white" />
                            </div>
                          {/if}
                        </div>
                        <p class="mt-2 line-clamp-2 text-sm font-medium dark:text-white">{p.name}</p>
                      </button>
                    {/each}
                  </div>
                  {#if tousLesGens || nombreGens > gensFiltres.length}
                    <div class="mt-2 flex justify-center">
                      <Button
                        size="small"
                        color="primary"
                        variant="ghost"
                        shape="round"
                        leadingIcon={tousLesGens ? mdiClose : mdiArrowRight}
                        class="flex place-items-center gap-2"
                        onclick={() => (tousLesGens = !tousLesGens)}
                      >
                        {tousLesGens ? t('Réduire') : t('Voir toutes les personnes')}
                      </Button>
                    </div>
                  {/if}
                </div>
              {/if}
            {:else if filtre === 'date'}
              <div>
                <Text class="pb-5">{t('Filtrez vos photos en fonction de leur date de capture par une plage prédéfinie ou une période personnalisée.')}</Text>
                <div class="flex flex-wrap gap-2">
                  {@render pastille(presetActuel === 'cetteAnnee', t('Cette année'), () => choisirPreset('cetteAnnee'), undefined, true)}
                  {@render pastille(presetActuel === 'anneeDerniere', t('Année dernière'), () => choisirPreset('anneeDerniere'), undefined, true)}
                  {@render pastille(presetActuel === 'trenteJours', t('30 derniers jours'), () => choisirPreset('trenteJours'), undefined, true)}
                  {@render pastille(presetActuel === 'perso', t('Personnalisé'), () => choisirPreset('perso'), undefined, true)}
                </div>
                {#if presetActuel === 'perso'}
                  <div id="date-range-selection" class="grille-auto-40 grid gap-5 py-5">
                    <div role="group" aria-label={t('Date de début')}>
                      <Text class="mb-2" fontWeight="medium">{t('Date de début')}</Text>
                      <DatePicker value={enDate(brouillon.du)} onChange={(d) => (brouillon.du = d?.isValid ? (d.toISODate() ?? undefined) : undefined)} />
                    </div>
                    <div role="group" aria-label={t('Date de fin')}>
                      <Text class="mb-2" fontWeight="medium">{t('Date de fin')}</Text>
                      <DatePicker value={enDate(brouillon.au)} onChange={(d) => (brouillon.au = d?.isValid ? (d.toISODate() ?? undefined) : undefined)} />
                    </div>
                  </div>
                {/if}
                {#if dateInvalide}<Text color="danger">{t('La date de début doit être avant la date de fin')}</Text>{/if}
              </div>
            {:else if filtre === 'lieux'}
              <div>
                <Text>{t('Chercher des photos prises dans une ville, un pays ou un lieu spécifique.')}</Text>
                <div class="grille-auto-40 mt-5 grid gap-5">
                  <ChampChoix libelle={t("Pays")} indication={t("Rechercher par pays...")} valeur={brouillon.pays} choix={suggestions.pays} onchoix={(v) => (brouillon.pays = v)} />
                  <ChampChoix libelle={t("Région")} indication={t("Rechercher par état/région...")} valeur={brouillon.region} choix={suggestions.region} onchoix={(v) => (brouillon.region = v)} />
                  <ChampChoix libelle={t("Ville")} indication={t("Rechercher par ville...")} valeur={brouillon.ville} choix={suggestions.ville} onchoix={(v) => (brouillon.ville = v)} />
                </div>
              </div>
            {:else if filtre === 'etiquettes'}
              <div>
                <Text class="pb-5">{t('Filtrer par étiquettes ajoutées pour organiser votre bibliothèque.')}</Text>
                {#if !etiquettes}
                  <LoadingSpinner />
                {:else}
                  <ChampChoix
                    libelle={t('Étiquettes')}
                    indication={t("Recherche d'étiquettes...")}
                    valeur={undefined}
                    choix={etiquettes.filter((e) => !(brouillon.etiquettes ?? []).includes(e.id)).map((e) => e.value)}
                    onchoix={(v) => {
                      const e = etiquettes?.find((x) => x.value === v);
                      if (e) brouillon.etiquettes = [...(brouillon.etiquettes ?? []), e.id];
                    }}
                  />
                  {#if brouillon.etiquettes?.length}
                    <section class="flex flex-wrap gap-2 pt-5">
                      {#each brouillon.etiquettes as id (id)}
                        <Button
                          size="small"
                          shape="round"
                          color="primary"
                          variant="outline"
                          trailingIcon={mdiClose}
                          onclick={() => (brouillon.etiquettes = (brouillon.etiquettes ?? []).filter((x) => x !== id))}>{nomEtiquette(id)}</Button
                        >
                      {/each}
                    </section>
                  {/if}
                {/if}
              </div>
            {:else if filtre === 'media'}
              <div id="media-type-selection">
                <fieldset>
                  <Text class="pb-5">{t('Afficher seulement les photos ou vidéos de votre bibliothèque.')}</Text>
                  <div class="flex flex-wrap gap-2">
                    {@render pastille(!brouillon.typeMedia, t('Tout'), () => (brouillon.typeMedia = undefined), undefined, true)}
                    {@render pastille(brouillon.typeMedia === 'image', t('Image'), () => (brouillon.typeMedia = 'image'), undefined, true)}
                    {@render pastille(brouillon.typeMedia === 'video', t('Vidéo'), () => (brouillon.typeMedia = 'video'), undefined, true)}
                  </div>
                </fieldset>
              </div>
            {/if}
          </div>

          <div
            class="grid transition-[grid-template-rows] duration-200 ease-in-out {avances ? 'grid-rows-[1fr]' : 'grid-rows-[0fr]'}"
            inert={!avances}
          >
            <div class="overflow-hidden">
              <div class="my-5 h-px w-full bg-light-200 dark:bg-dark-600"></div>
              <div class="px-5" data-filtre="avances">
                <div id="camera-selection">
                  <Text fontWeight="medium" class="pb-5">{t('Appareil photo')}</Text>
                  <div class="grille-auto-40 grid gap-2">
                    <ChampChoix libelle={t("Marque")} indication={t("Rechercher par marque d'appareil photo...")} valeur={brouillon.marque} choix={suggestions.marque} onchoix={(v) => (brouillon.marque = v)} />
                    <ChampChoix libelle={t("Modèle")} indication={t("Rechercher par modèle d'appareil photo...")} valeur={brouillon.modele} choix={suggestions.modele} onchoix={(v) => (brouillon.modele = v)} />
                    <ChampChoix libelle={t("Modèle d'objectif")} indication={t("Chercher par modèle d'objectif...")} valeur={brouillon.objectif} choix={suggestions.objectif} onchoix={(v) => (brouillon.objectif = v)} />
                  </div>
                </div>
                {#if options.notes}
                  <div class="flex flex-col">
                    <Text class="my-5" fontWeight="medium">{t("Étoile d'évaluation")}</Text>
                    <div class="flex flex-wrap gap-2">
                      {#each [5, 4, 3, 2, 1] as n (n)}
                        {@render pastille(brouillon.note === n, '★'.repeat(n), () => (brouillon.note = brouillon.note === n ? undefined : n))}
                      {/each}
                    </div>
                  </div>
                {/if}
                <div id="display-options-selection">
                  <fieldset>
                    <Text class="py-5" fontWeight="medium">{t('Bibliothèque')}</Text>
                    <div class="flex flex-wrap gap-2">
                      {@render pastille(!!brouillon.favoris, t('Favoris'), () => (brouillon.favoris = !brouillon.favoris), undefined, true)}
                      {@render pastille(!!brouillon.archive, t('Archive'), () => (brouillon.archive = !brouillon.archive), undefined, true)}
                      {@render pastille(!!brouillon.pasDansAlbum, t('Pas dans un album'), () => (brouillon.pasDansAlbum = !brouillon.pasDansAlbum), undefined, true)}
                    </div>
                  </fieldset>
                </div>
              </div>
            </div>
          </div>

          <div class="my-5 h-px w-full bg-light-200 dark:bg-dark-600"></div>
          <div class="flex flex-wrap gap-2 px-5 pb-5">
            <Button
              size="small"
              variant={avancesActifs ? 'outline' : 'ghost'}
              leadingIcon={mdiTune}
              trailingIcon={avances ? mdiChevronUp : mdiChevronDown}
              onclick={() => (avances = !avances)}>{t('Filtres avancés')}</Button
            >
            <div class="flex-1"></div>
            <Button size="small" shape="round" variant="outline" color="secondary" class="bg-transparent" onclick={toutEffacer}>{t('Effacer tout')}</Button>
            <Button size="small" shape="round" onclick={chercher}>{t('Recherche')}</Button>
          </div>
        </div>
      {/if}
    </div>

    {#if brouillon.texte}
      <div class="absolute inset-y-0 inset-e-0 flex items-center pe-2">
        <IconButton
          onclick={() => {
            brouillon.texte = '';
            champ?.focus();
          }}
          icon={mdiClose}
          aria-label={t('Effacer')}
          size="medium"
          color="secondary"
          variant="ghost"
          shape="round"
        />
      </div>
    {/if}
    <div class="absolute inset-y-0 inset-s-0 flex items-center ps-2">
      <IconButton type="submit" aria-label={t('Recherche')} icon={mdiMagnify} size="medium" shape="round" color="secondary" variant="ghost" />
    </div>
  </form>
</div>

<style>
  .champ-recherche {
    outline: none;
  }
  .grille-gens {
    grid-template-columns: repeat(auto-fill, minmax(5rem, 1fr));
  }
  /* Immich's grid-auto-fit-40: columns of at least 10rem. */
  .grille-auto-40 {
    grid-template-columns: repeat(auto-fit, minmax(min(10rem, 100%), 1fr));
  }
</style>
