# Lot 11 : double sens et suppressions répercutées (09/11/2026)

Fin de l'étape 3. Un dossier et un album tenus identiques.

## Interface

- Troisième choix dans « Ajouter un dossier » : « Garder un dossier et un album identiques », avec un
  album existant ou un nouvel album au nom du dossier, et l'interrupteur « Répercuter les
  suppressions » (éteint par défaut, son effet expliqué sur place).
- Sur la page Synchro : « Identique à l'album « … » », envoyés et reçus, et un lien qui bascule
  « Suppressions répercutées / non répercutées ». Quand des suppressions dépassent le seuil, un
  bandeau « N suppressions attendent votre accord » ouvre la liste (chaque fichier et ce qui lui
  arrivera) avec « Appliquer » ou « Annuler ces suppressions » (les fichiers reviennent : ce qui a
  été effacé ici est reçu de nouveau, ce qui a quitté l'album y retourne).

## Moteur

- Un passage en deux temps : envoi du dossier dans l'album (comme une règle d'envoi), puis
  réception de l'album dans le dossier ; les fichiers envoyés ou reconnus sont liés à leur photo de
  l'album, et un fichier reçu n'est jamais renvoyé (même empreinte, reconnu au pré-vol).
- Sans miroir : une suppression reste de son côté (effacé ici : ni retéléchargé ni retiré de
  l'album ; retiré de l'album : gardé ici).
- Avec miroir (schéma 3 de la base : table des suppressions en attente) :
  - effacé ici : la photo quitte l'album, et reste dans la photothèque ;
  - retiré de l'album : le fichier local va dans la corbeille du système (crate `trash`, corbeille
    freedesktop sous Linux, corbeille de Windows), jamais effacé ;
  - délai de grâce de 10 minutes entre la première observation et l'action ; une suppression qui
    n'est plus observée (fichier revenu) est oubliée ;
  - au-delà de 20 fichiers ou de 5 % du dossier : rien sans l'accord de l'utilisateur ;
  - fichier témoin `.nicephore-dossier` posé à l'ajout (et quand le miroir est allumé) : s'il
    manque (disque débranché, dossier remplacé), rien n'est synchronisé et la règle le dit.
- Un fichier modifié d'un côté est gardé en double (nouvelle photo dans l'album) plutôt qu'écrasé.

## Vérifié

- Contre le serveur jetable :
  - sans miroir : photo locale montée dans l'album, photos de l'album reçues, rien de renvoyé au
    passage suivant, suppressions restées de leur côté ;
  - avec miroir et 42 photos : pas d'action avant le délai de grâce, photo effacée ici retirée de
    l'album mais toujours dans la photothèque, photo retirée de l'album mise dans la corbeille
    (une corbeille privée au test), 25 suppressions d'un coup en attente de confirmation puis
    appliquées une fois confirmées, plus rien sans le fichier témoin.
- Dans l'appli (51/51) : dossier vide relié à un album, ses 2 photos reçues, témoin posé, carte
  affichée, miroir allumé depuis la page, retrait. Portable, comptes et lecture seule sans
  régression ; tous les tests du moteur (11) et de l'API (5) verts.
