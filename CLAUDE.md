# Mattofessor: notes for Claude

A Windows companion app for League of Legends (Tauri 2 + Rust + Svelte 5), a Porofessor replacement
that reads the League client API (LCU) and public data only. Public repo, released through GitHub.

## Start here

- `STATUS.md`: where the app stands, what is next, known issues. Read it before starting.
- `README.md`: develop, test with the mock game, build, release, where the data comes from.

## Rules

- Before you finish a piece of work, update `STATUS.md`: move finished items to "Recently done",
  add what is next, note anything left unverified. Keep it short.
- Data comes only from the LCU, the Live Client API and the public sources listed in README.
  No scraping of porofessor.gg.
- Test game flows with the mock (`tools/mock_client.py` + `tools/dev-mock.cmd`). Never import
  runes or item sets into the real League client without asking: an import replaces a page.
- Check `League of Legends.exe` is not running before anything that sends synthetic keys,
  restarts the installed app or reinstalls it.
- Debug builds use `%APPDATA%\Mattofessor-dev`; keep it that way so a dev run never touches the
  installed app's database.
- Strings exist in English and Simplified Chinese; add both.

## Releasing

Bump the version in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`,
commit, then push an annotated tag (`git tag -a vX.Y.Z -m "notes"`). A failed tag build cannot be
re-tagged: bump the version instead. Do not wait on the build in the foreground; watch it with a
background command (`gh run watch <id> --exit-status`) and carry on.
