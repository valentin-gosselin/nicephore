# Lot 3 : albums et gestion des personnes (08/11/2026)

## Albums

- Page Albums (barre latérale) : les albums de l'utilisateur puis ceux partagés avec lui, couverture,
  nombre d'éléments, période, mention « partagé » ; filtre par nom ; tri par modification récente,
  date des photos, nom de A à Z ou de Z à A (retenu) ; bouton « Nouvel album ».
- Page d'un album : sa chronologie (`albumId` de `/timeline/buckets`), supprimer (confirmation :
  l'album seul disparaît, les photos restent). Comme dans Immich, un clic sur le titre ou sur la
  description les rend modifiables (Entrée ou sortie du champ enregistre, Échap annule) ; d'abord un
  crayon discret, que Valentin n'a pas trouvé. Même chose pour le nom d'une personne.
- Barre de sélection, partout : « Ajouter à un album » ouvre un choix des albums (les plus récemment
  modifiés d'abord, filtrables) ou crée un album au nom tapé ; la notification indique les doublons et
  propose d'ouvrir l'album. Dans un album, « Retirer de l'album » (les photos restent).

## Personnes

- Page Personnes (barre latérale) : toutes les personnes, recherche par nom, tri (nombre de photos,
  A à Z, Z à A), visages sans nom et personnes masquées sur demande, cœur pour les favorites.
  Explorer n'en montre plus que les 16 premières, avec un lien vers la page.
- Page d'une personne : nommer ou renommer, favori, masquer ou démasquer, fusionner. La fusion
  propose d'abord les visages les plus ressemblants (`/people?closestPersonId=`), permet de chercher
  un nom, puis demande une confirmation explicite (Immich ne sait pas annuler une fusion). Elle passe
  par `POST /people/{cible}/merge` : la fiche ouverte garde son nom, les autres s'y fondent.

## Immich jetable pour les tests d'écriture

`scripts/immich-jetable.sh demarrer|tester|arreter|detruire` : Immich 3.3.6 en Docker, local
(127.0.0.1:2299), sans apprentissage automatique, données dans `~/.cache/nicephore-immich-jetable`,
compte et 36 photos générées. Les tests (`crates/api_immich/tests/ecriture.rs`) refusent toute
adresse qui n'est pas locale.

Couvert : favori, archive, corbeille et restauration ; album créé, complété (doublon compris),
réduit, renommé, supprimé sans perte de photos ; personne nommée, favorite, masquée, fusionnée ;
original et vignette. Deux bugs trouvés ainsi : la réponse de fusion inclut la fiche cible (comptée
en trop), et Immich répond 400 « Not found or no ... access » pour un élément disparu (traité comme
introuvable).

Limite : sans apprentissage automatique, les personnes de test n'ont pas de visages ; `/people` ne
liste que les personnes ayant assez de visages, d'où des vérifications fiche par fiche.

## Vérifié au banc sur la vraie bibliothèque (lecture seule)

46 albums, page d'un album (55 éléments), choix d'album sur une sélection (46 proposés, refermé sans
rien ajouter), page Personnes (117), fusion proposée pour Alice (39 visages, refermée sans fusionner),
défilement à 60 i/s sans saccade.

## À tester à la main

1. Sélectionner quelques photos, « Ajouter à un album » : un album existant, puis un nouvel album.
2. Dans l'album : retirer une photo, renommer, décrire, puis supprimer un album d'essai.
3. Personnes : nommer un visage sans nom, mettre en favori, masquer puis démasquer.
4. Fusion : rattacher à une personne un ou deux visages sans nom qui sont bien elle.
