#!/bin/sh
# Tests the virtual folders service (nicephore --dossiers-virtuels) for real
# on this machine, against the disposable Immich server, with a private
# config and cache: mount from the list, second instance refused, files
# served, removal unmounts, idle service stops, SIGTERM unmounts.
set -eu
racine=$(cd "$(dirname "$0")/.." && pwd)
binaire="$racine/target/release/nicephore"
donnees_jetable="${NICEPHORE_IMMICH_JETABLE:-${XDG_CACHE_HOME:-$HOME/.cache}/nicephore-immich-jetable}"
. "$donnees_jetable/test.env"
T=$(mktemp -d "${XDG_CACHE_HOME:-$HOME/.cache}/nicephore-essai-service.XXXXXX")
export XDG_CONFIG_HOME="$T/config" XDG_CACHE_HOME="$T/cache"
export IMMICH_URL="$IMMICH_TEST_URL" IMMICH_API_KEY="$IMMICH_TEST_API_KEY"
config="$XDG_CONFIG_HOME/io.github.valentin-gosselin.nicephore"
mkdir -p "$config" "$T/montage"
ok() { echo "ok : $1"; }
echec() { echo "ÉCHEC : $1"; fusermount3 -uz "$T/montage" 2>/dev/null || true; exit 1; }

id=$(curl -s -H "x-api-key: $IMMICH_API_KEY" "$IMMICH_URL/api/users/me" | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
cle=$(printf '%s\n%s' "$IMMICH_URL" "$id" | sha1sum | cut -c1-16)
liste="$config/virtuels-$cle.json"
cat > "$liste" <<FIN
[{"id": 1, "point": "$T/montage", "selection": {"genre": "periode", "du": "2025-02-01", "au": "2025-02-28"}, "rangement": "anneeMois"}]
FIN

"$binaire" --dossiers-virtuels 2>"$T/service.log" &
service=$!
for _ in $(seq 60); do [ -d "$T/montage/2025/02" ] && break; sleep 0.5; done
[ -d "$T/montage/2025/02" ] || echec "dossier monté par le service"
ok "dossier monté par le service ($(ls "$T/montage/2025/02" | wc -l) fichiers)"
"$binaire" --dossiers-virtuels 2>>"$T/second.log" && grep -q "déjà lancé" "$T/second.log" && ok "une seule instance" || echec "seconde instance refusée"
premier=$(ls "$T/montage/2025/02" | head -1)
sha1sum < "$T/montage/2025/02/$premier" >/dev/null && ok "fichier lu à travers le service ($premier)"
[ "$(sha1sum < "$T/montage/2025/02/$premier")" = "$(sha1sum < "$donnees_jetable/images/$premier")" ] && ok "contenu identique à l'original" || echec "contenu"
grep -q '"monte":true' "$config/virtuels-etat.json" && ok "état publié pour l'appli" || echec "état publié"

echo '[]' > "$liste"
for _ in $(seq 20); do mount | grep -q "$T/montage" || break; sleep 0.5; done
mount | grep -q "$T/montage" && echec "démonté une fois retiré de la liste" || ok "démonté une fois retiré de la liste"
for _ in $(seq 80); do kill -0 $service 2>/dev/null || break; sleep 1; done
kill -0 $service 2>/dev/null && echec "arrêt sans dossier" || ok "s'arrête seul sans dossier"

cat > "$liste" <<FIN
[{"id": 2, "point": "$T/montage", "selection": {"genre": "periode", "du": "2025-02-01", "au": "2025-02-28"}, "rangement": "plat"}]
FIN
"$binaire" --dossiers-virtuels 2>>"$T/service.log" &
service=$!
for _ in $(seq 60); do [ -n "$(ls "$T/montage" 2>/dev/null)" ] && break; sleep 0.5; done
kill -TERM $service
for _ in $(seq 20); do kill -0 $service 2>/dev/null || break; sleep 0.5; done
mount | grep -q "$T/montage" && echec "démonté à l'arrêt" || ok "démonté proprement à l'arrêt"
[ -f "$config/virtuels-etat.json" ] && echec "état effacé à l'arrêt" || ok "état effacé à l'arrêt"
rm -rf "$T"
