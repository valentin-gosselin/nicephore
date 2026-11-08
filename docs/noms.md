# Un nom pour le client de bureau

*Recherche du 8 novembre 2026. Les disponibilités ont été vérifiées ce jour-là (GitHub, crates.io, npm, Flathub, recherche web) ; elles peuvent bouger.*

## Le cahier des charges, en une phrase

Un nom court, beau à dire en français comme en anglais, qui parle à la fois de **voir** (parcourir sa photothèque), de **garder** (la mémoire, les souvenirs de famille) et de **faire passer** d'un lieu à l'autre (la synchronisation dans les deux sens, les fichiers présents en apparence mais rapatriés à la demande). Sans « Immich », sans son logo, et sans ressembler à **Mimick**, le client Linux concurrent déjà publié sur Flathub (`dev.nicx.mimick`).

## Et la racine du mot Immich ?

J'ai cherché pour de vrai : la FAQ officielle (docs.immich.app/FAQ) ne dit rien de l'origine du nom, les tickets GitHub du projet non plus, et aucune interview trouvée n'explique le choix d'Alex Tran. Le seul « Immich » attesté ailleurs est une variante de nom de famille allemand (Imme), sans lien établi avec l'appli. Je n'ai donc rien bâti sur une étymologie qui serait inventée. Le clin d'œil, quand il existe, passe par le son : un seul finaliste commence par « Imm », et c'est volontaire.

## Les territoires explorés

1. **La mémoire et le souvenir** : Mnémosyne et sa sœur aînée Mnémé, l'anamnèse, la méthode des lieux (les *loci* des orateurs antiques), Proust et Combray, le breton *koun*, la rémanence.
2. **La photographie et ses gestes** : Nadar, Daguerre, Atget, le calotype, le tirage, l'image latente, le sténopé, Barthes et son *punctum*.
3. **L'archive et l'objet qui garde** : l'herbier, le vélin, le folio, l'écrin, le médaillon, le reliquaire, le diptyque, l'ambre, la nacre.
4. **Le passage et la marée** (et la Bretagne, à une heure de Rennes) : l'estran, l'étale, le jusant, le ressac, le mascaret, le marnage, la laisse de mer, l'aber, la navette, le sillage, la lisière, le seuil, Ithaque.
5. **La lumière et l'optique** : le diorama, la lanterne magique, le kinora, le lumen, la diapo, l'opale, la grisaille, l'héliographe.
6. **Les oiseaux voyageurs** : la sterne arctique, le courlis, le martinet, l'hirondelle, le colombier où rentrent les pigeons.
7. **Les mythes de la reconnaissance** : Argos (le chien qui reconnaît Ulysse après vingt ans), Iris la messagère, Janus aux deux visages, l'*imago*.
8. **Mon ajout, la botanique du souvenir** : l'immortelle, fleur qui sèche sans se faner.

## La longue liste (71 candidats)

| Territoire | Candidats | Sort |
|---|---|---|
| Mémoire | Mnémé, Mnémosyne, Anamnèse, Engram, Memento, Loci, **Combray**, Balbec, **Koun**, Eñvor, Rémanence, **Immortelle**, Vestige | Mnémé, Engram, Memento, Loci, Vestige : pris sur crates.io et npm, plusieurs projets « mémoire pour IA ». Mnémosyne : trop long. Eñvor : imprononçable hors Bretagne. Rémanence : joli, mais crate prise et 9 lettres un peu raides. |
| Photo | **Nadar**, Daguerre, Atget, Niépce, Calotype, Albumen, Halide, Sépia, Tirage, Latent, **Punctum**, Studium, **Sténopé**, Obscura, Lucida, Héliographe | Calotype : **deux applis photo de ce nom existent déjà** (appareil RAW iOS, éditeur RAW Android), écarté. Halide et Obscura : applis photo iOS célèbres, écartés. Lucida : police de caractères déposée. Sépia, Latent : pris partout. Atget, Niépce : orthographe piégeuse. |
| Archive et objets | Herbier, Vélin (Vellum), Folio, Écrin, Locket, Reliquaire, Diptyque (Diptych), Loupe, Ambre, Nacre | Diptych : **Diptic**, appli de collage photo connue (iOS, Mac, Windows), trop proche, écarté. Loupe : c'est la visionneuse d'images de GNOME (`org.gnome.Loupe` sur Flathub), écarté. Locket : appli de partage de photos très populaire, écarté. Folio : appli Flathub du même nom. Écrin : nasale intraduisible en anglais. |
| Passage et marée | **Estran**, **Étale**, Jusant, **Ressac**, Mascaret, Marnage, Laisse, Aber, Navette, Estuaire, Sillage, Lisière, Seuil, Ithaque | Jusant : jeu vidéo connu de Don't Nod (2023), écarté. Aber : crate et npm pris, et « mais » en allemand. Navette : crate prise et navigateur Rust homonyme. Marnage : appli de marées macOS homonyme. Laisse : se lit « lace » en anglais et évoque la laisse du chien. |
| Optique | Diorama, Lanterna, Kinora, Lumen, Diapo, Opale, Grisaille | Lanterna : bibliothèque Java très utilisée. Lumen : framework Laravel. Diorama : pris partout. Diapo : chaleureux (la soirée diapo en famille) mais très franco-français. |
| Oiseaux | **Sterne**, Courlis, Martinet, Hirondelle, Colombier, Tern | Tern : pris (analyseur JavaScript célèbre). Courlis : libre mais le lien à l'appli est plus mince que la sterne. Hirondelle et Colombier : longs et durs pour une bouche anglaise. |
| Mythes | Argos, Iris, Janus, Imago | Argos, Iris, Janus : saturés. Imago : clin d'œil « Im » tentant, mais **IMAGO est une grande agence de photos de presse**, écarté. |

## Les dix finalistes

### 1. Estran

**Prononciation** : français [ɛs.tʁɑ̃], « ess-tran » ; anglais « ESS-tran ».

**Origine et sens** : l'estran, c'est la bande de côte que la mer découvre à marée basse et recouvre à marée haute. Une terre qui existe deux fois par jour et disparaît deux fois par jour, sans jamais cesser d'être là. Les côtes bretonnes et la baie du Mont-Saint-Michel ont parmi les plus forts marnages d'Europe : on marche à pied sec là où, six heures plus tard, il y a des mètres d'eau.

**Pourquoi il colle** : c'est exactement le statut d'un fichier virtuel. La photo est sur le disque, on la voit, mais son contenu est tantôt découvert (téléchargé), tantôt recouvert (resté sur le serveur). Et le va-et-vient de la marée, c'est la synchronisation dans les deux sens. Le mot est court, rare, sans concurrent, et ancré à une heure de Rennes.

**Icône** : un carré arrondi coupé par une ligne d'onde ; au-dessus, un sable ocre clair, en dessous, un bleu ardoise. Un petit galet clair à moitié découvert sur la ligne.

**Accroche** : *Estran: your photo library, at high tide and low tide.*

**Disponibilité**
- GitHub : aucun projet de ce nom (les résultats sont « Estranged », un jeu, et « ESTranslate »). Libre.
- crates.io : 404, libre.
- npm : E404, libre.
- Flathub : aucune appli de ce nom.
- Web : aucune appli ni marque logicielle trouvée.
- Réserve : l'anglais y entend un écho de *estranged* (éloigné), léger.

### 2. Étale

**Prononciation** : français [e.tal], « é-tal » ; anglais « ay-TAHL ». S'écrit `etale` dans les dépôts.

**Origine et sens** : l'étale, c'est le moment où la mer ne monte plus et ne descend pas encore. Quelques minutes de calme parfait entre deux courants, où l'eau semble posée. Les marins l'attendent pour passer les chenaux difficiles. Les mathématiciens ont emprunté le mot (la cohomologie étale de Grothendieck), ce qui lui donne une petite aura savante.

**Pourquoi il colle** : c'est le nom de l'état idéal d'une synchronisation, celui où les deux rives sont identiques et où rien ne bouge plus. L'appli travaille pour atteindre l'étale. Cinq lettres, deux syllabes, une sonorité douce.

**Icône** : un signe « égal » fait de deux traits d'eau parfaitement plats, blanc sur bleu profond, avec un petit disque doré posé au-dessus comme un soleil sur l'horizon.

**Accroche** : *Étale: slack water for your photos, everything in its place on both shores.*

**Disponibilité**
- GitHub : Etaler (cadre de mémoire temporelle hiérarchique, 104 étoiles) et des dépôts de mathématiques ; rien en photo ni en synchro.
- crates.io : 404, libre.
- npm : E404, libre.
- Flathub : aucune appli de ce nom.
- Web : aucune appli trouvée.
- Réserve : l'accent sera souvent perdu (`etale`), et le mot est inconnu des anglophones (ce qui peut aussi être une force).

### 3. Immortelle

**Prononciation** : français [i.mɔʁ.tɛl] ; anglais « im-or-TELL ».

**Origine et sens** : l'immortelle (hélichryse) est une petite fleur jaune des dunes et des garrigues qui, une fois cueillie et séchée, garde sa forme et sa couleur pendant des années. Au XIXe siècle, on en tressait des couronnes pour les tombes, justement parce qu'elles ne fanaient pas : la fleur était le souvenir lui-même. C'est une fleur d'herbier idéale, qui traverse les années entre deux pages.

**Pourquoi il colle** : c'est la promesse de l'appli, que rien ne se perde. Et c'est le seul finaliste qui porte le **clin d'œil discret** demandé : il commence par « Imm », comme Immich, sans jamais le citer. Un connaisseur sourira, un juriste n'aura rien à redire.

**Icône** : une tête de fleur ronde vue de dessus, faite de petits pétales concentriques, jaune d'or sur fond graphite. Une forme qui tient très bien en 16 pixels.

**Accroche** : *Immortelle: the flower that never fades, for photos that never get lost.*

**Disponibilité**
- GitHub : quelques dépôts confidentiels (un CMS personnel, une maquette de site de vêtements) ; rien de notable.
- crates.io : 404, libre.
- npm : E404, libre.
- Flathub : aucune appli de ce nom.
- Web : aucune appli. « Immortelle » est une gamme de cosmétiques de L'Occitane, dans une tout autre classe de marque.
- Réserve : dix lettres, la limite haute, et quatre syllabes en français.

### 4. Nadar

**Prononciation** : français [na.daʁ] ; anglais « nah-DAR ».

**Origine et sens** : Nadar, de son vrai nom Félix Tournachon, est le grand portraitiste du Paris du XIXe siècle, celui de Baudelaire, de Sarah Bernhardt, de Victor Hugo. En 1858, il prend depuis un ballon la première photographie aérienne de l'histoire, près de Paris. C'est dans son ancien atelier du boulevard des Capucines que se tient, en 1874, la première exposition des impressionnistes. Jules Verne l'a même glissé dans *De la Terre à la Lune* sous l'anagramme de Michel Ardan.

**Pourquoi il colle** : le premier homme à photographier depuis le ciel donne son nom à une appli qui va chercher les photos dans le nuage (le vôtre). Nadar, c'est aussi le portrait, donc les visages, donc la famille. Deux syllabes, aucune difficulté dans aucune langue.

**Icône** : un ballon rond en silhouette, dont la nacelle est un petit objectif ; rouge brique sur fond crème.

**Accroche** : *Nadar: your whole library, seen from the sky.*

**Disponibilité**
- GitHub : aucun projet photo de ce nom (un client R pour une API « NADA », 4 étoiles).
- crates.io : 404, libre.
- npm : pris, mais par un paquet vide (version 0.1.0, sans README). Sans conséquence pour une appli Tauri, qui n'a pas besoin de publier sur npm.
- Flathub : aucune appli de ce nom.
- Web : aucun logiciel photo trouvé.
- Réserve : c'est le nom d'une personne réelle, morte en 1910 ; aucun problème légal, mais un hommage à assumer.

### 5. Combray

**Prononciation** : français [kɔ̃.bʁɛ] ; anglais « kom-BRAY ».

**Origine et sens** : Combray, c'est le village d'enfance du narrateur de *À la recherche du temps perdu*, celui qui ressurgit tout entier d'une tasse de thé et d'une madeleine. Le vrai village, Illiers, s'appelle Illiers-Combray depuis 1971 : la fiction a fini par rebaptiser la carte. Dans la chambre de Combray, l'enfant regardait aussi une lanterne magique projeter des images sur les murs.

**Pourquoi il colle** : c'est la suite naturelle de Madeleine. La madeleine est le déclencheur, Combray est le monde entier qui revient avec elle, comme une photothèque qu'on rouvre. Et pour un projet porté par un Français, c'est une signature culturelle élégante.

**Icône** : un faisceau de lanterne magique (un petit cône de lumière qui sort d'un boîtier) projetant un rectangle doré sur un mur bleu nuit.

**Accroche** : *Combray: every photo is a madeleine; Combray keeps them all within reach.*

**Disponibilité**
- GitHub : un analyseur syntaxique (2 étoiles), un site de galerie (0 étoile), un outil de recherche qualitative qui cite les madeleines (0 étoile). Rien de notable.
- crates.io : 404, libre.
- npm : E404, libre.
- Flathub : aucune appli de ce nom.
- Web : aucun logiciel de ce nom.
- Réserve : parle surtout aux lecteurs de Proust ; un utilisateur coréen ou brésilien n'y verra qu'un joli mot.

### 6. Koun

**Prononciation** : breton [kũːn], « koun » avec une nasale légère ; anglais « koon ».

**Origine et sens** : en breton, *koun* est le souvenir, défini par le Wiktionnaire breton comme « un sentiment tenu par la mémoire ». *Kaout koun* veut dire se souvenir, *dont koun* revenir à l'esprit, et *e koun* est la formule des dédicaces et des stèles : « en mémoire de ».

**Pourquoi il colle** : un mot de quatre lettres, venu de la langue du pays de Valentin, qui dit tout simplement « souvenir ». Il est neutre et tendre à la fois, ne ressemble à aucun mot technique, se tape en une seconde et sonne bien dans toutes les langues.

**Icône** : un nœud simple, comme le nœud qu'on fait à son mouchoir pour ne pas oublier ; un seul trait bleu ardoise sur fond lin.

**Accroche** : *Koun: Breton for "memory", kept on both sides.*

**Disponibilité**
- GitHub : aucun projet de ce nom (les résultats sont des homonymes partiels : kouncil, kount).
- crates.io : 404, libre.
- npm : E404, libre.
- Flathub : aucune appli de ce nom.
- Web : aucune appli trouvée.
- Réserve : quatre lettres, donc moins « trouvable » au premier coup dans un moteur de recherche (« koun photos » réglera ça).

### 7. Punctum

**Prononciation** : français [pɔ̃k.tɔm] ; anglais « PUNK-tum ».

**Origine et sens** : dans *La Chambre claire* (1980), son dernier livre, Roland Barthes distingue dans une photo le *studium*, ce qui nous intéresse poliment, et le *punctum*, le détail qui nous point, nous blesse, nous touche sans qu'on sache pourquoi. Le livre tout entier tourne autour d'une photo de sa mère enfant, dans un jardin d'hiver. En latin, *punctum* est la piqûre, le petit point.

**Pourquoi il colle** : c'est le mot le plus profond de la liste sur ce qu'est une photo de famille. Une photothèque de 80 000 images ne vaut que pour quelques *punctums*, et l'appli est là pour qu'on les retrouve. Le latin le rend international d'emblée.

**Icône** : un seul point lumineux, légèrement décentré, dans un carré sombre. Rouge vif sur noir graphite.

**Accroche** : *Punctum: for the detail that pierces you, wherever it is stored.*

**Disponibilité**
- GitHub : une police de caractères en matrice de points (52 étoiles), un moteur de rendu de nuages de points (7 étoiles). Rien en photo.
- crates.io : **pris**, par un petit cadre de shaders (version 0.1.1, 22 téléchargements). Sans conséquence pour l'appli (le paquet Rust interne peut s'appeler autrement), mais à savoir.
- npm : E404, libre.
- Flathub : aucune appli de ce nom.
- Web : le concept de Barthes, une exposition de 2014 à Salzbourg et le site d'un photographe ; aucun logiciel.
- Réserve : le mot est connu des photographes, donc il a pu être utilisé ailleurs pour des galeries ou des studios.

### 8. Sterne

**Prononciation** : français [stɛʁn] ; anglais « stern ».

**Origine et sens** : la sterne arctique fait la plus longue migration du monde animal, de l'Arctique à l'Antarctique et retour, chaque année. Elle vit ainsi deux étés par an, et on dit souvent qu'aucun autre animal ne voit autant la lumière du jour. Petite, blanche, la queue en ciseaux, elle ne se perd jamais. Bonus pour les lettrés : Laurence Sterne, l'auteur de *Tristram Shandy*, le roman de la mémoire qui digresse.

**Pourquoi il colle** : un aller-retour entre deux pôles, chaque année, sans jamais perdre le chemin de la maison : c'est la synchronisation bidirectionnelle, et c'est aussi « la lumière » de la photo. Six lettres, une syllabe.

**Icône** : une silhouette de sterne en vol, ailes en chevron et queue fourchue, blanche sur bleu profond. La forme évoque discrètement deux flèches.

**Accroche** : *Sterne: like the Arctic tern, your photos fly both ways and always find their way home.*

**Disponibilité**
- GitHub : rien en photo (des outils personnels, un moteur de blog).
- crates.io : 404, libre.
- npm : E404, libre.
- Flathub : aucune appli de ce nom.
- Web : **Sterna Mail**, client mail Android sur F-Droid, nom voisin (Sterna, le genre latin) mais domaine différent.
- Réserve : en anglais, *stern* veut dire sévère, et la poupe d'un bateau ; ça se raconte, mais il faudra le raconter.

### 9. Sténopé

**Prononciation** : français [ste.nɔ.pe] ; anglais « STEN-oh-pay ». S'écrit `stenope` dans les dépôts.

**Origine et sens** : le sténopé est l'appareil photo le plus simple qui soit : une boîte noire percée d'un trou d'épingle. Le mot vient du grec *stenos* (étroit) et *opê* (ouverture). Par ce trou minuscule passe pourtant une image entière, renversée, nette, du monde du dehors. Beaucoup d'ateliers photo pour enfants commencent par là.

**Pourquoi il colle** : une toute petite ouverture qui laisse entrer tout un paysage, c'est une fenêtre de bureau sur une photothèque de plusieurs téraoctets restée sur le serveur. Et c'est l'origine même de la photographie, donc un hommage au geste.

**Icône** : un carré noir percé d'un point blanc, d'où part un petit cône de lumière qui s'ouvre sur un rectangle clair.

**Accroche** : *Sténopé: a pinhole onto your entire library.*

**Disponibilité**
- GitHub : **Stenope**, générateur de sites statiques pour Symfony (140 étoiles), homonyme notable dans le monde PHP, mais hors du domaine photo.
- crates.io : 404, libre.
- npm : E404, libre.
- Flathub : aucune appli de ce nom.
- Web : une ancienne appli iPhone « Assistant Sténopé » (2011), nom différent.
- Réserve : deux accents, et les anglophones disent *pinhole*.

### 10. Ressac

**Prononciation** : français [ʁə.sak] ; anglais « ruh-SAHK ».

**Origine et sens** : le ressac est le retour de la vague sur elle-même après qu'elle a frappé la côte. L'eau arrive, se brise, repart, et la vague suivante la croise. C'est un mouvement sans fin et sans direction unique, qu'on entend autant qu'on le voit.

**Pourquoi il colle** : c'est l'image la plus physique de la synchronisation dans les deux sens : ce qui part du PC revient du serveur, ce qui arrive du téléphone repart vers le disque. Deux syllabes, un son net qui claque, un mot que tout le littoral connaît.

**Icône** : deux vagues stylisées qui s'enroulent l'une vers l'autre et forment ensemble le symbole universel de la synchronisation ; bleu ardoise et écume blanche.

**Accroche** : *Ressac: every wave goes back to the sea, every photo finds its way home.*

**Disponibilité**
- GitHub : RESSAC_Use_Case (AdaCore, projet d'aéronautique, 7 étoiles) et un lecteur RSS personnel (0 étoile).
- crates.io : 404, libre.
- npm : E404, libre.
- Flathub : aucune appli de ce nom.
- Web : aucun logiciel grand public ; seulement le projet RESSAC de l'IRT Saint-Exupéry (aéronautique).
- Réserve : le double « s » et le « r » français ne sont pas évidents pour un anglophone.

## Tableau récapitulatif des vérifications

| Nom | Lettres | GitHub (projet notable du domaine) | crates.io | npm | Flathub | Conflit connu |
|---|---|---|---|---|---|---|
| Estran | 6 | aucun | libre | libre | aucun | aucun |
| Étale | 5 | aucun | libre | libre | aucun | aucun |
| Immortelle | 10 | aucun | libre | libre | aucun | L'Occitane (cosmétiques) |
| Nadar | 5 | aucun | libre | paquet vide | aucun | aucun |
| Combray | 7 | aucun | libre | libre | aucun | aucun |
| Koun | 4 | aucun | libre | libre | aucun | aucun |
| Punctum | 7 | aucun | **pris** (petite crate) | libre | aucun | aucun |
| Sterne | 6 | aucun | libre | libre | aucun | Sterna Mail (mail Android) |
| Sténopé | 7 | Stenope (PHP, hors domaine) | libre | libre | aucun | aucun |
| Ressac | 6 | aucun | libre | libre | aucun | aucun |

Méthode : `gh search repos <nom> --limit 10`, l'API crates.io (`/api/v1/crates/<nom>`, 404 = libre), `npm view <nom> name` (E404 = libre), l'API de recherche Flathub (`/api/v2/search`, qui renvoie des résultats approchants : aucun finaliste n'y a d'appli homonyme), puis une recherche web par nom.

## Mon trio de cœur

1. **Estran**. C'est le seul nom qui décrit littéralement ce que fait l'appli de plus original : une photothèque qui est toujours là, tantôt découverte, tantôt recouverte, au rythme d'un va-et-vient. Il est libre partout, rare, court, et il vient de chez toi.
2. **Nadar**. Le plus photographique et le plus facile à porter à l'international : deux syllabes que tout le monde prononce, et une histoire (la première photo prise depuis le ciel) qui se raconte en une ligne dans un README.
3. **Immortelle**. Le plus tendre, celui qui parle le mieux des souvenirs de famille, avec en prime le clin d'œil « Imm » à Immich ; il passe troisième seulement parce qu'il touche la limite des dix lettres.

Ce qui les départage : Estran dit **comment** l'appli fonctionne, Nadar dit **ce qu'elle regarde**, Immortelle dit **pourquoi on la veut**. Si tu veux un nom qui serve aussi d'explication, prends Estran ; si tu veux le plus simple à faire connaître dans le monde entier, Nadar ; si tu veux celui qui fait sourire l'équipe Immich, Immortelle.
