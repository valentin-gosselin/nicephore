<script lang="ts">
  import { Button, Heading, ProgressBar, Switch, Text } from '@immich/ui';
  import { getVersion } from '@tauri-apps/api/app';
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import {
    definirDemarrageAuto,
    definirPrechargement,
    definirReglagesApp,
    etatPrechargement,
    cacheOriginaux,
    definirVisionneuseParDefaut,
    viderCacheOriginaux,
    infosCache,
    infosPortable,
    chercherMiseAJour,
    infosMiseAJour,
    installerMiseAJour,
    type InfosMiseAJour,
    type VersionDisponible,
    reglagesApp,
    viderCache,
    type EtatPrechargement,
    type InfosCache,
    type ReglagesApp,
    type ReglagesAppVue,
  } from '../../lib/api';
  import { notifier } from '../../lib/notifications.svelte';
  import { deconnecter, session } from '../../lib/session.svelte';
  import { choisirTheme, theme, type ChoixTheme } from '../../lib/theme.svelte';

  let cache = $state<InfosCache | null>(null);
  let originaux = $state<{ octets: number; fichiers: number; budget: number } | null>(null);
  let prechargement = $state<EtatPrechargement | null>(null);
  let version = $state('');
  let vidage = $state(false);
  let reglages = $state<ReglagesAppVue | null>(null);
  let portable = $state<string | null>(null);
  let maj = $state<InfosMiseAJour | null>(null);
  let disponible = $state<VersionDisponible | null | undefined>(undefined);
  let recherche = $state(false);
  let installation = $state<{ recus: number; total: number | null } | null>(null);
  let erreurMaj = $state<string | null>(null);

  async function chercher() {
    recherche = true;
    erreurMaj = null;
    try {
      disponible = await chercherMiseAJour();
    } catch (e) {
      erreurMaj = String(e);
    } finally {
      recherche = false;
    }
  }

  async function installer() {
    installation = { recus: 0, total: null };
    erreurMaj = null;
    try {
      // Nicéphore restarts on success.
      await installerMiseAJour();
    } catch (e) {
      erreurMaj = String(e);
      installation = null;
    }
  }

  async function changer(cle: keyof ReglagesApp, valeur: boolean) {
    if (!reglages) return;
    const { demarrageAuto: _d, zoneDisponible: _z, ...actuels } = reglages;
    try {
      await definirReglagesApp({ ...actuels, [cle]: valeur });
      reglages = await reglagesApp();
    } catch (e) {
      notifier(`Réglage impossible : ${e}`, 'erreur');
    }
  }

  async function changerDemarrage(actif: boolean) {
    try {
      await definirDemarrageAuto(actif);
      reglages = await reglagesApp();
    } catch (e) {
      notifier(`Démarrage automatique impossible : ${e}`, 'erreur');
    }
  }

  const options: { cle: keyof ReglagesApp; titre: string; detail: string }[] = [
    {
      cle: 'fermerEnArrierePlan',
      titre: 'Continuer dans la zone de notification',
      detail: 'Fermer la fenêtre la range dans la zone de notification, où Nicéphore continue ses envois ; on le quitte depuis le menu de son icône.',
    },
    {
      cle: 'pauseSurBatterie',
      titre: 'Attendre le secteur',
      detail: 'Sur un portable débranché, l’envoi attend que l’ordinateur soit rebranché.',
    },
    {
      cle: 'pauseReseauFacture',
      titre: 'Attendre un réseau non facturé',
      detail: 'Pas d’envoi sur un partage de connexion du téléphone ou un réseau marqué comme facturé.',
    },
    { cle: 'notifications', titre: 'Notifications', detail: 'Prévenir à la fin d’un envoi, ou s’il s’arrête sur une erreur.' },
  ];

  function origine(c: { origine: string; session: boolean }) {
    const quoi = c.session ? 'Session ouverte avec e-mail et mot de passe' : 'Clé API';
    if (c.origine === 'environnement') return 'Clé API de développement (variables IMMICH_URL et IMMICH_API_KEY).';
    if (c.origine === 'trousseau') return `${quoi}, gardée dans le trousseau du système.`;
    if (c.origine === 'coffre') return `${quoi}, gardée à côté de Nicéphore et chiffrée par votre phrase de passe.`;
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
    originaux = await cacheOriginaux().catch(() => null);
  }

  async function basculerPrechargement(actif: boolean) {
    try {
      await definirPrechargement(actif);
      prechargement = await etatPrechargement();
    } catch (e) {
      notifier(`Réglage impossible : ${e}`, 'erreur');
    }
  }

  const nombre = (n: number) => n.toLocaleString('fr-FR');

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
    etatPrechargement().then((e) => (prechargement = e));
    reglagesApp().then((r) => (reglages = r));
    infosPortable().then((d) => (portable = d));
    infosMiseAJour().then((m) => (maj = m));
    const arretMaj = listen<{ recus: number; total: number | null }>('mise-a-jour', ({ payload }) => (installation = payload));
    let dernierRafraichissement = 0;
    const arret = listen<EtatPrechargement>('prechargement', ({ payload }) => {
      prechargement = payload;
      // Refresh the disk usage from time to time while it grows.
      if (performance.now() - dernierRafraichissement > 3000) {
        dernierRafraichissement = performance.now();
        void rafraichirCache();
      }
    });
    return () => {
      void arret.then((f) => f());
      void arretMaj.then((f) => f());
    };
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

    {#if reglages}
      {@const r = reglages}
      <section class="space-y-4" aria-label="Envoi en arrière-plan">
        <Heading size="small">Envoi en arrière-plan</Heading>
        {#if !r.portable}
          <label class="flex items-center justify-between gap-6">
            <span>
              <Text>Lancer Nicéphore à l’ouverture de session</Text>
              <Text size="small" color="muted">Il démarre discrètement dans la zone de notification pour envoyer vos dossiers.</Text>
            </span>
            <Switch checked={r.demarrageAuto} onCheckedChange={(v: boolean) => changerDemarrage(v)} />
          </label>
        {/if}
        {#each options as o (o.cle)}
          <label class="flex items-center justify-between gap-6">
            <span>
              <Text>{o.titre}</Text>
              <Text size="small" color="muted">
                {o.detail}{#if o.cle === 'fermerEnArrierePlan' && !r.zoneDisponible}
                  Le bureau n’offre pas de zone de notification : fermer la fenêtre quitte Nicéphore.{/if}
              </Text>
            </span>
            <Switch checked={r[o.cle]} onCheckedChange={(v: boolean) => changer(o.cle, v)} />
          </label>
        {/each}
      </section>
    {/if}

    <section class="space-y-3">
      <Heading size="small">Visionneuse de cet ordinateur</Heading>
      <Text size="small" color="muted">
        Ouvrir les photos et vidéos de l’ordinateur avec Nicéphore : on parcourt le dossier, et un nuage indique celles
        qui ne sont pas encore sur Immich, avec un bouton pour les envoyer.
      </Text>
      <Button
        variant="outline"
        onclick={async () => {
          try {
            notifier(await definirVisionneuseParDefaut(), 'succes');
          } catch (e) {
            notifier(String(e), 'erreur');
          }
        }}
      >
        Faire de Nicéphore la visionneuse par défaut
      </Button>
    </section>

    <section class="space-y-3">
      <Heading size="small">Vignettes sur cet ordinateur</Heading>
      <label class="flex items-center justify-between gap-6">
        <span>
          <Text>Garder toutes les vignettes</Text>
          <Text size="small" color="muted">
            Télécharge en tâche de fond la vignette de chaque photo, une seule fois : la chronologie défile alors sans
            attente et reste consultable sans le serveur. Se met en pause pendant que vous faites défiler.
          </Text>
        </span>
        {#if prechargement}
          <Switch checked={prechargement.actif} onCheckedChange={(v: boolean) => basculerPrechargement(v)} />
        {/if}
      </label>
      {#if prechargement?.actif && prechargement.total > 0}
        <ProgressBar progress={Math.min(1, prechargement.presentes / prechargement.total)} />
        <Text size="small" color="muted">
          {nombre(prechargement.presentes)} sur {nombre(prechargement.total)} vignettes{#if prechargement.enCours}, téléchargement en cours{:else if prechargement.termine}, à jour{/if}{#if prechargement.erreurs}
            ({nombre(prechargement.erreurs)} sans vignette sur le serveur){/if}
        </Text>
      {/if}
    </section>

    <section class="space-y-3">
      <Heading size="small">Cache des images</Heading>
      {#if cache}
        <Text size="small">
          Vignettes : {mo(cache.vignettes.octets)} ({nombre(cache.vignettes.fichiers)} images, au plus
          {mo(cache.vignettes.budget)}).
        </Text>
        <Text size="small">
          Aperçus de la visionneuse : {mo(cache.apercus.octets)} ({nombre(cache.apercus.fichiers)} images, au plus
          {mo(cache.apercus.budget)}). Les plus anciens sont effacés au-delà.
        </Text>
        <Text size="small" color="muted" class="break-all">{cache.dossier}</Text>
      {/if}
      {#if originaux && originaux.fichiers}
        <Text size="small">
          Originaux ouverts depuis les dossiers virtuels : {mo(originaux.octets)} ({nombre(originaux.fichiers)} fichiers, au
          plus {mo(originaux.budget)}).
        </Text>
      {/if}
      <div class="flex gap-2">
        <Button variant="outline" loading={vidage} onclick={vider}>Vider le cache</Button>
        {#if originaux && originaux.fichiers}
          <Button variant="outline" onclick={async () => { await viderCacheOriginaux(); await rafraichirCache(); }}>
            Vider les originaux
          </Button>
        {/if}
      </div>
    </section>

    {#if maj}
      <section class="space-y-3" aria-label="Mises à jour">
        <Heading size="small">Mises à jour</Heading>
        {#if !maj.possibles}
          <Text size="small" color="muted">
            Cette copie de Nicéphore {maj.versionActuelle} a été compilée localement : pas de mises à jour automatiques.
          </Text>
        {:else}
          <div class="flex items-center gap-3">
            <Text size="small" class="flex-1">
              {#if disponible === undefined}Version {maj.versionActuelle}.{:else if disponible === null}Nicéphore {maj.versionActuelle}
                est à jour.{:else}Nicéphore {disponible.version} est disponible (vous avez {maj.versionActuelle}).{/if}
            </Text>
            <Button variant="outline" size="small" loading={recherche} onclick={chercher}>Rechercher</Button>
          </div>
          {#if disponible}
            {#if disponible.notes}<p class="notes whitespace-pre-line rounded-xl p-3 text-sm">{disponible.notes}</p>{/if}
            {#if maj.installable}
              {#if installation}
                <ProgressBar progress={installation.total ? installation.recus / installation.total : 0} />
                <Text size="small" color="muted">Téléchargement et vérification de la signature, puis redémarrage…</Text>
              {:else}
                <Button onclick={installer}>Installer et redémarrer</Button>
              {/if}
            {:else}
              <Text size="small" color="muted">
                {portable ? 'Remplacez le programme sur votre support' : 'Mettez à jour par votre gestionnaire de paquets (Discover, dnf)'}
                avec la nouvelle version : <span class="break-all">{maj.pageDesVersions}</span>
              </Text>
            {/if}
          {/if}
        {/if}
        {#if erreurMaj}<Text size="small" color="danger">{erreurMaj}</Text>{/if}
      </section>
    {/if}

    {#if portable}
      <section class="space-y-2" aria-label="Mode portable">
        <Heading size="small">Mode portable</Heading>
        <Text size="small">
          Tout ce que Nicéphore garde (réglages, connexion chiffrée, dossiers envoyés, cache des images) est rangé à côté
          de lui, rien sur cet ordinateur. Pour revenir au mode normal, retirez le fichier « portable » posé à côté du
          programme.
        </Text>
        <Text size="small" color="muted" class="break-all">{portable}</Text>
      </section>
    {/if}

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

<style>
  .notes {
    background: color-mix(in oklab, currentColor 5%, transparent);
  }
</style>
