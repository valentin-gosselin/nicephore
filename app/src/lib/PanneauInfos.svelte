<script lang="ts">
  // Information panel of the viewer, editable in place: description, star
  // rating, capture date, location and tags.
  import { LoadingSpinner } from '@immich/ui';
  import { mdiClose, mdiPencilOutline, mdiPlus, mdiStar, mdiStarOutline } from '@mdi/js';
  import { etiqueter, infosAsset, modifierInfos, type ModificationInfos } from './api';
  import { formaterDate, versLocal } from './dates';
  import DialogueDate from './DialogueDate.svelte';
  import DialogueLieu from './DialogueLieu.svelte';
  import DialogueTag from './DialogueTag.svelte';
  import { notifier } from './notifications.svelte';
  import TexteModifiable from './TexteModifiable.svelte';

  let { id, onmodifie }: { id: string; onmodifie?: (date: boolean) => void } = $props();

  let infos = $state<Record<string, any> | null>(null);
  let erreur = $state<string | null>(null);
  let dialogue = $state<'date' | 'lieu' | 'tag' | null>(null);

  function charger(demande: string) {
    return infosAsset(demande)
      .then((r) => demande === id && (infos = r))
      .catch((e) => demande === id && (erreur = String(e)));
  }

  $effect(() => {
    const demande = id;
    infos = null;
    erreur = null;
    dialogue = null;
    void charger(demande);
  });

  const exif = $derived(infos?.exifInfo ?? {});
  const date = $derived(infos ? formaterDate(exif.dateTimeOriginal ?? infos.fileCreatedAt, exif.timeZone) : '');
  const appareil = $derived([exif.make, exif.model].filter(Boolean).join(' '));
  const reglages = $derived(
    [
      exif.fNumber && `f/${exif.fNumber}`,
      exif.exposureTime && `${exif.exposureTime} s`,
      exif.focalLength && `${exif.focalLength} mm`,
      exif.iso && `ISO ${exif.iso}`,
    ]
      .filter(Boolean)
      .join(' · '),
  );
  const lieu = $derived([exif.city, exif.state, exif.country].filter(Boolean).join(', '));
  const taille = $derived(
    exif.fileSizeInByte ? `${(exif.fileSizeInByte / 1_048_576).toLocaleString('fr-FR', { maximumFractionDigits: 1 })} Mo` : '',
  );
  const dimensions = $derived(exif.exifImageWidth ? `${exif.exifImageWidth} × ${exif.exifImageHeight}` : '');
  const personnes = $derived(((infos?.people ?? []) as { name: string }[]).map((p) => p.name).filter(Boolean));
  const tags = $derived(((infos?.tags ?? []) as { id: string; value: string }[]).map((t) => t.value));
  const note = $derived<number>(exif.rating ?? 0);

  async function modifier(modification: ModificationInfos, message: string) {
    const demande = id;
    try {
      await modifierInfos([demande], modification);
      notifier(message, 'succes');
      onmodifie?.(modification.date !== undefined);
      // Date and location are applied by a server job: read again shortly.
      await charger(demande);
      if (modification.date !== undefined || modification.latitude !== undefined) setTimeout(() => charger(demande), 1500);
    } catch (e) {
      notifier(`Modification impossible : ${e}`, 'erreur');
    }
  }

  async function changerTag(nom: string, retirer: boolean) {
    const demande = id;
    try {
      await etiqueter(nom, [demande], retirer);
      notifier(retirer ? `Tag « ${nom} » retiré` : `Tag « ${nom} » ajouté`, 'succes');
      await charger(demande);
    } catch (e) {
      notifier(`Tag impossible : ${e}`, 'erreur');
    }
  }
</script>

{#snippet icone(chemin: string, taille = 16)}
  <svg viewBox="0 0 24 24" width={taille} height={taille} aria-hidden="true"><path fill="currentColor" d={chemin} /></svg>
{/snippet}

<aside class="panneau h-full w-80 shrink-0 overflow-y-auto bg-gray-950 p-5 text-sm">
  <h2 class="mb-4 text-base font-semibold">Infos</h2>
  {#if erreur}
    <p class="text-red-400">{erreur}</p>
  {:else if !infos}
    <LoadingSpinner />
  {:else}
    <dl class="space-y-4">
      <div>
        <TexteModifiable
          valeur={exif.description ?? ''}
          vide="Ajouter une description"
          onvalider={(v) => modifier({ description: v }, 'Description enregistrée')}
        />
      </div>
      <div>
        <dt>Note</dt>
        <dd><div class="flex gap-0.5" role="radiogroup" aria-label="Note">
          {#each [1, 2, 3, 4, 5] as n (n)}
            <button
              type="button"
              class="etoile"
              class:allumee={n <= note}
              role="radio"
              aria-checked={n === note}
              aria-label={`${n} étoile${n > 1 ? 's' : ''}`}
              onclick={() => modifier({ note: n === note ? null : n }, n === note ? 'Note retirée' : `${n} étoile${n > 1 ? 's' : ''}`)}
            >
              {@render icone(n <= note ? mdiStar : mdiStarOutline, 20)}
            </button>
          {/each}
        </div></dd>
      </div>
      <div>
        <dt>Date</dt>
        <dd>
          <button type="button" class="modifiable" onclick={() => (dialogue = 'date')}>
            {date}{@render icone(mdiPencilOutline)}
          </button>
        </dd>
      </div>
      <div>
        <dt>Lieu</dt>
        <dd>
          <button type="button" class="modifiable" class:absent={!lieu} onclick={() => (dialogue = 'lieu')}>
            {lieu || 'Ajouter un lieu'}{@render icone(mdiPencilOutline)}
          </button>
        </dd>
      </div>
      <div>
        <dt>Tags</dt>
        <dd class="flex flex-wrap gap-1.5">
          {#each tags as t (t)}
            <span class="tag">
              {t}
              <button type="button" aria-label={`Retirer le tag ${t}`} onclick={() => changerTag(t, true)}>{@render icone(mdiClose, 14)}</button>
            </span>
          {/each}
          <button type="button" class="tag ajout" onclick={() => (dialogue = 'tag')}>{@render icone(mdiPlus, 14)} Tag</button>
        </dd>
      </div>
      <div>
        <dt>Fichier</dt>
        <dd class="break-all">{infos.originalFileName}</dd>
        <dd class="opacity-70">{[dimensions, taille].filter(Boolean).join(' · ')}</dd>
      </div>
      {#if appareil}
        <div>
          <dt>Appareil</dt>
          <dd>{appareil}</dd>
          {#if exif.lensModel}<dd class="opacity-70">{exif.lensModel}</dd>{/if}
          {#if reglages}<dd class="opacity-70">{reglages}</dd>{/if}
        </div>
      {/if}
      {#if personnes.length}<div><dt>Personnes</dt><dd>{personnes.join(', ')}</dd></div>{/if}
    </dl>
  {/if}
</aside>

{#if dialogue === 'date' && infos}
  <DialogueDate
    nombre={1}
    initiale={versLocal(exif.dateTimeOriginal ?? infos.fileCreatedAt, exif.timeZone)}
    fuseauInitial={exif.timeZone ?? null}
    onvalider={(d, f) => {
      dialogue = null;
      void modifier({ date: d, fuseau: f }, 'Date enregistrée');
    }}
    onfermer={() => (dialogue = null)}
  />
{:else if dialogue === 'lieu'}
  <DialogueLieu
    nombre={1}
    latitude={exif.latitude ?? null}
    longitude={exif.longitude ?? null}
    onvalider={(lat, lon) => {
      dialogue = null;
      void modifier({ latitude: lat, longitude: lon }, 'Lieu enregistré');
    }}
    onfermer={() => (dialogue = null)}
  />
{:else if dialogue === 'tag'}
  <DialogueTag
    nombre={1}
    onvalider={(nom) => {
      dialogue = null;
      void changerTag(nom, false);
    }}
    onfermer={() => (dialogue = null)}
  />
{/if}

<style>
  dt {
    margin-bottom: 2px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    opacity: 0.55;
  }
  .modifiable {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    text-align: left;
  }
  .modifiable :global(svg) {
    opacity: 0;
    transition: opacity 120ms;
  }
  .modifiable:hover :global(svg) {
    opacity: 0.6;
  }
  .absent {
    font-style: italic;
    opacity: 0.6;
  }
  .etoile {
    color: color-mix(in oklab, currentColor 45%, transparent);
  }
  .etoile.allumee {
    color: #f2b52e;
  }
  .tag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    border-radius: 9999px;
    padding: 2px 6px 2px 10px;
    font-size: 12px;
    background: color-mix(in oklab, currentColor 12%, transparent);
  }
  .tag.ajout {
    padding: 2px 10px 2px 6px;
    background: transparent;
    border: 1px dashed color-mix(in oklab, currentColor 30%, transparent);
  }
</style>
