# Retouches du 08/11 au soir (retours de Valentin)

- **Logo absent du bureau** : les fichiers d'icône étaient bons et KDE les trouvait, mais KWin
  (lancé à 12 h 57) et la barre Plasma (19 h 31) gardaient en mémoire l'ancienne icône
  « nicephore », installée avant le nouveau logo (20 h 17). `scripts/installer-lanceur.sh`
  envoie désormais le signal `org.kde.KIconLoader.iconChanged` en plus de reconstruire le cache.
- **Barre de titre intégrée** : plus de cadre du système. L'en-tête de l'appli sert de barre de
  titre (zones vides pour déplacer la fenêtre, double-clic pour l'agrandir), avec réduire,
  agrandir ou restaurer, fermer à droite (rouge au survol). Une bande fine fait de même sur les
  écrans de chargement, de connexion et de déverrouillage, et la barre de la visionneuse permet
  aussi de déplacer la fenêtre. Le redimensionnement par les bords est assuré par Tauri sous Linux
  et Windows. Fermer passe par la demande de fermeture habituelle (zone de notification si des
  dossiers sont envoyés).
- **Personnes dans les infos d'une photo** : avatars ronds des visages reconnus, en haut du
  panneau comme dans la webapp ; un clic ferme la visionneuse et ouvre la page de la personne.
- **Avatar du compte en haut à droite** : photo de profil Immich (`/users/{id}/profile-image`,
  mise en cache comme les vignettes) ou initiales sur la couleur du compte ; menu avec nom,
  e-mail, serveur et version, Réglages, Se déconnecter.

## Vérifié

- Scénario d'écriture 43/43 : menu du compte, boutons de fenêtre, agrandir puis restaurer,
  réduction acceptée (Wayland ne dit pas à une appli qu'elle est réduite : seul l'appel est
  vérifiable).
- Banc en lecture seule : une photo d'Alice montre son visage dans les infos, le clic ouvre sa page
  et ferme la visionneuse ; défilement toujours à 60 i/s.
- Non vérifiable ici : l'icône dans ta barre Plasma (relance Nicéphore pour la voir) et le
  redimensionnement par les bords à la souris.

## Plusieurs comptes (nuit du 08 au 09/11)

- Le menu de l'avatar liste les autres comptes mémorisés (cadenas pour un compte scellé en
  portable), « Ajouter un compte » (écran de connexion avec « Annuler, revenir au compte actuel »)
  et « Se déconnecter », qui n'oublie que le compte courant : le suivant prend la main.
- `comptes.json` remplace `compte.json` (repris tout seul à la prochaine connexion) ; chaque secret
  a son entrée dans le trousseau, ou son propre coffre en portable.
- Une base d'envoi par compte (`envoi-<clé>.sqlite`) : un dossier réglé pour un compte ne part
  jamais vers un autre. L'ancienne base va au premier compte connecté après la mise à jour.
- Chaque changement de compte recharge toute l'interface : rien de l'ancien compte ne reste à
  l'écran.
- « Réglages » quitte le bas de la barre latérale : il est dans le menu de l'avatar.

Vérifié : tests unitaires (plusieurs comptes scellés, doublon mis à jour, déconnexion qui laisse
l'autre, ancien format converti) ; `scripts/banc.sh --comptes` sur le serveur jetable, qui a
désormais un second utilisateur avec ses 3 photos : ajout du second compte, ses propres photos,
retour au premier par sa phrase, déconnexion puis reprise par le compte restant, deux bases d'envoi
distinctes. Non-régression : portable 8/8, écriture 43/43, lecture seule à 60 i/s. Le chemin du
trousseau réel (hors portable) n'est pas testable dans la session virtuelle.
