<script lang="ts">
  // Search results. The criteria live in the URL (set by the search panel
  // of the header); the active ones show as chips above the results, like in
  // the Immich web app, each removable on its own.
  import { Icon, Text } from '@immich/ui';
  import { mdiClose } from '@mdi/js';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { DateTime } from 'luxon';
  import { personne, rechercher, tags, type CriteresRecherche, type ModeRecherche, type Tag } from '../../lib/api';
  import Chronologie from '../../lib/Chronologie.svelte';
  import { estVide, lireCriteres, nomsPersonnes, urlRecherche } from '../../lib/criteres-url';
  import { locale, t, tp } from '../../lib/i18n.svelte';
  import { ChronologieListe } from '../../lib/modele-liste.svelte';
  import { remplacerProchaineNavigation } from '../../lib/navigation.svelte';
  import type { Vue } from '../../lib/vues';

  const criteres = $derived(lireCriteres(page.url.searchParams));
  const noms = $derived(nomsPersonnes(page.url.searchParams));
  const cle = $derived(page.url.search);

  const libellesModes: Record<ModeRecherche, string> = $derived({
    contenu: t('Contexte'),
    fichierNom: t('Nom du fichier'),
    description: t('Description'),
    chemin: t('Chemin complet ou dossier'),
    texte: t('OCR'),
  });

  function changer(modif: Partial<CriteresRecherche>) {
    remplacerProchaineNavigation();
    void goto(urlRecherche({ ...criteres, ...modif }, noms), { replace: true });
  }

  // Names of the people and tags of the chips.
  let nomsTrouves = $state<Record<string, string>>({});
  $effect(() => {
    for (const id of criteres.personnes ?? []) {
      if (noms[id] || nomsTrouves[id] !== undefined) continue;
      nomsTrouves[id] = '';
      personne(id)
        .then((p) => (nomsTrouves[id] = p.name || t('Pas de nom')))
        .catch(() => {});
    }
  });
  let etiquettes = $state<Tag[]>([]);
  $effect(() => {
    if (criteres.etiquettes?.length && !etiquettes.length)
      tags()
        .then((liste) => (etiquettes = liste))
        .catch(() => {});
  });

  const date = (v: string) => DateTime.fromISO(v).setLocale(locale()).toLocaleString({ year: 'numeric', month: 'long', day: 'numeric' });

  type Puce = { cle: string; libelle: string; valeur?: string; retirer: Partial<CriteresRecherche> };
  const puces = $derived.by((): Puce[] => {
    const c = criteres;
    const liste: Puce[] = [];
    if (c.texte?.trim()) liste.push({ cle: 'texte', libelle: libellesModes[c.mode ?? 'contenu'], valeur: c.texte, retirer: { texte: '' } });
    if (c.personnes?.length)
      liste.push({
        cle: 'personnes',
        libelle: t('Personnes'),
        valeur: c.personnes.map((id) => noms[id] || nomsTrouves[id] || '…').join(', '),
        retirer: { personnes: [] },
      });
    if (c.du) liste.push({ cle: 'du', libelle: t('Date de début'), valeur: date(c.du), retirer: { du: undefined } });
    if (c.au) liste.push({ cle: 'au', libelle: t('Date de fin'), valeur: date(c.au), retirer: { au: undefined } });
    if (c.ville) liste.push({ cle: 'ville', libelle: t('Ville'), valeur: c.ville, retirer: { ville: undefined } });
    if (c.region) liste.push({ cle: 'region', libelle: t('Région'), valeur: c.region, retirer: { region: undefined } });
    if (c.pays) liste.push({ cle: 'pays', libelle: t('Pays'), valeur: c.pays, retirer: { pays: undefined } });
    if (c.etiquettes?.length)
      liste.push({
        cle: 'etiquettes',
        libelle: t('Étiquettes'),
        valeur: c.etiquettes.map((id) => etiquettes.find((e) => e.id === id)?.value ?? '…').join(', '),
        retirer: { etiquettes: [] },
      });
    if (c.typeMedia)
      liste.push({ cle: 'type', libelle: t('Type de média'), valeur: c.typeMedia === 'video' ? t('Vidéo') : t('Image'), retirer: { typeMedia: undefined } });
    if (c.marque) liste.push({ cle: 'marque', libelle: t('Marque d’appareil'), valeur: c.marque, retirer: { marque: undefined } });
    if (c.modele) liste.push({ cle: 'modele', libelle: t('Modèle d’appareil'), valeur: c.modele, retirer: { modele: undefined } });
    if (c.objectif) liste.push({ cle: 'objectif', libelle: t('Modèle d’objectif'), valeur: c.objectif, retirer: { objectif: undefined } });
    if (c.note) liste.push({ cle: 'note', libelle: t('Étoile d’évaluation'), valeur: tp(c.note, '{n} étoile', '{n} étoiles'), retirer: { note: undefined } });
    if (c.favoris) liste.push({ cle: 'favoris', libelle: t('Favori'), retirer: { favoris: false } });
    if (c.archive) liste.push({ cle: 'archive', libelle: t('Dans les archives'), retirer: { archive: false } });
    if (c.pasDansAlbum) liste.push({ cle: 'horsAlbum', libelle: t('Dans aucun album'), retirer: { pasDansAlbum: false } });
    return liste;
  });

  function modele(c: CriteresRecherche) {
    return new ChronologieListe((numero) => rechercher(c, numero));
  }

  const vue: Vue = $derived({
    id: 'recherche',
    titre: t('Recherche'),
    chemin: '/recherche',
    filtre: {},
    vide: t('Aucun résultat. Essayez un synonyme ou un mot-clé plus général'),
  });
</script>

<svelte:head><title>{t('Recherche')} · Nicéphore</title></svelte:head>

<div class="flex h-full flex-col">
  {#if puces.length}
    <section class="puces flex shrink-0 flex-wrap justify-center gap-2.5 px-5 py-4" aria-label={t('Filtres de la recherche')}>
      {#each puces as p (p.cle)}
        <div class="puce inline-flex max-w-full items-center rounded-full py-1 ps-1 pe-1 text-xs" data-cle={p.cle}>
          <span class="etiquette shrink-0 rounded-full px-3 py-1.5 font-medium">{p.libelle}</span>
          {#if p.valeur}<span class="valeur min-w-0 truncate px-3 py-1.5">{p.valeur}</span>{/if}
          <button
            type="button"
            class="retirer ms-0.5 flex size-7 shrink-0 items-center justify-center rounded-full"
            aria-label={t('Supprimer le filtre')}
            title={t('Supprimer le filtre')}
            onclick={() => changer(p.retirer)}
          >
            <Icon icon={mdiClose} size="14" />
          </button>
        </div>
      {/each}
    </section>
  {/if}

  <div class="min-h-0 flex-1">
    {#if estVide(criteres)}
      <div class="flex h-full items-center justify-center p-8">
        <Text color="muted">{t('Tapez une recherche en haut de la fenêtre, ou choisissez un filtre.')}</Text>
      </div>
    {:else}
      {#key cle}
        <Chronologie vue={{ ...vue, titre: criteres.texte ? t('« {texte} »', { texte: criteres.texte }) : t('Résultats') }} modele={modele(criteres)} />
      {/key}
    {/if}
  </div>
</div>

<style>
  .puce {
    color: var(--immich-ui-primary-500);
    background: color-mix(in oklab, var(--immich-ui-primary-500) 12%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--immich-ui-primary-500) 18%, transparent);
  }
  .etiquette {
    color: var(--immich-ui-light);
    background: var(--immich-ui-primary-500);
  }
  .valeur {
    max-width: min(36rem, 55vw);
    color: var(--immich-ui-dark);
  }
  .retirer:hover {
    background: color-mix(in oklab, var(--immich-ui-primary-500) 18%, transparent);
  }
</style>
