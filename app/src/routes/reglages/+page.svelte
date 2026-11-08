<script lang="ts">
  import { Button, Heading, Text } from '@immich/ui';
  import { getVersion } from '@tauri-apps/api/app';
  import { onMount } from 'svelte';
  import { infosCache, viderCache, type InfosCache } from '../../lib/api';
  import { notifier } from '../../lib/notifications.svelte';
  import { deconnecter, session } from '../../lib/session.svelte';
  import { choisirTheme, theme, type ChoixTheme } from '../../lib/theme.svelte';

  let cache = $state<InfosCache | null>(null);
  let version = $state('');
  let vidage = $state(false);

  function origine(c: { origine: string; session: boolean }) {
    const quoi = c.session ? 'Session ouverte avec e-mail et mot de passe' : 'Clé API';
    if (c.origine === 'environnement') return 'Clé API de développement (variables IMMICH_URL et IMMICH_API_KEY).';
    if (c.origine === 'trousseau') return `${quoi}, gardée dans le trousseau du système.`;
    return `${quoi}, non mémorisée : il faudra se reconnecter au prochain lancement.`;
  }

  const themes: { valeur: ChoixTheme; libelle: string }[] = [
    { valeur: 'systeme', libelle: 'Comme le système' },
    { valeur: 'clair', libelle: 'Clair' },
    { valeur: 'sombre', libelle: 'Sombre' },
  ];

  const mo = (octets: number) => `${(octets / 1_048_576).toLocaleString('fr-FR', { maximumFractionDigits: 0 })} Mo`;

  async function rafraichirCache() {
    cache = await infosCache().catch(() => null);
  }

  async function vider() {
    vidage = true;
    try {
      await viderCache();
      notifier('Cache vidé : les images seront retéléchargées au besoin.', 'succes');
    } catch (e) {
      notifier(`Impossible de vider le cache : ${e}`, 'erreur');
    } finally {
      vidage = false;
      await rafraichirCache();
    }
  }

  onMount(() => {
    void rafraichirCache();
    getVersion().then((v) => (version = v));
  });
</script>

<svelte:head><title>Réglages · Nicéphore</title></svelte:head>

<div class="h-full overflow-y-auto">
  <div class="mx-auto max-w-2xl space-y-10 p-8">
    <Heading size="large">Réglages</Heading>

    {#if session.connexion}
      {@const c = session.connexion}
      <section class="space-y-3">
        <Heading size="small">Compte</Heading>
        <dl class="grid grid-cols-[10rem_1fr] gap-x-4 gap-y-2 text-sm">
          <dt class="opacity-60">Utilisateur</dt>
          <dd>{c.utilisateur.name} ({c.utilisateur.email})</dd>
          <dt class="opacity-60">Serveur</dt>
          <dd class="break-all">{c.urlServeur}</dd>
          <dt class="opacity-60">Version d’Immich</dt>
          <dd>{c.version}</dd>
          <dt class="opacity-60">Connexion</dt>
          <dd>{origine(c)}</dd>
        </dl>
        {#if c.origine !== 'environnement'}
          <Button variant="outline" color="danger" onclick={() => deconnecter()}>Se déconnecter</Button>
        {/if}
      </section>
    {/if}

    <section class="space-y-3">
      <Heading size="small">Apparence</Heading>
      <div class="flex gap-2" role="radiogroup" aria-label="Thème">
        {#each themes as t (t.valeur)}
          <Button
            variant={theme.choix === t.valeur ? 'filled' : 'outline'}
            role="radio"
            aria-checked={theme.choix === t.valeur}
            onclick={() => choisirTheme(t.valeur)}
          >
            {t.libelle}
          </Button>
        {/each}
      </div>
    </section>

    <section class="space-y-3">
      <Heading size="small">Cache des images</Heading>
      {#if cache}
        <Text size="small">
          {mo(cache.octets)} utilisés sur {mo(cache.budget)} ({cache.fichiers.toLocaleString('fr-FR')} images). Les plus
          anciennes sont effacées au-delà.
        </Text>
        <Text size="small" color="muted" class="break-all">{cache.dossier}</Text>
      {/if}
      <Button variant="outline" loading={vidage} onclick={vider}>Vider le cache</Button>
    </section>

    <section class="space-y-2">
      <Heading size="small">À propos</Heading>
      <Text size="small">Nicéphore {version}, client de bureau non officiel pour Immich.</Text>
      <Text size="small" color="muted">
        Logiciel libre sous licence AGPL 3, sans lien avec le projet Immich. Code source :
        github.com/valentin-gosselin/nicephore
      </Text>
    </section>
  </div>
</div>
