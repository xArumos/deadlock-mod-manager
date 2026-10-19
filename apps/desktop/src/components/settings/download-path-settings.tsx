import { Button } from "@deadlock-mods/ui/components/button";
import { Input } from "@deadlock-mods/ui/components/input";
import { Label } from "@deadlock-mods/ui/components/label";
import { Switch } from "@deadlock-mods/ui/components/switch";
import { toast } from "@deadlock-mods/ui/components/sonner";
import { FolderOpen } from "@deadlock-mods/ui/icons";
import { useMutation, useQuery } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import logger from "@/lib/logger";
import {
  getModsStorePath,
  MODS_STORE_PATH_QUERY_KEY,
  resolveModsStorePath,
} from "@/lib/mods-store";
import { usePersistedStore } from "@/lib/store";
import { isTauriError } from "@/types/tauri";

export const DownloadPathSettings = () => {
  const { t } = useTranslation();
  const useCustomDownloadPath = usePersistedStore(
    (state) => state.useCustomDownloadPath,
  );
  const setUseCustomDownloadPath = usePersistedStore(
    (state) => state.setUseCustomDownloadPath,
  );
  const setDownloadPath = usePersistedStore((state) => state.setDownloadPath);

  const { data: storePath } = useQuery({
    queryKey: MODS_STORE_PATH_QUERY_KEY,
    queryFn: getModsStorePath,
  });

  const onMoveError = (error: Error) => {
    logger.errorOnly(error);
    toast.error(t("settings.downloadPathMoveFailed"), {
      description: isTauriError(error) ? error.message : undefined,
    });
  };

  const chooseMutation = useMutation({
    mutationFn: async () => {
      const selected = await open({
        directory: true,
        multiple: false,
        title: t("settings.browseDownloadPath"),
        defaultPath: usePersistedStore.getState().downloadPath || undefined,
      });
      if (!selected) {
        return null;
      }
      await resolveModsStorePath(selected);
      return selected;
    },
    onSuccess: (selected) => {
      if (!selected) {
        return;
      }
      setDownloadPath(selected);
      setUseCustomDownloadPath(true);
      toast.success(t("settings.downloadPathMoved"));
    },
    onError: onMoveError,
  });

  const automaticMutation = useMutation({
    mutationFn: () => resolveModsStorePath(null),
    onSuccess: () => {
      setUseCustomDownloadPath(false);
      toast.success(t("settings.downloadPathMoved"));
    },
    onError: onMoveError,
  });

  const isMoving = chooseMutation.isPending || automaticMutation.isPending;

  return (
    <div className='flex flex-col gap-4'>
      <div className='flex items-center justify-between gap-4'>
        <Label
          className='font-bold text-sm'
          htmlFor='toggle-custom-download-path'>
          {t("settings.useCustomDownloadPath")}
        </Label>
        <Switch
          checked={useCustomDownloadPath}
          disabled={isMoving}
          id='toggle-custom-download-path'
          onCheckedChange={(enabled) =>
            enabled ? chooseMutation.mutate() : automaticMutation.mutate()
          }
        />
      </div>

      {!useCustomDownloadPath && (
        <p className='text-muted-foreground text-sm'>
          {t("settings.downloadPathAutomatic")}
        </p>
      )}

      <div className='space-y-2'>
        <Label className='font-bold text-sm' htmlFor='download-path'>
          {t("settings.currentDownloadPath")}
        </Label>
        <div className='flex gap-2'>
          <Input
            className='flex-1 font-mono text-sm'
            id='download-path'
            readOnly
            value={isMoving ? t("settings.downloadPathMoving") : storePath}
          />
          {useCustomDownloadPath && (
            <Button
              disabled={isMoving}
              onClick={() => chooseMutation.mutate()}
              size='icon'
              title={t("settings.browseDownloadPath")}
              variant='outline'>
              <FolderOpen className='h-4 w-4' />
            </Button>
          )}
        </div>
      </div>
    </div>
  );
};
