#!/bin/sh
# Installs a desktop launcher and icons for the development build, so the app
# shows up in the application menu (Linux, freedesktop desktop entries).
set -eu
racine=$(cd "$(dirname "$0")/.." && pwd)
donnees="${XDG_DATA_HOME:-$HOME/.local/share}"
source_icone="$racine/app/src-tauri/icons/source.svg"

for taille in 16 22 24 32 48 64 128 256 512; do
    dossier="$donnees/icons/hicolor/${taille}x${taille}/apps"
    mkdir -p "$dossier"
    magick -background none -density 300 "$source_icone" -resize "${taille}x${taille}" "$dossier/nicephore.png"
done
mkdir -p "$donnees/icons/hicolor/scalable/apps"
cp "$source_icone" "$donnees/icons/hicolor/scalable/apps/nicephore.svg"

mkdir -p "$donnees/applications"
cat >"$donnees/applications/nicephore.desktop" <<FIN
[Desktop Entry]
Type=Application
Name=Nicéphore
GenericName=Photothèque Immich
GenericName[en]=Immich photo library
Comment=Parcourir sa photothèque Immich depuis le bureau
Comment[en]=Browse your Immich photo library from the desktop
Exec=$racine/scripts/lancer.sh
Icon=nicephore
Terminal=false
StartupNotify=true
StartupWMClass=nicephore
Categories=Graphics;Photography;Viewer;
Keywords=photos;immich;gallery;galerie;souvenirs;
Actions=compatibilite;

[Desktop Action compatibilite]
Name=Lancer en mode compatibilité (X11)
Name[en]=Launch in compatibility mode (X11)
Exec=$racine/scripts/lancer.sh --mode compat
FIN

update-desktop-database "$donnees/applications" >/dev/null 2>&1 || true
touch "$donnees/icons/hicolor"
command -v kbuildsycoca6 >/dev/null && kbuildsycoca6 >/dev/null 2>&1 || true
echo "Lanceur installé : $donnees/applications/nicephore.desktop"
