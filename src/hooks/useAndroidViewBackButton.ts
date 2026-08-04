import { useEffect } from 'react'
import { isAndroid } from '@/lib/tauri'
import { useUIStore } from '@/store/uiStore'

/**
 * Wires the Android hardware back button to view-level navigation.
 *
 * When the user is on any non-home view (Library, Statistics, Shelves, Trash, …)
 * we keep a single marker entry (`#shiori-view`) on the history stack. Pressing
 * the hardware back button pops it, and we retrace the navigation history via
 * `uiStore.goBack()`, re-arming after each step until we land back on Home.
 * Once on Home the marker is gone, so a further back press falls through to the
 * WebView default and the app exits — standard Android behaviour.
 *
 * Overlays (reader, dialogs) use {@link useBackButton}, which pushes its OWN
 * `#view-<id>` hash on top of our marker. Because those sit above `#shiori-view`,
 * a back press while an overlay is open pops the overlay's hash first; our marker
 * is still present, so this handler no-ops and lets the overlay close itself. No
 * double navigation.
 */
const VIEW_HASH = '#shiori-view'

export function useAndroidViewBackButton() {
  const currentView = useUIStore((s) => s.currentView)

  // ── Listener: retrace history when our marker is popped ──
  useEffect(() => {
    if (!isAndroid) return

    const handlePopState = () => {
      // An overlay (reader/dialog) sitting above us was closed — its hash was
      // popped, ours is still here. Leave view navigation alone.
      if (window.location.hash === VIEW_HASH) return

      const { currentView: view, goBack } = useUIStore.getState()
      if (view === 'home') return // already home: allow the app to exit

      goBack()

      // Re-arm for the next level unless goBack landed us on Home.
      if (useUIStore.getState().currentView !== 'home' && window.location.hash !== VIEW_HASH) {
        window.history.pushState(null, '', window.location.pathname + window.location.search + VIEW_HASH)
      }
    }

    window.addEventListener('popstate', handlePopState)
    return () => window.removeEventListener('popstate', handlePopState)
  }, [])

  // ── Arm/disarm the marker as the view changes ──
  useEffect(() => {
    if (!isAndroid) return

    if (currentView !== 'home') {
      // Ensure exactly one marker is present for the current view.
      if (window.location.hash !== VIEW_HASH) {
        window.history.pushState(null, '', window.location.pathname + window.location.search + VIEW_HASH)
      }
    } else if (window.location.hash === VIEW_HASH) {
      // Returned Home via the UI (bottom nav / go-home) while the marker is still
      // on the stack — pop it so history stays clean and back exits immediately.
      window.history.back()
    }
  }, [currentView])
}
