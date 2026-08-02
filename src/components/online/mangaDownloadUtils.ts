import type { UnifiedChapter } from "./OnlineMangaDetailView";

/** Per-chapter download lifecycle state, keyed by chapter id. */
export type ChapterDownloadStatus =
  | "queued"
  | "downloading"
  | "done"
  | "failed";
export type ChapterDownloadStatusMap = Record<string, ChapterDownloadStatus>;

/** Unique chapter title used for CBZ filenames / import (matches the download loop contract). */
export function buildChapterDownloadTitle(
  ch: Pick<UnifiedChapter, "title" | "chapter">,
): string {
  if (ch.title) {
    return ch.title.toLowerCase().includes("chapter")
      ? ch.title
      : `Chapter ${ch.chapter} - ${ch.title}`;
  }
  return `Chapter ${ch.chapter}`;
}

/** Short display label, e.g. "Chapter 12: The Return" / "Oneshot". */
export function chapterDisplayLabel(
  ch: Pick<UnifiedChapter, "title" | "chapter">,
): string {
  const chapterNumStr =
    ch.chapter && ch.chapter !== "?" ? `Chapter ${ch.chapter}` : "";
  if (ch.title) return chapterNumStr ? `${chapterNumStr}: ${ch.title}` : ch.title;
  return chapterNumStr || "Oneshot";
}

/** Tally per-chapter download statuses. */
export function countChapterStatuses(status: ChapterDownloadStatusMap): {
  queued: number;
  downloading: number;
  done: number;
  failed: number;
} {
  const counts = { queued: 0, downloading: 0, done: 0, failed: 0 };
  for (const s of Object.values(status)) counts[s]++;
  return counts;
}

/** Sort chapters ascending by volume then chapter number (same order the old dialog used). */
export function sortChaptersAscending(
  chapters: UnifiedChapter[],
): UnifiedChapter[] {
  return [...chapters].sort((a, b) => {
    const aVol = a.volume === "None" ? 0 : Number(a.volume) || 0;
    const bVol = b.volume === "None" ? 0 : Number(b.volume) || 0;
    const aChap = Number(a.chapter) || 0;
    const bChap = Number(b.chapter) || 0;
    if (aVol !== bVol) return aVol - bVol;
    return aChap - bChap;
  });
}
