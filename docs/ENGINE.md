# Engine

The engine is the `engine/` crate (Rust, no dependency on Tauri or the UI). The Tauri app only exposes its functions as commands and the React interface calls them through `src/lib/api.ts`.

## Model

- **Instance**: a folder `<game root>\<game folder>_<code>` (for example `Documents\Electronic Arts\The Sims 4_solo`) holding the full contents of the game's documents folder: saves, mods, tray, options and screenshots. Its metadata (name, kind, description, role, play time) lives in `.plumbob.json` inside the folder. Folders without that file are listed as unregistered instances and can be adopted without touching their contents.
- **Active instance**: the one the game folder points to. The game folder (`The Sims 4`) is a Windows junction created with `mklink /J`, which needs no administrator rights. Switching instance replaces the junction; no instance folder is ever renamed.
- **First run**: if the game folder is a real folder, it is renamed to an instance folder and replaced by a junction. The rename is retried a few times and, if Windows still denies it, the error includes a diagnostic line (see Diagnostics).
- **Settings**: `settings.json` in `%LOCALAPPDATA%\Plumbob Launcher`, next to `order.json`, the snapshots and the screenshot index. Paths come from the Windows known-folder API, so a relocated Documents folder is handled.

## Activation

1. Refuse if `TS4_x64.exe` is running.
2. Take an automatic snapshot of the instance being activated.
3. Remove the current junction (only the link is deleted) and create one to the chosen instance. If the new junction fails, the previous one is restored.

## Snapshots

Manual and automatic backups stored under `snapshots\<code>\`. Automatic ones are taken before activating, before registering the first instance, before enabling the shared tray and when a game session closes. The latest ones are kept and manual ones never expire. Restoring takes a safety snapshot first.

## Launching

- Steam: `steam://rungameid/<id>`.
- EA app: `origin2://game/launch` with the offer ids read from the install, falling back to `TS4_x64.exe`.
- Epic and custom: `TS4_x64.exe` from the game root, or a custom command, link or program with arguments.
- Multiplayer: the configured S4MP launcher.
- Sessions are tracked by polling for the game process. If the multiplayer launcher closes without the game appearing, or the wait is cancelled, the session ends.

## Archive

An instance that is not active can be archived: its cache is optionally cleared, then it is either moved or compressed into a zip, always under `Electronic Arts\Archived`. A `<code>.json` file stores the instance summary so archived instances can be listed without opening the zip. Restoring moves or extracts the instance back and relinks the shared tray if it was synced.

## Diagnostics

Reports whether the game folder is under OneDrive, how many files are cloud-only or locked by another program, and whether a folder can be created and renamed next to the game folder.

## Tests

Every engine feature is covered by integration tests in `engine/tests` that run on temporary directories before touching real folders:

```
cargo test -p s4im-engine
```
