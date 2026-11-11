<script lang="ts">
  // "Envoi" page: folders of this computer sent to Immich, engine progress
  // and the log of what was done.
  import { Button, Heading, IconButton, ProgressBar, Switch, Text } from '@immich/ui';
  import { mdiDeleteOutline, mdiFolderEyeOutline, mdiFolderArrowUpOutline, mdiFolderDownloadOutline, mdiFolderSyncOutline, mdiFolderPlusOutline, mdiPause, mdiPlay, mdiRefresh } from '@mdi/js';
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import {
    activerRegleEnvoi,
    ajouterDouble,
    ajouterDoubleBibliotheque,
    ajouterVirtuel,
    dossiersVirtuels,
    retirerVirtuel,
    type DossierVirtuel,
    ajouterReception,
    confirmerSuppressions,
    definirMiroir,
    definirSansRaw,
    refuserSuppressions,
    suppressionsEnAttente,
    type SuppressionEnAttente,
    ajouterRegleEnvoi,
    recevoirDeNouveau,
    echecsEnvoi,
    etatEnvoi,
    journalEnvoi,
    lancerEnvoi,
    pauseEnvoi,
    reessayerEnvoi,
    reglesEnvoi,
    supprimerRegleEnvoi,
    type Cible,
    type Selection,
    type EntreeJournal,
    type EtatEnvoi,
    type FichierEnEchec,
    type RegleEnvoi,
  } from '../../lib/api';
  import Dialogue from '../../lib/Dialogue.svelte';
  import DialogueRegle, { type Demande } from '../../lib/DialogueRegle.svelte';
  import { locale, t, tp } from '../../lib/i18n.svelte';
  import { notifier } from '../../lib/notifications.svelte';

  let regles = $state<RegleEnvoi[] | null>(null);
  let erreurBase = $state<string | null>(null);
  let etat = $state<EtatEnvoi | null>(null);
  let journal = $state<EntreeJournal[]>([]);
  let ajout = $state(false);
  let aRetirer = $state<RegleEnvoi | null>(null);
  let echecs = $state<{ regle: number; fichiers: FichierEnEchec[] } | null>(null);

  const nombre = (n: number) => n.toLocaleString(locale());
  const mo = (octets: number) =>
    octets >= 1_073_741_824
      ? t('{n} Go', { n: (octets / 1_073_741_824).toLocaleString(locale(), { maximumFractionDigits: 1 }) })
      : t('{n} Mo', { n: (octets / 1_048_576).toLocaleString(locale(), { maximumFractionDigits: 0 }) });
  const nomDossier = (d: string) => d.split(/[\\/]/).filter(Boolean).pop() ?? d;

  function libelleSelection(s: Selection | null) {
    if (!s) return '';
    switch (s.genre) {
      case 'album':
        return t('l’album « {nom} »', { nom: s.nom });
      case 'personne':
        return t('les photos de {nom}', { nom: s.nom });
      case 'favoris':
        return t('les favoris');
      case 'periode':
        return s.du && s.au
          ? t('du {du} au {au}', { du: dateCourte(s.du), au: dateCourte(s.au) })
          : s.du
            ? t('depuis le {du}', { du: dateCourte(s.du) })
            : t('jusqu’au {au}', { au: dateCourte(s.au ?? '') });
      default:
        return t('toute la photothèque');
    }
  }
  const dateCourte = (iso: string) => new Date(`${iso}T12:00:00`).toLocaleDateString(locale(), { day: 'numeric', month: 'short', year: 'numeric' });

  /** "Synchronisé avec ..." line of a two-way rule. */
  function resumeDouble(regle: RegleEnvoi) {
    const sel = libelleSelection(regle.selection);
    let texte = t('Synchronisé avec {selection}', { selection: regle.selection?.genre === 'periode' ? t('les photos {periode}', { periode: sel }) : sel });
    if (regle.cible.genre === 'albumParDossier') texte += t(', un album par dossier');
    if (regle.dossierRecus) texte += t(', reçues dans « {dossier} »', { dossier: regle.dossierRecus });
    return texte;
  }

  let virtuels = $state<DossierVirtuel[]>([]);

  async function retirerDossierVirtuel(d: DossierVirtuel) {
    try {
      await retirerVirtuel(d.id);
      notifier(t('« {dossier} » n’affiche plus la sélection.', { dossier: nomDossier(d.point) }), 'succes');
      await rafraichir();
    } catch (e) {
      notifier(String(e), 'erreur');
    }
  }

  let enAttente = $state<{ regle: RegleEnvoi; liste: SuppressionEnAttente[] } | null>(null);

  async function voirSuppressions(regle: RegleEnvoi) {
    enAttente = { regle, liste: await suppressionsEnAttente(regle.id).catch(() => []) };
  }

  async function deciderSuppressions(appliquer: boolean) {
    if (!enAttente) return;
    const { regle } = enAttente;
    enAttente = null;
    try {
      const n = appliquer ? await confirmerSuppressions(regle.id) : await refuserSuppressions(regle.id);
      notifier(
        appliquer
          ? tp(n, '{n} suppression confirmée.', '{n} suppressions confirmées.')
          : tp(n, '{n} suppression annulée : les fichiers reviennent.', '{n} suppressions annulées : les fichiers reviennent.'),
        'succes',
      );
      await rafraichir();
    } catch (e) {
      notifier(String(e), 'erreur');
    }
  }

  async function basculerRaw(regle: RegleEnvoi) {
    try {
      await definirSansRaw(regle.id, !regle.sansRaw);
      await rafraichir();
    } catch (e) {
      notifier(String(e), 'erreur');
    }
  }

  async function basculerMiroir(regle: RegleEnvoi) {
    try {
      await definirMiroir(regle.id, !regle.miroir);
      await rafraichir();
    } catch (e) {
      notifier(String(e), 'erreur');
    }
  }

  async function recevoirEncore(regle: RegleEnvoi) {
    const n = await recevoirDeNouveau(regle.id).catch(() => 0);
    notifier(tp(n, '{n} fichier à recevoir de nouveau.', '{n} fichiers à recevoir de nouveau.'), 'succes');
    await rafraichir();
  }

  function libelleCible(c: Cible) {
    if (c.genre === 'album') return t('Album « {nom} »', { nom: c.nom });
    if (c.genre === 'albumParDossier') return t('Un album par dossier');
    return t('Photothèque, sans album');
  }

  // French texts, translated where they are displayed.
  const libellesJournal: Record<string, string> = {
    envoye: 'Envoyé',
    album_cree: 'Album créé',
    album_disparu: 'Album supprimé sur le serveur, pas recréé',
    dans_la_corbeille: 'Déjà sur le serveur, dans sa corbeille',
    ignore: 'Ignoré',
    abandon: 'Abandonné après plusieurs essais',
    recu: 'Reçu',
    retire_de_l_album: 'Effacé ici, retiré de l’album',
    mis_a_la_corbeille: 'Retiré de l’album, mis à la corbeille ici',
    corbeille_impossible: 'Corbeille impossible, fichier gardé',
    retire_ici: 'Effacé ici, pas retéléchargé',
    retire_du_serveur: 'Retiré sur le serveur, gardé ici',
    regle_ajoutee: 'Dossier ajouté',
    regle_supprimee: 'Dossier retiré',
  };

  function detailJournal(e: EntreeJournal) {
    switch (e.action) {
      case 'album_cree':
        return `« ${e.detail ?? ''} »`;
      case 'regle_ajoutee':
      case 'regle_supprimee':
        return e.detail ?? '';
      default:
        return [e.chemin, e.detail && `(${e.detail})`].filter(Boolean).join(' ');
    }
  }

  const heure = (ms: number) =>
    new Date(ms).toLocaleString(locale(), { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' });

  const description = $derived.by(() => {
    if (!etat) return '';
    if (!etat.connecte) return t('Non connecté au serveur.');
    if (etat.enPause) return t('En pause.');
    if (etat.attente) return t('En attente : {attente}. L’envoi reprendra tout seul.', { attente: etat.attente });
    const sur = etat.total ? t(' ({faits} sur {total})', { faits: nombre(etat.faits), total: nombre(etat.total) }) : '';
    switch (etat.phase) {
      case 'analyse':
        return t('Lecture des dossiers…');
      case 'empreintes':
        return t('Calcul des empreintes des nouveaux fichiers') + sur;
      case 'verification':
        return t('Comparaison avec le serveur') + sur;
      case 'envoi':
        return t('Envoi{sur}, {faits} sur {total}', { sur, faits: mo(etat.octetsFaits), total: mo(etat.octetsTotal) });
      case 'albums':
        return t('Rangement dans les albums…');
      case 'reception':
        return t('Réception{sur}, {faits} sur {total}', { sur, faits: mo(etat.octetsFaits), total: mo(etat.octetsTotal) });
      default:
        return etat.dernierPassage ? t('À jour, dernière vérification à {heure}.', { heure: heure(etat.dernierPassage) }) : t('En attente.');
    }
  });
  const progression = $derived(
    !etat || etat.phase === 'inactif'
      ? null
      : (etat.phase === 'envoi' || etat.phase === 'reception') && etat.octetsTotal
        ? etat.octetsFaits / etat.octetsTotal
        : etat.total
          ? etat.faits / etat.total
          : null,
  );

  async function rafraichir() {
    try {
      regles = await reglesEnvoi();
      erreurBase = null;
    } catch (e) {
      erreurBase = String(e);
      regles = [];
    }
    journal = await journalEnvoi(60).catch(() => []);
    virtuels = await dossiersVirtuels().catch(() => []);
  }

  async function ajouter(demande: Demande) {
    try {
      const r = demande.sansRaw;
      if (demande.sens === 'montante') await ajouterRegleEnvoi(demande.dossier, demande.cible, r);
      else if (demande.sens === 'doubleBibliotheque')
        await ajouterDoubleBibliotheque(demande.dossier, demande.cible, demande.selection, demande.dossierRecus, r);
      else if (demande.sens === 'double') await ajouterDouble(demande.dossier, demande.album, demande.nouvelAlbum, demande.miroir, r);
      else if (demande.sens === 'virtuel') await ajouterVirtuel(demande.dossier, demande.selection, demande.rangement, r);
      else await ajouterReception(demande.dossier, demande.selection, demande.rangement, r);
      notifier(t('« {dossier} » ajouté : la synchro commence.', { dossier: nomDossier(demande.dossier) }), 'succes');
      await rafraichir();
      return true;
    } catch (e) {
      notifier(String(e), 'erreur');
      return false;
    }
  }

  async function basculer(regle: RegleEnvoi, actif: boolean) {
    try {
      await activerRegleEnvoi(regle.id, actif);
      await rafraichir();
    } catch (e) {
      notifier(String(e), 'erreur');
    }
  }

  async function retirer(regle: RegleEnvoi) {
    aRetirer = null;
    try {
      await supprimerRegleEnvoi(regle.id);
      notifier(t('« {dossier} » n’est plus synchronisé.', { dossier: nomDossier(regle.dossier) }), 'succes');
      await rafraichir();
    } catch (e) {
      notifier(String(e), 'erreur');
    }
  }

  async function voirEchecs(regle: RegleEnvoi) {
    echecs = { regle: regle.id, fichiers: await echecsEnvoi(regle.id).catch(() => []) };
  }

  async function reessayer(regle: number) {
    const n = await reessayerEnvoi(regle).catch(() => 0);
    echecs = null;
    notifier(tp(n, '{n} fichier remis dans la file.', '{n} fichiers remis dans la file.'), 'succes');
    await rafraichir();
  }

  onMount(() => {
    void rafraichir();
    etatEnvoi().then((e) => (etat = e));
    let precedente = '';
    const arret = listen<EtatEnvoi>('envoi', ({ payload }) => {
      etat = payload;
      // Counters and log follow the phases and the end of every pass (a
      // quick pass may only show as idle to idle), not every byte.
      const cle = `${payload.phase}/${payload.regle}/${payload.phase === 'envoi' ? payload.faits : ''}/${payload.dernierPassage}`;
      if (cle !== precedente) {
        precedente = cle;
        void rafraichir();
      }
    });
    return () => void arret.then((f) => f());
  });
</script>

<svelte:head><title>{t('Synchro')} · Nicéphore</title></svelte:head>

<div class="h-full overflow-y-auto">
  <div class="mx-auto max-w-3xl space-y-8 p-8">
    <div class="flex items-start justify-between gap-4">
      <div class="space-y-1">
        <Heading size="large">{t('Synchronisation')}</Heading>
        <Text color="muted">
          {t('Des dossiers de cet ordinateur envoyés vers Immich, et des photos d’Immich reçues dans des dossiers, au fur et à mesure. Rien n’est jamais supprimé, ni ici ni sur le serveur.')}
        </Text>
      </div>
      <Button class="shrink-0 whitespace-nowrap" leadingIcon={mdiFolderPlusOutline} onclick={() => (ajout = true)} disabled={!!erreurBase}>
        {t('Ajouter un dossier')}
      </Button>
    </div>

    {#if erreurBase}
      <p class="rounded-xl bg-red-500/10 p-4 text-sm text-red-500">{erreurBase}</p>
    {/if}

    {#if etat && regles?.length}
      <section class="etat space-y-3 rounded-2xl p-4" aria-label={t('État de l’envoi')}>
        <div class="flex items-center gap-3">
          <Text class="min-w-0 flex-1">{description}</Text>
          <IconButton
            icon={mdiRefresh}
            aria-label={t('Vérifier maintenant')}
            variant="ghost"
            color="secondary"
            shape="round"
            disabled={etat.enPause || etat.phase !== 'inactif'}
            onclick={() => lancerEnvoi()}
          />
          <IconButton
            icon={etat.enPause ? mdiPlay : mdiPause}
            aria-label={etat.enPause ? t('Reprendre l’envoi') : t('Mettre l’envoi en pause')}
            variant="ghost"
            color="secondary"
            shape="round"
            onclick={() => pauseEnvoi(!etat?.enPause)}
          />
        </div>
        {#if progression != null}<ProgressBar progress={Math.min(1, progression)} />{/if}
        {#if etat.fichier && etat.phase !== 'inactif'}
          <Text size="small" color="muted" class="truncate">{etat.fichier}</Text>
        {/if}
        {#if etat.erreur}<p class="text-sm text-red-500">{etat.erreur}</p>{/if}
      </section>
    {/if}

    {#if regles && !regles.length && !erreurBase}
      <div class="vide space-y-2 rounded-2xl p-8 text-center">
        <Text>{t('Aucun dossier n’est envoyé pour l’instant.')}</Text>
        <Text size="small" color="muted">
          {t('Choisissez par exemple le dossier où arrivent les photos du téléphone ou de l’appareil photo.')}
        </Text>
      </div>
    {/if}

    <ul class="space-y-3">
      {#each regles ?? [] as regle (regle.id)}
        {@const erreur = regle.dossierAbsent ? t('Dossier introuvable (disque débranché ?) : rien n’est fait en attendant.') : etat?.erreursRegles[regle.id]}
        <li class="regle space-y-2 rounded-2xl p-4" class:inactive={!regle.actif}>
          <div class="flex items-center gap-3">
            <svg viewBox="0 0 24 24" width="24" height="24" aria-hidden="true" class="shrink-0 opacity-70">
              <path
                fill="currentColor"
                d={regle.sens === 'descendante' ? mdiFolderDownloadOutline : regle.sens === 'double' ? mdiFolderSyncOutline : mdiFolderArrowUpOutline}
              />
            </svg>
            <div class="min-w-0 flex-1">
              <div class="truncate font-semibold">{nomDossier(regle.dossier)}</div>
              <div class="truncate text-xs opacity-60" title={regle.dossier}>{regle.dossier}</div>
            </div>
            <Switch
              checked={regle.actif}
              aria-label={regle.actif ? t('Suspendre ce dossier') : t('Reprendre ce dossier')}
              onCheckedChange={(v: boolean) => basculer(regle, v)}
            />
            <IconButton
              icon={mdiDeleteOutline}
              aria-label={regle.sens === 'descendante'
                ? t('Ne plus recevoir dans {dossier}', { dossier: nomDossier(regle.dossier) })
                : regle.sens === 'double'
                  ? t('Ne plus synchroniser {dossier}', { dossier: nomDossier(regle.dossier) })
                  : t('Ne plus envoyer {dossier}', { dossier: nomDossier(regle.dossier) })}
              variant="ghost"
              color="secondary"
              shape="round"
              onclick={() => (aRetirer = regle)}
            />
          </div>
          {#if regle.sens === 'double' && regle.cible.genre !== 'album'}
            <div class="flex flex-wrap gap-x-4 gap-y-1 text-sm">
              <span class="opacity-70">
                {resumeDouble(regle)}
              </span>
              <span class="compteurs">
                {tp(regle.bilan.envoyes, '{n} envoyé', '{n} envoyés')} · {tp(regle.bilan.recus, '{n} reçu', '{n} reçus')}{#if regle.bilan.aRecevoir}
                  · {t('{n} à recevoir', { n: nombre(regle.bilan.aRecevoir) })}{/if}
              </span>
              {#if regle.bilan.echecs + regle.bilan.ignores}
                <button class="lien text-red-500" onclick={() => voirEchecs(regle)}>
                  {tp(regle.bilan.echecs + regle.bilan.ignores, '{n} fichier non envoyé', '{n} fichiers non envoyés')}
                </button>
              {/if}
              {#if regle.bilan.echecsReception}
                <button class="lien" onclick={() => recevoirEncore(regle)}>
                  {tp(regle.bilan.echecsReception, '{n} fichier en échec : recevoir de nouveau', '{n} fichiers en échec : recevoir de nouveau')}
                </button>
              {/if}
            </div>
          {:else if regle.sens === 'double'}
            {@const attente = etat?.confirmations?.[regle.id] ?? 0}
            <div class="flex flex-wrap gap-x-4 gap-y-1 text-sm">
              <span class="opacity-70">{t('Identique à l’album « {nom} »', { nom: regle.cible.genre === 'album' ? regle.cible.nom : '' })}</span>
              <span class="compteurs">{tp(regle.bilan.envoyes, '{n} envoyé', '{n} envoyés')} · {tp(regle.bilan.recus, '{n} reçu', '{n} reçus')}</span>
              <button class="lien" onclick={() => basculerMiroir(regle)}>
                {regle.miroir ? t('Suppressions répercutées') : t('Suppressions non répercutées')}
              </button>
            </div>
            {#if attente}
              <div class="attente flex items-center gap-3 rounded-xl p-3 text-sm">
                <span class="flex-1">{tp(attente, '{n} suppression attend votre accord.', '{n} suppressions attendent votre accord.')}</span>
                <Button size="small" variant="outline" onclick={() => voirSuppressions(regle)}>{t('Voir')}</Button>
              </div>
            {/if}
          {:else if regle.sens === 'descendante'}
            <div class="flex flex-wrap gap-x-4 gap-y-1 text-sm">
              <span class="opacity-70">
                {t('Reçoit {selection}', { selection: libelleSelection(regle.selection) })}{regle.rangement === 'anneeMois' ? t(', par année et mois') : ''}
              </span>
              <span class="compteurs">
                {tp(regle.bilan.recus, '{n} reçu', '{n} reçus')}{#if regle.bilan.aRecevoir} · {t('{n} à recevoir', { n: nombre(regle.bilan.aRecevoir) })}{/if}{#if regle.bilan.retiresServeur}
                  · {tp(regle.bilan.retiresServeur, '{n} retiré sur le serveur, gardé ici', '{n} retirés sur le serveur, gardés ici')}{/if}
              </span>
              {#if regle.bilan.retiresIci + regle.bilan.echecsReception}
                <button class="lien" onclick={() => recevoirEncore(regle)}>
                  {tp(regle.bilan.retiresIci + regle.bilan.echecsReception, '{n} fichier effacé ici ou en échec : recevoir de nouveau', '{n} fichiers effacés ici ou en échec : recevoir de nouveau')}
                </button>
              {/if}
            </div>
          {:else}
          <div class="flex flex-wrap gap-x-4 gap-y-1 text-sm">
            <span class="opacity-70">{libelleCible(regle.cible)}</span>
            <span class="compteurs">
              {tp(regle.bilan.envoyes, '{n} envoyé', '{n} envoyés')} · {t('{n} déjà sur le serveur', { n: nombre(regle.bilan.presents) })}{#if regle.bilan.enAttente}
                · {t('{n} en attente', { n: nombre(regle.bilan.enAttente) })}{/if}
            </span>
            {#if regle.bilan.echecs + regle.bilan.ignores}
              <button class="lien text-red-500" onclick={() => voirEchecs(regle)}>
                {tp(regle.bilan.echecs + regle.bilan.ignores, '{n} fichier non envoyé', '{n} fichiers non envoyés')}
              </button>
            {/if}
          </div>
          {/if}
          <div class="flex items-center gap-2 text-xs opacity-80">
            <Switch
              checked={!regle.sansRaw}
              aria-label={t('Inclure les fichiers RAW')}
              onCheckedChange={() => basculerRaw(regle)}
            />
            <span>{t('Inclure les fichiers RAW (DNG, ARW, CR2…)')}{regle.sansRaw ? t(' : laissés de côté') : ''}</span>
          </div>
          {#if erreur}<p class="text-sm text-red-500">{erreur}</p>{/if}
        </li>
      {/each}
    </ul>

    {#if virtuels.length}
      <section class="space-y-3">
        <Heading size="small">{t('Dossiers virtuels')}</Heading>
        <ul class="space-y-3">
          {#each virtuels as d (d.id)}
            <li class="regle space-y-2 rounded-2xl p-4">
              <div class="flex items-center gap-3">
                <svg viewBox="0 0 24 24" width="24" height="24" aria-hidden="true" class="shrink-0 opacity-70"><path fill="currentColor" d={mdiFolderEyeOutline} /></svg>
                <div class="min-w-0 flex-1">
                  <div class="truncate font-semibold">{nomDossier(d.point)}</div>
                  <div class="truncate text-xs opacity-60" title={d.point}>{d.point}</div>
                </div>
                <IconButton
                  icon={mdiDeleteOutline}
                  aria-label={t('Retirer le dossier virtuel {dossier}', { dossier: nomDossier(d.point) })}
                  variant="ghost"
                  color="secondary"
                  shape="round"
                  onclick={() => retirerDossierVirtuel(d)}
                />
              </div>
              <div class="flex flex-wrap gap-x-4 gap-y-1 text-sm">
                <span class="opacity-70">
                  {t('Montre {selection}', { selection: libelleSelection(d.selection) })}{d.rangement === 'anneeMois' ? t(', par année et mois') : ''}{d.sansRaw ? t(', sans RAW') : ''}
                </span>
                {#if d.monte}<span class="compteurs">{tp(d.fichiers, '{n} fichier visible, téléchargé à l’ouverture', '{n} fichiers visibles, téléchargés à l’ouverture')}</span>{/if}
              </div>
              {#if d.erreur}<p class="text-sm text-red-500">{d.erreur}</p>{/if}
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if journal.length}
      <section class="space-y-2">
        <Heading size="small">{t('Dernières actions')}</Heading>
        <ol class="journal text-sm">
          {#each journal as entree (entree.id)}
            <li class="flex gap-3 py-1">
              <span class="w-28 shrink-0 opacity-60">{heure(entree.horodatage)}</span>
              <span class="shrink-0">{libellesJournal[entree.action] ? t(libellesJournal[entree.action]) : entree.action}</span>
              <span class="min-w-0 truncate opacity-70" title={detailJournal(entree)}>{detailJournal(entree)}</span>
            </li>
          {/each}
        </ol>
      </section>
    {/if}
  </div>
</div>

{#if ajout}
  <DialogueRegle onvalider={ajouter} onfermer={() => (ajout = false)} />
{/if}

{#if aRetirer}
  {@const regle = aRetirer}
  <Dialogue
    titre={regle.sens === 'descendante' ? t('Ne plus recevoir dans ce dossier ?') : regle.sens === 'double' ? t('Ne plus synchroniser ce dossier ?') : t('Ne plus envoyer ce dossier ?')}
    onfermer={() => (aRetirer = null)}
  >
    <div class="space-y-4">
      <Text>
        {t('Nicéphore oubliera « {dossier} ». Les fichiers restent sur cet ordinateur et les photos déjà envoyées restent sur le serveur.', { dossier: nomDossier(regle.dossier) })}
      </Text>
      <div class="flex justify-end gap-2">
        <Button variant="ghost" color="secondary" onclick={() => (aRetirer = null)}>{t('Annuler')}</Button>
        <Button color="danger" onclick={() => retirer(regle)}>{regle.sens === 'descendante' ? t('Ne plus recevoir') : regle.sens === 'double' ? t('Ne plus synchroniser') : t('Ne plus envoyer')}</Button>
      </div>
    </div>
  </Dialogue>
{/if}

{#if enAttente}
  {@const a = enAttente}
  <Dialogue titre={t('Suppressions en attente')} onfermer={() => (enAttente = null)} largeur={620}>
    <div class="space-y-3">
      <Text size="small">
        {t('Beaucoup de fichiers ont disparu d’un côté d’un coup : Nicéphore attend votre accord avant de faire de même de l’autre côté. Si c’est une erreur (dossier vidé par mégarde, album modifié), annulez : les fichiers reviennent.')}
      </Text>
      <ul class="max-h-80 space-y-1 overflow-y-auto text-sm">
        {#each a.liste as s (s.cote + s.assetId)}
          <li class="flex gap-3">
            <span class="min-w-0 flex-1 truncate" title={s.chemin}>{s.chemin}</span>
            <span class="shrink-0 opacity-70">
              {s.cote === 'serveur' ? t('effacé ici, sortira de l’album') : t('sorti de l’album, ira à la corbeille')}
            </span>
          </li>
        {/each}
      </ul>
      <div class="flex justify-end gap-2">
        <Button variant="ghost" color="secondary" onclick={() => deciderSuppressions(false)}>{t('Annuler ces suppressions')}</Button>
        <Button color="danger" onclick={() => deciderSuppressions(true)}>{t('Appliquer')}</Button>
      </div>
    </div>
  </Dialogue>
{/if}

{#if echecs}
  {@const liste = echecs}
  <Dialogue titre={t('Fichiers non envoyés')} onfermer={() => (echecs = null)} largeur={620}>
    <div class="space-y-3">
      <ul class="space-y-2 text-sm">
        {#each liste.fichiers as f (f.chemin)}
          <li>
            <div class="break-all font-medium">{f.chemin}</div>
            <div class="opacity-70">{f.erreur ?? t('raison inconnue')}{#if f.essais} · {tp(f.essais, '{n} essai', '{n} essais')}{/if}</div>
          </li>
        {/each}
      </ul>
      <div class="flex justify-end">
        <Button onclick={() => reessayer(liste.regle)}>{t('Réessayer')}</Button>
      </div>
    </div>
  </Dialogue>
{/if}

<style>
  .etat,
  .regle,
  .vide {
    background: color-mix(in oklab, currentColor 5%, transparent);
  }
  .regle.inactive {
    opacity: 0.6;
  }
  .attente {
    color: #b45309;
    background: color-mix(in oklab, #f59e0b 14%, transparent);
  }
  :global(.dark) .attente {
    color: #fbbf24;
  }
  .lien {
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .journal li + li {
    border-top: 1px solid color-mix(in oklab, currentColor 8%, transparent);
  }
</style>
