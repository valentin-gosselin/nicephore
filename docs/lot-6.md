# Lot 6 : envoi des dossiers vers Immich (08/11/2026)

Premier lot de l'étape 2. Un dossier de l'ordinateur est relié au serveur : ses photos et vidéos y
partent, au fur et à mesure qu'elles arrivent.

## Page Envoi (barre latérale, « Cet ordinateur »)

- « Ajouter un dossier » ouvre le sélecteur du système, puis on choisit où vont les photos :
  - un album par dossier (nom du dossier qui contient la photo ; celles à la racine vont dans un
    album au nom du dossier relié), créé au premier envoi ;
  - un album existant, choisi dans la liste (identifié par son id, jamais par son nom) ;
  - la photothèque seule.
- Pour chaque dossier : envoyés, déjà sur le serveur, en attente, non envoyés (liste avec la raison
  et bouton « Réessayer »), interrupteur pour le suspendre, corbeille pour ne plus l'envoyer (rien
  n'est supprimé, ni le dossier ni les photos du serveur).
- Bandeau d'état : phase en cours, progression (en octets pendant l'envoi), fichier courant,
  « Vérifier maintenant », pause et reprise. Un point pulse sur « Envoi » dans la barre latérale
  pendant le travail.
- « Dernières actions » : journal (envoyé, album créé, ignoré, abandonné, déjà dans la corbeille du
  serveur, dossier ajouté ou retiré).

## Moteur (crate `crates/envoi`)

- Base SQLite (WAL) dans le dossier de données de l'appli : règles, état de chaque fichier
  (taille, date, SHA-1, état, id de l'asset, essais), albums des dossiers, journal. Schéma
  versionné ; copie `envoi.sqlite.vN.sauvegarde` avant toute migration ; une base d'une version
  plus récente est refusée sans y toucher, et la page l'affiche.
- Un passage par règle active : parcours du dossier (liens symboliques non suivis, fichiers et
  dossiers cachés ignorés, `.part`, `.tmp`, `.crdownload`… ignorés, extensions acceptées lues sur le
  serveur par `/server/media-types`) ; empreinte SHA-1 des seuls fichiers nouveaux ou changés
  (taille ou date) ; pré-vol `bulk-upload-check` par lots de 500 ; envoi en flux depuis le disque,
  trois à la fois, avec l'en-tête `x-immich-checksum` ; puis ajout aux albums par lots de 1 000.
- Réveils : événements du système de fichiers (`notify`, récursif, 3 s de calme après une rafale),
  bouton « Vérifier maintenant », essais à refaire arrivés à échéance, et toutes les 15 minutes.
- Garde-fous (repris des défauts relevés chez Mimick) :
  - rien n'est jamais supprimé ; un dossier illisible ou absent (disque débranché) arrête la règle
    sans rien conclure ;
  - un fichier doit être inchangé depuis 10 s, jugé sur la plus récente de ses dates (modification,
    création de la copie, changement d'état) : une copie qui garde la date d'origine (carte SD,
    explorateur Windows) est bien vue comme en cours ;
  - un fichier qui change pendant son empreinte ou avant son envoi est repris au passage suivant ;
  - un doublon déjà dans la corbeille du serveur y reste et n'est pas remis dans un album ;
  - un album supprimé sur le serveur n'est jamais recréé : la règle le signale (album choisi) ou le
    journal le note (album par dossier) ;
  - essais limités avec délai croissant (30 s, 1 min, 2 min… jusqu'à 6 h) : 8 pour une coupure
    réseau ou un serveur surchargé, 3 pour un refus ; session expirée ou droit manquant : tout
    s'arrête et l'erreur s'affiche, sans boucler.
- Pas de délai global sur l'envoi d'un gros fichier (vidéo de plusieurs Go) : la limite est
  proportionnelle à la taille, au moins 64 ko/s.

## Vérifié

- 10 tests unitaires (base, inventaire, empreinte, copie antidatée vue récente, noms d'albums).
- 3 tests contre le Immich jetable (`scripts/immich-jetable.sh tester`) : dossier avec sous-dossier,
  copie déjà présente, fichiers à ignorer, fichier récent différé, repassage sans renvoi, fichier
  modifié renvoyé ; album choisi puis supprimé sur le serveur ; boucle réelle avec surveillance :
  une photo déposée dans un sous-dossier part seule.
- Scénario dans l'appli (`scripts/banc.sh --jetable`, 31/31) : page vide, dossier ajouté, trois
  photos envoyées et la copie reconnue, texte ignoré, albums créés, pause et reprise, album du
  sous-dossier visible dans Albums avec ses 2 éléments, dossier retiré. Seul le sélecteur de dossier
  du système n'est pas cliqué (le scénario passe le chemin directement).
- Banc en lecture seule sur la vraie bibliothèque : base d'envoi privée au banc (aucune règle),
  défilement toujours à 60 i/s.

## Reste pour l'étape 2

- Lot 7 : icône dans la zone de notification, démarrage avec la session, notifications de fin
  d'envoi, pause sur batterie ou réseau facturé.
- Lot 8 : mode portable.
- Lot 9 : mises à jour signées, paquets (AppImage, rpm, NSIS, exe portable) et intégration continue.
