import { create } from 'zustand';
import { listen } from '@tauri-apps/api/event';

export interface DownloadProgress {
  target_id: string; // url or id
  status: 'downloading' | 'completed' | 'error';
  downloaded_bytes: number;
  total_bytes: number | null;
  /** Optional human-readable title, registered by the frontend when a download starts. */
  title?: string;
}

interface OnlineDownloadStore {
  downloads: Record<string, DownloadProgress>;
  setDownload: (id: string, progress: DownloadProgress) => void;
  /** Remember the book title for a target id (merged, never clobbers progress). */
  registerDownload: (id: string, title: string) => void;
  clearDownload: (id: string) => void;
  initializeListeners: () => void;
}

let listenersInitialized = false;

export const useOnlineDownloadStore = create<OnlineDownloadStore>((set) => ({
  downloads: {},
  setDownload: (id, progress) =>
    set((state) => ({
      downloads: {
        ...state.downloads,
        [id]: progress,
      },
    })),
  registerDownload: (id, title) =>
    set((state) => {
      const existing = state.downloads[id];
      // The backend payload has no title — merge it into whatever progress
      // state already exists (or seed a minimal entry if the first progress
      // event hasn't arrived yet).
      return {
        downloads: {
          ...state.downloads,
          [id]: existing
            ? { ...existing, title }
            : {
                target_id: id,
                status: 'downloading',
                downloaded_bytes: 0,
                total_bytes: null,
                title,
              },
        },
      };
    }),
  clearDownload: (id) =>
    set((state) => {
      const newDownloads = { ...state.downloads };
      delete newDownloads[id];
      return { downloads: newDownloads };
    }),
  initializeListeners: () => {
    if (listenersInitialized) return;
    listenersInitialized = true;
    
    listen<DownloadProgress>('online-book-download-progress', (event) => {
      const payload = event.payload;
      set((state) => ({
        downloads: {
          ...state.downloads,
          // Preserve the frontend-registered title across progress events.
          [payload.target_id]: {
            ...payload,
            title: state.downloads[payload.target_id]?.title,
          },
        },
      }));
      
      if (payload.status === 'completed' || payload.status === 'error') {
        setTimeout(() => {
          set((state) => {
            const newDownloads = { ...state.downloads };
            delete newDownloads[payload.target_id];
            return { downloads: newDownloads };
          });
        }, 3000); // clear after 3 seconds
      }
    });
  },
}));
