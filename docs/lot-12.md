# Lot 12 : fichiers virtuels sous Linux (09/11/2026)

Premier lot de l'étape 4. Une sélection d'Immich montrée comme un dossier ordinaire, sans rien
télécharger avant qu'un programme n'ouvre un fichier.

## Interface

- Quatrième choix dans « Ajouter un dossier » : « Voir une sélection sans la télécharger » (renommé « Fichiers virtuels » le 11/11). Même
  choix que pour recevoir (album, personne, favoris, période, toute la photothèque ; à plat ou par
  année et mois), dans un dossier vide. Avant de valider : nombre de photos et volume visibles.
- Section « Dossiers virtuels » sur la page Synchro : dossier, sélection, « N fichiers visibles,
  téléchargés à l'ouverture », retrait.
- Réglages : place occupée par les originaux ouverts (20 Go au plus) et « Vider les originaux ».

## Fonctionnement (crate `crates/virtuel`)

- Système de fichiers FUSE (`fuser` 0.18, sans libfuse : montage par `fusermount3`), en lecture
  seule : écriture, création et suppression refusées par le noyau. Le dossier choisi doit être
  vide (jamais de fichiers masqués) ; un montage orphelin laissé par un plantage est libéré avant
  de remonter.
- Les fichiers ont leur vraie taille et leur date d'origine ; lister ne télécharge rien.
- La première lecture d'un fichier télécharge l'original dans le cache (fichier temporaire,
  empreinte SHA-1 comparée à celle du serveur, puis renommage) ; deux lectures simultanées
  partagent le même téléchargement ; les suivantes viennent du cache. Les lectures qui attendent
  le réseau sont servies depuis une tâche, le système de fichiers ne se bloque jamais.
- Cache plafonné (20 Go) : au-delà, les originaux lus le moins récemment partent.
- Numéros d'inode stables d'une actualisation à l'autre ; la sélection est relue toutes les
  5 minutes.
- Liste par compte (`virtuels-<compte>.json`) : montée à la connexion, démontée au changement de
  compte, à la déconnexion et à la sortie de l'appli.

## Vérifié

- Tests unitaires : arbre par année et mois avec homonymes et inodes stables, élagage du cache.
- Montage réel sur l'hôte contre le serveur jetable (février 2025) : refus d'un dossier non vide,
  liste sans téléchargement avec les vraies tailles, première lecture téléchargée une fois avec
  la bonne empreinte, seconde lecture depuis le cache, écriture et suppression refusées, dossier
  vide après démontage.
- Dans l'appli (54/54) : dossier virtuel monté avec ses 6 fichiers, affiché sur la page, retiré ;
  aucun montage restant après la fermeture (contrôlé par `banc.sh`).
- Windows : pas encore (lot 13, API Cloud Files) ; l'ajout y est refusé avec un message clair.

## Persistance (09/11, retour de Valentin : le dossier disparaissait à la fermeture)

- Sous Linux, un dossier virtuel n'existe que tant qu'un programme le sert. Les dossiers sont donc
  confiés à un petit service sans fenêtre, le même programme lancé avec `--dossiers-virtuels`
  (`app/src-tauri/src/service_virtuel.rs`) : il suit le compte actif, monte et démonte les
  dossiers selon la liste écrite par l'appli, les actualise toutes les 5 minutes, publie son état
  (`virtuels-etat.json`) et se démonte proprement à l'arrêt. Une seule instance (verrou) ; il
  s'arrête seul une minute après le dernier dossier retiré.
- L'appli le lance (détaché, sa propre session) dès qu'un dossier virtuel existe, et l'inscrit au
  démarrage de session (`~/.config/autostart/nicephore-dossiers-virtuels.desktop`, retiré avec le
  dernier dossier). Une AppImage lance l'AppImage elle-même. Fermer Nicéphore ne touche plus aux
  dossiers virtuels.
- En mode portable (comptes scellés par une phrase de passe), les dossiers restent servis par
  l'appli, comme avant.
- Vérifié : `scripts/essai-service-virtuel.sh` sur l'hôte (montage depuis la liste, une seule
  instance, contenu identique à l'original, état publié, démontage au retrait, arrêt seul sans
  dossier, démontage propre à l'arrêt), et dans l'appli (55/55) : dossier confié au service
  encore servi après la fermeture de l'appli, démonté à l'arrêt du service.
