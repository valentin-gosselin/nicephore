# Retouches du 23 novembre : envoi par glisser-déposer

Demande de Valentin : envoyer une série de photos d'un dossier en la glissant dans Nicéphore,
comme dans Immich web. Ses choix : un dossier déposé demande son album ; un bouton « Envoyer »
dans la barre du haut, comme Immich.

## Ce qui marche

- Déposer des fichiers ou des dossiers n'importe où dans la fenêtre (hors visionneuse locale) :
  la fenêtre s'assombrit sous le glisser (« Déposez les fichiers n'importe où pour envoyer »,
  texte d'Immich, donc traduit partout).
- Sur la page d'un album : tout va dans cet album, sans question.
- Ailleurs, un dossier déposé ouvre le choix d'album : « Dans la bibliothèque seulement, sans
  album », un album existant, ou un nouvel album dont le nom du dossier est déjà rempli. Des
  fichiers seuls partent sans question, dans la bibliothèque.
- Bouton « Envoyer » de la barre (icône seule sur une fenêtre étroite) : « Des photos et
  vidéos… » ou « Un dossier… », puis la même suite que le dépôt.
- Panneau en bas à droite : progression (en octets), Envoyé, Doublons, Erreurs, album visé,
  fichier en cours, « Annuler » (les photos déjà parties restent), erreurs dépliables. Les
  dépôts faits pendant un envoi s'ajoutent à la file.
- Les vues (chronologie, album) se rechargent à la fin de chaque lot.

## Côté moteur (`envoi_manuel.rs`)

- Dossiers parcourus en entier (fichiers cachés et temporaires écartés), formats refusés par le
  serveur laissés de côté (signalé pour les fichiers déposés seuls).
- Empreintes SHA-1 par lots de 200, cherchées sur le serveur avec l'empreinte de la première
  version pour une photo pivotée par Nicéphore : rien n'est renvoyé en double. Les doublons
  vont quand même dans l'album demandé, comme dans Immich web.
- Trois envois à la fois, lus en flux depuis le disque.
- `envoyer_fichier_local` (visionneuse) partage maintenant `meta_envoi` avec ce module.

## Banc

`banc.sh --jetable` : 13 vérifications de plus (96 en tout). Le dépôt y est l'événement que le
système envoie à la fenêtre ; le vrai glisser depuis Dolphin n'est pas testable au banc.
Textes : 23 nouveaux, l'anglais repris d'Immich quand il existe, traduits dans les 26 langues.

# Partage et boutons de l'album, comme Immich web

Demande de Valentin : les boutons d'Immich web sur l'album, surtout le partage par lien, les
mêmes dans la photothèque, et une page « Liens partagés ».

- Page album : « Ajouter des photos » (la photothèque en mode choix, « Ajouter » en haut),
  « Partager », « Télécharger » (tout l'album dans un dossier à son nom), « Diaporama », et le
  menu ⋮ : envoyer des photos ou un dossier de l'ordinateur dans l'album, récents ou anciens en
  premier (ordre de l'album, suivi par la chronologie), supprimer l'album.
- « Partager » d'un album : les personnes (propriétaire, membres en visionneur ou éditeur,
  inviter des comptes du serveur, retirer) et ses liens publics.
- Lien public : description, adresse personnalisée, mot de passe, expiration (de 30 minutes à
  un an, ou jamais), téléchargement, envoi par les visiteurs (non par défaut, alors qu'Immich
  l'ouvre par défaut), métadonnées. Créé, il est copié dans le presse-papiers. L'adresse suit le
  domaine externe réglé dans Immich, sinon l'adresse du serveur dans Nicéphore.
- Sélection (photothèque, favoris, album…) : « Partager » crée un lien vers les photos
  choisies ; dans un album, « Définir comme couverture d'album » pour une photo seule.
- Page « Liens partagés » dans la barre latérale : tous les liens, copier, modifier, supprimer
  (confirmé sur la ligne même). Immich ne donne qu'une photo par lien dans sa liste : les liens
  de photos sont complétés un par un pour en afficher le nombre.
- Immich v3 range le propriétaire parmi les membres (rôle `owner`), et ne renvoie pas l'album à
  la création d'un lien : les deux sont pris en compte.

Banc : test d'écriture de l'API (liens, couverture, ordre, membres) et 18 vérifications de plus
dans `banc.sh --jetable` (114 en tout). 70 textes nouveaux, l'anglais repris d'Immich quand il
existe, traduits dans les 26 langues.

À part : le test d'API `edition_des_infos_et_tags` échoue une fois sur trois (il relit un tag
aussitôt retiré), sans rapport avec ces changements.
