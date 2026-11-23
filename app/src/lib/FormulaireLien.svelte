<script lang="ts">
  // Options of a public link, like Immich web: description, custom address,
  // password, expiry, and what visitors may do. New link or changes to one.
  import { Button, Field, Input, PasswordInput, Select, Switch, Text } from '@immich/ui';
  import { creerLien, modifierLien, type ChampsLien, type LienPartage } from './api';
  import { locale, t } from './i18n.svelte';

  let {
    lien = null,
    album = null,
    assets = [],
    onfini,
    onannuler,
  }: {
    /** The link changed; none for a new one. */
    lien?: LienPartage | null;
    /** New link to an album… */
    album?: string | null;
    /** …or to these photos. */
    assets?: string[];
    onfini: (lien: LienPartage, nouveau: boolean) => void;
    onannuler: () => void;
  } = $props();

  // svelte-ignore state_referenced_locally
  const depart = lien;
  const versAlbum = $derived(depart ? depart.type === 'ALBUM' : !!album);
  let description = $state(depart?.description ?? '');
  let slug = $state(depart?.slug ?? '');
  let motDePasse = $state(depart?.password ?? '');
  let telechargement = $state(depart?.allowDownload ?? true);
  // Immich lets visitors upload by default: here it is asked for.
  let envoi = $state(depart?.allowUpload ?? false);
  let metadonnees = $state(depart?.showMetadata ?? true);
  /** Minutes until expiry, `0` never, `garder` the current one. */
  let expiration = $state<string>(depart?.expiresAt ? 'garder' : '0');
  let enCours = $state(false);
  let erreur = $state<string | null>(null);

  const DUREES: [string, string][] = [
    ['0', 'Jamais'],
    ['30', '30 minutes'],
    ['60', '1 heure'],
    ['360', '6 heures'],
    ['1440', '1 jour'],
    ['10080', '7 jours'],
    ['43200', '30 jours'],
    ['129600', '3 mois'],
    ['525600', '1 an'],
  ];
  const options = $derived([
    ...(depart?.expiresAt
      ? [{ value: 'garder', label: t('Garder : expire le {date}', { date: new Date(depart.expiresAt).toLocaleString(locale(), { dateStyle: 'medium', timeStyle: 'short' }) }) }]
      : []),
    ...DUREES.map(([value, label]) => ({ value, label: t(label) })),
  ]);

  async function valider(e: SubmitEvent) {
    e.preventDefault();
    enCours = true;
    erreur = null;
    const champs: ChampsLien = {
      description: description.trim() || null,
      slug: slug.trim() || null,
      password: motDePasse || null,
      allowDownload: telechargement,
      allowUpload: versAlbum && envoi,
      showMetadata: metadonnees,
    };
    if (expiration !== 'garder') {
      const minutes = Number(expiration);
      champs.expiresAt = minutes ? new Date(Date.now() + minutes * 60_000).toISOString() : null;
    }
    try {
      if (depart) onfini(await modifierLien(depart.id, champs), false);
      else
        onfini(
          await creerLien(album ? { ...champs, type: 'ALBUM', albumId: album } : { ...champs, type: 'INDIVIDUAL', assetIds: assets }),
          true,
        );
    } catch (raison) {
      erreur = String(raison);
    } finally {
      enCours = false;
    }
  }
</script>

<form class="formulaire-lien space-y-4" onsubmit={valider}>
  <Field label={t('Description')}>
    <Input bind:value={description} placeholder={t('Pour la famille, les amis…')} />
  </Field>
  <Field label={t('Adresse personnalisée')}>
    <Input bind:value={slug} placeholder={t('vacances-2026 (facultatif)')} />
  </Field>
  <Field label={t('Mot de passe')}>
    <PasswordInput bind:value={motDePasse} autocomplete="new-password" placeholder={t('Aucun')} />
  </Field>
  <Field label={t('Expire')}>
    <Select options={options} value={expiration} onChange={(v) => (expiration = String(v))} />
  </Field>
  <div class="space-y-3 pt-1">
    <label class="flex items-center justify-between gap-4">
      <span>{t('Permettre le téléchargement par des utilisateurs non connectés')}</span>
      <Switch checked={telechargement} onCheckedChange={(v: boolean) => (telechargement = v)} />
    </label>
    {#if versAlbum}
      <label class="flex items-center justify-between gap-4">
        <span>{t('Permettre l’envoi par des utilisateurs non connectés')}</span>
        <Switch checked={envoi} onCheckedChange={(v: boolean) => (envoi = v)} />
      </label>
    {/if}
    <label class="flex items-center justify-between gap-4">
      <span>{t('Afficher les métadonnées')}</span>
      <Switch checked={metadonnees} onCheckedChange={(v: boolean) => (metadonnees = v)} />
    </label>
  </div>
  {#if erreur}<Text color="danger">{erreur}</Text>{/if}
  <div class="flex justify-end gap-2 pt-2">
    <Button variant="ghost" color="secondary" onclick={onannuler}>{t('Annuler')}</Button>
    <Button type="submit" loading={enCours}>{depart ? t('Enregistrer') : t('Créer le lien')}</Button>
  </div>
</form>
