/**
 * readerCapabilities.ts
 *
 * Capability map for the ReaderSettings panel — which setting groups are
 * shown for each format. Pure module so it can be unit-tested without
 * mounting the settings UI.
 */
import type { ReaderFormat } from './ReaderSettings';

export interface ReaderCapabilities {
  /** Font size / family / line height / alignment / spacing controls */
  typography: boolean;
  /** Layout mode controls (continuous flow, two-page view) */
  layout: boolean;
  /** Page transition (page-flip) controls */
  pageTransition: boolean;
  /** PDF zoom / reading width controls */
  pdfZoom: boolean;
}

/**
 * - epub: full premium settings (typography + layout + page transitions)
 * - pdf: typography (applied to the text layer) + zoom/width controls
 * - manga: no typography settings
 * - html-rendered text formats (mobi/azw/azw3/docx/fb2/txt/html/htm/md):
 *   typography + layout (page-flip is not wired for them — see GenericHtmlReader)
 */
export function getReaderCapabilities(format: ReaderFormat): ReaderCapabilities {
  switch (format) {
    case 'pdf':
      return { typography: true, layout: false, pageTransition: false, pdfZoom: true };
    case 'manga':
      return { typography: false, layout: false, pageTransition: false, pdfZoom: false };
    case 'epub':
      return { typography: true, layout: true, pageTransition: true, pdfZoom: false };
    default:
      // HTML-rendered text formats
      return { typography: true, layout: true, pageTransition: false, pdfZoom: false };
  }
}
