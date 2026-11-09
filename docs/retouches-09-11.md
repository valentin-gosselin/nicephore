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
