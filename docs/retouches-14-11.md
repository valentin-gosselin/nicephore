# Retouches du 14 novembre : la visionneuse des fichiers de l'ordinateur

Retours de Valentin après essai de la visionneuse locale (Nicéphore ouvert depuis le
gestionnaire de fichiers) : il manquait le zoom, le clic droit pour copier et le plein écran,
et une fenêtre revenait parfois toute seule. Il a choisi la liste complète des ajouts.

## Fenêtre qui revenait seule

Une photo ouverte depuis le gestionnaire de fichiers arme un filet de sécurité : si la
visionneuse n'a pas montré la fenêtre au bout de 5 s (fichier lent), elle se montre quand même.
Ce minuteur n'était jamais annulé : une fenêtre fermée dans les 5 s après l'ouverture revenait
avec la même photo. Le filet (`filet-fenetre.ts`) est maintenant désarmé dès que la visionneuse
montre la fenêtre elle-même. Vérifié au banc : fenêtre fermée tout de suite, toujours fermée
6,5 s plus tard, puis rouverte normalement par la photo suivante.

## Visionneuse locale

- Zoom : molette autour du pointeur, double clic entre « ajusté » et 100 %, touches `+`, `-`,
  `0` (ajusté) et `1` (100 %), glisser pour se déplacer, photo toujours bord à bord de la zone.
  Le pourcentage s'affiche un instant à chaque changement. L'image est redimensionnée et non
  agrandie par une transformation : elle reste nette à tous les niveaux (`zoom.svelte.ts`).
- Rotation (`R`, `Maj+R`) affichée tout de suite, puis enregistrée dans le fichier après une
  courte pause (plusieurs tours d'affilée : une seule écriture), et avant toute autre photo. Voir
  plus bas « Rotation enregistrée ».
- Plein écran de la fenêtre elle-même (`F`, `F11`, Échap pour sortir) : barre et bande passent
  par-dessus la photo et s'effacent, curseur compris, après 2,5 s sans mouvement. Le plein écran
  de la page web refuse une touche ou un clic simulés : c'est celui de la fenêtre qui est utilisé.
- Diaporama (`D`) : 5 s par photo, les vidéos jusqu'à leur fin, barre de progression.
- Panneau d'infos (`I`) lu dans les EXIF (crate `kamadak-exif`, l'aperçu JPEG pour les RAW
  illisibles) : date de prise de vue, lieu (par le serveur Immich, `/map/reverse-geocode`,
  sinon les coordonnées), fichier, dimensions, dossier, appareil, objectif et réglages.
- Bande des photos du dossier, avec un bouton qui la montre ou la masque ; le choix est retenu.
  Vignettes de 240 px faites à la demande pour les seules tuiles visibles, deux à la fois, en
  cache (`vignettes-locales`).
- Clic droit (menu de `@immich/ui`, ouvert sous le pointeur) et bouton « Plus d'actions » :
  copier l'image (`Ctrl+C`), copier le chemin (`Ctrl+Maj+C`), afficher dans le dossier
  (`org.freedesktop.FileManager1.ShowItems`, Explorateur sous Windows), ouvrir avec (sélecteur
  d'applications du portail XDG, `OpenAs_RunDLL` sous Windows), pivoter, plein écran, diaporama,
  infos, envoyer, envoyer dans un album, ouvrir dans la bibliothèque, corbeille.
- Corbeille du système (`Suppr`, crate `trash`), photo suivante, et « Annuler » qui remet le
  fichier en place. Dossier vidé : un message, pas de fenêtre vide.
- Glisser la photo vers une autre application (plugin `tauri-plugin-drag`), quand elle n'est
  pas zoomée.
- Immich : une seule icône, son sens en infobulle. Nuage coché vert : déjà sur Immich, un clic
  l'ouvre dans la bibliothèque (nouvelle page `/photo`, Échap revient à la photo locale). Nuage
  orange : pas encore sur Immich, un clic l'envoie. Envoi dans un album existant ou nouveau par
  le menu.
- La fenêtre suit la photo comme avant, et aussi le panneau d'infos (elle s'élargit), la bande
  et la rotation.

## Rotation enregistrée (second essai de Valentin)

Une rotation n'était qu'affichée : fermée puis rouverte, la photo revenait droite, et les autres
visionneuses ne la voyaient pas. Elle est maintenant écrite dans le fichier :

- JPEG : seule l'étiquette d'orientation EXIF change (`orientation.rs`), sans toucher aux pixels.
  Réécrite en place quand elle existe, ajoutée sinon (nouvel IFD0, ou segment EXIF neuf). Testé
  contre une autre lecture EXIF (`kamadak-exif`).
- PNG, TIFF, BMP : image réencodée, sans perte. WebP : réencodé en WebP sans perte, donc souvent
  plus lourd qu'un WebP compressé d'origine. Leurs métadonnées ne sont pas reprises.
- RAW et GIF (animations) : pas modifiés, la rotation reste à l'affichage (et sur Immich).
- Écriture à côté puis renommage par-dessus : jamais une photo à moitié écrite. Droits gardés,
  date de modification gardée à deux secondes près (pour que la synchro voie le changement).

Photo déjà sur Immich : pivotée aussi par l'éditeur du serveur (`PUT /assets/{id}/edits`, en bêta
depuis Immich 2.5), qui garde l'original tel qu'envoyé et refait vignettes et aperçus. Nicéphore
demande désormais ses images avec `edited=true` : une photo pivotée ou recadrée dans Immich s'y
montre comme dans Immich. Le thumbhash change avec la rotation, le cache se renouvelle seul.

Le fichier pivoté change d'empreinte. Pour qu'il reste la même photo, la base d'envoi (schéma 6,
table `liens`) retient l'empreinte de sa première version. Le moteur de synchro et l'état de la
visionneuse cherchent les deux empreintes sur le serveur : photo toujours reconnue (nuage vert),
jamais renvoyée en double. Test du moteur sur le serveur jetable :
`une_photo_pivotee_n_est_pas_renvoyee`.

## Écran noir à la réouverture

La même photo rouverte depuis le gestionnaire de fichiers (fenêtre fermée entre-temps) restait
noire : la visionneuse remettait sa taille à zéro sans la recharger, son adresse n'ayant pas
changé, et la fenêtre attendait le filet de 5 s. Chaque ouverture recharge maintenant la photo.

## Visionneuse de la bibliothèque

Clic droit : copier l'image (`Ctrl+C`, rendu pleine résolution ou aperçu), enregistrer
l'original dans un dossier, favori, ajouter à un album, diaporama, plein écran, infos,
corbeille. Échap ferme d'abord le menu ouvert, jamais la visionneuse avec.

## Banc

`banc.sh --fichier` passe de 19 à 59 vérifications, plus une relue sur le serveur (la rotation faite par l'éditeur d'Immich), avec son propre dossier de données : la
corbeille qu'il remplit et vide est la sienne, jamais celle de la session. Non testés au banc,
car ils ouvriraient des fenêtres sur le vrai bureau : « Afficher dans le dossier », « Ouvrir
avec » et le glisser vers une autre application.

Textes : 35 nouveaux, en anglais et traduits automatiquement dans les 26 langues.
