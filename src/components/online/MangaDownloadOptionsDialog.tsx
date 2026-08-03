import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import type { UnifiedChapter } from './OnlineMangaDetailView';
import { sortChaptersAscending } from './mangaDownloadUtils';

interface MangaDownloadOptionsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  chapters: UnifiedChapter[];
  title: string;
  onDownload: (selectedChapters: UnifiedChapter[]) => void;
}

const PRESETS = [5, 10, 25] as const;

/**
 * "Download Manga" picker: download the next 5 / 10 / 25 chapters
 * (oldest first) or every chapter of the series. Used from the header
 * "Download Manga" button (mobile + desktop) and the desktop dock.
 */
export function MangaDownloadOptionsDialog({
  open,
  onOpenChange,
  chapters,
  title,
  onDownload,
}: MangaDownloadOptionsDialogProps) {
  const handleDownload = (count: number | 'ALL') => {
    const sorted = sortChaptersAscending(chapters);
    const selected = count === 'ALL' ? sorted : sorted.slice(0, count);
    onDownload(selected);
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        aria-describedby="manga-download-options-description"
        className="w-[calc(100vw-2rem)] max-w-[460px] bg-background/90 backdrop-blur-2xl text-foreground border-border/50 shadow-2xl rounded-3xl overflow-hidden p-0"
      >
        <DialogDescription id="manga-download-options-description" className="sr-only">
          Choose how many chapters of this manga to download.
        </DialogDescription>

        {/* Header with gradient line */}
        <div className="relative px-8 pt-8 pb-4">
          <div className="absolute top-0 left-0 right-0 h-1 bg-gradient-to-r from-transparent via-primary/50 to-transparent opacity-50" />
          <DialogHeader>
            <DialogTitle className="text-2xl font-bold tracking-tight text-foreground">
              Download Manga
            </DialogTitle>
          </DialogHeader>
          <p className="text-sm text-foreground/70 leading-relaxed mt-2">
            Choose how many chapters of{" "}
            <span className="font-medium text-foreground">“{title}”</span> to
            download for offline reading. Chapters are downloaded oldest
            first and added to your library automatically.
          </p>
        </div>

        {/* Options */}
        <div className="px-8 py-2 pb-6 flex flex-col gap-3">
          {PRESETS.map((count) => (
            <Button
              key={count}
              variant="outline"
              className="group relative justify-between h-14 bg-foreground/[0.02] border-foreground/5 hover:border-foreground/10 hover:bg-foreground/[0.04] font-medium transition-all rounded-xl"
              onClick={() => handleDownload(count)}
            >
              <span className="flex items-center gap-3 text-foreground/90 group-hover:text-foreground">
                <span className="w-8 h-8 rounded-lg bg-foreground/5 flex items-center justify-center border border-foreground/5">
                  <span className="text-xs font-bold text-muted-foreground group-hover:text-primary transition-colors">
                    {count}
                  </span>
                </span>
                Next {count} Chapters
              </span>
              <span className="text-primary opacity-0 -translate-x-2 group-hover:opacity-100 group-hover:translate-x-0 transition-all">
                ↓
              </span>
            </Button>
          ))}

          <Button
            variant="outline"
            className="group relative justify-between h-14 bg-primary/10 border-primary/20 hover:bg-primary/20 hover:border-primary/30 font-bold transition-all rounded-xl shadow-[0_0_20px_-10px_rgba(var(--primary),0.3)]"
            onClick={() => handleDownload('ALL')}
          >
            <span className="flex items-center gap-3 text-primary">
              <span className="w-8 h-8 rounded-lg bg-primary/20 flex items-center justify-center border border-primary/20">
                <span className="text-xs font-bold text-primary">∞</span>
              </span>
              All Chapters ({chapters.length})
            </span>
            <span className="text-primary opacity-0 -translate-x-2 group-hover:opacity-100 group-hover:translate-x-0 transition-all">
              ↓
            </span>
          </Button>
        </div>

        {/* Footer */}
        <DialogFooter className="px-8 py-4 bg-muted/30 border-t border-foreground/10">
          <Button
            variant="ghost"
            className="hover:bg-foreground/5 rounded-xl font-medium"
            onClick={() => onOpenChange(false)}
          >
            Cancel
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
