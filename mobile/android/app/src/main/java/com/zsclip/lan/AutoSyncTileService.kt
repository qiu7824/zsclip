package com.zsclip.lan

import android.os.Build
import android.service.quicksettings.Tile
import android.service.quicksettings.TileService

class AutoSyncTileService : TileService() {
    override fun onStartListening() {
        super.onStartListening()
        updateTile()
    }

    override fun onClick() {
        super.onClick()
        if (LanAutoSyncService.isEnabled(this)) {
            LanAutoSyncService.stop(this)
            LanUi.showToast(this, "多端自动同步已关闭")
        } else {
            if (!LanPrefs.hasPairing(this) && !LanPrefs.hasWebDavConfig(this)) {
                LanUi.openMainFromTile(this, "请先完成配对或配置 WebDAV 后再开启多端自动同步")
                updateTile()
                return
            }
            val error = LanAutoSyncService.start(this)
            if (error == null) {
                LanUi.showToast(this, "多端自动同步正在启动")
            } else {
                LanUi.openMainFromTile(this, error)
            }
        }
        updateTile()
    }

    private fun updateTile() {
        qsTile?.apply {
            label = "多端自动同步"
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                subtitle = if (
                    LanAutoSyncService.isEnabled(this@AutoSyncTileService) &&
                    !LanAutoSyncService.isRunning(this@AutoSyncTileService)
                ) {
                    "后台同步中"
                } else {
                    LanProtocol.multiAutoSyncStateLabel(
                        LanPrefs.hasPairing(this@AutoSyncTileService),
                        LanPrefs.hasWebDavConfig(this@AutoSyncTileService),
                        LanAutoSyncService.isRunning(this@AutoSyncTileService)
                    )
                }
            }
            state = if (LanAutoSyncService.isEnabled(this@AutoSyncTileService)) Tile.STATE_ACTIVE else Tile.STATE_INACTIVE
            updateTile()
        }
    }
}
