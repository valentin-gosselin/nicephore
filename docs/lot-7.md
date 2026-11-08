# Lot 7 : envoi en arrière-plan (08/11/2026)

## Zone de notification

- Icône de Nicéphore dans la zone de notification (StatusNotifierItem sous KDE, via
  libayatana-appindicator), avec un menu : « Ouvrir Nicéphore », l'état de l'envoi (« à jour »,
  « 3 sur 12 », « en attente (sur batterie) », « en pause »…), pause ou reprise, « Quitter
  Nicéphore ». Sous Linux un clic sur l'icône ouvre le menu (limite des indicateurs), sous Windows
  un clic gauche ouvre la fenêtre.
- Fermer la fenêtre la masque si un dossier est envoyé et que le réglage le permet : l'envoi
  continue. La première fois, une notification le dit. Sans dossier actif, ou sans zone de
  notification sur le bureau, fermer quitte comme avant.

## Démarrage avec la session

- Réglage « Lancer Nicéphore à l'ouverture de session » : entrée `~/.config/autostart/nicephore.desktop`
  sous Linux (elle vise l'AppImage elle-même quand l'appli tourne depuis une AppImage, son point de
  montage changeant à chaque lancement), clé `Run` de l'utilisateur sous Windows.
- Lancé ainsi (`--arriere-plan`), Nicéphore reste dans la zone de notification si un dossier est
  envoyé ; sinon la fenêtre s'ouvre normalement. La fenêtre est créée cachée puis montrée, pour ne
  pas clignoter au démarrage discret.

## Conditions d'envoi

- « Attendre le secteur » : un portable débranché attend. Seules les batteries du système comptent,
  et seulement quand elles se déchargent : la batterie d'une souris sans fil (`scope=Device`, cas du
  PC fixe avec sa Logitech) est ignorée.
- « Attendre un réseau non facturé » : propriété `Metered` de NetworkManager (oui ou « deviné oui »,
  typiquement le partage de connexion d'un téléphone). Pas encore détecté sous Windows.
- Vérifiées avant chaque passage et avant chaque fichier : débrancher en plein envoi arrête la suite,
  la page affiche « En attente : sur batterie. L'envoi reprendra tout seul. », et le moteur
  regarde de nouveau chaque minute. En cas de doute (rien de lisible), l'envoi continue.

## Notifications

- Fin d'un passage qui a envoyé des fichiers (« 12 photos et vidéos envoyées vers Immich ») et arrêt
  sur une erreur (session expirée…), seulement si la fenêtre n'est pas au premier plan. Réglage pour
  les couper.

## Corrigé en route

- Les notifications partent hors du relais d'état : l'appel D-Bus peut attendre son délai maximal
  quand aucun serveur de notifications ne répond, et figeait la page Envoi.
- La page Envoi se rafraîchit à la fin de chaque passage, pas seulement aux changements de phase :
  un passage plus court que le pas du relais (200 ms) passait inaperçu (« 0 envoyé » affiché alors
  que les 3 photos étaient parties). Ce défaut existait au lot 6, masqué par le hasard du minutage.

## Vérifié

- Tests unitaires : batterie de souris ignorée, portable branché ou non, réglages relus avec
  valeurs par défaut, entrée `.desktop` échappée, résumés du menu ; test manuel sur le système réel
  (NetworkManager lu, pas de batterie système).
- Moteur contre le Immich jetable : « attente sur batterie » (rien ne part, puis tout part une fois
  rebranché, compteur de fin de passage juste).
- Scénario dans l'appli (38/38) avec un dossier de configuration privé (l'entrée d'autostart créée
  par le test ne touche jamais la vraie session) : réglages basculés et relus, démarrage avec la
  session activé puis retiré, icône créée, fenêtre fermée pendant qu'un dossier est envoyé puis
  rouverte comme depuis le menu de l'icône.
- Banc en lecture seule : fenêtre affichée au lancement, défilement à 60 i/s.
- Non vérifiable ici : le rendu réel de l'icône et de son menu dans la barre de Plasma, les
  notifications à l'écran (la session virtuelle n'a ni barre ni serveur de notifications), et tout
  le code Windows (registre, batterie), qui sera compilé par l'intégration continue du lot 9.
