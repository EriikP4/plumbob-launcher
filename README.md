<p align="center"><img src="assets/banner.png" alt="Plumbob Launcher" width="100%"></p>

# Plumbob Launcher

A launcher and instance manager for The Sims 4: keep separate saves, mods and settings per game, like a Minecraft launcher.

Plumbob Launcher lets you keep as many independent Sims 4 setups as you want on the same PC. Each one (an **instance**) has its own mods, saves, tray, screenshots and settings. Pick an instance, press Play, and the launcher points the game at it. Nothing in the game files is patched, so switching is instant and reversible.

> Windows only. Not affiliated with, endorsed by or sponsored by Electronic Arts or Maxis. The Sims is a trademark of Electronic Arts Inc.

## Key features

- **Instances**: create, import, duplicate, convert (single-player / multiplayer) and delete, with safe confirmations.
- **Archive**: store instances you are not using, optionally clearing their cache and compressing them, and restore them later.
- **One-click play** through Steam, the EA app, Epic Games or a custom command, with optional Sims 4 Mod Manager and multiplayer launching, and play-time tracking.
- **Backups**: manual and automatic snapshots of saves and tray, with easy restore.
- **Shared tray** between instances, merged safely.
- **Screenshots manager** across all instances, and saves with thumbnails.
- **Health and cache tools**: detects key mods and their versions, warns about common problems, and cleans caches and error reports.
- **Notes** in Markdown, **custom music** manager, and **multiplayer tools** (host/client roles and a mod hash check).
- **Recommended game settings** for new instances (mods on, no telemetry, quieter music, windowed).
- Works with any game language, in English and Spanish, with themes and animations.

## How it works

The game always uses one folder inside `Documents\Electronic Arts`. The launcher keeps each instance in its own folder next to it and makes the game's folder a junction (a folder link) to the active instance:

```
Documents\Electronic Arts\
  The Sims 4          <- junction to the active instance
  The Sims 4_sp       <- instance "sp" (single-player)
  The Sims 4_mp1      <- instance "mp1" (multiplayer 1)
  SharedTray          <- tray shared between instances
  Archived            <- archived instances
```

## Installing

Download the installer or the portable zip from Releases and run it. No other setup is needed: on first launch the launcher finds your game folder and asks you to register your current game as the first instance.

## Building

Requirements: Windows 10 or 11 with WebView2, Rust (stable) and a recent Node.js.

```
npm install
npm run tauri dev      # desktop app with hot reload
npm run tauri build    # installer
cargo test -p s4im-engine
```

The interface alone can be run in the browser with sample data using `npm run dev`.

## Project layout

- `src/`: React and TypeScript interface (Vite).
- `engine/`: the Rust engine with all the logic and its tests.
- `src-tauri/`: the Tauri 2 desktop shell.

## Status

Work in progress.
