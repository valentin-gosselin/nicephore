<script lang="ts">
  import { LoadingSpinner } from '@immich/ui';
  import { infosAsset } from './api';

  let { id }: { id: string } = $props();

  let infos = $state<Record<string, any> | null>(null);
  let erreur = $state<string | null>(null);

  $effect(() => {
    const demande = id;
    infos = null;
    erreur = null;
    infosAsset(demande)
      .then((r) => demande === id && (infos = r))
      .catch((e) => demande === id && (erreur = String(e)));
  });

  const exif = $derived(infos?.exifInfo ?? {});
  const date = $derived(infos ? formaterDate(exif.dateTimeOriginal ?? infos.fileCreatedAt, exif.timeZone) : '');

  /** Formats the capture date in its own time zone. Immich stores either an
   * IANA name ("Europe/Paris") or a fixed offset ("UTC+1", "UTC-03:30"). */
  function formaterDate(iso: string, fuseau: string | null | undefined): string {
    const options: Intl.DateTimeFormatOptions = { dateStyle: 'full', timeStyle: 'short' };
    let instant = new Date(iso);
    const decalage = fuseau?.match(/^UTC([+-])(\d{1,2})(?::?(\d{2}))?$/);
    if (decalage) {
      const minutes = (Number(decalage[2]) * 60 + Number(decalage[3] ?? 0)) * (decalage[1] === '-' ? -1 : 1);
      instant = new Date(instant.getTime() + minutes * 60_000);
      return instant.toLocaleString('fr-FR', { ...options, timeZone: 'UTC' });
    }
    try {
      return instant.toLocaleString('fr-FR', { ...options, timeZone: fuseau ?? undefined });
    } catch {
      return instant.toLocaleString('fr-FR', options);
    }
  }
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
  const personnes = $derived(
    ((infos?.people ?? []) as { name: string }[]).map((p) => p.name).filter(Boolean),
  );
</script>

<aside class="panneau h-full w-80 shrink-0 overflow-y-auto bg-gray-950 p-5 text-sm">
  <h2 class="mb-4 text-base font-semibold">Infos</h2>
  {#if erreur}
    <p class="text-red-400">{erreur}</p>
  {:else if !infos}
    <LoadingSpinner />
  {:else}
    <dl class="space-y-4">
      {#if infos.exifInfo?.description}
        <div><dd class="whitespace-pre-line">{infos.exifInfo.description}</dd></div>
      {/if}
      <div><dt>Date</dt><dd>{date}</dd></div>
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
      {#if lieu}<div><dt>Lieu</dt><dd>{lieu}</dd></div>{/if}
      {#if personnes.length}<div><dt>Personnes</dt><dd>{personnes.join(', ')}</dd></div>{/if}
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
