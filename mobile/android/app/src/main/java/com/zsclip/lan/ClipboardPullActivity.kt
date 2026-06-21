package com.zsclip.lan

import android.app.Activity
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import kotlin.concurrent.thread

class ClipboardPullActivity : Activity() {
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
        main.postDelayed({ pullClipboardAndClose() }, 80)
    }

    private fun pullClipboardAndClose() {
        thread(name = "zsclip-clipboard-pull") {
            var failed = false
            val message = try {
                LanClient.pullAvailableTransportToClipboard(this, force = false)
            } catch (e: Exception) {
                failed = true
                "自动拉取失败：${e.message}".also {
                    LanPrefs.saveSyncStatus(this, false, it)
                }
            }
            if (failed) {
                LanUi.showToast(this, message)
            }
            main.post { finish() }
        }
    }

    companion object {
        const val EXTRA_AUTO_PULL = "com.zsclip.lan.AUTO_PULL"
    }
}
