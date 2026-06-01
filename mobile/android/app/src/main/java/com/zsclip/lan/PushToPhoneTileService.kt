package com.zsclip.lan

import android.os.Build
import android.service.quicksettings.Tile
import android.service.quicksettings.TileService
import kotlin.concurrent.thread

class PushToPhoneTileService : TileService() {
    override fun onStartListening() {
        super.onStartListening()
        qsTile?.apply {
            label = "拉取到手机"
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                subtitle = LanProtocol.tileStateLabel(LanPrefs.pairedHost(this@PushToPhoneTileService), LanPrefs.token(this@PushToPhoneTileService), false)
            }
            state = Tile.STATE_INACTIVE
            updateTile()
        }
    }

    override fun onClick() {
        super.onClick()
        if (!LanPrefs.hasPairing(this)) {
            LanUi.openMainFromTile(this, "请先完成配对后再使用“拉取到手机”")
            return
        }
        qsTile?.apply {
            state = Tile.STATE_ACTIVE
            updateTile()
        }
        thread(name = "zsclip-push-to-phone") {
            val message = try {
                LanClient.pullLatestToClipboard(this, force = true)
            } catch (e: Exception) {
                "拉取到手机失败：${e.message}".also {
                    LanPrefs.saveSyncStatus(this, false, it)
                }
            }
            LanUi.showToast(this, message)
            try {
                qsTile?.apply {
                    label = "拉取到手机"
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
