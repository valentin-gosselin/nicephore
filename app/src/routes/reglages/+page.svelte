<script lang="ts">
  import { Button, Heading, ProgressBar, Select, Switch, Text } from '@immich/ui';
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
  import { choisirLangue, langue, LANGUES, locale, t } from '../../lib/i18n.svelte';

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
      notifier(t('Réglage impossible : {e}', { e: String(e) }), 'erreur');
    }
  }

  async function changerDemarrage(actif: boolean) {
    try {
      await definirDemarrageAuto(actif);
      reglages = await reglagesApp();
    } catch (e) {
      notifier(t('Démarrage automatique impossible : {e}', { e: String(e) }), 'erreur');
    }
  }

  const options: { cle: keyof ReglagesApp; titre: string; detail: string }[] = $derived([
    {
      cle: 'fermerEnArrierePlan',
      titre: t('Continuer dans la zone de notification'),
      detail: t('Fermer la fenêtre la range dans la zone de notification, où Nicéphore continue ses envois ; on le quitte depuis le menu de son icône.'),
    },
    {
      cle: 'pauseSurBatterie',
      titre: t('Attendre le secteur'),
      detail: t('Sur un portable débranché, l’envoi attend que l’ordinateur soit rebranché.'),
    },
    {
      cle: 'pauseReseauFacture',
      titre: t('Attendre un réseau non facturé'),
      detail: t('Pas d’envoi sur un partage de connexion du téléphone ou un réseau marqué comme facturé.'),
    },
    { cle: 'notifications', titre: t('Notifications'), detail: t('Prévenir à la fin d’un envoi, ou s’il s’arrête sur une erreur.') },
  ]);

  function origine(c: { origine: string; session: boolean }) {
    const quoi = c.session ? t('Session ouverte avec e-mail et mot de passe') : t('Clé API');
    if (c.origine === 'environnement') return t('Clé API de développement (variables IMMICH_URL et IMMICH_API_KEY).');
    if (c.origine === 'trousseau') return t('{quoi}, gardée dans le trousseau du système.', { quoi });
    if (c.origine === 'coffre') return t('{quoi}, gardée à côté de Nicéphore et chiffrée par votre phrase de passe.', { quoi });
    return t('{quoi}, non mémorisée : il faudra se reconnecter au prochain lancement.', { quoi });
  }

  const themes: { valeur: ChoixTheme; libelle: string }[] = $derived([
    { valeur: 'systeme', libelle: t('Comme le système') },
    { valeur: 'clair', libelle: t('Clair') },
    { valeur: 'sombre', libelle: t('Sombre') },
  ]);

  const mo = (octets: number) => t('{n} Mo', { n: (octets / 1_048_576).toLocaleString(locale(), { maximumFractionDigits: 0 }) });

  async function rafraichirCache() {
    cache = await infosCache().catch(() => null);
    originaux = await cacheOriginaux().catch(() => null);
  }

  async function basculerPrechargement(actif: boolean) {
    try {
      await definirPrechargement(actif);
      prechargement = await etatPrechargement();
    } catch (e) {
      notifier(t('Réglage impossible : {e}', { e: String(e) }), 'erreur');
    }
  }

  const nombre = (n: number) => n.toLocaleString(locale());

  async function vider() {
    vidage = true;
    try {
      await viderCache();
      notifier(t('Cache vidé : les images seront retéléchargées au besoin.'), 'succes');
    } catch (e) {
      notifier(t('Impossible de vider le cache : {e}', { e: String(e) }), 'erreur');
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

<svelte:head><title>{t('Réglages')} · Nicéphore</title></svelte:head>

<div class="h-full overflow-y-auto">
  <div class="mx-auto max-w-2xl space-y-10 p-8">
    <Heading size="large">{t('Réglages')}</Heading>

    {#if session.connexion}
      {@const c = session.connexion}
      <section class="space-y-3">
        <Heading size="small">{t('Compte')}</Heading>
        <dl class="grid grid-cols-[10rem_1fr] gap-x-4 gap-y-2 text-sm">
          <dt class="opacity-60">{t('Utilisateur')}</dt>
          <dd>{c.utilisateur.name} ({c.utilisateur.email})</dd>
          <dt class="opacity-60">{t('Serveur')}</dt>
          <dd class="break-all">{c.urlServeur}</dd>
          <dt class="opacity-60">{t('Version d’Immich')}</dt>
          <dd>{c.version}</dd>
          <dt class="opacity-60">{t('Connexion')}</dt>
          <dd>{origine(c)}</dd>
        </dl>
        {#if c.origine !== 'environnement'}
          <Button variant="outline" color="danger" onclick={() => deconnecter()}>{t('Se déconnecter')}</Button>
        {/if}
      </section>
    {/if}

    <section class="space-y-3">
      <Heading size="small">{t('Apparence')}</Heading>
      <div class="flex gap-2" role="radiogroup" aria-label={t('Thème')}>
        {#each themes as th (th.valeur)}
          <Button
            variant={theme.choix === th.valeur ? 'filled' : 'outline'}
            role="radio"
            aria-checked={theme.choix === th.valeur}
            onclick={() => choisirTheme(th.valeur)}
          >
            {th.libelle}
          </Button>
        {/each}
      </div>
    </section>

    <section class="space-y-3">
      <Heading size="small">{t('Langue')}</Heading>
      <Select
        class="w-80"
        title={t('Langue')}
        options={[
          { value: 'systeme', label: t('Comme le système') },
          ...LANGUES.map((l) => ({
            value: l.code,
            label: l.part < 1 ? t('{nom}, traduit à {part} %', { nom: l.nom, part: Math.round(l.part * 100) }) : l.nom,
          })),
        ]}
        value={langue.choix}
        onChange={(valeur) => choisirLangue(valeur)}
      />
      <Text size="small" color="muted">
        {t('Au-delà du français et de l’anglais, les langues viennent des traductions de la communauté Immich, pour les textes que Nicéphore partage avec Immich. Le reste s’affiche en anglais.')}
      </Text>
    </section>

    {#if reglages}
      {@const r = reglages}
      <section class="space-y-4" aria-label={t('Envoi en arrière-plan')}>
        <Heading size="small">{t('Envoi en arrière-plan')}</Heading>
        {#if !r.portable}
          <label class="flex items-center justify-between gap-6">
            <span>
              <Text>{t('Lancer Nicéphore à l’ouverture de session')}</Text>
              <Text size="small" color="muted">{t('Il démarre discrètement dans la zone de notification pour envoyer vos dossiers.')}</Text>
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
                  {t('Le bureau n’offre pas de zone de notification : fermer la fenêtre quitte Nicéphore.')}{/if}
              </Text>
            </span>
            <Switch checked={r[o.cle]} onCheckedChange={(v: boolean) => changer(o.cle, v)} />
          </label>
        {/each}
      </section>
    {/if}

    <section class="space-y-3">
      <Heading size="small">{t('Visionneuse de cet ordinateur')}</Heading>
      <Text size="small" color="muted">
        {t('Ouvrir les photos et vidéos de l’ordinateur avec Nicéphore : on parcourt le dossier, et un nuage indique celles qui ne sont pas encore sur Immich, avec un bouton pour les envoyer.')}
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
        {t('Faire de Nicéphore la visionneuse par défaut')}
      </Button>
    </section>

    <section class="space-y-3">
      <Heading size="small">{t('Vignettes sur cet ordinateur')}</Heading>
      <label class="flex items-center justify-between gap-6">
        <span>
          <Text>{t('Garder toutes les vignettes')}</Text>
          <Text size="small" color="muted">
            {t('Télécharge en tâche de fond la vignette de chaque photo, une seule fois : la chronologie défile alors sans attente et reste consultable sans le serveur. Se met en pause pendant que vous faites défiler.')}
          </Text>
        </span>
        {#if prechargement}
          <Switch checked={prechargement.actif} onCheckedChange={(v: boolean) => basculerPrechargement(v)} />
        {/if}
      </label>
      {#if prechargement?.actif && prechargement.total > 0}
        <ProgressBar progress={Math.min(1, prechargement.presentes / prechargement.total)} />
        <Text size="small" color="muted">
          {t('{presentes} sur {total} vignettes', { presentes: nombre(prechargement.presentes), total: nombre(prechargement.total) })}{#if prechargement.enCours}{t(', téléchargement en cours')}{:else if prechargement.termine}{t(', à jour')}{/if}{#if prechargement.erreurs}
            {t('({n} sans vignette sur le serveur)', { n: nombre(prechargement.erreurs) })}{/if}
        </Text>
      {/if}
    </section>

    <section class="space-y-3">
      <Heading size="small">{t('Cache des images')}</Heading>
      {#if cache}
        <Text size="small">
          {t('Vignettes : {taille} ({n} images, au plus {max}).', { taille: mo(cache.vignettes.octets), n: nombre(cache.vignettes.fichiers), max: mo(cache.vignettes.budget) })}
        </Text>
        <Text size="small">
          {t('Aperçus de la visionneuse : {taille} ({n} images, au plus {max}). Les plus anciens sont effacés au-delà.', { taille: mo(cache.apercus.octets), n: nombre(cache.apercus.fichiers), max: mo(cache.apercus.budget) })}
        </Text>
        <Text size="small" color="muted" class="break-all">{cache.dossier}</Text>
      {/if}
      {#if originaux && originaux.fichiers}
        <Text size="small">
          {t('Originaux ouverts depuis les dossiers virtuels : {taille} ({n} fichiers, au plus {max}).', { taille: mo(originaux.octets), n: nombre(originaux.fichiers), max: mo(originaux.budget) })}
        </Text>
      {/if}
      <div class="flex gap-2">
        <Button variant="outline" loading={vidage} onclick={vider}>{t('Vider le cache')}</Button>
        {#if originaux && originaux.fichiers}
          <Button variant="outline" onclick={async () => { await viderCacheOriginaux(); await rafraichirCache(); }}>
            {t('Vider les originaux')}
          </Button>
        {/if}
      </div>
    </section>

    {#if maj}
      <section class="space-y-3" aria-label={t('Mises à jour')}>
        <Heading size="small">{t('Mises à jour')}</Heading>
        {#if !maj.possibles}
          <Text size="small" color="muted">
            {t('Cette copie de Nicéphore {version} a été compilée localement : pas de mises à jour automatiques.', { version: maj.versionActuelle })}
          </Text>
        {:else}
          <div class="flex items-center gap-3">
            <Text size="small" class="flex-1">
              {#if disponible === undefined}{t('Version {version}.', { version: maj.versionActuelle })}{:else if disponible === null}{t('Nicéphore {version} est à jour.', { version: maj.versionActuelle })}{:else}{t('Nicéphore {version} est disponible (vous avez {actuelle}).', { version: disponible.version, actuelle: maj.versionActuelle })}{/if}
            </Text>
            <Button variant="outline" size="small" loading={recherche} onclick={chercher}>{t('Rechercher')}</Button>
          </div>
          {#if disponible}
            {#if disponible.notes}<p class="notes whitespace-pre-line rounded-xl p-3 text-sm">{disponible.notes}</p>{/if}
            {#if maj.installable}
              {#if installation}
                <ProgressBar progress={installation.total ? installation.recus / installation.total : 0} />
                <Text size="small" color="muted">{t('Téléchargement et vérification de la signature, puis redémarrage…')}</Text>
              {:else}
                <Button onclick={installer}>{t('Installer et redémarrer')}</Button>
              {/if}
            {:else}
              <Text size="small" color="muted">
                {portable ? t('Remplacez le programme sur votre support') : t('Mettez à jour par votre gestionnaire de paquets (Discover, dnf)')}
                {t('avec la nouvelle version :')} <span class="break-all">{maj.pageDesVersions}</span>
              </Text>
            {/if}
          {/if}
        {/if}
        {#if erreurMaj}<Text size="small" color="danger">{erreurMaj}</Text>{/if}
      </section>
    {/if}

    {#if portable}
      <section class="space-y-2" aria-label={t('Mode portable')}>
        <Heading size="small">{t('Mode portable')}</Heading>
        <Text size="small">
          {t('Tout ce que Nicéphore garde (réglages, connexion chiffrée, dossiers envoyés, cache des images) est rangé à côté de lui, rien sur cet ordinateur. Pour revenir au mode normal, retirez le fichier « portable » posé à côté du programme.')}
        </Text>
        <Text size="small" color="muted" class="break-all">{portable}</Text>
      </section>
    {/if}

    <section class="space-y-2">
      <Heading size="small">{t('À propos')}</Heading>
      <Text size="small">{t('Nicéphore {version}, client de bureau non officiel pour Immich.', { version })}</Text>
      <Text size="small" color="muted">
        {t('Logiciel libre sous licence AGPL 3, sans lien avec le projet Immich. Code source :')}
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
