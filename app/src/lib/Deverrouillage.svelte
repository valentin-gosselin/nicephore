<script lang="ts">
  // Portable mode: the remembered connection is sealed with a passphrase,
  // asked at every launch.
  import { Alert, Button, Field, Heading, PasswordInput, Text } from '@immich/ui';
  import { deverrouiller, type Connexion } from './api';
  import { t } from './i18n.svelte';

  let {
    serveur,
    onconnecte,
    onautrement,
  }: { serveur: string; onconnecte: (c: Connexion) => void; onautrement: () => void } = $props();

  let phrase = $state('');
  let enCours = $state(false);
  let erreur = $state<string | null>(null);

  async function valider(e: SubmitEvent) {
    e.preventDefault();
    enCours = true;
    erreur = null;
    try {
      onconnecte(await deverrouiller(phrase));
    } catch (raison) {
      erreur = String(raison);
      phrase = '';
    } finally {
      enCours = false;
    }
  }
</script>

<div class="flex min-h-0 flex-1 items-center justify-center overflow-y-auto p-6">
  <form class="w-full max-w-md space-y-6" onsubmit={valider}>
    <div class="flex items-center gap-3">
      <img src="/icone.svg" alt="" class="h-12 w-12" />
      <Heading size="large">Nicéphore</Heading>
    </div>
    <div class="space-y-1">
      <Text color="muted">{t('Connexion gardée sur ce support, déverrouillée par sa phrase de passe.')}</Text>
      <Text size="small" class="break-all font-medium">{serveur}</Text>
    </div>
    <Field label={t('Phrase de passe')} required>
      <PasswordInput bind:value={phrase} autocomplete="current-password" />
    </Field>
    {#if erreur}<Alert color="danger" title={t('Impossible de déverrouiller')}>{erreur}</Alert>{/if}
    <Button type="submit" fullWidth loading={enCours} disabled={!phrase}>{t('Déverrouiller')}</Button>
    <button type="button" class="w-full text-center text-sm opacity-70 hover:opacity-100" onclick={onautrement}>
      {t('Se connecter autrement')}
    </button>
  </form>
</div>
