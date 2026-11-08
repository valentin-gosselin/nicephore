# Lot 5 : édition des infos (08/11/2026)

## Dans la visionneuse (panneau Infos, touche I)

- Description : un clic sur le texte (ou « Ajouter une description ») l'ouvre en saisie, Entrée
  enregistre, Échap annule.
- Note : cinq étoiles ; un clic sur la note déjà donnée la retire.
- Date : boîte de dialogue avec date, heure et fuseau (fuseau d'origine de la photo par défaut,
  sinon celui du système). Envoyée en heure locale avec son décalage, comme la webapp.
- Lieu : petite carte MapLibre, un clic pose l'épingle, coordonnées affichées ; le lieu (ville,
  pays) est calculé ensuite par le serveur, d'où une seconde lecture des infos 1,5 s plus tard.
- Tags : pastilles avec une croix pour retirer, bouton « Tag » qui propose les tags existants
  (saisie libre acceptée : le tag est créé par `PUT /tags`, sans doublon).
- Le cœur de la visionneuse suit maintenant l'état réel (il ne se rafraîchissait pas après un clic).

## Sur une sélection (barre du haut)

- « Modifier la date », « Modifier le lieu », « Ajouter un tag » pour tous les éléments
  sélectionnés, en un appel (`PUT /assets` en lot, `PUT /tags/{id}/assets`).

## Chronologie après un changement de date

- Une photo redatée change de mois, parfois vers un mois qui n'existait pas dans la chronologie.
  La liste des mois est relue (les mois connus gardent leur état, les mois chargés sont rechargés,
  les nouveaux apparaissent), et seulement à la fermeture de la visionneuse pour que la position
  affichée reste valable. Les chargements en cours sont maintenant suivis par mois et non par
  indice, ce qui rend cette relecture sûre.

## API

- `ModificationAssets` gagne date, fuseau, latitude, longitude, note (avec la nuance « absente »
  contre « nulle » pour retirer une note) et description.
- `tags()`, `tag_nomme(nom)`, `etiqueter(tag, ids, retirer)`.

## Vérifié

- Tests d'écriture sur le Immich jetable : 5/5 (dont `edition_des_infos_et_tags`).
- Scénario dans l'appli sur le serveur jetable : 22/22, dont description, 4 étoiles puis retrait,
  tag ajouté puis retiré, date passée au 15 juin 2020 14:30, favori depuis la visionneuse, photo
  redatée toujours comptée dans la chronologie, tag sur une sélection de deux éléments.
- Lieu : vérifié par les tests d'API (latitude et longitude relues) ; le scénario ne clique pas
  encore dans la petite carte.
- Banc en lecture seule sur la vraie bibliothèque : rien de cassé, défilement à 60 i/s.
