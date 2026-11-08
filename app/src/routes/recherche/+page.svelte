<script lang="ts">
  // Search results with their filters. The criteria live in the URL; any
  // change navigates to a new URL, which rebuilds the result list.
  import { Text } from '@immich/ui';
  import { mdiClose } from '@mdi/js';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { rechercher, type CriteresRecherche, type ModeRecherche } from '../../lib/api';
  import Chronologie from '../../lib/Chronologie.svelte';
  import { estVide, lireCriteres, nomsPersonnes, urlRecherche } from '../../lib/criteres-url';
  import { ChronologieListe } from '../../lib/modele-liste.svelte';
  import { remplacerProchaineNavigation } from '../../lib/navigation.svelte';
  import type { Vue } from '../../lib/vues';

  const criteres = $derived(lireCriteres(page.url.searchParams));
  const noms = $derived(nomsPersonnes(page.url.searchParams));
  const cle = $derived(page.url.search);

  const modes: { valeur: ModeRecherche; libelle: string; aide: string }[] = [
    { valeur: 'contenu', libelle: 'Contenu', aide: 'Ce que montre la photo : « plage au coucher du soleil », « chien dans la neige »' },
    { valeur: 'fichierNom', libelle: 'Nom de fichier', aide: 'Nom du fichier d’origine, par exemple IMG_1234' },
    { valeur: 'description', libelle: 'Description', aide: 'Texte de la description saisie dans Immich' },
    { valeur: 'texte', libelle: 'Texte dans l’image', aide: 'Mots lus dans la photo : panneaux, affiches, documents' },
  ];

  function changer(modif: Partial<CriteresRecherche>) {
    remplacerProchaineNavigation();
    void goto(urlRecherche({ ...criteres, ...modif }, noms), { replace: true });
  }

  function retirerPersonne(id: string) {
    changer({ personnes: (criteres.personnes ?? []).filter((p) => p !== id) });
  }

  function modele(c: CriteresRecherche) {
    return new ChronologieListe((numero) => rechercher(c, numero));
  }

  const vue: Vue = { id: 'recherche', titre: 'Recherche', chemin: '/recherche', filtre: {}, vide: 'Aucun résultat.' };
</script>

<svelte:head><title>Recherche · Nicéphore</title></svelte:head>

<div class="flex h-full flex-col">
  <div class="filtres flex shrink-0 flex-wrap items-center gap-2 border-b border-gray-200 px-5 py-3 dark:border-gray-800">
    {#each modes as m (m.valeur)}
      <button class="pastille" class:active={criteres.mode === m.valeur} title={m.aide} onclick={() => changer({ mode: m.valeur })}>
        {m.libelle}
      </button>
    {/each}
    <span class="separateur"></span>
    <select
      class="champ"
      aria-label="Type"
      value={criteres.typeMedia ?? ''}
      onchange={(e) => changer({ typeMedia: (e.currentTarget.value || undefined) as CriteresRecherche['typeMedia'] })}
    >
      <option value="">Photos et vidéos</option>
      <option value="image">Photos</option>
      <option value="video">Vidéos</option>
    </select>
    <label class="flex items-center gap-1 text-sm">
      Du <input class="champ" class:vide={!criteres.du} type="date" value={criteres.du ?? ''} onchange={(e) => changer({ du: e.currentTarget.value || undefined })} />
    </label>
    <label class="flex items-center gap-1 text-sm">
      au <input class="champ" class:vide={!criteres.au} type="date" value={criteres.au ?? ''} onchange={(e) => changer({ au: e.currentTarget.value || undefined })} />
    </label>
    <button class="pastille" class:active={criteres.favoris} onclick={() => changer({ favoris: !criteres.favoris })}>Favoris</button>
    <button class="pastille" class:active={criteres.archive} onclick={() => changer({ archive: !criteres.archive })}>
      Dans l’archive
    </button>
    {#if criteres.ville}
      <button class="pastille active" onclick={() => changer({ ville: undefined })}>
        {criteres.ville}
        <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true"><path fill="currentColor" d={mdiClose} /></svg>
      </button>
    {/if}
    {#each criteres.personnes ?? [] as id (id)}
      <button class="pastille active" onclick={() => retirerPersonne(id)}>
        {noms[id] || 'Personne'}
        <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true"><path fill="currentColor" d={mdiClose} /></svg>
      </button>
    {/each}
  </div>

  <div class="min-h-0 flex-1">
    {#if estVide(criteres)}
      <div class="flex h-full items-center justify-center p-8">
        <Text color="muted">Tapez une recherche en haut de la fenêtre, ou choisissez un filtre.</Text>
      </div>
    {:else}
      {#key cle}
        <Chronologie vue={{ ...vue, titre: criteres.texte ? `« ${criteres.texte} »` : 'Résultats' }} modele={modele(criteres)} />
      {/key}
    {/if}
  </div>
</div>

<style>
  .pastille {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    border-radius: 9999px;
    padding: 4px 12px;
    font-size: 13px;
    border: 1px solid color-mix(in oklab, currentColor 20%, transparent);
  }
  .pastille:hover {
    background: color-mix(in oklab, currentColor 8%, transparent);
  }
  .pastille.active {
    color: var(--immich-ui-primary-500);
    border-color: var(--immich-ui-primary-500);
    background: color-mix(in oklab, var(--immich-ui-primary-500) 12%, transparent);
  }
  .separateur {
    width: 1px;
    height: 20px;
    margin: 0 4px;
    background: color-mix(in oklab, currentColor 20%, transparent);
  }
  .champ {
    border-radius: 8px;
    padding: 3px 8px;
    font-size: 13px;
    background: transparent;
    border: 1px solid color-mix(in oklab, currentColor 20%, transparent);
    color-scheme: light dark;
  }
  /* WebKit shows today's date in an empty date field: dim it. */
  .champ.vide {
    opacity: 0.45;
  }
  .champ option {
    color: initial;
  }
</style>
