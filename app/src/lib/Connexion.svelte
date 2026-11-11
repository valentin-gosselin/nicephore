<script lang="ts">
  // Login in two steps, like the Immich apps: the server address first (the
  // server says which login methods it offers), then email and password.
  // An API key remains available as an advanced option.
  import { Alert, Button, Field, Heading, Input, PasswordInput, Switch, Text } from '@immich/ui';
  import { onMount } from 'svelte';
  import { t } from './i18n.svelte';
  import {
    infosPortable,
    modesConnexion,
    seConnecter,
    seConnecterMotDePasse,
    type Connexion,
    type ModesConnexion,
  } from './api';

  let { onconnecte, onannuler }: { onconnecte: (c: Connexion) => void; onannuler?: () => void } = $props();

  const CLE_SERVEUR = 'nicephore-dernier-serveur';

  let url = $state('');
  let modes = $state<ModesConnexion | null>(null);
  let email = $state('');
  let motDePasse = $state('');
  let cle = $state('');
  let avecCle = $state(false);
  let memoriser = $state(true);
  /** Portable mode: the remembered connection is sealed with a passphrase. */
  let portable = $state(false);
  let phrase = $state('');
  let phraseBis = $state('');
  const phraseValide = $derived(!portable || !memoriser || ([...phrase].length >= 8 && phrase === phraseBis));
  let enCours = $state(false);
  let erreur = $state<string | null>(null);

  /** Adds a scheme when missing: http for local addresses, https otherwise. */
  function completer(adresse: string) {
    const brute = adresse.trim();
    if (/^https?:\/\//i.test(brute)) return brute;
    const local = /^(localhost|\d{1,3}(\.\d{1,3}){3}|[^./]+\.(lan|local|home))(:\d+)?(\/|$)/i.test(brute);
    return `${local ? 'http' : 'https'}://${brute}`;
  }

  async function chercherServeur(e: SubmitEvent) {
    e.preventDefault();
    enCours = true;
    erreur = null;
    try {
      modes = await modesConnexion(completer(url));
      url = modes.urlServeur;
      avecCle = !modes.passwordLogin;
      try {
        localStorage.setItem(CLE_SERVEUR, url);
      } catch {
        // Only a convenience.
      }
    } catch (raison) {
      erreur = t('Serveur Immich introuvable à cette adresse : {raison}', { raison: String(raison) });
    } finally {
      enCours = false;
    }
  }

  async function seConnecterAuServeur(e: SubmitEvent) {
    e.preventDefault();
    enCours = true;
    erreur = null;
    try {
      const scellement = portable && memoriser ? phrase : undefined;
      onconnecte(
        avecCle
          ? await seConnecter(url, cle, memoriser, scellement)
          : await seConnecterMotDePasse(url, email, motDePasse, memoriser, scellement),
      );
    } catch (raison) {
      erreur = String(raison);
      motDePasse = '';
    } finally {
      enCours = false;
    }
  }

  onMount(() => {
    infosPortable().then((d) => (portable = !!d)).catch(() => {});
    try {
      url = localStorage.getItem(CLE_SERVEUR) ?? '';
    } catch {
      url = '';
    }
  });
</script>

<div class="flex min-h-0 flex-1 items-center justify-center overflow-y-auto p-6">
  <div class="w-full max-w-md space-y-6">
    <div class="flex items-center gap-3">
      <img src="/icone.svg" alt="" class="h-12 w-12" />
      <Heading size="large">{onannuler ? t('Ajouter un compte') : 'Nicéphore'}</Heading>
    </div>

    {#if !modes}
      <form class="space-y-5" onsubmit={chercherServeur}>
        <Text color="muted">{t('Adresse de votre serveur Immich, la même que dans le navigateur.')}</Text>
        <Field label={t('Adresse du serveur')} required>
          <Input bind:value={url} placeholder={t('photos.exemple.fr ou 192.168.1.10:2283')} autocomplete="url" />
        </Field>
        {#if erreur}<Alert color="danger" title={t('Connexion impossible')}>{erreur}</Alert>{/if}
        <Button type="submit" fullWidth loading={enCours} disabled={!url.trim()}>{t('Continuer')}</Button>
      </form>
    {:else}
      <form class="space-y-5" onsubmit={seConnecterAuServeur}>
        <div class="flex items-center justify-between gap-3">
          <Text size="small" color="muted" class="break-all">{modes.urlServeur}</Text>
          <button type="button" class="shrink-0 text-sm text-primary hover:underline" onclick={() => (modes = null)}>
            {t('Changer')}
          </button>
        </div>

        {#if avecCle}
          <Field label={t('Clé API')} required>
            <PasswordInput bind:value={cle} autocomplete="off" />
          </Field>
          <Text size="small" color="muted">{t('À créer dans Immich : Paramètres du compte, Clés API.')}</Text>
        {:else}
          <Field label={t('Adresse e-mail')} required>
            <Input bind:value={email} type="email" autocomplete="username" />
          </Field>
          <Field label={t('Mot de passe')} required>
            <PasswordInput bind:value={motDePasse} autocomplete="current-password" />
          </Field>
        {/if}
        {#if modes.oauth}
          <Text size="small" color="muted">{t('La connexion OAuth de ce serveur n’est pas encore prise en charge.')}</Text>
        {/if}

        <label class="flex items-center justify-between gap-4">
          <span>
            <Text>{t('Rester connecté')}</Text>
            <Text size="small" color="muted">
              {#if portable}
                {avecCle
                  ? t('La clé est gardée à côté de Nicéphore, chiffrée par une phrase de passe demandée à chaque lancement. Jamais votre mot de passe.')
                  : t('Seul le jeton de session est gardé à côté de Nicéphore, chiffré par une phrase de passe demandée à chaque lancement. Jamais votre mot de passe.')}
              {:else}
                {avecCle
                  ? t('La clé est gardée dans le trousseau du système, jamais votre mot de passe.')
                  : t('Seul le jeton de session est gardé dans le trousseau du système, jamais votre mot de passe.')}
              {/if}
            </Text>
          </span>
          <Switch bind:checked={memoriser} />
        </label>
        {#if portable && memoriser}
          <Field label={t('Phrase de passe (8 caractères au moins)')} required>
            <PasswordInput bind:value={phrase} autocomplete="new-password" />
          </Field>
          <Field label={t('La même, pour vérifier')} required>
            <PasswordInput bind:value={phraseBis} autocomplete="new-password" />
          </Field>
          {#if phraseBis && phrase !== phraseBis}<Text size="small" color="danger">{t('Les deux phrases diffèrent.')}</Text>{/if}
        {/if}
        {#if erreur}<Alert color="danger" title={t('Connexion impossible')}>{erreur}</Alert>{/if}
        <Button
          type="submit"
          fullWidth
          loading={enCours}
          disabled={(avecCle ? !cle : !email || !motDePasse) || !phraseValide}
        >
          {t('Se connecter')}
        </Button>
        {#if modes.passwordLogin}
          <button type="button" class="w-full text-center text-sm opacity-70 hover:opacity-100" onclick={() => (avecCle = !avecCle)}>
            {avecCle ? t('Utiliser mon e-mail et mon mot de passe') : t('Utiliser une clé API à la place')}
          </button>
        {/if}
      </form>
    {/if}
    {#if onannuler}
      <button type="button" class="w-full text-center text-sm opacity-70 hover:opacity-100" onclick={onannuler}>
        {t('Annuler, revenir au compte actuel')}
      </button>
    {/if}
  </div>
</div>
