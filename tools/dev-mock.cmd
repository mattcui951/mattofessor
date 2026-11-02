@echo off
rem Runs the app in dev mode against tools/mock_client.py (start that first).
set MATTOFESSOR_LOCKFILE=%~dp0.mock\lockfile
set MATTOFESSOR_GAME_EXE=Mattofessor Mock Game.exe
cd /d "%~dp0.."
npm run tauri dev
