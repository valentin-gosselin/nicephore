# Lot 15 : Nicéphore comme visionneuse de l'ordinateur (09/11/2026)

Retour de Valentin : ouvrir une image avec Nicéphore ouvrait l'appli, pas l'image ; il voulait le
nuage de l'appli mobile et un bouton pour envoyer tout de suite.

- Ouvrir une photo ou une vidéo de l'ordinateur avec Nicéphore (gestionnaire de fichiers,
  « Ouvrir avec ») l'affiche en plein écran, même sans connexion. Flèches (et ← →) pour les autres
  photos et vidéos du même dossier, position « 2 / 42 », taille.
- Nuage dans la barre : « Sur Immich » (nuage coché, reconnue par son empreinte SHA-1), « Dans la
  corbeille d'Immich », « Pas sur Immich » (nuage barré) avec « Envoyer maintenant », ou « Non
  connecté ».
- Si Nicéphore tourne déjà, la photo s'ouvre dans sa fenêtre (une seule instance,
  `tauri-plugin-single-instance`).
- Seul le dossier de la photo ouverte devient lisible par l'interface (protocole `asset` de Tauri,
  autorisation par dossier, sans les sous-dossiers). Formats affichés : JPEG, PNG, GIF, WebP, AVIF,
  BMP, MP4, WebM, MOV ; les autres (RAW, HEIC) montrent un message et peuvent quand même être
  envoyés.
- Visionneuse par défaut : types déclarés dans les paquets (rpm, AppImage, installeur Windows) et
  dans le lanceur de développement (`%F`, `MimeType`) ; Réglages, « Faire de Nicéphore la
  visionneuse par défaut » (`xdg-mime default` sous Linux, sur demande seulement ; sous Windows la
  page des applications par défaut s'ouvre, le système ne permet pas plus).
- Corrigé en route : `scripts/lancer.sh` transmettait le nom du mode de rendu au lieu des fichiers.
- Vérifié : `scripts/banc.sh --fichier` (7/7), photo passée en argument comme par Dolphin et
  affichée, « Pas sur Immich », envoi, « Sur Immich », voisine déjà sur Immich, seconde ouverture
  reçue par la fenêtre déjà lancée. Non-régression complète verte.

## Option RAW (même soirée)

- Chaque dossier synchronisé (envoi, réception, deux sens) et chaque dossier virtuel peut laisser
  de côté les fichiers RAW (DNG, ARW, CR2, CR3, NEF, ORF, RAF, RW2…, reconnus par l'extension).
  Case « Inclure les fichiers RAW » dans la boîte d'ajout, cochée par défaut ; mention cliquable
  « Fichiers RAW inclus / laissés de côté » sur la carte d'un dossier. Base au schéma 4.
- L'option passe dès la création du dossier, pour qu'aucun premier passage n'envoie les RAW avant
  qu'elle s'applique.
- Garde-fou : en double sens avec suppressions répercutées, laisser les RAW de côté ne les fait pas
  passer pour « retirés de l'album » : ils ne sont ni mis à la corbeille ni renvoyés, juste ignorés.
- Vérifié contre le serveur jetable (JPEG et RAW jumeaux, RAW exclu après coup : rien de touché ;
  nouveau RAW non envoyé, nouveau JPEG envoyé) et dans l'appli (57/57).
- Test de pause précisé : un envoi coupé par la pause alors que le serveur avait déjà tout reçu
  revient « déjà sur le serveur » à la reprise (même empreinte), sans doublon ; le test compte
  désormais envoyés et déjà présents.
