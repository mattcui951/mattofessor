# Mattofessor: notes for Claude

A Windows and macOS companion app for League of Legends (Tauri 2 + Rust + Svelte 5), a Porofessor replacement
that reads the League client API (LCU) and public data only. Public repo, released through GitHub.

## Start here

- `STATUS.md`: where the app stands, what is next, known issues. Read it before starting.
- `README.md`: develop, test with the mock game, build, release, where the data comes from.

## Rules

- Before you finish a piece of work, update `STATUS.md`: move finished items to "Recently done",
  add what is next, note anything left unverified. Keep it short.
- Data comes only from the LCU, the Live Client API and the public sources listed in README.
  No scraping of porofessor.gg.
- Test game flows with the mock (`tools/mock_client.py` + `tools/dev-mock.cmd`, or `tools/dev-mock.sh`
  on macOS). Never import
  runes or item sets into the real League client without asking: an import replaces a page.
- Check League's game (`League of Legends.exe`, macOS `LeagueofLegends`) is not running before
  anything that sends synthetic keys,
  restarts the installed app or reinstalls it.
- Debug builds use `%APPDATA%\Mattofessor-dev` (macOS `~/Library/Application Support/Mattofessor-dev`);
  keep it that way so a dev run never touches the
  installed app's database.
- System-specific code lives behind one interface per concern: `os/`, `ocr/`, `hotkeys/`,
  `secret.rs` (a `windows.rs` and a `macos.rs` each). A change to one system's file gets the same
  change in the other's, and UI text that names the system (Windows, Ctrl, PC) gets a macOS
  variant (`isMac` in `src/lib/platform.ts`, `...Mac` keys). From Windows the macOS code cannot be
  built locally: after pushing, check the CI run (`.github/workflows/ci.yml`: tests, clippy and a
  build on both systems; the macOS app is attached to the run) and fix what fails there.
- Strings exist in every language in `src/lib/locales/` (eight now); add each. README "Languages" says how to add a language.

## Releasing

First refresh the shipped copies (what a new install shows before it can reach GitHub): download
`content.json` from the `content` branch over `src-tauri/data/content.json`, and `meta.json`,
`mayhem.json` and `patch-notes.json` from the `data` branch into `src-tauri/data/`, then commit them.
Never edit either branch by hand: the app publishes `content`, the refresh-data workflow `data`.

In App HQ, call the `propose_release` tool with the version and the release notes: App HQ runs
the tests, bumps the version files, tags, pushes, watches the build and checks `latest.json`.
Commit first. Without App HQ (a terminal session), do it by hand:

Bump the version in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`,
commit, then push an annotated tag (`git tag -a vX.Y.Z -m "notes"`). A failed tag build cannot be
re-tagged: bump the version instead. Do not wait on the build in the foreground; watch it with a
background command (`gh run watch <id> --exit-status`) and carry on.
