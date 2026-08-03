import { useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { isAndroid } from '@/lib/tauri';
import { useReadingSettings } from '@/store/premiumReaderStore';

/**
 * Keeps the Android device screen awake while the reader is mounted and the
 * "Keep Screen On" reading setting is enabled (default ON).
 *
 * - Mount: applies the flag per the current setting.
 * - Setting toggle: applies/clears the flag live via `set_keep_screen_on`.
 * - Unmount: clears the flag (only the reader should keep the screen on).
 *
 * Non-Android platforms no-op (the backend command itself is a no-op there).
 */
export function useKeepScreenOn(): void {
  const keepScreenOn = useReadingSettings((state) => state.keepScreenOn);

  useEffect(() => {
    if (!isAndroid) return;
    invoke('set_keep_screen_on', { enabled: keepScreenOn }).catch(() => {
      // Best-effort: MainActivity sets FLAG_KEEP_SCREEN_ON at launch by
      // default, so a failed invoke only affects the "off" case.
    });
    return () => {
      invoke('set_keep_screen_on', { enabled: false }).catch(() => {});
    };
  }, [keepScreenOn]);
}
