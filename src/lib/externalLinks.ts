/**
 * externalLinks.ts
 *
 * Shared interception of EXTERNAL links (http://, https://, mailto:) inside
 * book readers. Internal links (hash anchors, epubcfi(...), relative chapter
 * paths, data:/blob: inlined resources) must keep working as-is — only
 * absolute external hrefs are routed to the system browser.
 *
 * On Android/Tauri, a plain click on an external <a> would navigate the whole
 * app WebView away (window.open() returns null) — see OnlineMangaView's
 * openInBrowser for the same precedent. shellOpen() hands the URL to the
 * system browser instead.
 */
import { open as shellOpen } from '@tauri-apps/plugin-shell';
import { isTauri } from '@/lib/tauri';

/** Matches absolute external URLs (http/https) and mailto: — case-insensitive. */
const EXTERNAL_HREF_RE = /^(https?:|mailto:)/i;

/**
 * True when the href points outside the app: absolute http(s) URLs or
 * mailto:. Everything else (anchors, epubcfi, relative paths, data: URIs,
 * blob:, javascript:) is internal and must NOT be intercepted.
 */
export function isExternalHref(href: string): boolean {
  return EXTERNAL_HREF_RE.test(href.trim());
}

/**
 * Shared opener for external URLs (browser links, update pages, etc.).
 *
 * On Tauri (Android + desktop) hands the URL to the system browser via the
 * shell plugin — never navigates the app WebView (window.open() returns null
 * on Android). In a plain browser falls back to a new tab with
 * noopener,noreferrer. Failures are swallowed: callers fire-and-forget.
 */
export function openExternal(url: string): void | Promise<void> {
  if (isTauri) {
    // System browser on Android AND desktop Tauri — never navigate the app.
    return shellOpen(url).catch(() => {});
  }
  window.open(url, '_blank', 'noopener,noreferrer');
}

/**
 * Delegated click handler for reader content containers.
 *
 * Finds the closest <a> ancestor of the click target; if it's an external
 * href, prevents the default navigation (which would hijack the Android
 * WebView), stops propagation, and opens the URL in the system browser
 * (Tauri shell) or a new tab (plain browser). Internal links are left
 * untouched so native behaviors (hash scroll, epubcfi, relative chapter
 * links) keep working.
 *
 * @returns true if the click was an external link and was handled.
 */
export function handleExternalLinkClick(e: MouseEvent, container: HTMLElement | null): boolean {
  const target = e.target as Element | null;
  if (!target || typeof target.closest !== 'function') return false;

  const anchor = target.closest('a');
  if (!anchor) return false;
  // Only intercept links that live inside the content container — never the
  // reader chrome (top bar, sidebar, toolbar).
  if (container && !container.contains(anchor)) return false;

  const href = anchor.getAttribute('href') ?? '';
  if (!isExternalHref(href)) return false;

  e.preventDefault();
  e.stopPropagation();

  void openExternal(href);
  return true;
}
