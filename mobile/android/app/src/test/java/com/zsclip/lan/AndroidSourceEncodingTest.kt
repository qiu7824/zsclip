package com.zsclip.lan

import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

class AndroidSourceEncodingTest {
    @Test
    fun bdd_android_sources_do_not_contain_common_mojibake_markers() {
        val root = findAndroidAppRoot()
        val files = listOf(File(root, "src/main"), File(root, "src/test"))
            .flatMap { sourceRoot ->
                sourceRoot.walkTopDown()
                    .filter { it.isFile && it.extension in setOf("kt", "xml") }
                    .toList()
            }
        val markers = listOf(
            0x951f,
            0xfffd,
            0x6d93,
            0x9365,
            0x6f76,
            0x93ba,
            0x7487,
            0x9422,
            0x9225,
            0x6769,
            0x704f,
            0x58c0,
            0x7ed4
        ).map { it.toChar().toString() }

        val hits = files.flatMap { file ->
            file.readLines(Charsets.UTF_8)
                .mapIndexedNotNull { index, line ->
                    if (markers.any { line.contains(it) }) {
                        "${file.relativeTo(root)}:${index + 1}:$line"
                    } else {
                        null
                    }
                }
        }

        assertTrue(hits.joinToString("\n"), hits.isEmpty())
    }

    @Test
    fun bdd_manifest_labels_are_backed_by_utf8_string_resources() {
        val root = findAndroidAppRoot()
        val manifest = File(root, "src/main/AndroidManifest.xml").readText(Charsets.UTF_8)
        val strings = File(root, "src/main/res/values/strings.xml").readText(Charsets.UTF_8)

        assertTrue(manifest.contains("android:label=\"@string/app_name\""))
        assertTrue(manifest.contains("android:label=\"@string/tile_auto_sync\""))
        assertTrue(manifest.contains("android:label=\"@string/tile_pull_to_phone\""))
        assertTrue(manifest.contains("android:label=\"@string/tile_push_to_computer\""))
        assertTrue(strings.contains("<string name=\"app_name\">剪贴板同步</string>"))
        assertTrue(strings.contains("<string name=\"tile_auto_sync\">多端自动同步</string>"))
        assertTrue(strings.contains("<string name=\"tile_pull_to_phone\">拉取到手机</string>"))
        assertTrue(strings.contains("<string name=\"tile_push_to_computer\">推送到电脑</string>"))
        assertTrue(strings.contains("<string name=\"media_history_title\">剪贴板记录</string>"))
        assertTrue(strings.contains("<string name=\"image_preview_title\">图片预览</string>"))
    }

    @Test
    fun bdd_manifest_exposes_required_mobile_entrypoints() {
        val root = findAndroidAppRoot()
        val manifest = File(root, "src/main/AndroidManifest.xml").readText(Charsets.UTF_8)

        assertTrue(manifest.contains("android.permission.INTERNET"))
        assertTrue(manifest.contains("android.permission.CHANGE_WIFI_STATE"))
        assertTrue(manifest.contains("android.permission.WAKE_LOCK"))
        assertTrue(manifest.contains("android.permission.FOREGROUND_SERVICE_CONNECTED_DEVICE"))
        assertTrue(manifest.contains("android.permission.FOREGROUND_SERVICE_DATA_SYNC"))
        assertTrue(manifest.contains("android.intent.action.SEND"))
        assertTrue(manifest.contains("android.intent.action.SEND_MULTIPLE"))
        assertTrue(manifest.contains("android:mimeType=\"text/plain\""))
        assertTrue(manifest.contains("android:mimeType=\"image/*\""))
        assertTrue(manifest.contains("android.intent.action.VIEW"))
        assertTrue(manifest.contains("android.intent.category.BROWSABLE"))
        assertTrue(manifest.contains("android:scheme=\"zsclip\""))
        assertTrue(manifest.contains("android:host=\"pair\""))
        assertTrue(manifest.contains("android:name=\".LanAutoSyncService\""))
        assertTrue(manifest.contains("android:foregroundServiceType=\"connectedDevice|dataSync\""))
        assertTrue(manifest.contains("android:name=\".SettingsActivity\""))
        assertTrue(manifest.contains("android:name=\".AutoSyncTileService\""))
        assertTrue(manifest.contains("android:name=\".PushToPhoneTileService\""))
        assertTrue(manifest.contains("android:name=\".PushToComputerTileService\""))
        assertTrue(!manifest.contains("android.intent.action.PROCESS_TEXT"))
        assertTrue(manifest.contains("android:name=\".ClipboardSyncActivity\""))
        assertTrue(manifest.contains("android:name=\".ClipboardPullActivity\""))
        assertTrue(manifest.contains("android:name=\".MediaActivity\""))
        assertTrue(manifest.contains("android:name=\".ImagePreviewActivity\""))
        assertTrue(manifest.contains("androidx.core.content.FileProvider"))
        assertTrue(manifest.contains("android:resource=\"@xml/file_paths\""))
        assertTrue(manifest.contains("android.permission.BIND_QUICK_SETTINGS_TILE"))
        assertTrue(manifest.contains("android.service.quicksettings.action.QS_TILE"))
    }

    @Test
    fun bdd_android_delivery_metadata_is_release_ready() {
        val root = findAndroidAppRoot()
        val androidRoot = root.parentFile
        val appGradle = File(root, "build.gradle").readText(Charsets.UTF_8)
        val gradleProperties = File(androidRoot, "gradle.properties").readText(Charsets.UTF_8)
        val gitignore = File(androidRoot, ".gitignore").readText(Charsets.UTF_8)
        val smokeScript = File(androidRoot, "smoke-adb.ps1").readText(Charsets.UTF_8)

        assertTrue(appGradle.contains("versionCode 909"))
        assertTrue(appGradle.contains("versionName \"0.9.9\""))
        assertTrue(appGradle.contains("androidx.core:core-ktx"))
        assertTrue(appGradle.contains("androidx.work:work-runtime-ktx"))
        assertTrue(gradleProperties.lines().contains("android.useAndroidX=true"))
        assertTrue(gitignore.lines().contains(".kotlin/"))
        assertTrue(gitignore.lines().contains("app/build/"))
        assertTrue(smokeScript.contains("adb was not found in PATH"))
        assertTrue(smokeScript.contains("[switch]\$DryRun"))
        assertTrue(smokeScript.contains("[switch]\$CheckAutoSync"))
        assertTrue(smokeScript.contains("DRY-RUN adb"))
        assertTrue(smokeScript.contains("gradle assembleDebug"))
        assertTrue(smokeScript.contains("adb devices"))
        assertTrue(smokeScript.contains("com.zsclip.lan/.MainActivity"))
        assertTrue(smokeScript.contains("zsclip://pair?host="))
        assertTrue(smokeScript.contains("android.intent.action.SEND"))
        assertTrue(smokeScript.contains("dumpsys activity services com.zsclip.lan"))
        assertTrue(smokeScript.contains("ZSClip Android smoke test dry-run completed."))
    }

    private fun findAndroidAppRoot(): File {
        return generateSequence(File(".").canonicalFile) { it.parentFile }
            .firstOrNull { File(it, "src/main").isDirectory && File(it, "src/test").isDirectory }
            ?: File(".").canonicalFile
    }
}
