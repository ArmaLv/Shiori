import * as Dialog from '@radix-ui/react-dialog';
import { create } from 'zustand';
import { Download, X, Inbox } from 'lucide-react';
import { useOnlineDownloadStore } from '@/store/onlineDownloadStore';
import { DownloadProgressBar } from './DownloadProgressBar';

// ──────────────────────────────────────────────────────────────────────────
// Shared open-state for the queue panel (button lives in OnlineBooksView,
// the panel itself is mounted globally in GlobalDialogs so it works from
// anywhere a download can start).
// ──────────────────────────────────────────────────────────────────────────

// eslint-disable-next-line react-refresh/only-export-components -- shared UI open-state, co-located with the panel
interface DownloadQueueUIState {
  open: boolean;
  setOpen: (open: boolean) => void;
}

// eslint-disable-next-line react-refresh/only-export-components -- shared UI open-state, co-located with the panel
export const useDownloadQueueUI = create<DownloadQueueUIState>((set) => ({
  open: false,
  setOpen: (open) => set({ open }),
}));

// ──────────────────────────────────────────────────────────────────────────
// DownloadsButton — icon + active-count badge; opens the queue panel.
// ──────────────────────────────────────────────────────────────────────────

export function DownloadsButton() {
  const downloads = useOnlineDownloadStore((s) => s.downloads);
  const setOpen = useDownloadQueueUI((s) => s.setOpen);

  const activeCount = Object.values(downloads).filter(
    (d) => d.status === 'downloading'
  ).length;
  const totalCount = Object.keys(downloads).length;

  return (
    <button
      type="button"
      onClick={() => setOpen(true)}
      title={totalCount > 0 ? `Downloads (${totalCount})` : 'Downloads'}
      className="relative flex items-center justify-center w-10 h-10 rounded-full bg-card/70 hover:bg-card backdrop-blur-xl border border-border/50 shadow-lg shadow-black/10 hover:border-primary/40 text-muted-foreground hover:text-foreground transition-colors"
    >
      <Download className="w-4 h-4" />
      {activeCount > 0 && (
        <span className="absolute -top-1 -right-1 min-w-[18px] h-[18px] px-1 rounded-full bg-primary text-primary-foreground text-[10px] font-bold flex items-center justify-center border-2 border-background shadow-sm">
          {activeCount}
        </span>
      )}
    </button>
  );
}

// ──────────────────────────────────────────────────────────────────────────
// DownloadQueuePanel — right-hand slide-over listing every tracked download
// (active + recently finished). Renders nothing when closed.
// ──────────────────────────────────────────────────────────────────────────

export function DownloadQueuePanel() {
  const open = useDownloadQueueUI((s) => s.open);
  const setOpen = useDownloadQueueUI((s) => s.setOpen);
  const downloads = useOnlineDownloadStore((s) => s.downloads);

  const entries = Object.values(downloads);
  const activeCount = entries.filter((d) => d.status === 'downloading').length;

  return (
    <Dialog.Root open={open} onOpenChange={setOpen}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-background/60 backdrop-blur-sm z-50 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0" />
        <Dialog.Content
          aria-describedby="download-queue-description"
          className="fixed right-0 top-0 h-full w-full max-w-sm bg-card/95 backdrop-blur-2xl border-l border-border/50 shadow-2xl z-50 flex flex-col data-[state=open]:animate-in data-[state=open]:slide-in-from-right data-[state=closed]:animate-out data-[state=closed]:slide-out-to-right duration-300 overflow-hidden"
        >
          <Dialog.Description id="download-queue-description" className="sr-only">
            Active book downloads and their progress.
          </Dialog.Description>

          {/* Header */}
          <div className="flex-none flex items-center justify-between px-5 py-4 border-b border-border/50">
            <div className="flex items-center gap-2.5">
              <div className="w-9 h-9 rounded-xl bg-primary/10 border border-primary/20 flex items-center justify-center">
                <Download className="w-4 h-4 text-primary" />
              </div>
              <div>
                <Dialog.Title className="text-sm font-semibold text-foreground tracking-tight leading-none">
                  Downloads
                </Dialog.Title>
                <p className="text-[11px] text-muted-foreground mt-1">
                  {entries.length === 0
                    ? 'Nothing in flight'
                    : `${activeCount} active · ${entries.length} total`}
                </p>
              </div>
            </div>
            <Dialog.Close asChild>
              <button
                className="p-2 bg-card/40 hover:bg-card/80 border border-border/50 rounded-xl transition-all duration-300 text-muted-foreground hover:text-foreground"
                title="Close downloads"
              >
                <X className="w-4 h-4" />
              </button>
            </Dialog.Close>
          </div>

          {/* List */}
          <div className="flex-1 overflow-y-auto custom-scrollbar p-4 space-y-3">
            {entries.length === 0 ? (
              <div className="flex flex-col items-center justify-center py-16 text-muted-foreground">
                <Inbox className="w-10 h-10 mb-3 opacity-30" />
                <p className="text-sm font-medium">No downloads yet</p>
                <p className="text-xs opacity-70 mt-1 text-center">
                  Downloads started from Online Library will show up here.
                </p>
              </div>
            ) : (
              entries.map((entry) => (
                <DownloadProgressBar
                  key={entry.target_id}
                  bookTitle={entry.title || entry.target_id}
                  progress={entry}
                />
              ))
            )}
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

export default DownloadQueuePanel;
