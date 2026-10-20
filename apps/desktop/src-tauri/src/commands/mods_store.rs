use std::path::{Path, PathBuf};

use tauri_plugin_fs::FsExt;

use super::state::DOWNLOAD_MANAGER;
use crate::app_runtime::AppHandle;
use crate::errors::Error;
use crate::mod_manager::mods_store;

pub(crate) fn current_store(app_handle: &AppHandle) -> Result<PathBuf, Error> {
  let app_data =
    crate::runtime_environment::app_local_data_dir(app_handle).map_err(Error::Tauri)?;
  Ok(mods_store::active_path(&app_data))
}

/// Local imports write into the store from the renderer, which is otherwise
/// only allowed into app data.
pub(crate) fn allow_renderer_access(app_handle: &AppHandle, store: &Path) {
  if let Err(error) = app_handle.fs_scope().allow_directory(store, true) {
    log::warn!(
      "Could not grant access to mods store {}: {error}",
      store.display()
    );
  }
}

#[tauri::command]
pub async fn get_mods_store_path(app_handle: AppHandle) -> Result<String, Error> {
  Ok(current_store(&app_handle)?.to_string_lossy().into_owned())
}

/// Puts the mods store in `custom_path`, or in the automatic location when it
/// is absent, moving existing files there. Returns the store folder.
#[tauri::command]
pub async fn resolve_mods_store_path(
  app_handle: AppHandle,
  custom_path: Option<String>,
) -> Result<String, Error> {
  let app_data =
    crate::runtime_environment::app_local_data_dir(&app_handle).map_err(Error::Tauri)?;

  let target = match custom_path {
    Some(chosen) => {
      let chosen = PathBuf::from(chosen);
      if !chosen.is_absolute() {
        return Err(Error::InvalidInput(format!(
          "Mods folder must be an absolute path: {}",
          chosen.display()
        )));
      }
      crate::runtime_environment::ensure_e2e_managed_path(&chosen)
        .map_err(Error::UnauthorizedPath)?;
      mods_store::custom_store_path(&chosen)
    }
    None if crate::runtime_environment::is_e2e_active() => {
      mods_store::app_data_store_path(&app_data)
    }
    None => {
      let game_path = super::state::game_path().ok();
      mods_store::automatic_store_path(game_path.as_deref(), &app_data)
    }
  };

  if target != mods_store::active_path(&app_data) && downloads_busy().await {
    return Err(Error::InvalidInput(
      "Wait for downloads to finish before moving the mods folder".to_string(),
    ));
  }

  let store = tokio::task::spawn_blocking(move || mods_store::switch_to(&app_data, target))
    .await
    .map_err(|error| Error::BackgroundTaskFailed(error.to_string()))??;
  allow_renderer_access(&app_handle, &store);
  log::info!("Mods store is at {}", store.display());
  Ok(store.to_string_lossy().into_owned())
}

async fn downloads_busy() -> bool {
  match DOWNLOAD_MANAGER.get() {
    Some(manager) => manager.is_busy().await,
    None => false,
  }
}
