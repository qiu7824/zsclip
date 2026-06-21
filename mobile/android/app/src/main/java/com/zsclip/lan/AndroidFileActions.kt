package com.zsclip.lan

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.os.Environment
import android.webkit.MimeTypeMap
import androidx.core.content.FileProvider
import java.io.File
import java.io.FileOutputStream

object AndroidFileActions {
    fun downloadsDir(context: Context): File =
        File(context.getExternalFilesDir(Environment.DIRECTORY_DOWNLOADS) ?: context.filesDir, "ZSClip").apply {
            mkdirs()
        }

    fun cacheShareDir(context: Context): File =
        File(context.cacheDir, "share").apply {
            mkdirs()
        }

    fun uriFor(context: Context, file: File) =
        FileProvider.getUriForFile(context, "${context.packageName}.files", file)

    fun writeDownloadsFile(context: Context, name: String, bytes: ByteArray): File =
        writeFile(downloadsDir(context), safeFileName(name), bytes)

    fun writeShareFile(context: Context, name: String, bytes: ByteArray): File =
        writeFile(cacheShareDir(context), safeFileName(name), bytes)

    fun openFile(context: Context, file: File) {
        val uri = uriFor(context, file)
        val intent = Intent(Intent.ACTION_VIEW)
            .setDataAndType(uri, mimeType(file))
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        launchChooser(context, intent, "打开文件")
    }

    fun shareFile(context: Context, file: File) {
        val uri = uriFor(context, file)
        val intent = Intent(Intent.ACTION_SEND)
            .setType(mimeType(file))
            .putExtra(Intent.EXTRA_STREAM, uri)
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        launchChooser(context, intent, "分享文件")
    }

    fun safeFileName(value: String): String {
        val cleaned = value
            .replace(Regex("""[<>:"/\\|?*\p{Cntrl}]"""), "_")
            .trim()
            .trim('.')
        return cleaned.ifBlank { "file.bin" }.take(120)
    }

    private fun writeFile(dir: File, name: String, bytes: ByteArray): File {
        val out = File(dir, name)
        FileOutputStream(out).use { stream -> stream.write(bytes) }
        return out
    }

    private fun mimeType(file: File): String {
        val extension = file.extension.lowercase()
        return MimeTypeMap.getSingleton().getMimeTypeFromExtension(extension)
            ?: if (extension == "png") "image/png" else "application/octet-stream"
    }

    private fun launchChooser(context: Context, intent: Intent, title: String) {
        val chooser = Intent.createChooser(intent, title)
        if (context !is Activity) {
            chooser.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        }
        runCatching { context.startActivity(chooser) }
            .onFailure { LanUi.showToast(context, "没有可用的应用") }
    }
}
