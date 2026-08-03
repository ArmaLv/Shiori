package io.github.vinayydv3695.shiori

import android.content.Intent
import android.os.Bundle
import android.view.ActionMode
import android.view.Menu
import android.view.WindowManager
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    // Keep the screen awake while reading (default ON). The reader can
    // clear/re-add this flag at runtime via the set_keep_screen_on command
    // when the "Keep Screen On" reading setting is toggled.
    window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
  }

  override fun onNewIntent(intent: Intent) {
    // Update the activity's intent so that plugins checking activity.intent
    // (e.g., AuthPlugin.load() for cold-start scenarios) see the latest intent.
    setIntent(intent)
    super.onNewIntent(intent)
  }

  // Disable native text-selection popup by clearing the action mode menu
  override fun onActionModeStarted(mode: ActionMode?) {
      mode?.menu?.clear()
      super.onActionModeStarted(mode)
  }
}

