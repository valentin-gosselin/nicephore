<script lang="ts">
  // Details of a photo of this computer, read from its EXIF: same look as
  // the info panel of the library, read only.
  import { LoadingSpinner } from '@immich/ui';
  import { infosFichierLocal, lieuDuPoint, type FichierOuvert, type InfosLocales } from './api';
  import { locale, t } from './i18n.svelte';

  let { fichier, dimensions }: { fichier: FichierOuvert; dimensions: { l: number; h: number } | null } = $props();

  let infos = $state<InfosLocales | null>(null);
  let lieu = $state<string | null>(null);

  $effect(() => {
    const chemin = fichier.chemin;
    infos = null;
    lieu = null;
    infosFichierLocal(chemin)
      .then(async (i) => {
        if (chemin !== fichier.chemin) return;
        infos = i;
        if (i.latitude != null && i.longitude != null) {
          const l = await lieuDuPoint(i.latitude, i.longitude).catch(() => null);
          if (chemin === fichier.chemin) lieu = l;
        }
      })
      .catch(() => chemin === fichier.chemin && (infos = vide));
  });

  const vide: InfosLocales = {
    datePrise: null,
    appareil: null,
    objectif: null,
    focale: null,
    ouverture: null,
    exposition: null,
    iso: null,
    latitude: null,
    longitude: null,
  };

  const options: Intl.DateTimeFormatOptions = { dateStyle: 'full', timeStyle: 'short' };
  // The camera's own clock: its offset, when written, is left aside.
  const date = $derived(
    infos?.datePrise
      ? new Date(infos.datePrise.slice(0, 19)).toLocaleString(locale(), options)
      : new Date(fichier.modifieLe).toLocaleString(locale(), options),
  );
  const nombre = (n: number, decimales = 1) => n.toLocaleString(locale(), { maximumFractionDigits: decimales });
  const exposition = (s: number) => (s >= 1 ? `${nombre(s)} s` : `1/${Math.round(1 / s)} s`);
  const reglages = $derived(
    infos
      ? [
          infos.ouverture && `ƒ/${nombre(infos.ouverture)}`,
          infos.exposition && exposition(infos.exposition),
          infos.focale && `${nombre(infos.focale, 0)} mm`,
          infos.iso && `ISO ${infos.iso}`,
        ]
          .filter(Boolean)
          .join(' · ')
      : '',
  );
  const taille = $derived(
    fichier.taille >= 1_048_576
      ? t('{taille} Mo', { taille: nombre(fichier.taille / 1_048_576) })
      : t('{n} ko', { n: Math.ceil(fichier.taille / 1024) }),
  );
  const mesures = $derived(
    [dimensions && `${dimensions.l} × ${dimensions.h}`, dimensions && `${nombre((dimensions.l * dimensions.h) / 1e6)} MP`, taille]
      .filter(Boolean)
      .join(' · '),
  );
  const dossier = $derived(fichier.chemin.slice(0, Math.max(fichier.chemin.lastIndexOf('/'), fichier.chemin.lastIndexOf('\\'))));
  const coordonnees = $derived(
    infos?.latitude != null && infos.longitude != null ? `${nombre(infos.latitude, 5)}, ${nombre(infos.longitude, 5)}` : '',
  );
</script>

<aside class="panneau h-full w-80 shrink-0 overflow-y-auto bg-gray-950 p-5 text-sm" aria-label={t('Infos')}>
  <h2 class="mb-4 text-base font-semibold">{t('Infos')}</h2>
  {#if !infos}
    <LoadingSpinner />
  {:else}
    <dl class="space-y-4">
      <div>
        <dt>{infos.datePrise ? t('Date') : t('Modifiée le')}</dt>
        <dd>{date}</dd>
      </div>
      {#if coordonnees}
        <div>
          <dt>{t('Lieu')}</dt>
          {#if lieu}<dd>{lieu}</dd>{/if}
          <dd class:opacity-70={lieu}>{coordonnees}</dd>
        </div>
      {/if}
      <div>
        <dt>{t('Fichier')}</dt>
        <dd class="break-all">{fichier.nom}</dd>
        <dd class="opacity-70">{mesures}</dd>
      </div>
      <div>
        <dt>{t('Dossier')}</dt>
        <dd class="break-all opacity-90">{dossier}</dd>
      </div>
      {#if infos.appareil || infos.objectif || reglages}
        <div>
          <dt>{t('Appareil')}</dt>
          {#if infos.appareil}<dd>{infos.appareil}</dd>{/if}
          {#if infos.objectif}<dd class="opacity-70">{infos.objectif}</dd>{/if}
          {#if reglages}<dd class="opacity-70">{reglages}</dd>{/if}
        </div>
      {/if}
    </dl>
  {/if}
</aside>

<style>
  dt {
    margin-bottom: 2px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    opacity: 0.55;
  }
</style>
