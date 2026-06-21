package com.zsclip.lan

import android.app.Activity
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import kotlin.concurrent.thread

class ClipboardSyncActivity : Activity() {
    private val main = Handler(Looper.getMainLooper())
    private var started = false
    private var focusAttempts = 0

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.setDimAmount(0f)
        runWhenFocused()
    }

    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (hasFocus) {
            runWhenFocused()
        }
    }

    private fun runWhenFocused() {
        if (started) {
            return
        }
        if (!hasWindowFocus() && focusAttempts < 10) {
            focusAttempts += 1
            main.postDelayed({ runWhenFocused() }, 80)
            return
        }
        started = true
        main.postDelayed({ pushClipboardAndClose() }, 80)
    }

    private fun pushClipboardAndClose() {
        thread(name = "zsclip-clipboard-sync") {
            var failed = false
            val message = try {
                LanClient.pushChangedClipboardTextToAvailableTransport(this)
            } catch (e: Exception) {
                failed = true
                "推送失败：${e.message}".also {
                    LanPrefs.saveSyncStatus(this, false, it)
                }
            }
            if (!failed) {
                LanPrefs.updateSyncStatusMessage(this, true, message)
            }
            if (failed) {
                LanUi.showToast(this, message)
            }
            main.post { finish() }
        }
    }
}
