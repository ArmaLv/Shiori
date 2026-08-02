import { CheckCircle2, AlertCircle, Loader2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { DownloadProgress } from '@/store/onlineDownloadStore';

interface DownloadProgressBarProps {
  bookTitle: string;
  progress: DownloadProgress | undefined;
}

/** Format bytes as "X.X MB" (1 decimal). */
function formatMb(bytes: number): string {
  return (bytes / (1024 * 1024)).toFixed(1);
}

export function DownloadProgressBar({ bookTitle, progress }: DownloadProgressBarProps) {
  if (!progress) return null;

  const hasTotal = progress.total_bytes !== null && progress.total_bytes > 0;
  const totalBytes = progress.total_bytes;
  const percent = hasTotal && totalBytes !== null
    ? Math.min(100, Math.round((progress.downloaded_bytes / totalBytes) * 100))
    : null;

  const downloaded = formatMb(progress.downloaded_bytes);
  const total = totalBytes !== null ? formatMb(totalBytes) : null;

  const isDone = progress.status === 'completed';
  const isError = progress.status === 'error';

  return (
    <div className="p-4 rounded-2xl bg-secondary/20 border border-border/50 backdrop-blur-md animate-in fade-in">
      <div className="flex items-center justify-between gap-3 mb-2">
        <p className="text-xs font-semibold text-foreground truncate">{bookTitle}</p>
        {isDone ? (
          <CheckCircle2 className="w-4 h-4 text-green-400 shrink-0" />
        ) : isError ? (
          <AlertCircle className="w-4 h-4 text-red-400 shrink-0" />
        ) : (
          <Loader2 className="w-4 h-4 text-primary animate-spin shrink-0" />
        )}
      </div>

      {/* Progress bar */}
      <div
        data-testid="download-progress-bar"
        className="h-2 rounded-full bg-muted/40 overflow-hidden"
      >
        {isDone ? (
          <div className="h-full rounded-full bg-green-400/80" style={{ width: '100%' }} />
        ) : isError ? (
          <div className="h-full rounded-full bg-red-400/60" style={{ width: '100%' }} />
        ) : percent !== null ? (
          <div
            className="h-full rounded-full bg-primary transition-all duration-300 ease-out"
            style={{ width: `${percent}%` }}
          />
        ) : (
          <div className="h-full w-1/3 rounded-full bg-primary/70 animate-pulse" />
        )}
      </div>

      <div className="flex items-center justify-between mt-2">
        <span
          className={cn(
            'text-[11px] font-medium',
            isDone ? 'text-green-400' : isError ? 'text-red-400' : 'text-muted-foreground'
          )}
        >
          {isDone
            ? 'Added to library'
            : isError
              ? 'Download failed'
              : 'Downloading…'}
        </span>
        <span className="text-[11px] font-medium text-muted-foreground tabular-nums">
          {isDone ? `${downloaded} MB` : total ? `${downloaded} MB / ${total} MB` : `${downloaded} MB`}
        </span>
      </div>
    </div>
  );
}
