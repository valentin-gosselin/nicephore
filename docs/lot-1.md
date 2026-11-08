# Lot 1 : navigation, connexion, sélection multiple (08/11/2026)

## Contenu

- Barre latérale : Photos, Favoris, Archive, Corbeille, Réglages. Explorer, Carte, Personnes et Albums
  apparaissent grisés avec le lot qui les apportera. Repliée en icônes sous 960 px de large.
- Une chronologie par vue (filtres `isFavorite`, `visibility=archive`, `isTrashed` de
  `/timeline/buckets`), avec titre, nombre d'éléments et message quand la vue est vide.
- Connexion en deux temps comme les applis Immich : l'adresse du serveur (sans `http://` au besoin),
  qui renvoie ses modes de connexion (`/server/features`), puis e-mail et mot de passe
  (`POST /auth/login`). Seul le jeton de session est gardé, jamais le mot de passe ; la session
  apparaît dans les appareils connectés d'Immich et la déconnexion la révoque (`/auth/logout`).
  Clé API en option avancée. OAuth signalé mais pas encore géré (désactivé sur le serveur de test).
  L'adresse va dans `~/.config/io.github.valentin-gosselin.nicephore/compte.json`, le secret dans le
  trousseau du système (Secret Service, donc KWallet sous KDE ; Gestionnaire d'identification sous
  Windows).
- Thème : « Comme le système » par défaut, lu par Tauri (portail freedesktop sous Linux) plutôt que
  par WebKitGTK, et suivi en direct ; sombre au démarrage pour éviter un flash blanc.
- Réglages : compte et mode de connexion, déconnexion, thème, cache (taille, vidage), à propos.
- Sélection multiple : coche au survol de chaque vignette, coche par journée, clic en mode
  sélection, Ctrl+clic, Maj+clic pour une plage, Ctrl+A (mois chargés), Échap, Suppr.
- Actions groupées : favori (ajout ou retrait), archiver ou désarchiver, corbeille (annulable depuis
  la notification), restaurer depuis la corbeille, télécharger les originaux dans un dossier choisi
  (fichier temporaire puis renommage, jamais d'écrasement : `photo (1).jpg`). Pas de suppression
  définitive dans l'appli.

## Mesures (banc du 08/11, KWin virtuel)

59 à 60 i/s, 0 ou 1 saccade en défilement normal et rapide, vignettes affichées à 100 %. Un composant
d'icône par vignette faisait tomber à 23 i/s et 47 saccades : les vignettes utilisent du SVG direct.

Tests d'intégration contre le serveur (`cargo test -p api_immich -- --ignored`, avec `IMMICH_URL`) :
lecture des modes de connexion, refus d'un mauvais mot de passe.

## Droits de clé API nécessaires (si connexion par clé)

Lecture : `asset.read`, `asset.view`, `asset.download`, `timeline.read`, `user.read`, `server.about`.
Actions : `asset.update` (favori, archive), `asset.delete` (corbeille ; la restauration demande
peut-être un droit de corbeille en plus, à vérifier sur le serveur).

## À tester à la main (actions d'écriture, jamais lancées par le banc)

1. Lancement depuis le menu : adresse du serveur, puis e-mail et mot de passe, « Rester connecté » ;
   relance : pas de nouvelle saisie (jeton lu dans KWallet). La session apparaît dans Immich
   (Paramètres du compte, Appareils) et disparaît à la déconnexion.
2. Favori, archive, désarchivage, corbeille, « Annuler », restauration, sur des photos sans enjeu.
3. Téléchargement de quelques originaux dans un dossier de test.
4. Réglages : changement de thème, vidage du cache, déconnexion.
