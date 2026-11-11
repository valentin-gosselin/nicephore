#!/bin/sh
# Showcase for the README: a local Immich server of its own (port 2298, apart
# from the disposable test server), filled with CC0 photos from Wikimedia
# Commons (tests/vitrine/photos.json), and the in-app tour that takes the
# screenshots in English.
#   vitrine.sh demarrer   start it and seed it (photos downloaded once)
#   vitrine.sh captures   take the screenshots (banc/<date>/v*.png)
#   vitrine.sh arreter    stop it (data kept)
#   vitrine.sh detruire   stop it and delete its data
set -eu
racine=$(cd "$(dirname "$0")/.." && pwd)
export DONNEES="${NICEPHORE_IMMICH_VITRINE:-${XDG_CACHE_HOME:-$HOME/.cache}/nicephore-immich-vitrine}"
export PORT="${PORT_VITRINE:-2298}"
export COMPOSE_PROJECT_NAME=nicephore-immich-vitrine
composition="$racine/tests/immich-jetable/docker-compose.yml"

case "${1:-}" in
    demarrer)
        mkdir -p "$DONNEES/upload" "$DONNEES/postgres"
        docker compose -f "$composition" up -d --quiet-pull
        python3 -I "$racine/tests/vitrine/remplir.py" "http://127.0.0.1:$PORT" "$DONNEES"
        ;;
    captures)
        NICEPHORE_IMMICH_VITRINE="$DONNEES" LANG=en_US.UTF-8 LC_ALL=en_US.UTF-8 LANGUAGE=en_US \
            timeout 900 "$racine/scripts/banc.sh" --vitrine
        ;;
    arreter)
        docker compose -f "$composition" down
        ;;
    detruire)
        docker compose -f "$composition" down -v
        # Postgres files belong to the container user: delete them from inside.
        docker run --rm -v "$DONNEES:/donnees" docker.io/valkey/valkey:9 rm -rf /donnees/postgres /donnees/upload
        rm -rf "$DONNEES"
        ;;
    *)
        echo "usage : $0 demarrer|captures|arreter|detruire" >&2
        exit 2
        ;;
esac
