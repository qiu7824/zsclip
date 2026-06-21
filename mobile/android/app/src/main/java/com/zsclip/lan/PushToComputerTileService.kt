package com.zsclip.lan

import android.os.Build
import android.os.Handler
import android.os.Looper
import android.service.quicksettings.Tile
import android.service.quicksettings.TileService

class PushToComputerTileService : TileService() {
    private val main = Handler(Looper.getMainLooper())

    override fun onStartListening() {
        super.onStartListening()
        qsTile?.apply {
            label = "推送到电脑"
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                subtitle = LanProtocol.multiPushStateLabel(
                    LanPrefs.hasPairing(this@PushToComputerTileService),
                    LanPrefs.hasWebDavConfig(this@PushToComputerTileService),
                    false
                )
            }
            state = Tile.STATE_INACTIVE
            updateTile()
        }
    }

    override fun onClick() {
        super.onClick()
        if (!LanPrefs.hasPairing(this) && !LanPrefs.hasWebDavConfig(this)) {
            LanUi.openMainFromTile(this, "请先完成配对或配置 WebDAV 后再使用“推送到电脑”")
            return
        }
        qsTile?.apply {
            state = Tile.STATE_ACTIVE
            updateTile()
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            qsTile?.subtitle = "正在读取剪贴板"
        }
        qsTile?.updateTile()
        LanUi.openClipboardSyncFromTile(this)
        main.postDelayed({
            qsTile?.apply {
                label = "推送到电脑"
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                    subtitle = "点击推送当前剪贴板"
                }
                state = Tile.STATE_INACTIVE
                updateTile()
            }
        }, 3000)
    }
}
