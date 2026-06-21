package com.zsclip.lan

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.graphics.drawable.Icon
import android.os.Build
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.os.PowerManager
import androidx.work.Constraints
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.ExistingWorkPolicy
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.concurrent.thread

class LanAutoSyncService : Service() {
    private val main = Handler(Looper.getMainLooper())
    private val stopped = AtomicBoolean(false)
    private val workerStarted = AtomicBoolean(false)
    private var worker: Thread? = null
    private var wakeLock: PowerManager.WakeLock? = null
    private var lastPullActivityKey = ""
    private var lastPullActivityAt = 0L

    override fun onCreate() {
        super.onCreate()
        stopped.set(false)
        if (!LanPrefs.autoSync(this) || !LanPrefs.autoSyncNotification(this)) {
            running.set(false)
            stopSelf()
            return
        }
        createChannel()
        try {
            startForegroundForTransport(notification(LanPrefs.lastSyncStatusText(this)))
            running.set(true)
        } catch (e: Exception) {
            running.set(false)
            LanPrefs.saveSyncStatus(this, false, startFailureMessage(e))
            stopSelf()
        }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == ACTION_STOP) {
            LanPrefs.saveAutoSync(this, false)
            cancelFallbackWork(this)
            stopSelf(startId)
            return START_NOT_STICKY
        }
        if (!LanPrefs.autoSync(this)) {
            stopSelf(startId)
            return START_NOT_STICKY
        }
        if (!LanPrefs.hasPairing(this) && !LanPrefs.hasWebDavConfig(this)) {
            val message = "自动同步已关闭：请先完成配对或配置 WebDAV"
            LanPrefs.saveAutoSync(this, false)
            cancelFallbackWork(this)
            LanPrefs.saveSyncStatus(this, false, message)
            notifyStatus(message)
            stopSelf(startId)
            return START_NOT_STICKY
        }
        if (!LanPrefs.autoSyncNotification(this)) {
            scheduleFallbackWork(this)
            running.set(false)
            stopSelf(startId)
            return START_NOT_STICKY
        }
        scheduleFallbackWork(this)
        if (!workerStarted.compareAndSet(false, true)) {
            return START_STICKY
        }
        acquireWakeLock()
        worker = thread(name = "zsclip-lan-auto-sync") {
            while (!stopped.get()) {
                val message = try {
                    autoSyncOnce()
                } catch (e: Exception) {
                    "同步失败：${e.message}".also {
                        LanPrefs.saveSyncStatus(this, false, it)
                    }
                }
                notifyStatus(message)
                refreshWakeLock()
                try {
                    Thread.sleep(POLL_INTERVAL_MS)
                } catch (_: InterruptedException) {
                    break
                }
            }
        }
        return START_STICKY
    }

    override fun onDestroy() {
        stopped.set(true)
        worker?.interrupt()
        worker = null
        workerStarted.set(false)
        running.set(false)
        releaseWakeLock()
        super.onDestroy()
    }

    override fun onTimeout(startId: Int, fgsType: Int) {
        val message = "实时检测已被 Android 暂停；手机后台复制请用快捷磁贴或分享入口推送"
        LanPrefs.saveSyncStatus(this, false, message)
        running.set(false)
        stopSelf(startId)
    }

    override fun onBind(intent: Intent?): IBinder? = null

    private fun createChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel =
                NotificationChannel(CHANNEL, "ZSClip 多端同步", NotificationManager.IMPORTANCE_LOW)
            val manager = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
            manager.createNotificationChannel(channel)
        }
    }

    private fun startForegroundForTransport(notification: Notification) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            val type = if (LanPrefs.hasPairing(this)) {
                ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE
            } else {
                ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC
            }
            startForeground(NOTIFICATION_ID, notification, type)
        } else {
            startForeground(NOTIFICATION_ID, notification)
        }
    }

    private fun acquireWakeLock() {
        val manager = getSystemService(Context.POWER_SERVICE) as PowerManager
        wakeLock = manager.newWakeLock(
            PowerManager.PARTIAL_WAKE_LOCK,
            "$packageName:auto-sync"
        ).apply {
            setReferenceCounted(false)
        }
        refreshWakeLock()
    }

    private fun refreshWakeLock() {
        val lock = wakeLock ?: return
        if (!lock.isHeld) {
            lock.acquire(WAKE_LOCK_TIMEOUT_MS)
        }
    }

    private fun releaseWakeLock() {
        wakeLock?.let { lock ->
            if (lock.isHeld) {
                lock.release()
            }
        }
        wakeLock = null
    }

    private fun notifyStatus(text: String) {
        main.post {
            if (stopped.get()) {
                return@post
            }
            val manager = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
            manager.notify(NOTIFICATION_ID, notification(text))
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
        val target = LanPrefs.pairedDeviceName(this)
            .ifBlank { LanPrefs.pairedHost(this) }
            .ifBlank { LanPrefs.webDavConfig(this).remoteDir }
        val builder = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            Notification.Builder(this, CHANNEL)
        } else {
            @Suppress("DEPRECATION")
            Notification.Builder(this)
        }
        return builder
            .setContentTitle("剪贴板同步")
            .setContentText(text)
            .setSubText(target.take(24))
            .setStyle(Notification.BigTextStyle().bigText(text))
            .setSmallIcon(R.drawable.ic_sync_tile)
            .setContentIntent(pendingIntent)
            .addAction(
                Notification.Action.Builder(
                    Icon.createWithResource(this, android.R.drawable.ic_menu_close_clear_cancel),
                    "停止",
                    stopIntent
                ).build()
            )
            .setOngoing(true)
            .build()
    }

    private fun autoSyncOnce(): String {
        val messages = mutableListOf<String>()
        val pending = LanClient.pendingRemoteTextForAutoSync(this)
        if (pending != null) {
            val message = try {
                LanClient.pullAvailableTransportToClipboard(this, force = false)
            } catch (e: Exception) {
                val fallback = "检测到${pending.transport}新文本，打开剪贴板同步后写入手机剪贴板：${pending.preview}"
                LanPrefs.saveSyncStatus(this, false, "$fallback；${e.message}")
                launchPullActivityIfNeeded(pending.key)
                fallback
            }
            if (!LanProtocol.isAutoSyncNoopMessage(message)) {
                messages += message
            }
        }
        return messages.joinToString("\n").ifBlank { "没有新内容" }
    }

    private fun launchPullActivityIfNeeded(key: String) {
        val now = System.currentTimeMillis()
        if (key == lastPullActivityKey && now - lastPullActivityAt < PULL_ACTIVITY_THROTTLE_MS) {
            return
        }
        lastPullActivityKey = key
        lastPullActivityAt = now
        main.post {
            if (stopped.get()) {
                return@post
            }
            try {
                startActivity(LanUi.clipboardPullIntent(this, auto = true))
            } catch (e: Exception) {
                val message = "自动拉取需要打开 ZSClip 后完成：${e.message}"
                LanPrefs.saveSyncStatus(this, false, message)
                notifyStatus(message)
            }
        }
    }

    companion object {
        private const val CHANNEL = "zsclip_lan_sync"
        private const val NOTIFICATION_ID = 1001
        private const val ACTION_STOP = "com.zsclip.lan.STOP_AUTO_SYNC"
        private const val ACTION_START = "com.zsclip.lan.START_AUTO_SYNC"
        private const val POLL_INTERVAL_MS = 5000L
        private const val PULL_ACTIVITY_THROTTLE_MS = 15000L
        private const val WAKE_LOCK_TIMEOUT_MS = 10 * 60 * 1000L
        private val running = AtomicBoolean(false)

        fun isRunning(context: Context): Boolean = running.get()

        fun isEnabled(context: Context): Boolean = LanPrefs.autoSync(context)

        fun isNotificationEnabled(context: Context): Boolean =
            LanPrefs.autoSyncNotification(context)

        fun start(context: Context): String? {
            if (!LanPrefs.hasPairing(context) && !LanPrefs.hasWebDavConfig(context)) {
                return "请先完成配对或配置 WebDAV 后再开启自动同步"
            }
            LanPrefs.saveAutoSync(context, true)
            scheduleFallbackWork(context)
            if (!LanPrefs.autoSyncNotification(context)) {
                stopRealtimeService(context)
                val message = "无通知后台同步已开启；手机后台复制请用快捷磁贴或分享入口推送"
                LanPrefs.updateSyncStatusMessage(context, true, message)
                return null
            }
            return try {
                val intent = Intent(context, LanAutoSyncService::class.java).setAction(ACTION_START)
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                    context.startForegroundService(intent)
                } else {
                    context.startService(intent)
                }
                null
            } catch (e: Exception) {
                val message = startFailureMessage(e)
                LanPrefs.saveSyncStatus(context, false, message)
                message
            }
        }

        fun resumeIfEnabled(context: Context): String? {
            if (!isEnabled(context)) {
                return null
            }
            if (!LanPrefs.autoSyncNotification(context)) {
                scheduleFallbackWork(context)
                stopRealtimeService(context)
                return null
            }
            if (isRunning(context)) {
                scheduleFallbackWork(context)
                return null
            }
            return start(context)
        }

        fun stop(context: Context) {
            LanPrefs.saveAutoSync(context, false)
            cancelFallbackWork(context)
            stopRealtimeService(context)
        }

        fun setNotificationEnabled(context: Context, enabled: Boolean): String {
            LanPrefs.saveAutoSyncNotification(context, enabled)
            if (!LanPrefs.autoSync(context)) {
                return if (enabled) "通知栏状态已开启" else "通知栏状态已关闭"
            }
            return if (enabled) {
                start(context) ?: "通知栏状态已开启"
            } else {
                scheduleFallbackWork(context)
                stopRealtimeService(context)
                val message = "通知栏状态已关闭，自动同步改为后台运行；手机后台复制请用快捷磁贴或分享入口推送"
                LanPrefs.updateSyncStatusMessage(context, true, message)
                message
            }
        }

        private fun stopRealtimeService(context: Context) {
            context.stopService(Intent(context, LanAutoSyncService::class.java))
            running.set(false)
        }

        private fun scheduleFallbackWork(context: Context) {
            val constraints = Constraints.Builder()
                .setRequiredNetworkType(NetworkType.CONNECTED)
                .build()
            val request = PeriodicWorkRequestBuilder<AutoSyncWorker>(
                FALLBACK_INTERVAL_MINUTES,
                TimeUnit.MINUTES
            )
                .setConstraints(constraints)
                .build()
            WorkManager.getInstance(context.applicationContext).enqueueUniquePeriodicWork(
                FALLBACK_WORK_NAME,
                ExistingPeriodicWorkPolicy.UPDATE,
                request
            )
            WorkManager.getInstance(context.applicationContext).enqueueUniqueWork(
                FALLBACK_NOW_WORK_NAME,
                ExistingWorkPolicy.REPLACE,
                OneTimeWorkRequestBuilder<AutoSyncWorker>()
                    .setConstraints(constraints)
                    .build()
            )
        }

        internal fun cancelFallbackWork(context: Context) {
            val manager = WorkManager.getInstance(context.applicationContext)
            manager.cancelUniqueWork(FALLBACK_WORK_NAME)
            manager.cancelUniqueWork(FALLBACK_NOW_WORK_NAME)
        }

        private fun startFailureMessage(error: Exception): String {
            val detail = error.message?.trim().orEmpty()
            return if (detail.isBlank()) {
                "实时自动同步启动失败，后台同步仍会继续；手机后台复制请用快捷磁贴或分享入口推送"
            } else {
                "实时自动同步启动失败，后台同步仍会继续：$detail；手机后台复制请用快捷磁贴或分享入口推送"
            }
        }

        private const val FALLBACK_WORK_NAME = "zsclip-auto-sync-fallback"
        private const val FALLBACK_NOW_WORK_NAME = "zsclip-auto-sync-now"
        private const val FALLBACK_INTERVAL_MINUTES = 15L
    }
}
