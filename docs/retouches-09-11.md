# Retouches du 09/11 (retours de Valentin)

- Plus de sélection « façon page web » : en faisant défiler ou en cliquant, les images et les
  textes ne se surlignaient plus en bleu comme dans un navigateur. Sélection désactivée dans toute
  l'appli, sauf dans les champs de saisie ; plus d'image fantôme au glisser.
- Fermer la fenêtre la range dans la zone de notification dès que l'option est active (elle l'est
  par défaut), même sans dossier en cours d'envoi : avant, sans dossier synchronisé, fermer
  quittait l'appli. On quitte depuis le menu de l'icône. Lancé avec la session, Nicéphore démarre
  rangé dans la zone de notification.
- Vérifié dans l'appli (59/59) : sélection désactivée sur la page, fenêtre fermée sans aucun dossier
  rangée dans la zone de notification puis rouverte ; visionneuse, portable, comptes et lecture
  seule sans régression.

## Vidéos de l'ordinateur dans la visionneuse

- Une vidéo ouverte depuis le gestionnaire de fichiers (Nicéphore étant la visionneuse par défaut)
  restait noire : WebKitGTK ne lit pas les vidéos servies par le protocole interne de Tauri. Elles
  passent désormais par le relais local déjà utilisé pour les vidéos du serveur (127.0.0.1, jeton
  aléatoire), avec la lecture par plages (`Range`) qu'exige le lecteur ; seuls les fichiers ouverts
  dans la visionneuse y sont enregistrés, par numéro, jamais par chemin.
- Toutes les vidéos sont essayées (MP4, MOV, WebM, MKV, AVI…) ; si le système ne sait pas en
  décoder une, un message clair remplace l'écran noir.
- La vidéo de Valentin (`essai.mp4`) est en H.264 4K, que ses décodeurs GStreamer savent lire :
  c'était bien le protocole, pas le format.
- Vérifié : plages (tests unitaires), banc `--fichier` 14/14 avec une vidéo H.264 960 × 720 lue et
  la fenêtre à ses proportions ; vidéos du serveur toujours lues.

## Icône de la zone de notification sous Plasma

- Rangée dans la fenêtre contextuelle de la zone de notification (icônes masquées), l'icône de
  Nicéphore disparaissait et l'entrée s'appelait « nicephore ». L'icône de Tauri passe par
  libappindicator, qui transmet à Plasma un chemin de fichier temporaire et aucun titre.
- Sous Linux, Nicéphore publie désormais lui-même son icône par le protocole standard
  (StatusNotifierItem, crate `ksni`) : titre « Nicéphore », images de l'icône en 32, 64, 128 et
  256 pixels, info-bulle avec l'état de l'envoi, même menu qu'avant. Le clic gauche ouvre la
  fenêtre. Windows garde l'icône de Tauri.
- Sans hôte de zone de notification (bureau qui n'en a pas), l'icône n'est pas créée et fermer la
  fenêtre quitte, comme avant.
- Le banc lance un hôte de zone minimal sur son bus privé (`examples/hote-zone.rs`) qui relit ce
  qu'un panneau verrait. Vérifié : banc `--jetable` 59/59 et `--fichier` 14/14, avec titre, quatre
  tailles d'images et menu lus par l'hôte.

## Deux sens avec toute la photothèque

- Le double sens ne reliait un dossier qu'à un album. Il peut désormais relier un dossier de
  l'ordinateur à toute la photothèque (ou à ce qui a été pris depuis une date) : le dossier part
  sur le serveur comme un envoi (photothèque seule ou un album par sous-dossier), et les photos
  du serveur absentes du dossier descendent dans un sous-dossier « Depuis Immich » (ou directement
  dans le dossier), par année et mois.
- Pas de doublon : une photo est reconnue à son contenu (SHA-1), où qu'elle soit dans le dossier
  et quel que soit son nom. Elle n'est ni renvoyée ni retéléchargée. Les photos reçues sont notées
  comme connues : elles ne sont ni ré-analysées au passage suivant, ni rangées dans des albums.
- Rien n'est supprimé ni écrasé : pas de suppressions répercutées avec la photothèque (refusé par
  le moteur). Une photo reçue effacée ou déplacée ici n'est pas retéléchargée ; « recevoir de
  nouveau » ne reprend que les échecs pour un double sens.
- Tant qu'un fichier du dossier est en cours d'écriture, la réception attend le passage suivant :
  sinon sa copie du serveur pourrait descendre en double.
- Avant d'ajouter : nombre et volume du dossier, du serveur, et place nécessaire (au moins ce que
  le dossier n'a pas déjà) ; refus si le disque ne suffit pas.
- Schéma de la base : version 5 (colonne `dossier_recus`), sauvegarde automatique avant migration.
- Vérifié : test d'intégration `double_sens_avec_la_phototheque` (copie renommée au fond d'un
  sous-dossier non retéléchargée, nouvelle photo envoyée seule, pas d'album pour les reçues, fichier
  déplacé ni retéléchargé ni renvoyé), suite jetable du moteur 13/13, banc `--jetable` 62/62.

## Arrêt du 9 novembre à 19 h 07 et veille de l'interface

- Nicéphore s'est fermé pendant la grande synchro : KWin a coupé sa connexion (« Data too big for
  buffer… error in client communication »), le fil principal ne lisant plus les messages de
  l'affichage. La cause du blocage n'est pas encore établie.
- Nicéphore surveille désormais son fil principal (`veille_interface.rs`) : au-delà de 3 s sans
  réponse, « interface figée » dans le journal, puis la durée du gel quand il repart. Le banc
  `--fichier` provoque un gel de 5 s et vérifie les deux messages.

## Visionneuse : RAW sans message transitoire

- En passant d'une photo à un RAW, « Aperçu indisponible pour ce format » s'affichait environ une
  seconde avant l'aperçu. La photo précédente reste maintenant affichée jusqu'à ce que la suivante
  soit prête (aperçu du RAW ou vidéo compris).
- L'aperçu déjà extrait ne relit plus tout le RAW (25 Mo et plus) : taille lue dans le petit JPEG
  en cache, orientation dans le premier Mo du RAW. Les aperçus des fichiers précédent et suivant
  sont préparés à l'avance.
- Vérifié : le banc `--fichier` guette le message pendant la navigation ; il échoue sur l'ancienne
  visionneuse et passe (15/15) sur la nouvelle.
