#!/bin/sh
# Disposable Immich server for the write tests (albums, people, trash...).
#   immich-jetable.sh demarrer   start it and seed an account with photos
#   immich-jetable.sh arreter    stop it (data kept)
#   immich-jetable.sh detruire   stop it and delete its data
#   immich-jetable.sh tester     run the write tests against it
set -eu
racine=$(cd "$(dirname "$0")/.." && pwd)
export DONNEES="${NICEPHORE_IMMICH_JETABLE:-${XDG_CACHE_HOME:-$HOME/.cache}/nicephore-immich-jetable}"
export PORT="${PORT:-2299}"
composition="$racine/tests/immich-jetable/docker-compose.yml"

case "${1:-}" in
    demarrer)
        mkdir -p "$DONNEES/upload" "$DONNEES/postgres"
        docker compose -f "$composition" up -d --quiet-pull
        python3 -I "$racine/tests/immich-jetable/remplir.py" "http://127.0.0.1:$PORT" "$DONNEES"
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
    tester)
        set -a
        . "$DONNEES/test.env"
        set +a
        cd "$racine" && cargo test -p api_immich --test ecriture -- --ignored --test-threads=1 --nocapture && cargo test -p envoi --test jetable -- --ignored --test-threads=1
        ;;
    *)
        echo "usage : $0 demarrer|arreter|detruire|tester" >&2
        exit 2
        ;;
esac
