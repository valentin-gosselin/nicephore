"""Builds the folder sent by the in-app upload scenario: variants of a seeded
photo (a JPEG comment makes a new checksum), one exact copy of a photo
already on the server, and a text file that must be skipped (plus, when
a third folder is given, the folder of the two-way library scenario). Every file is
dated an hour ago so it counts as complete. Standard library only."""

import os
import pathlib
import sys
import time

images = pathlib.Path(sys.argv[1])
sortie = pathlib.Path(sys.argv[2])
jeton = str(int(time.time()))
source = (images / "essai-00.jpg").read_bytes()


def variante(texte):
    commentaire = texte.encode()
    longueur = (len(commentaire) + 2).to_bytes(2, "big")
    return source[:2] + b"\xff\xfe" + longueur + commentaire + source[2:]


fichiers = {
    "racine.jpg": variante(f"racine {jeton}"),
    f"Balade banc {jeton}/un.jpg": variante(f"un {jeton}"),
    f"Balade banc {jeton}/deux.jpg": variante(f"deux {jeton}"),
    "copie.jpg": (images / "essai-01.jpg").read_bytes(),
    "notes.txt": b"pas une photo",
}
il_y_a_une_heure = time.time() - 3600
for nom, contenu in fichiers.items():
    chemin = sortie / nom
    chemin.parent.mkdir(parents=True, exist_ok=True)
    chemin.write_bytes(contenu)
    os.utime(chemin, (il_y_a_une_heure, il_y_a_une_heure))
print(f"dossier d'envoi prêt : {sortie}")

# Optional third folder, for two ways with the library: a photo of the
# server (February 2025) renamed deep in a folder of its own, and a new one.
if len(sys.argv) > 3:
    deux = pathlib.Path(sys.argv[3])
    for nom, contenu in {
        "2025/Anniversaire/ma-copie.jpg": (images / "essai-07.jpg").read_bytes(),
        "nouvelle.jpg": variante(f"deux sens {jeton}"),
    }.items():
        chemin = deux / nom
        chemin.parent.mkdir(parents=True, exist_ok=True)
        chemin.write_bytes(contenu)
        os.utime(chemin, (il_y_a_une_heure, il_y_a_une_heure))
    print(f"dossier deux sens prêt : {deux}")
