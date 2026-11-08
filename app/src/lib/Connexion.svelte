<script lang="ts">
  import { Alert, Button, Field, Heading, Input, PasswordInput, Text } from '@immich/ui';
  import { seConnecter, type Connexion } from './api';

  let { onconnecte }: { onconnecte: (c: Connexion) => void } = $props();

  let url = $state('');
  let cle = $state('');
  let enCours = $state(false);
  let erreur = $state<string | null>(null);

  async function valider(e: SubmitEvent) {
    e.preventDefault();
    enCours = true;
    erreur = null;
    try {
      onconnecte(await seConnecter(url, cle));
    } catch (raison) {
      erreur = String(raison);
    } finally {
      enCours = false;
    }
  }
</script>

<div class="flex h-full items-center justify-center p-6">
  <form class="w-full max-w-md space-y-5" onsubmit={valider}>
    <Heading size="large">Connexion au serveur</Heading>
    <Text color="muted">
      Créez une clé API dans Immich (Paramètres du compte, Clés API) puis collez-la ici. Elle reste en mémoire
      pendant ce prototype.
    </Text>
    <Field label="Adresse du serveur" required>
      <Input bind:value={url} placeholder="http://192.168.1.10:2283" autocomplete="url" />
    </Field>
    <Field label="Clé API" required>
      <PasswordInput bind:value={cle} autocomplete="off" />
    </Field>
    {#if erreur}<Alert color="danger" title="Connexion impossible">{erreur}</Alert>{/if}
    <Button type="submit" fullWidth loading={enCours} disabled={!url || !cle}>Se connecter</Button>
  </form>
</div>
