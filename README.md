<img src="docs/images/icone.png" alt="" width="96" align="right">

# Nicéphore

Unofficial desktop client for [Immich](https://immich.app), the self-hosted photo and video
management server. Not affiliated with or endorsed by the Immich project.

Named after Nicéphore Niépce, who took the oldest surviving photograph, a view from his window
at Le Gras (about 1827). The icon redraws that photo as a leaning De Stijl grid: the wing on
the left, the large lit roof rising towards the tower, and the sky, each zone in colour.

Status: early development, not released yet.

- Browsing like the web app: timeline with a date scrubber, favorites, archive, trash, viewer with
  video and slideshow, search, Explore, people (naming, merging), albums, map, memories,
  multi-selection with bulk actions, editing of date, location, description, rating and tags.
- Sending local folders to the server: one album per folder, a chosen album or the library only;
  files already on the server are recognized by their checksum and never sent twice; nothing is
  ever deleted, neither locally nor on the server. Keeps going from the tray, can start with the
  session, waits on battery or on a metered connection.
- Portable mode: a file named `portable` next to the executable (or the AppImage) keeps
  everything in a `donnees` folder beside it, with the remembered connection sealed by a
  passphrase.
- Signed in-app updates for the AppImage and the Windows installer.

## Goals

- Browse the whole library like the web app, natively on Windows and Linux.
- Two-way sync between local folders and the server: upload (done), selective download, virtual
  files (placeholders downloaded on demand).

## Layout

- `crates/api_immich`: typed client for the Immich API (session or API key authentication).
- `crates/cache_vignettes`: on-disk image cache shared by the UI.
- `crates/envoi`: upload engine (SQLite queue, folder watching, checksum pre-check, retries).
- `app/`: Tauri 2 shell (`src-tauri/`) and Svelte 5 UI built with `@immich/ui`.
- `docs/`: design notes and per-milestone reports (French).

## Development

Linux build dependencies (Fedora): `webkit2gtk4.1-devel gtk3-devel libsoup3-devel
javascriptcoregtk4.1-devel librsvg2-devel libappindicator-gtk3-devel openssl-devel`, plus Rust and
Node.js.

```sh
cd app
npm install
npx tauri dev          # development
npx tauri build        # release
```

For development, `IMMICH_URL` and `IMMICH_API_KEY` connect the app at startup
(`scripts/lancer.sh --dev` reads them from `~/.config/nicephore/dev.env`). Otherwise the app shows
a login screen like the web app and keeps only the session token, in the operating system keyring.

Testing:

- `cargo test --workspace`: unit tests.
- `scripts/immich-jetable.sh demarrer|tester|arreter`: a disposable Immich server in Docker
  (local only, generated photos) and the write tests of the API client and upload engine against
  it. Write tests never target a real library.
- `scripts/banc.sh`: read-only benchmark in an invisible virtual KWin session (scrolling frame
  times, thumbnail fill rate, screenshots); `--jetable` drives the real interface through a write
  scenario against the disposable server; `--portable` checks portable mode over two launches;
  `scripts/essai-mise-a-jour.sh --construire` tests an in-app update end to end with a throwaway
  signing key.

Releases are built by GitHub Actions on `v*` tags (`.github/workflows/publication.yml`).

## License

AGPL-3.0-or-later. See [LICENSE](LICENSE).
