use s4im_engine::{
    check_game_folder, create_instance, detect_game_folders, game_dir, instance_dir, list_instances, resolve_game_folder,
    set_game_folder_name, GameFolderState, InstanceKind,
};
use std::fs;

#[test]
fn works_with_a_game_folder_in_another_language() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fs::create_dir_all(root.join("The Sims 4").join("Mods")).unwrap();
    fs::create_dir_all(root.join("SharedTray")).unwrap();
    fs::create_dir_all(root.join("Random Folder")).unwrap();

    assert_eq!(detect_game_folders(root), ["The Sims 4"]);
    assert_eq!(resolve_game_folder(root, None).as_deref(), Some("The Sims 4"));
    assert_eq!(resolve_game_folder(root, Some("Los Sims 4")).as_deref(), Some("Los Sims 4"));

    let ok = check_game_folder(&root.join("The Sims 4"));
    assert_eq!(ok.state, GameFolderState::Ok);
    assert_eq!(ok.root.as_deref(), Some(root));
    assert_eq!(ok.name.as_deref(), Some("The Sims 4"));
    assert_eq!(check_game_folder(&root.join("Random Folder")).state, GameFolderState::Invalid);
    assert_eq!(check_game_folder(&root.join("nada")).state, GameFolderState::Invalid);

    fs::create_dir_all(root.join("Los Sims 4").join("saves")).unwrap();
    assert_eq!(resolve_game_folder(root, None), None);

    set_game_folder_name("The Sims 4");
    assert_eq!(game_dir(root), root.join("The Sims 4"));
    create_instance(root, "Solo", "", "solo", InstanceKind::Solo).unwrap();
    assert_eq!(instance_dir(root, "solo"), root.join("The Sims 4_solo"));
    assert!(root.join("The Sims 4_solo").join(".plumbob.json").is_file());
    let list = list_instances(root).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, "solo");
}

#[test]
fn shared_tray_is_always_named_shared_tray() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    assert_eq!(s4im_engine::shared_tray_path(root), root.join("SharedTray"));
    fs::create_dir_all(root.join("Tray_Compartido")).unwrap();
    assert_eq!(s4im_engine::shared_tray_path(root), root.join("SharedTray"));
    set_game_folder_name("Los Sims 4");
    assert_eq!(s4im_engine::shared_tray_path(root), root.join("SharedTray"));
    set_game_folder_name("The Sims 4");
    assert_eq!(s4im_engine::shared_tray_path(root), root.join("SharedTray"));
}
