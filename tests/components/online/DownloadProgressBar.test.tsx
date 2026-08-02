import { describe, it, expect, beforeAll, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { DownloadProgressBar } from '@/components/online/DownloadProgressBar';
import type { DownloadProgress } from '@/store/onlineDownloadStore';

// matchMedia is required by the app's UI helpers (cn/animate-in usage).
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

const downloading = (overrides: Partial<DownloadProgress> = {}): DownloadProgress => ({
  target_id: 'https://example.com/book.epub',
  status: 'downloading',
  downloaded_bytes: 1_572_864, // 1.5 MB
  total_bytes: 10_485_760, // 10 MB
  ...overrides,
});

describe('DownloadProgressBar', () => {
  it('renders nothing when progress is undefined', () => {
    const { container } = render(<DownloadProgressBar bookTitle="Some Book" progress={undefined} />);
    expect(container.firstChild).toBeNull();
  });

  it('shows title, percent bar and MB/MB readout while downloading', () => {
    render(<DownloadProgressBar bookTitle="Some Book" progress={downloading()} />);

    expect(screen.getByText('Some Book')).toBeInTheDocument();
    expect(screen.getByText('Downloading…')).toBeInTheDocument();
    expect(screen.getByText('1.5 MB / 10.0 MB')).toBeInTheDocument();

    // 1572864 / 10485760 = 15% → fill width should be 15%
    const fill = screen.getByTestId('download-progress-bar').firstChild as HTMLElement;
    expect(fill).toHaveStyle({ width: '15%' });
  });

  it('uses an indeterminate bar when total_bytes is unknown', () => {
    render(
      <DownloadProgressBar bookTitle="Some Book" progress={downloading({ total_bytes: null })} />
    );

    expect(screen.getByText('Downloading…')).toBeInTheDocument();
    // No "X / Y" readout — only the downloaded amount
    expect(screen.getByText('1.5 MB')).toBeInTheDocument();
    expect(screen.queryByText(/\/ \d+\.\d MB/)).not.toBeInTheDocument();
  });

  it('shows the completed state with a check when status is completed', () => {
    render(
      <DownloadProgressBar
        bookTitle="Some Book"
        progress={downloading({ status: 'completed', downloaded_bytes: 10_485_760 })}
      />
    );

    expect(screen.getByText('Added to library')).toBeInTheDocument();
    expect(screen.getByText('10.0 MB')).toBeInTheDocument();
    expect(screen.queryByText('Downloading…')).not.toBeInTheDocument();
  });

  it('shows a failed state when status is error', () => {
    render(
      <DownloadProgressBar
        bookTitle="Some Book"
        progress={downloading({ status: 'error' })}
      />
    );

    expect(screen.getByText('Download failed')).toBeInTheDocument();
  });

  it('clamps the percent at 100', () => {
    render(
      <DownloadProgressBar
        bookTitle="Some Book"
        progress={downloading({ downloaded_bytes: 50_000_000 })}
      />
    );

    const fill = screen.getByTestId('download-progress-bar').firstChild as HTMLElement;
    expect(fill).toHaveStyle({ width: '100%' });
  });
});
