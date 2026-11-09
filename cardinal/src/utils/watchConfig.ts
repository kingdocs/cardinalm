import { invoke } from '@tauri-apps/api/core';

export type WatchConfigPayload = {
  watchRoot: string;
  ignorePaths: string[];
  includePaths: string[];
  ignoreFileRules: string[];
};

export const setWatchConfig = (payload: WatchConfigPayload): Promise<void> => {
  return invoke('set_watch_config', payload);
};
