package com.zsclip.lan

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.concurrent.thread

class LanAutoSyncService : Service() {
    private val main = Handler(Looper.getMainLooper())
    private val stopped = AtomicBoolean(false)
    private val workerStarted = AtomicBoolean(false)

    override fun onCreate() {
        super.onCreate()
        stopped.set(false)
        running.set(true)
        LanPrefs.saveAutoSync(this, true)
        createChannel()
        startForeground(1001, notification(LanPrefs.lastSyncStatusText(this)))
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == ACTION_STOP) {
            stopSelf(startId)
            return START_NOT_STICKY
        }
        if (!LanPrefs.hasPairing(this)) {
            notifyStatus("请先完成配对")
            stopSelf(startId)
            return START_NOT_STICKY
        }
        if (!workerStarted.compareAndSet(false, true)) {
            return START_STICKY
        }
        thread(name = "zsclip-lan-auto-sync") {
            while (!stopped.get()) {
                val message = try {
                    LanClient.pullLatestToClipboard(this, force = false)
                } catch (e: Exception) {
                    "同步失败：${e.message}".also {
                        LanPrefs.saveSyncStatus(this, false, it)
                    }
                }
                notifyStatus(message)
                Thread.sleep(5000)
            }
        }
        return START_STICKY
    }

    override fun onDestroy() {
        stopped.set(true)
        workerStarted.set(false)
        running.set(false)
        LanPrefs.saveAutoSync(this, false)
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null

    private fun createChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel =
                NotificationChannel(CHANNEL, "ZSClip 局域网同步", NotificationManager.IMPORTANCE_LOW)
            val manager = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
            manager.createNotificationChannel(channel)
        }
    }

    private fun notifyStatus(text: String) {
        main.post {
            val manager = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
            manager.notify(1001, notification(text))
        }
    }

    private fun notification(text: String): Notification {
        val pendingIntent = PendingIntent.getActivity(
            this,
            0,
            Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )
        val stopIntent = PendingIntent.getService(
            this,
            1,
            Intent(this, LanAutoSyncService::class.java).setAction(ACTION_STOP),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )
        val target = LanPrefs.pairedDeviceName(this).ifBlank { LanPrefs.pairedHost(this) }
        val builder = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            Notification.Builder(this, CHANNEL)
        } else {
            @Suppress("DEPRECATION")
            Notification.Builder(this)
        }
        return builder
            .setContentTitle("ZSClip 局域网自动同步")
            .setContentText(text)
            .setSubText(target.take(24))
            .setStyle(Notification.BigTextStyle().bigText(text))
            .setSmallIcon(android.R.drawable.stat_notify_sync)
            .setContentIntent(pendingIntent)
            .addAction(android.R.drawable.ic_menu_close_clear_cancel, "停止", stopIntent)
            .setOngoing(true)
            .build()
    }

    companion object {
        private const val CHANNEL = "zsclip_lan_sync"
        private const val ACTION_STOP = "com.zsclip.lan.STOP_AUTO_SYNC"
        private val running = AtomicBoolean(false)

        fun isRunning(context: Context): Boolean = running.get() || LanPrefs.autoSync(context)

        fun start(context: Context) {
            val intent = Intent(context, LanAutoSyncService::class.java)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                context.startForegroundService(intent)
            } else {
                context.startService(intent)
            }
        }

        fun stop(context: Context) {
            context.stopService(Intent(context, LanAutoSyncService::class.java))
            running.set(false)
            LanPrefs.saveAutoSync(context, false)
        }
    }
}
