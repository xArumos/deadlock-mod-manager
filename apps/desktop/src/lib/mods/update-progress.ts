// Share of each mod's slice of the progress bar spent downloading. Matches
// the 80% weighting `batch_update_mods` uses for its "installing" event, so
// the bar never moves backwards when installing starts.
export const DOWNLOAD_SHARE = 0.8;

interface BatchUpdateProgressInput {
  completedMods: number;
  totalMods: number;
  downloadPercentage: number;
}

export const getBatchUpdateOverallProgress = ({
  completedMods,
  totalMods,
  downloadPercentage,
}: BatchUpdateProgressInput): number => {
  if (totalMods <= 0) return 0;

  const download = Number.isFinite(downloadPercentage)
    ? Math.min(Math.max(downloadPercentage, 0), 100)
    : 0;

  return (
    ((completedMods + (download / 100) * DOWNLOAD_SHARE) / totalMods) * 100
  );
};
