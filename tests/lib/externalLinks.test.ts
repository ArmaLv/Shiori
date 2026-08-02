import { describe, it, expect, vi, afterEach } from 'vitest';
import { isExternalHref, handleExternalLinkClick } from '@/lib/externalLinks';

describe('externalLinks.ts', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  describe('isExternalHref', () => {
    const cases: Array<[string, boolean]> = [
      // External — absolute http(s) URLs (case-insensitive, whitespace-tolerant)
      ['https://example.com', true],
      ['http://example.com', true],
      ['HTTPS://EXAMPLE.COM', true],
      ['Http://example.com/path?q=1#frag', true],
      ['  https://example.com  ', true],
      // External — mailto:
      ['mailto:user@example.com', true],
      // Internal — everything else stays untouched
      ['#chapter-3', false],
      ['/chapters/2.html', false],
      ['chapters/2.html', false],
      ['../chapters/2.html', false],
      ['epubcfi(/6/2[chap02]!/4/2)', false],
      ['data:text/html;base64,PHNjcmlwdD4=', false],
      ['javascript:alert(1)', false],
      ['', false],
    ];

    it.each(cases)('%s → %s', (href, expected) => {
      expect(isExternalHref(href)).toBe(expected);
    });
  });

  describe('handleExternalLinkClick', () => {
    const mount = (html: string): { container: HTMLElement; anchor: HTMLAnchorElement } => {
      const container = document.createElement('div');
      container.innerHTML = html;
      document.body.appendChild(container);
      return { container, anchor: container.querySelector('a')! };
    };

    it('intercepts external links: prevents default, stops propagation, opens a new window', () => {
      const openSpy = vi.fn();
      vi.stubGlobal('open', openSpy);
      const { container, anchor } = mount('<a href="https://example.com">link</a>');

      let result = false;
      let bubbled = false;
      anchor.addEventListener('click', (e) => {
        result = handleExternalLinkClick(e, container);
      });
      container.addEventListener('click', () => {
        bubbled = true;
      });

      const event = new MouseEvent('click', { bubbles: true, cancelable: true });
      anchor.dispatchEvent(event);
      expect(result).toBe(true);
      expect(event.defaultPrevented).toBe(true);
      expect(bubbled).toBe(false);
      expect(openSpy).toHaveBeenCalledWith('https://example.com', '_blank', 'noopener,noreferrer');
    });

    it('lets internal links through untouched (no preventDefault, no stopPropagation)', () => {
      const { container, anchor } = mount('<a href="#chapter-2">link</a>');

      let result: boolean | null = null;
      let bubbled = false;
      anchor.addEventListener('click', (e) => {
        result = handleExternalLinkClick(e, container);
      });
      container.addEventListener('click', () => {
        bubbled = true;
      });

      const event = new MouseEvent('click', { bubbles: true, cancelable: true });
      anchor.dispatchEvent(event);
      expect(result).toBe(false);
      expect(event.defaultPrevented).toBe(false);
      expect(bubbled).toBe(true);
    });

    it('returns false when the click target is not inside an anchor', () => {
      const container = document.createElement('div');
      container.innerHTML = '<p>plain text</p>';
      document.body.appendChild(container);

      let result: boolean | null = null;
      container.addEventListener('click', (e) => {
        result = handleExternalLinkClick(e, container);
      });
      container.querySelector('p')!.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
      expect(result).toBe(false);
    });

    it('ignores anchors outside the content container', () => {
      const container = document.createElement('div');
      container.innerHTML = '<p>content</p>';
      document.body.appendChild(container);
      // Anchor lives OUTSIDE the container — e.g. reader chrome — but inside
      // the delegated listener's scope (the reader root).
      const outside = document.createElement('a');
      outside.href = 'https://example.com';
      document.body.appendChild(outside);

      let result: boolean | null = null;
      document.body.addEventListener('click', (e) => {
        result = handleExternalLinkClick(e, container);
      });
      const event = new MouseEvent('click', { bubbles: true, cancelable: true });
      outside.dispatchEvent(event);
      expect(result).toBe(false);
      expect(event.defaultPrevented).toBe(false);
    });
  });
});
