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
        if (LanAutoSyncService.isRunning(this)) {
            LanAutoSyncService.stop(this)
            LanUi.showToast(this, "局域网自动同步已关闭")
        } else {
            if (!LanPrefs.hasPairing(this)) {
                LanUi.openMainFromTile(this, "请先完成配对后再开启局域网自动同步")
                updateTile()
                return
            }
            LanAutoSyncService.start(this)
            LanUi.showToast(this, "局域网自动同步已开启")
        }
        updateTile()
    }

    private fun updateTile() {
        qsTile?.apply {
            label = "局域网自动同步"
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                subtitle = LanProtocol.tileStateLabel(
                    LanPrefs.pairedHost(this@AutoSyncTileService),
                    LanPrefs.token(this@AutoSyncTileService),
                    LanAutoSyncService.isRunning(this@AutoSyncTileService)
                )
            }
            state = if (LanAutoSyncService.isRunning(this@AutoSyncTileService)) Tile.STATE_ACTIVE else Tile.STATE_INACTIVE
            updateTile()
        }
    }
}
