import { BookDown } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";

interface MangaDownloadConfirmDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  chapterCount: number;
  onConfirm: () => void;
}

/** Confirm dialog shown before "Download Manga" downloads every chapter. */
export function MangaDownloadConfirmDialog({
  open,
  onOpenChange,
  title,
  chapterCount,
  onConfirm,
}: MangaDownloadConfirmDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        aria-describedby="manga-download-all-description"
        className="w-[calc(100vw-2rem)] max-w-[420px] bg-background/90 backdrop-blur-2xl text-foreground border-border/50 shadow-2xl rounded-3xl overflow-hidden p-0"
      >
        <DialogDescription id="manga-download-all-description" className="sr-only">
          Download all chapters of this manga.
        </DialogDescription>

        <div className="relative px-7 pt-7 pb-5">
          <div className="absolute top-0 left-0 right-0 h-1 bg-gradient-to-r from-transparent via-primary/50 to-transparent opacity-50" />
          <DialogHeader>
            <DialogTitle className="text-xl font-bold tracking-tight text-foreground">
              Download Manga
            </DialogTitle>
          </DialogHeader>
          <p className="text-sm text-foreground/70 leading-relaxed mt-2">
            Download all{" "}
            <span className="font-semibold text-foreground">{chapterCount}</span>{" "}
            chapter{chapterCount === 1 ? "" : "s"} of{" "}
            <span className="font-medium text-foreground">“{title}”</span>?
            Original-quality images will be saved as CBZ files and imported
            into your library.
          </p>
        </div>

        <DialogFooter className="px-7 py-4 bg-muted/30 border-t border-foreground/10">
          <Button
            variant="ghost"
            className="hover:bg-foreground/5 rounded-xl font-medium"
            onClick={() => onOpenChange(false)}
          >
            Cancel
          </Button>
          <Button
            className="gap-1.5 rounded-xl font-semibold"
            onClick={() => {
              onOpenChange(false);
              onConfirm();
            }}
          >
            <BookDown className="w-4 h-4" />
            Download All
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
