//! The mods store holds each mod's downloaded and imported files under
//! `<store>/<mod id>/`. Downloads are staged there before their VPKs are copied
//! into the game, so by default it lives in the game's Steam library to keep
//! large archives off the system drive.

use crate::errors::Error;
use crate::mod_manager::filesystem_helper::FileSystemHelper;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, RwLock};

const LOCATION_FILE: &str = "mods-store-location.json";
const APP_FOLDER: &str = "deadlock-mod-manager";
const STORE_FOLDER: &str = "mods";

static ACTIVE: LazyLock<RwLock<Option<PathBuf>>> = LazyLock::new(|| RwLock::new(None));

#[derive(Serialize, Deserialize)]
struct RecordedLocation {
  path: PathBuf,
}

/// Where the store lived before it could be moved, and where it falls back to.
pub fn app_data_store_path(app_data: &Path) -> PathBuf {
  app_data.join(STORE_FOLDER)
}

pub fn active_path(app_data: &Path) -> PathBuf {
  ACTIVE
    .read()
    .ok()
    .and_then(|active| active.clone())
    .unwrap_or_else(|| app_data_store_path(app_data))
}

fn set_active(path: PathBuf) {
  if let Ok(mut active) = ACTIVE.write() {
    *active = Some(path);
  }
}

/// Picks up the location recorded by the last move, so anything that runs
/// before the frontend resolves the location still finds the files.
pub fn restore(app_data: &Path) -> PathBuf {
  if let Some(recorded) = read_recorded(app_data) {
    set_active(recorded);
  }
  active_path(app_data)
}

fn read_recorded(app_data: &Path) -> Option<PathBuf> {
  let content = fs::read_to_string(app_data.join(LOCATION_FILE)).ok()?;
  match serde_json::from_str::<RecordedLocation>(&content) {
    Ok(location) => Some(location.path),
    Err(error) => {
      log::warn!("Ignoring unreadable mods store location: {error}");
      None
    }
  }
}

fn record(app_data: &Path, store: &Path) -> Result<(), Error> {
  fs::create_dir_all(app_data)?;
  let content = serde_json::to_string_pretty(&RecordedLocation {
    path: store.to_path_buf(),
  })
  .map_err(|error| Error::InvalidInput(format!("Failed to encode mods store location: {error}")))?;
  fs::write(app_data.join(LOCATION_FILE), content)?;
  Ok(())
}

/// The store gets its own subfolder so that clearing the download cache can
/// never touch other files in the folder the user picked.
pub fn custom_store_path(chosen: &Path) -> PathBuf {
  chosen.join(APP_FOLDER).join(STORE_FOLDER)
}

/// `<library>/steamapps/common/Deadlock` becomes `<library>/deadlock-mod-manager/mods`:
/// on the game's drive, but outside the folder Steam verifies and replaces.
fn library_store_path(game_path: &Path) -> Option<PathBuf> {
  let steamapps = game_path.parent()?.parent()?;
  if !steamapps
    .file_name()?
    .to_str()?
    .eq_ignore_ascii_case("steamapps")
  {
    return None;
  }
  Some(custom_store_path(steamapps.parent()?))
}

/// Uses the game's Steam library when it is on another drive than app data and
/// can be written to. Otherwise the store stays in app data.
pub fn automatic_store_path(game_path: Option<&Path>, app_data: &Path) -> PathBuf {
  let fallback = app_data_store_path(app_data);
  let Some(candidate) = game_path.and_then(library_store_path) else {
    return fallback;
  };
  if on_same_volume(&candidate, &fallback) {
    return fallback;
  }
  if let Err(error) = ensure_writable(&candidate) {
    log::warn!(
      "Keeping the mods store in app data, {} is not writable: {error}",
      candidate.display()
    );
    return fallback;
  }
  candidate
}

#[cfg(unix)]
fn volume_id(path: &Path) -> Option<u64> {
  use std::os::unix::fs::MetadataExt;
  path
    .ancestors()
    .find_map(|ancestor| fs::metadata(ancestor).ok())
    .map(|metadata| metadata.dev())
}

#[cfg(windows)]
fn volume_id(path: &Path) -> Option<String> {
  match path.components().next()? {
    std::path::Component::Prefix(prefix) => {
      Some(prefix.as_os_str().to_string_lossy().to_ascii_uppercase())
    }
    _ => None,
  }
}

fn on_same_volume(a: &Path, b: &Path) -> bool {
  matches!((volume_id(a), volume_id(b)), (Some(a), Some(b)) if a == b)
}

fn ensure_writable(dir: &Path) -> Result<(), Error> {
  fs::create_dir_all(dir)?;
  let probe = dir.join(".dmm-write-test");
  fs::write(&probe, b"")?;
  fs::remove_file(&probe)?;
  Ok(())
}

/// Copies then deletes file by file, so the store can move across drives.
fn relocate(from: &Path, to: &Path) -> Result<(), Error> {
  if !from.exists() {
    return Ok(());
  }
  let from_resolved = from.canonicalize()?;
  let to_resolved = to.canonicalize()?;
  if from_resolved == to_resolved {
    return Ok(());
  }
  if to_resolved.starts_with(&from_resolved) || from_resolved.starts_with(&to_resolved) {
    return Err(Error::InvalidInput(
      "The new mods folder can't be inside the current one, or contain it".to_string(),
    ));
  }

  log::info!(
    "Moving mods store from {} to {}",
    from.display(),
    to.display()
  );
  FileSystemHelper::new().move_directory_contents(from, to)?;
  if let Err(error) = fs::remove_dir(from) {
    log::warn!(
      "Could not remove old mods store {}: {error}",
      from.display()
    );
  }
  // Drop the empty `deadlock-mod-manager` folder a previous move created.
  if let Some(parent) = from.parent()
    && parent.file_name().is_some_and(|name| name == APP_FOLDER)
  {
    fs::remove_dir(parent).ok();
  }
  Ok(())
}

/// Makes `target` the store, moving files over from the current one, and
/// remembers it for the next launch. On failure the current store stays active.
pub fn switch_to(app_data: &Path, target: PathBuf) -> Result<PathBuf, Error> {
  ensure_writable(&target)?;
  relocate(&active_path(app_data), &target)?;
  record(app_data, &target)?;
  set_active(target.clone());
  Ok(target)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn library_store_sits_beside_steamapps() {
    let game = Path::new("/games/SteamLibrary/steamapps/common/Deadlock");
    assert_eq!(
      library_store_path(game),
      Some(PathBuf::from(
        "/games/SteamLibrary/deadlock-mod-manager/mods"
      ))
    );
  }

  #[test]
  fn library_store_needs_a_steam_layout() {
    assert_eq!(library_store_path(Path::new("/home/me/deadlock")), None);
  }

  #[test]
  fn automatic_store_stays_in_app_data_without_a_game() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
      automatic_store_path(None, root.path()),
      root.path().join("mods")
    );
  }

  #[test]
  fn automatic_store_stays_in_app_data_on_the_same_drive() {
    let root = tempfile::tempdir().unwrap();
    let game = root.path().join("lib/steamapps/common/Deadlock");
    fs::create_dir_all(&game).unwrap();
    let app_data = root.path().join("app-data");

    assert_eq!(
      automatic_store_path(Some(&game), &app_data),
      app_data.join("mods")
    );
    assert!(!root.path().join("lib/deadlock-mod-manager").exists());
  }

  #[test]
  fn relocate_moves_downloads_and_local_mods() {
    let root = tempfile::tempdir().unwrap();
    let from = root.path().join("old");
    let to = root.path().join("new");
    fs::create_dir_all(from.join("local-1/files")).unwrap();
    fs::write(from.join("local-1/files/a.vpk"), b"vpk").unwrap();
    fs::create_dir_all(from.join("123")).unwrap();
    fs::write(from.join("123/mod.zip"), b"zip").unwrap();
    fs::create_dir_all(&to).unwrap();

    relocate(&from, &to).unwrap();

    assert_eq!(fs::read(to.join("local-1/files/a.vpk")).unwrap(), b"vpk");
    assert_eq!(fs::read(to.join("123/mod.zip")).unwrap(), b"zip");
    assert!(!from.exists());
  }

  #[test]
  fn relocate_removes_the_folder_a_previous_move_created() {
    let root = tempfile::tempdir().unwrap();
    let from = custom_store_path(&root.path().join("old"));
    let to = root.path().join("new");
    fs::create_dir_all(from.join("123")).unwrap();
    fs::create_dir_all(&to).unwrap();

    relocate(&from, &to).unwrap();

    assert!(to.join("123").is_dir());
    assert!(!root.path().join("old").join(APP_FOLDER).exists());
  }

  #[test]
  fn relocate_refuses_a_target_inside_the_store() {
    let root = tempfile::tempdir().unwrap();
    let from = root.path().join("store");
    let to = from.join("nested");
    fs::create_dir_all(&to).unwrap();

    assert!(relocate(&from, &to).is_err());
    assert!(to.is_dir());
  }

  #[test]
  fn relocate_to_the_same_folder_does_nothing() {
    let root = tempfile::tempdir().unwrap();
    let store = root.path().join("store");
    fs::create_dir_all(store.join("123")).unwrap();

    relocate(&store, &store).unwrap();

    assert!(store.join("123").is_dir());
  }

  #[test]
  fn recorded_location_survives_a_restart() {
    let root = tempfile::tempdir().unwrap();
    let store = root.path().join("elsewhere");
    record(root.path(), &store).unwrap();

    assert_eq!(read_recorded(root.path()), Some(store));
  }
}
