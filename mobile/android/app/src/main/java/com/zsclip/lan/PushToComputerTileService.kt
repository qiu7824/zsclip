package com.zsclip.lan

import android.os.Build
import android.service.quicksettings.Tile
import android.service.quicksettings.TileService
import kotlin.concurrent.thread

class PushToComputerTileService : TileService() {
    override fun onStartListening() {
        super.onStartListening()
        qsTile?.apply {
            label = "推送到电脑"
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                subtitle = LanProtocol.tileStateLabel(
                    LanPrefs.pairedHost(this@PushToComputerTileService),
                    LanPrefs.token(this@PushToComputerTileService),
                    false
                )
            }
            state = Tile.STATE_INACTIVE
            updateTile()
        }
    }

    override fun onClick() {
        super.onClick()
        if (!LanPrefs.hasPairing(this)) {
            LanUi.openMainFromTile(this, "请先完成配对后再使用“推送到电脑”")
            return
        }
        qsTile?.apply {
            state = Tile.STATE_ACTIVE
            updateTile()
        }
        thread(name = "zsclip-push-to-computer") {
            val message = try {
                LanClient.pushClipboardTextToComputer(this)
            } catch (e: Exception) {
                "推送到电脑失败：${e.message}".also {
                    LanPrefs.saveSyncStatus(this, false, it)
                }
            }
            if (message.startsWith("推送到电脑失败")) {
                LanUi.openMainFromTile(this, message)
            } else {
                LanUi.showToast(this, message)
            }
            try {
                qsTile?.apply {
                    label = "推送到电脑"
                    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                        subtitle = message.take(40)
                    }
                    state = Tile.STATE_INACTIVE
                    updateTile()
                }
            } catch (_: Exception) {
            }
        }
    }
}
