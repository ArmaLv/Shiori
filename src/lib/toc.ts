import type { TocEntry } from '@/lib/tauri';

/**
 * Parse a TOC/annotation location string into a chapter (or page) index.
 * Handles all location formats produced by the format adapters:
 * EPUB CFIs ("epubcfi(/0/12)", "epubcfi(/12/)", "epubcfi(/12)"),
 * generic chapter formats ("chapter_12", "chapter:12", "mobi-chapter-12", ...)
 * and PDF page formats ("page:12", "page-12").
 */
export function parseTocLocationToIndex(location: string): number | null {
  // EPUB variants: epubcfi(/0/12), epubcfi(/12/), epubcfi(/12)
  const cfiNestedMatch = location.match(/epubcfi\(\/\d+\/(\d+)\b/i);
  if (cfiNestedMatch) return parseInt(cfiNestedMatch[1], 10);

  const cfiSimpleMatch = location.match(/epubcfi\(\/(\d+)\b/i);
  if (cfiSimpleMatch) return parseInt(cfiSimpleMatch[1], 10);

  // Generic chapter formats
  const chapterMatch = location.match(/(?:^|[^\w])(?:chapter|chapter_|chapter-|html-chapter-|md-chapter-|fb2-chapter-|docx-chapter-|generic-chapter-|mobi-chapter-|[a-z0-9]+-chapter-)(\d+)/i);
  if (chapterMatch) return parseInt(chapterMatch[1], 10);

  // Renderer fallback: "chapter:12"
  const chapterColon = location.match(/^chapter:(\d+)/i);
  if (chapterColon) return parseInt(chapterColon[1], 10);

  // PDF TOC style: "page:12" / annotation style "page-12"
  const pageMatch = location.match(/^page[:-](\d+)/i);
  if (pageMatch) {
    const pageNumber = parseInt(pageMatch[1], 10);
    return Number.isNaN(pageNumber) ? null : pageNumber;
  }

  return null;
}

export interface FlatTocEntry {
  entry: TocEntry;
  /** Parsed chapter/page index for this entry. */
  index: number;
}

/** Flatten a TOC tree into document order, keeping only entries with a parseable location. */
export function flattenToc(toc: TocEntry[]): FlatTocEntry[] {
  const flat: FlatTocEntry[] = [];
  const walk = (entries: TocEntry[]) => {
    for (const entry of entries) {
      const index = parseTocLocationToIndex(entry.location);
      if (index !== null && !Number.isNaN(index)) {
        flat.push({ entry, index });
      }
      if (entry.children && entry.children.length > 0) {
        walk(entry.children);
      }
    }
  };
  walk(toc);
  return flat;
}

/**
 * Find the TOC entry the reader is currently inside.
 *
 * A book's spine usually has more chapters than the TOC lists (cover, split
 * files, untitled sections), so an exact index match often fails. Instead the
 * current entry is the one with the greatest index that is still <= the
 * current chapter index. When several entries share that index (multiple
 * anchors into the same file) the first one in document order wins.
 */
export function findCurrentTocEntry(toc: TocEntry[], currentIndex: number): TocEntry | null {
  let best: FlatTocEntry | null = null;
  for (const flat of flattenToc(toc)) {
    if (flat.index <= currentIndex && (best === null || flat.index > best.index)) {
      best = flat;
    }
  }
  return best?.entry ?? null;
}
