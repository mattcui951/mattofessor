#!/bin/sh
# Runs the app in dev mode against tools/mock_client.py (start that first). macOS; see dev-mock.cmd.
here="$(cd "$(dirname "$0")" && pwd)"
export MATTOFESSOR_LOCKFILE="$here/.mock/lockfile"
export MATTOFESSOR_GAME_EXE="Mattofessor Mock Game"
cd "$here/.." && exec npm run tauri dev
