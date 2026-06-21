package com.zsclip.lan

import android.content.Context
import androidx.work.Worker
import androidx.work.WorkerParameters

class AutoSyncWorker(
    appContext: Context,
    workerParams: WorkerParameters
) : Worker(appContext, workerParams) {
    override fun doWork(): Result {
        if (!LanPrefs.autoSync(applicationContext)) {
            LanAutoSyncService.cancelFallbackWork(applicationContext)
            return Result.success()
        }
        if (LanAutoSyncService.isRunning(applicationContext)) {
            return Result.success()
        }
        if (!LanPrefs.hasPairing(applicationContext) && !LanPrefs.hasWebDavConfig(applicationContext)) {
            val message = "自动同步已关闭：请先完成配对或配置 WebDAV"
            LanPrefs.saveAutoSync(applicationContext, false)
            LanAutoSyncService.cancelFallbackWork(applicationContext)
            LanPrefs.saveSyncStatus(applicationContext, false, message)
            return Result.success()
        }
        return try {
            val pending = LanClient.pendingRemoteTextForAutoSync(applicationContext)
            if (pending != null) {
                LanPrefs.updateSyncStatusMessage(
                    applicationContext,
                    true,
                    "后台检测到${pending.transport}新文本，打开 ZSClip 后写入手机剪贴板：${pending.preview}"
                )
            }
            Result.success()
        } catch (e: Exception) {
            val message = "后台自动同步失败：${e.message}"
            LanPrefs.saveSyncStatus(applicationContext, false, message)
            Result.retry()
        }
    }
}
