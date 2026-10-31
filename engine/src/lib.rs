#[cfg(windows)]
mod archive;
mod cache;
#[cfg(windows)]
mod diag;
mod dbpf;
#[cfg(windows)]
mod derive;
mod discover;
mod error;
mod fsutil;
mod gamefolder;
mod health;
mod install;
#[cfg(windows)]
mod instances;
#[cfg(windows)]
mod lan;
#[cfg(windows)]
mod link;
#[cfg(windows)]
mod manage;
mod model;
mod modhash;
mod music;
mod options;
mod order;
mod savethumb;
mod saves;
#[cfg(windows)]
mod session;
mod settings;
mod shots;
mod snapshot;
#[cfg(windows)]
mod tray;
mod announcements;
mod changelog;
mod nointro;
mod updates;

#[cfg(windows)]
pub use archive::{archive_instance, archive_root, list_archived, restore_archived, ArchivedInstance};
#[cfg(windows)]
pub use diag::{diagnose, Diagnostics};
pub use dbpf::{read_thumbnails, thumbnail_to_png, Thumbnail, BUST_TYPES};
pub use discover::{active_state, list_instances, read_game_version, read_instance_meta};
pub use error::{EngineError, Result};
pub use cache::{cache_items, clear_cache, CacheItem, ClearReport};
pub use fsutil::{find_child, shared_tray_path};
pub use gamefolder::{
    check_game_folder, detect_game_folders, game_folder_name, legacy_prefix, resolve_game_folder, set_game_folder_name, GameFolderCheck,
    GameFolderState, DEFAULT_GAME_FOLDER,
};
pub use modhash::{code_of, codes_match, compare_mods, parse_code, scan_mods, ModChange, ModDiff, ModFile, ModScan};
pub use music::{add_music, delete_track, list_music, resolve_track, Genre, Track, GENRES};
pub use install::{check_game_root, game_exe, read_install_version, resolve_game_root, RootCheck, RootState};
pub use health::{analyze_health, resolve_instance_file, space_children, space_top_files, Health, HealthWarning, ModStatus, SpaceEntry};
#[cfg(windows)]
pub use instances::{
    activate_instance, active_instance_id, adopt_instance, create_instance, game_dir, game_running,
    import_instance, instance_dir, needs_setup, process_running, register_main, valid_code,
};
#[cfg(windows)]
pub use derive::{derive_instance, list_mod_dir, scan_base, BaseScan, DeriveSelection, ModEntry, ModGroupInfo};
#[cfg(windows)]
pub use lan::{connect as lan_connect, connect_error as lan_connect_error, ip_in_nets, lan_diff, local_addresses, local_nets, parse_adapters, parse_target as lan_parse_target, start_host, sync_from_host, LanAddress, LanChange, LanDiff, LanDownload, LanHost, LanHostInfo, LanHostStatus, LanItem, LanPending, LanPlan, LanReport, LocalNet, DEFAULT_PORT as LAN_PORT};
#[cfg(windows)]
pub use link::{create_junction, junction_target, remove_junction};
#[cfg(windows)]
pub use manage::{
    convert_instance, delete_instance, duplicate_instance, set_instance_role, s4mp_leftovers, update_instance, DeleteStats,
    S4MP_LEFTOVERS,
};
pub use announcements::{parse_announcements, parse_date, sample_announcements, sanitize_link, today as announcements_today, unread_count, Announcement, AnnouncementLink, ANNOUNCEMENTS_URL, MAX_JSON_BYTES as ANNOUNCEMENTS_MAX_BYTES};
pub use changelog::{changelog_langs, changelog_url, clean_changelog, pick_changelog, CHANGELOG_BASE, MAX_CHANGELOG_BYTES};
pub use nointro::{extract_package as extract_no_intro, installed_file as no_intro_installed, record as record_no_intro, remove_installed as remove_no_intro, MAX_ZIP_BYTES as NO_INTRO_MAX_BYTES, NO_INTRO_URL};
pub use updates::{first_section, is_newer, is_trusted_download, parse_version, pick_update, should_notify, verify_signature, UpdateAsset, UpdateInfo, DOWNLOAD_PREFIX, RELEASES_API, UPDATER_PUBKEY};
pub use model::{ActiveState, Config, ImportMode, InstanceKind, InstanceMeta, InstanceSummary, Role};
pub use options::{apply_game_options, GameOptions, MusicChoice};
pub use order::{apply_order, load_order, save_order};
pub use savethumb::{extract_thumbnail_jpeg, jpeg_to_transparent_png};
pub use saves::{list_saves, SaveInfo};
pub use shots::{
    delete_shots, delete_shots_with, import_shots, index_exists, list_shots, move_shots, update_index, Shot,
    Transfer, IMPORTED,
};
#[cfg(windows)]
pub use session::{
    candidate_install_dirs, close_session, detect_install_dir, detect_s4mm, ea_offer_ids,
    effective_game_args, launch_game, launch_s4mm, launch_s4mp, mark_played, split_command, wait_for_session, wait_for_session_ex,
    SessionEnd, GAME_ARGS, GAME_PROCESS, S4MM_PROCESS,
};
pub use settings::{
    load_settings, save_settings, BootSettings, MultiplayerSettings, Platform, S4mmSettings, Settings,
    ShotsSettings,
    ThemeSettings,
};
pub use snapshot::{
    clean_stale_tmp, create_snapshot, set_auto_snapshots, delete_snapshot, list_snapshots, prune_auto_snapshots, restore_snapshot,
    SnapshotInfo, SnapshotKind,
};
#[cfg(windows)]
pub use tray::{
    delete_tray_old, disable_tray_sync, enable_tray_sync, tray_old_info, TrayOldInfo, is_synced as tray_is_synced, link_new_instance_tray,
    shared_tray_dir, MergeReport, TrayDisable, SHARED_TRAY,
};
