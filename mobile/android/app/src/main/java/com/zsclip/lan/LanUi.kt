package com.zsclip.lan

import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.service.quicksettings.TileService
import android.widget.Toast

object LanUi {
    const val EXTRA_SHARED_TEXT = "com.zsclip.lan.SHARED_TEXT"
    const val EXTRA_STATUS_MESSAGE = "com.zsclip.lan.STATUS_MESSAGE"

    fun showToast(context: Context, message: String) {
        Handler(Looper.getMainLooper()).post {
            Toast.makeText(context.applicationContext, message, Toast.LENGTH_LONG).show()
        }
    }

    fun mainIntent(context: Context, message: String? = null, sharedText: String? = null): Intent =
        Intent(context, MainActivity::class.java)
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP)
            .apply {
                if (!message.isNullOrBlank()) {
                    putExtra(EXTRA_STATUS_MESSAGE, message)
                }
                if (!sharedText.isNullOrBlank()) {
                    putExtra(EXTRA_SHARED_TEXT, sharedText)
                }
            }

    fun openMainFromTile(service: TileService, message: String, sharedText: String? = null) {
        Handler(Looper.getMainLooper()).post {
            Toast.makeText(service.applicationContext, message, Toast.LENGTH_LONG).show()
            val intent = mainIntent(service, message, sharedText)
            if (Build.VERSION.SDK_INT >= 34) {
                val pendingIntent = PendingIntent.getActivity(
                    service,
                    0,
                    intent,
                    PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
                )
                service.startActivityAndCollapse(pendingIntent)
            } else {
                @Suppress("DEPRECATION")
                service.startActivityAndCollapse(intent)
            }
        }
    }
}
