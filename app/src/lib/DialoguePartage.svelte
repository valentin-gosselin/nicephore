<script lang="ts">
  // « Partager », like Immich web. For an album: the people it is shared
  // with (viewer or editor, invite, remove) and its public links. For a
  // selection of photos: a public link to them, copied once made.
  import { Button, Checkbox, IconButton, LoadingSpinner, Select, Text } from '@immich/ui';
  import { mdiAccountPlusOutline, mdiClose, mdiLinkVariantPlus } from '@mdi/js';
  import { onMount } from 'svelte';
  import {
    ajouterMembres,
    changerRole,
    copierTexte,
    liensPartages,
    membresAlbum,
    retirerMembre,
    supprimerLien,
    utilisateursServeur,
    type LienPartage,
    type MembresAlbum,
    type RoleAlbum,
    type Utilisateur,
  } from './api';
  import Avatar from './Avatar.svelte';
  import Dialogue from './Dialogue.svelte';
  import FormulaireLien from './FormulaireLien.svelte';
  import { t, tp } from './i18n.svelte';
  import LigneLien from './LigneLien.svelte';
  import { notifier } from './notifications.svelte';

  let {
    album = null,
    assets = [],
    onfermer,
  }: {
    album?: { id: string; nom: string; possede: boolean } | null;
    assets?: string[];
    onfermer: () => void;
  } = $props();

  // svelte-ignore state_referenced_locally
  const cible = album;
  /** `liste`: people and links; `lien`: the options of a link; `inviter`. */
  let vue = $state<'liste' | 'lien' | 'inviter'>(cible ? 'liste' : 'lien');
  let lienEdite = $state<LienPartage | null>(null);
  let liens = $state<LienPartage[] | null>(cible ? null : []);
  let membres = $state<MembresAlbum | null>(null);
  let utilisateurs = $state<Utilisateur[] | null>(null);
  let invites = $state<Record<string, boolean>>({});
  let roleInvites = $state<RoleAlbum>('viewer');
  let erreur = $state<string | null>(null);

  const ROLES = $derived([
    { value: 'viewer', label: t('Visionneur') },
    { value: 'editor', label: t('Éditeur') },
  ]);

  async function chargerLiens() {
    if (!cible) return;
    try {
      liens = await liensPartages(cible.id);
    } catch (e) {
      erreur = String(e);
    }
  }

  async function chargerMembres() {
    if (!cible) return;
    try {
      membres = await membresAlbum(cible.id);
    } catch (e) {
      erreur = String(e);
    }
  }

  onMount(() => {
    void chargerLiens();
    void chargerMembres();
  });

  const titre = $derived(
    vue === 'inviter'
      ? t('Inviter une personne')
      : vue === 'lien'
        ? lienEdite
          ? t('Modifier le lien')
          : t('Créer un lien pour partager')
        : cible
          ? t('Partager « {nom} »', { nom: cible.nom })
          : tp(assets.length, 'Partager {n} photo', 'Partager {n} photos'),
  );

  async function lienFait(lien: LienPartage, nouveau: boolean) {
    if (nouveau) {
      try {
        await copierTexte(lien.url);
        notifier(t('Lien créé et copié : {url}', { url: lien.url }), 'succes');
      } catch {
        notifier(t('Lien créé : {url}', { url: lien.url }), 'succes');
      }
    }
    if (cible) await chargerLiens();
    else liens = [lien, ...(liens ?? []).filter((l) => l.id !== lien.id)];
    lienEdite = null;
    vue = 'liste';
  }

  function annulerLien() {
    lienEdite = null;
    // A selection without a link yet: nothing else to show.
    if (!cible && !liens?.length) onfermer();
    else vue = 'liste';
  }

  async function supprimer(lien: LienPartage) {
    try {
      await supprimerLien(lien.id);
      liens = (liens ?? []).filter((l) => l.id !== lien.id);
      notifier(t('Lien supprimé'), 'succes');
    } catch (e) {
      notifier(t('Suppression impossible : {raison}', { raison: String(e) }), 'erreur');
    }
  }

  async function ouvrirInvitations() {
    vue = 'inviter';
    invites = {};
    if (!utilisateurs) utilisateurs = await utilisateursServeur().catch((e) => ((erreur = String(e)), []));
  }

  const invitables = $derived(
    (utilisateurs ?? []).filter((u) => u.id !== membres?.proprietaire.id && !membres?.membres.some((m) => m.utilisateur.id === u.id)),
  );
  const nombreInvites = $derived(Object.values(invites).filter(Boolean).length);

  async function inviter() {
    if (!cible) return;
    const choisis = Object.entries(invites)
      .filter(([, oui]) => oui)
      .map(([utilisateur]) => ({ utilisateur, role: roleInvites }));
    try {
      await ajouterMembres(cible.id, choisis);
      notifier(tp(choisis.length, 'Album partagé avec {n} personne', 'Album partagé avec {n} personnes'), 'succes');
      await chargerMembres();
      vue = 'liste';
    } catch (e) {
      notifier(t('Partage impossible : {e}', { e: String(e) }), 'erreur');
    }
  }

  async function role(utilisateur: Utilisateur, nouveau: RoleAlbum) {
    if (!cible) return;
    try {
      await changerRole(cible.id, utilisateur.id, nouveau);
      await chargerMembres();
    } catch (e) {
      notifier(t('Modification impossible : {raison}', { raison: String(e) }), 'erreur');
    }
  }

  async function retirer(utilisateur: Utilisateur) {
    if (!cible) return;
    try {
      await retirerMembre(cible.id, utilisateur.id);
      notifier(t('{nom} n’a plus accès à l’album', { nom: utilisateur.name }), 'succes');
      await chargerMembres();
    } catch (e) {
      notifier(t('Retrait impossible : {e}', { e: String(e) }), 'erreur');
    }
  }
</script>

<Dialogue {titre} {onfermer} largeur={540}>
  {#if erreur}<Text color="danger" class="mb-3">{erreur}</Text>{/if}

  {#if vue === 'lien'}
    <FormulaireLien lien={lienEdite} album={cible?.id ?? null} {assets} onfini={lienFait} onannuler={annulerLien} />
  {:else if vue === 'inviter'}
    {#if !utilisateurs}
      <LoadingSpinner />
    {:else if !invitables.length}
      <Text color="muted">{t('Cet album est déjà partagé avec tous les comptes du serveur, ou il n’y en a pas d’autre.')}</Text>
    {:else}
      <ul class="space-y-1">
        {#each invitables as u (u.id)}
          <li>
            <label class="personne flex cursor-pointer items-center gap-3 rounded-lg p-2">
              <Checkbox checked={!!invites[u.id]} onCheckedChange={(v: boolean) => (invites[u.id] = v)} />
              <Avatar utilisateur={u} />
              <span class="min-w-0">
                <span class="block truncate font-medium">{u.name}</span>
                <span class="block truncate text-xs opacity-60">{u.email}</span>
              </span>
            </label>
          </li>
        {/each}
      </ul>
      <div class="mt-4 flex items-center gap-3">
        <span class="text-sm">{t('Rôle')}</span>
        <Select class="w-44" options={ROLES} value={roleInvites} onChange={(v) => (roleInvites = v as RoleAlbum)} />
        <span class="flex-1"></span>
        <Button variant="ghost" color="secondary" onclick={() => (vue = 'liste')}>{t('Annuler')}</Button>
        <Button disabled={!nombreInvites} onclick={inviter}>{t('Ajouter')}</Button>
      </div>
    {/if}
  {:else}
    {#if cible}
      <section class="mb-6">
        <h3 class="mb-2 text-sm font-semibold opacity-70">{t('Personnes')}</h3>
        {#if !membres}
          <LoadingSpinner />
        {:else}
          <ul class="space-y-1">
            <li class="flex items-center gap-3 p-2">
              <Avatar utilisateur={membres.proprietaire} />
              <span class="min-w-0 flex-1">
                <span class="block truncate font-medium">{membres.proprietaire.name}</span>
                <span class="block truncate text-xs opacity-60">{membres.proprietaire.email}</span>
              </span>
              <span class="text-sm opacity-70">{t('Propriétaire')}</span>
            </li>
            {#each membres.membres as m (m.utilisateur.id)}
              <li class="membre flex items-center gap-3 p-2">
                <Avatar utilisateur={m.utilisateur} />
                <span class="min-w-0 flex-1">
                  <span class="block truncate font-medium">{m.utilisateur.name}</span>
                  <span class="block truncate text-xs opacity-60">{m.utilisateur.email}</span>
                </span>
                {#if cible.possede}
                  <Select class="w-36" options={ROLES} value={m.role} onChange={(v) => void role(m.utilisateur, v as RoleAlbum)} />
                  <IconButton
                    icon={mdiClose}
                    aria-label={t('Retirer {nom} de l’album', { nom: m.utilisateur.name })}
                    variant="ghost"
                    color="secondary"
                    shape="round"
                    onclick={() => void retirer(m.utilisateur)}
                  />
                {:else}
                  <span class="text-sm opacity-70">{m.role === 'editor' ? t('Éditeur') : t('Visionneur')}</span>
                {/if}
              </li>
            {/each}
          </ul>
          {#if cible.possede}
            <Button class="mt-2" variant="ghost" leadingIcon={mdiAccountPlusOutline} onclick={() => void ouvrirInvitations()}>{t('Inviter une personne')}</Button>
          {/if}
        {/if}
      </section>
    {/if}

    <section>
      <h3 class="mb-2 text-sm font-semibold opacity-70">{t('Liens partagés')}</h3>
      {#if liens === null}
        <LoadingSpinner />
      {:else}
        {#each liens as l (l.id)}
          <LigneLien
            lien={l}
            titre={!cible}
            onmodifier={() => {
              lienEdite = l;
              vue = 'lien';
            }}
            onsupprimer={() => void supprimer(l)}
          />
        {:else}
          <Text size="small" color="muted">{t('Aucun lien pour l’instant : un lien permet de partager sans compte Immich.')}</Text>
        {/each}
      {/if}
      {#if cible}
        <Button
          class="mt-2"
          variant="ghost"
          leadingIcon={mdiLinkVariantPlus}
          onclick={() => {
            lienEdite = null;
            vue = 'lien';
          }}
        >
          {t('Créer un lien')}
        </Button>
      {/if}
    </section>
  {/if}
</Dialogue>

<style>
  .personne:hover {
    background: color-mix(in oklab, currentColor 6%, transparent);
  }
</style>
