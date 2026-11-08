#!/bin/sh
# End-to-end test of in-app updates with a throwaway signing key (never the
# release key): copies a 0.0.1 AppImage into a test folder, serves a 0.0.2
# AppImage and its signed manifest on 127.0.0.1:8799, then runs
# scripts/banc.sh --mise-a-jour. The 0.0.1 copy must find 0.0.2, check its
# signature, replace itself (keeping the old one) and restart as 0.0.2.
#
# --construire first builds both versions in the toolbox, with a fresh test
# key pair and the local endpoint (http allowed for this test only).
set -eu
racine=$(cd "$(dirname "$0")/.." && pwd)
T="${NICEPHORE_ESSAI_MAJ:-${XDG_CACHE_HOME:-$HOME/.cache}/nicephore-maj-test}"

if [ "${1:-}" = "--construire" ]; then
    mkdir -p "$T/cle"
    toolbox run -c immich-bureau bash -lc "
        set -e
        cd '$racine/app'
        node node_modules/@tauri-apps/cli/tauri.js signer generate -w '$T/cle/test.key' -p '' --ci -f >/dev/null
        export TAURI_SIGNING_PRIVATE_KEY=\$(cat '$T/cle/test.key') TAURI_SIGNING_PRIVATE_KEY_PASSWORD=''
        for v in 0.0.1 0.0.2; do
            printf '{\"version\":\"%s\",\"plugins\":{\"updater\":{\"pubkey\":\"%s\",\"endpoints\":[\"http://127.0.0.1:8799/latest.json\"],\"dangerousInsecureTransportProtocol\":true}}}' \$v \$(cat '$T/cle/test.key.pub') > '$T/config-'\$v.json
            node node_modules/@tauri-apps/cli/tauri.js build --bundles appimage --config '$T/config-'\$v.json 2>&1 | grep -E 'error|Finished 1'
            rm -rf '$T/build-'\$v && mkdir -p '$T/build-'\$v
            cp '$racine'/target/release/bundle/appimage/*_\${v}_*.AppImage* '$T/build-'\$v/
        done
    "
fi

rm -rf "$T/app" "$T/serveur"
mkdir -p "$T/app" "$T/serveur"
cp "$T"/build-0.0.1/*.AppImage "$T/app/Nicephore.AppImage"
cp "$T"/build-0.0.2/*.AppImage "$T/serveur/Nicephore_0.0.2.AppImage"
signature=$(cat "$T"/build-0.0.2/*.AppImage.sig)
# --signature-fausse: the 0.0.1 signature offered for the 0.0.2 file; the
# update must be refused and the AppImage left as it was.
fausse=""
for argument in "$@"; do
    [ "$argument" = "--signature-fausse" ] && fausse=1
done
if [ -n "$fausse" ]; then
    signature=$(cat "$T"/build-0.0.1/*.AppImage.sig)
fi
cat > "$T/serveur/latest.json" <<FIN
{
  "version": "0.0.2",
  "notes": "Version de test du banc : rien de neuf, sinon le numéro.",
  "pub_date": "2026-11-08T22:00:00Z",
  "platforms": {
    "linux-x86_64": {
      "signature": "$signature",
      "url": "http://127.0.0.1:8799/Nicephore_0.0.2.AppImage"
    }
  }
}
FIN

python3 -I -m http.server 8799 --bind 127.0.0.1 --directory "$T/serveur" >"$T/serveur.log" 2>&1 &
serveur=$!
trap 'kill $serveur 2>/dev/null' EXIT
sleep 1

"$racine/scripts/banc.sh" --mise-a-jour "$T/app/Nicephore.AppImage"

echo "--- vérifications"
if [ -n "$fausse" ]; then
    if [ "$(sha256sum < "$T/app/Nicephore.AppImage")" = "$(cat "$T"/build-0.0.1/*.AppImage | sha256sum)" ] \
        && grep -aq "installation refusée" "$(ls -td "$racine"/banc/*/ | head -1)sortie.txt"; then
        echo "ok : signature fausse refusée, AppImage intacte"
    else
        echo "ÉCHEC : la signature fausse n'a pas été refusée proprement"
    fi
    exit 0
fi
attendu=$(sha256sum < "$T/serveur/Nicephore_0.0.2.AppImage")
if [ "$(sha256sum < "$T/app/Nicephore.AppImage")" = "$attendu" ]; then
    echo "ok : l'AppImage est maintenant la 0.0.2"
else
    echo "ÉCHEC : l'AppImage n'a pas été remplacée"
fi
if [ -f "$T/app/Nicephore.AppImage.ancienne" ] \
    && [ "$(sha256sum < "$T/app/Nicephore.AppImage.ancienne")" = "$(cat "$T"/build-0.0.1/*.AppImage | sha256sum)" ]; then
    echo "ok : l'ancienne version est gardée à côté"
else
    echo "ÉCHEC : ancienne version absente"
fi
grep -c "GET /Nicephore_0.0.2.AppImage" "$T/serveur.log" | sed 's/^/téléchargements de la 0.0.2 : /'
