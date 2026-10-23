use notify::{Config as NotifyConfig, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

const QUIET: Duration = Duration::from_secs(2);

static CURRENT: Mutex<Option<RecommendedWatcher>> = Mutex::new(None);

pub fn watch(app: AppHandle, code: String, dirs: Vec<std::path::PathBuf>) {
    let (tx, rx) = channel::<()>();
    let emitter = app.clone();
    let name = code.clone();
    std::thread::spawn(move || {
        while rx.recv().is_ok() {
            loop {
                match rx.recv_timeout(QUIET) {
                    Ok(()) => continue,
                    Err(RecvTimeoutError::Timeout) => break,
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            }
            let _ = emitter.emit("folders-changed", name.clone());
        }
    });
    let sender: Sender<()> = tx;
    let mut watcher = match RecommendedWatcher::new(
        move |res: notify::Result<notify::Event>| {
            if res.is_ok() {
                let _ = sender.send(());
            }
        },
        NotifyConfig::default(),
    ) {
        Ok(w) => w,
        Err(_) => return,
    };
    for dir in dirs.iter().filter(|d| d.is_dir()) {
        let _ = watcher.watch(dir, RecursiveMode::Recursive);
    }
    *CURRENT.lock().unwrap() = Some(watcher);
}
