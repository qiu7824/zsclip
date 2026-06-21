package com.zsclip.lan

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

class AndroidTransportEntryPointTest {
    @Test
    fun bdd_home_screen_is_clipboard_history_not_manual_sender() {
        val root = findAndroidAppRoot()
        val main = File(root, "src/main/java/com/zsclip/lan/MainActivity.kt").readText()
        val settings = File(root, "src/main/java/com/zsclip/lan/SettingsActivity.kt").readText()

        assertTrue(main.contains("\"剪贴板记录\""))
        assertTrue(main.contains("Filter.Text"))
        assertTrue(main.contains("LanClient.fetchMobileHistoryItems(this, 80)"))
        assertTrue(main.contains("copyTextItem(item)"))
        assertTrue(main.contains("Intent(this, SettingsActivity::class.java)"))
        assertTrue(main.contains("applyInsets(root)"))
        assertTrue(main.contains("settingsIconButton"))
        assertFalse(main.contains("iconButton(\"设置\""))
        assertFalse(main.contains("发送文本"))
        assertFalse(main.contains("最近同步"))
        assertTrue(settings.contains("\"设置\""))
        assertTrue(settings.contains("\"关于\""))
        assertTrue(settings.contains("https://github.com/qiu7824/zsclip/"))
    }

    @Test
    fun bdd_settings_owns_connection_sync_tiles_diagnostics_and_about() {
        val root = findAndroidAppRoot()
        val settings = File(root, "src/main/java/com/zsclip/lan/SettingsActivity.kt").readText()

        assertTrue(settings.contains("sectionTitle(\"连接电脑\")"))
        assertTrue(settings.contains("sectionTitle(\"WebDAV\")"))
        assertTrue(settings.contains("LanAutoSyncService.setNotificationEnabled(this, next)"))
        assertTrue(settings.contains("requestTile(PushToComputerTileService::class.java"))
        assertTrue(settings.contains("requestTile(PushToPhoneTileService::class.java"))
        assertTrue(settings.contains("copyDiagnostics()"))
        assertTrue(settings.contains("paintToggleButton(autoSyncButton, autoSyncEnabled)"))
        assertTrue(settings.contains("paintToggleButton(notificationStatusButton, syncNotificationEnabled)"))
        assertFalse(settings.contains("关闭选中文本菜单"))
        assertFalse(settings.contains("setComponentEnabledSetting"))
    }

    @Test
    fun bdd_process_text_menu_is_not_registered_or_used() {
        val root = findAndroidAppRoot()
        val manifest = File(root, "src/main/AndroidManifest.xml").readText()
        val prefs = File(root, "src/main/java/com/zsclip/lan/LanPrefs.kt").readText()

        assertFalse(manifest.contains("android.intent.action.PROCESS_TEXT"))
        assertFalse(manifest.contains(".ProcessTextActivity"))
        assertFalse(File(root, "src/main/java/com/zsclip/lan/ProcessTextActivity.kt").exists())
        assertFalse(prefs.contains("KEY_PROCESS_TEXT_ENABLED"))
        assertFalse(prefs.contains("processTextEnabled"))
    }

    @Test
    fun bdd_history_api_and_rows_support_text_images_and_files() {
        val root = findAndroidAppRoot()
        val main = File(root, "src/main/java/com/zsclip/lan/MainActivity.kt").readText()
        val lanClient = File(root, "src/main/java/com/zsclip/lan/LanClient.kt").readText()
        val fileActions = File(root, "src/main/java/com/zsclip/lan/AndroidFileActions.kt").readText()

        assertTrue(lanClient.contains("val text: String?"))
        assertTrue(lanClient.contains("text = item.optString(\"text\")"))
        assertTrue(main.contains("textRow(item)"))
        assertTrue(main.contains("imageRow(item)"))
        assertTrue(main.contains("fileRow(item)"))
        assertTrue(main.contains("LanClient.fetchMobileImageBytes(this, item.id)"))
        assertTrue(main.contains("LanClient.downloadMobileFile(this, item.id, file)"))
        assertTrue(fileActions.contains("FileProvider.getUriForFile"))
    }

    @Test
    fun bdd_share_entrypoints_remain_but_text_manual_input_is_removed() {
        val root = findAndroidAppRoot()
        val main = File(root, "src/main/java/com/zsclip/lan/MainActivity.kt").readText()
        val manifest = File(root, "src/main/AndroidManifest.xml").readText()

        assertTrue(manifest.contains("android:mimeType=\"text/plain\""))
        assertTrue(manifest.contains("android:mimeType=\"image/*\""))
        assertTrue(main.contains("pushSharedText(sharedText)"))
        assertTrue(main.contains("LanClient.pushTextToAvailableTransport(this, text)"))
        assertTrue(main.contains("LanClient.pushImageToAvailableTransport(this, pngBytes, name)"))
        assertFalse(main.contains("EditText"))
    }

    @Test
    fun bdd_tiles_are_the_push_pull_entrypoints_and_show_state() {
        val root = findAndroidAppRoot()
        val pushTile = File(root, "src/main/java/com/zsclip/lan/PushToComputerTileService.kt").readText()
        val pullTile = File(root, "src/main/java/com/zsclip/lan/PushToPhoneTileService.kt").readText()
        val autoTile = File(root, "src/main/java/com/zsclip/lan/AutoSyncTileService.kt").readText()

        assertTrue(pushTile.contains("Tile.STATE_ACTIVE"))
        assertTrue(pushTile.contains("Tile.STATE_INACTIVE"))
        assertTrue(pushTile.contains("postDelayed"))
        assertTrue(pushTile.contains("LanUi.openClipboardSyncFromTile(this)"))
        assertTrue(pullTile.contains("Tile.STATE_ACTIVE"))
        assertTrue(pullTile.contains("Tile.STATE_INACTIVE"))
        assertTrue(pullTile.contains("LanClient.pullAvailableTransportToClipboard(this, force = true)"))
        assertTrue(pullTile.contains("if (failed)"))
        assertTrue(autoTile.contains("Tile.STATE_ACTIVE"))
        assertTrue(autoTile.contains("Tile.STATE_INACTIVE"))
    }

    @Test
    fun bdd_auto_sync_notification_no_longer_hosts_push_pull_actions() {
        val root = findAndroidAppRoot()
        val autoSync = File(root, "src/main/java/com/zsclip/lan/LanAutoSyncService.kt").readText()
        val notificationBlock = autoSync
            .substringAfter("private fun notification(text: String): Notification")
            .substringBefore("private fun autoSyncOnce()")

        assertFalse(notificationBlock.contains("clipboardSyncIntent"))
        assertFalse(notificationBlock.contains("clipboardPullIntent"))
        assertTrue(notificationBlock.contains("ACTION_STOP"))
        assertFalse(autoSync.contains("LanClient.autoSyncAvailableTransport(this, pullRemote = false)"))
        assertTrue(autoSync.contains("LanClient.pendingRemoteTextForAutoSync(this)"))
        assertTrue(autoSync.contains("LanClient.pullAvailableTransportToClipboard(this, force = false)"))
        assertTrue(autoSync.contains("KEY_AUTO_SYNC_NOTIFICATION") || File(root, "src/main/java/com/zsclip/lan/LanPrefs.kt").readText().contains("KEY_AUTO_SYNC_NOTIFICATION"))
    }

    @Test
    fun bdd_auto_sync_has_persistent_workmanager_fallback() {
        val root = findAndroidAppRoot()
        val appGradle = File(root, "build.gradle").readText()
        val autoSync = File(root, "src/main/java/com/zsclip/lan/LanAutoSyncService.kt").readText()
        val worker = File(root, "src/main/java/com/zsclip/lan/AutoSyncWorker.kt").readText()

        assertTrue(appGradle.contains("androidx.work:work-runtime-ktx"))
        assertTrue(autoSync.contains("enqueueUniquePeriodicWork"))
        assertTrue(autoSync.contains("ExistingPeriodicWorkPolicy.UPDATE"))
        assertTrue(autoSync.contains("OneTimeWorkRequestBuilder<AutoSyncWorker>()"))
        assertFalse(worker.contains("LanClient.autoSyncAvailableTransport(applicationContext, pullRemote = false)"))
        assertTrue(worker.contains("Result.retry()"))
    }

    @Test
    fun bdd_android_push_success_updates_status_without_success_toast() {
        val root = findAndroidAppRoot()
        val syncActivity = File(root, "src/main/java/com/zsclip/lan/ClipboardSyncActivity.kt").readText()
        val pullActivity = File(root, "src/main/java/com/zsclip/lan/ClipboardPullActivity.kt").readText()

        assertTrue(syncActivity.contains("LanPrefs.updateSyncStatusMessage(this, true, message)"))
        assertTrue(syncActivity.contains("if (failed)"))
        assertFalse(syncActivity.contains("LanUi.showToast(this, message)\n            main.post"))
        assertTrue(pullActivity.contains("if (failed)"))
        assertFalse(pullActivity.contains("!intent.getBooleanExtra"))
    }

    @Test
    fun bdd_recent_own_lan_push_is_recorded_and_skipped_on_pull() {
        val root = findAndroidAppRoot()
        val prefs = File(root, "src/main/java/com/zsclip/lan/LanPrefs.kt").readText()
        val client = File(root, "src/main/java/com/zsclip/lan/LanClient.kt").readText()

        assertTrue(prefs.contains("KEY_LAST_OWN_PUSH_KEY"))
        assertTrue(prefs.contains("saveLastOwnPush"))
        assertTrue(client.contains("LanPrefs.saveLastOwnPush(context, key, signature)"))
        assertTrue(client.contains("private fun isRecentOwnLanPush"))
        assertTrue(client.contains("跳过手机刚推送的记录"))
        assertTrue(client.contains("RECENT_OWN_PUSH_SKIP_MS"))
    }

    private fun findAndroidAppRoot(): File {
        return generateSequence(File(".").canonicalFile) { it.parentFile }
            .firstOrNull { File(it, "src/main").isDirectory && File(it, "src/test").isDirectory }
            ?: File(".").canonicalFile
    }
}
