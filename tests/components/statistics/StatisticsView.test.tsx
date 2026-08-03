import { describe, it, expect, beforeAll, vi } from 'vitest';
import { buildWeeklyBars } from '@/components/statistics/StatisticsView';
import type { DailyReadingStats } from '@/lib/tauri';

// Module import pulls in UI helpers that touch matchMedia.
beforeAll(() => {
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });
});

const stat = (date: string, overrides: Partial<DailyReadingStats> = {}): DailyReadingStats => ({
  date,
  total_seconds: 0,
  books_count: 0,
  sessions_count: 0,
  book_pages_read: 0,
  manga_pages_read: 0,
  ...overrides,
});

// Fixed "today" — 2025-06-15 is a Sunday; the week runs Mon 2025-06-09 → Sun 2025-06-15.
const NOW = new Date(2025, 5, 15);

describe('buildWeeklyBars', () => {
  it('zero-fills the last 7 days when there is no data at all', () => {
    const bars = buildWeeklyBars([], NOW);

    expect(bars).toHaveLength(7);
    // Week runs 2025-06-09 .. 2025-06-15
    expect(bars[0].date).toBe('2025-06-09');
    expect(bars[6].date).toBe('2025-06-15');
    for (const bar of bars) {
      expect(bar.seconds).toBe(0);
      expect(bar.pages).toBe(0);
      expect(bar.secondsPct).toBe(0);
      expect(bar.pagesPct).toBe(0);
    }
  });

  it('normalizes percentages against the week max (100 for max, 0 for zero)', () => {
    const bars = buildWeeklyBars(
      [
        stat('2025-06-10', { total_seconds: 3600, book_pages_read: 40 }), // max time + pages
        stat('2025-06-11', { total_seconds: 1800, book_pages_read: 20 }), // half of each
        stat('2025-06-12', { total_seconds: 0, book_pages_read: 0 }), // idle day
      ],
      NOW
    );

    const tue = bars.find(b => b.date === '2025-06-10')!;
    const wed = bars.find(b => b.date === '2025-06-11')!;
    const thu = bars.find(b => b.date === '2025-06-12')!;

    expect(tue.secondsPct).toBe(100);
    expect(tue.pagesPct).toBe(100);
    expect(wed.secondsPct).toBe(50);
    expect(wed.pagesPct).toBe(50);
    expect(thu.secondsPct).toBe(0);
    expect(thu.pagesPct).toBe(0);
  });

  it('sums book and manga pages into a single pages series', () => {
    const bars = buildWeeklyBars(
      [stat('2025-06-13', { book_pages_read: 12, manga_pages_read: 28 })],
      NOW
    );

    const fri = bars.find(b => b.date === '2025-06-13')!;
    expect(fri.pages).toBe(40);
    // Only day with pages → normalized to 100
    expect(fri.pagesPct).toBe(100);
  });

  it('handles a week where only time was read (pages all zero)', () => {
    const bars = buildWeeklyBars(
      [stat('2025-06-14', { total_seconds: 900 })],
      NOW
    );

    const sat = bars.find(b => b.date === '2025-06-14')!;
    expect(sat.secondsPct).toBe(100);
    expect(sat.pagesPct).toBe(0); // no pages at all → stays 0, no NaN
  });

  it('ignores stats outside the 7-day window', () => {
    const bars = buildWeeklyBars(
      [stat('2025-05-01', { total_seconds: 99999, book_pages_read: 999 })],
      NOW
    );

    expect(bars.every(b => b.seconds === 0 && b.pages === 0)).toBe(true);
  });
});
