/**
 * ConvertToEpubMenuItem.tsx
 *
 * Explicit, NON-destructive "Convert to EPUB" menu action. Replaces the old
 * auto-convert dialog flow: conversion only ever runs when the user asks,
 * and the original file / DB row are never touched.
 *
 * Usage:
 *   <ConvertToEpubMenuItem bookId={42} format="pdf" variant="menu" />
 *
 * - variant="menu"    → full-width dropdown/menu item (reader top bar, card menus)
 * - variant="button"  → primary/secondary Button (dialogs)
 * - variant="overlay" → renders nothing until a conversion starts (context menus
 *                       that trigger via their own onClick)
 *
 * Progress is shown with the shared <ConversionProgress> overlay, which listens
 * for `conversion-progress` events. Completion is also driven by the job queue
 * events (`conversion:progress` / `conversion:complete` / `conversion:error`).
 */
import { useCallback, useEffect, useRef, useState } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { FileOutput, Loader2 } from 'lucide-react';
import { api } from '@/lib/tauri';
import { logger } from '@/lib/logger';
import { ConversionProgress } from '@/components/reader/ConversionProgress';
import { useToastStore } from '@/store/toastStore';
import { useLibraryStore } from '@/store/libraryStore';
import { useReaderStore } from '@/store/readerStore';
import { useConversionStore, type ConversionJob } from '@/store/conversionStore';
import { Button } from '@/components/ui/button';

interface ConvertToEpubMenuItemProps {
  bookId: number;
  bookTitle?: string;
  /** Book's current file format — hides the action for non-convertible formats */
  format?: string;
  variant?: 'menu' | 'button' | 'overlay';
  /** After a successful conversion, swap the open reader to the new EPUB */
  reopenOnSuccess?: boolean;
  /** Called when the conversion finishes (success or failure) */
  onDone?: () => void;
}

/** Formats that are already EPUB (or not local files) — no conversion offered. */
const NON_CONVERTIBLE_FORMATS = new Set(['epub', 'online-manga']);

/** Grace period after `convert_book` resolves: if the backend ran synchronously
 *  (no job events), finish the flow with the returned path. */
const SYNC_RESULT_GRACE_MS = 2500;

export function ConvertToEpubMenuItem({
  bookId,
  bookTitle,
  format,
  variant = 'menu',
  reopenOnSuccess = false,
  onDone,
}: ConvertToEpubMenuItemProps) {
  const [isConverting, setIsConverting] = useState(false);

  const jobIdRef = useRef<string | null>(null);
  const resultPathRef = useRef<string | null>(null);
  const finishedRef = useRef(false);
  const graceTimerRef = useRef<number | null>(null);

  const finishSuccess = useCallback(() => {
    if (finishedRef.current) return;
    finishedRef.current = true;
    setIsConverting(false);
    useToastStore.getState().addToast({
      title: 'Converted to EPUB',
      description: 'The EPUB file is ready.',
      variant: 'success',
      duration: 3000,
    });
    if (reopenOnSuccess && resultPathRef.current) {
      // Swap the open reader to the freshly converted EPUB.
      useReaderStore.getState().setStartFromBeginning(false);
      useReaderStore.getState().openBook(bookId, resultPathRef.current, 'epub');
    }
    useLibraryStore.getState().loadInitialBooks().catch?.(() => {});
    onDone?.();
  }, [bookId, onDone, reopenOnSuccess]);

  const finishError = useCallback((message: string) => {
    if (finishedRef.current) return;
    finishedRef.current = true;
    setIsConverting(false);
    logger.error('[ConvertToEpub] Conversion failed:', message);
    useToastStore.getState().addToast({
      title: 'Conversion failed',
      description: message,
      variant: 'error',
    });
    onDone?.();
  }, [onDone]);

  const handleCancel = useCallback(() => {
    if (finishedRef.current) return;
    finishedRef.current = true;
    setIsConverting(false);
    if (jobIdRef.current) {
      useConversionStore.getState().cancelJob(jobIdRef.current).catch(() => {});
    }
    useToastStore.getState().addToast({
      title: 'Conversion cancelled',
      variant: 'info',
      duration: 2500,
    });
    onDone?.();
  }, [onDone]);

  // Completion driven by the conversion engine's job events.
  useEffect(() => {
    if (!isConverting) return;
    let active = true;
    const unlisteners: UnlistenFn[] = [];

    (async () => {
      try {
        const unProgress = await listen<ConversionJob>('conversion:progress', ({ payload }) => {
          if (!active) return;
          if (payload.book_id !== null && payload.book_id !== undefined && payload.book_id !== bookId) return;
          jobIdRef.current = payload.id;
        });
        const unComplete = await listen<{ job_id: string; output_path: string }>('conversion:complete', ({ payload }) => {
          if (!active) return;
          if (jobIdRef.current && payload.job_id !== jobIdRef.current) return;
          resultPathRef.current = payload.output_path;
          finishSuccess();
        });
        const unError = await listen<{ job_id: string; error: string }>('conversion:error', ({ payload }) => {
          if (!active) return;
          if (jobIdRef.current && payload.job_id !== jobIdRef.current) return;
          finishError(payload.error || 'Conversion failed');
        });
        unlisteners.push(unProgress, unComplete, unError);
      } catch (err) {
        logger.error('[ConvertToEpub] Failed to subscribe to conversion events:', err);
      }
    })();

    return () => {
      active = false;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [isConverting, bookId, finishSuccess, finishError]);

  // Cleanup the grace timer on unmount.
  useEffect(() => () => {
    if (graceTimerRef.current !== null) window.clearTimeout(graceTimerRef.current);
  }, []);

  const handleConvert = useCallback(async () => {
    if (isConverting) return;
    finishedRef.current = false;
    jobIdRef.current = null;
    resultPathRef.current = null;
    setIsConverting(true);
    try {
      const result = await api.convertBook(bookId);
      resultPathRef.current = result.new_path;
      // If the backend completed synchronously (no job events will arrive),
      // finish after a short grace period.
      graceTimerRef.current = window.setTimeout(() => {
        if (!finishedRef.current) finishSuccess();
      }, SYNC_RESULT_GRACE_MS);
    } catch (err) {
      logger.error('[ConvertToEpub] convert_book failed:', err);
      finishError(err instanceof Error ? err.message : String(err));
    }
  }, [bookId, isConverting, finishSuccess, finishError]);

  const handleProgressComplete = useCallback(() => {
    // Fired by <ConversionProgress> when a `conversion-progress` event hits 100%.
    if (!finishedRef.current) finishSuccess();
  }, [finishSuccess]);

  if (!format || NON_CONVERTIBLE_FORMATS.has(format.toLowerCase())) return null;

  const overlay = isConverting && (
    <ConversionProgress
      visible
      bookTitle={bookTitle}
      onComplete={handleProgressComplete}
      onCancel={handleCancel}
    />
  );

  if (variant === 'button') {
    return (
      <>
        <Button
          variant="secondary"
          size="sm"
          className="w-full sm:w-auto rounded-full bg-secondary/50 hover:bg-secondary border border-border/50 shadow-sm"
          onClick={handleConvert}
          disabled={isConverting}
          title="Create an EPUB copy of this book (the original file is kept)"
        >
          {isConverting ? <Loader2 className="w-4 h-4 mr-2 animate-spin" /> : <FileOutput className="w-4 h-4 mr-2" />}
          Convert to EPUB
        </Button>
        {overlay}
      </>
    );
  }

  if (variant === 'overlay') {
    return <>{overlay}</>;
  }

  return (
    <>
      <button
        type="button"
        onClick={handleConvert}
        disabled={isConverting}
        className="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm font-medium text-foreground/90 hover:bg-primary hover:text-primary-foreground transition-colors duration-150 disabled:opacity-50"
        title="Create an EPUB copy of this book (the original file is kept)"
      >
        {isConverting ? <Loader2 className="w-4 h-4 animate-spin" /> : <FileOutput className="w-4 h-4" />}
        Convert to EPUB
      </button>
      {overlay}
    </>
  );
}

export default ConvertToEpubMenuItem;
