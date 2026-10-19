import { invoke } from "@tauri-apps/api/core";
import { queryClient } from "@/lib/client";
import logger from "@/lib/logger";
import { usePersistedStore } from "@/lib/store";

export const MODS_STORE_PATH_QUERY_KEY = ["mods-store-path"] as const;

export const getModsStorePath = () => invoke<string>("get_mods_store_path");

/**
 * Moves the mods store into `customPath`, or to the automatic location when it
 * is null, and returns the folder it ended up in.
 */
export const resolveModsStorePath = async (
  customPath: string | null,
): Promise<string> => {
  const path = await invoke<string>("resolve_mods_store_path", { customPath });
  queryClient.setQueryData(MODS_STORE_PATH_QUERY_KEY, path);
  return path;
};

/** Without a custom folder, the mods store follows the game to its drive. */
export const followGameDrive = async (): Promise<void> => {
  if (usePersistedStore.getState().useCustomDownloadPath) {
    return;
  }
  try {
    await resolveModsStorePath(null);
  } catch (error) {
    logger
      .withError(error)
      .error("Failed to move the mods folder to the game's drive");
  }
};
