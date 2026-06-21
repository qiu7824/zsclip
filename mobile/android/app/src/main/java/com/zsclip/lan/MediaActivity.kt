package com.zsclip.lan

import android.app.Activity
import android.content.res.Configuration
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.util.TypedValue
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.Button
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import kotlin.concurrent.thread

class MediaActivity : Activity() {
    private val main = Handler(Looper.getMainLooper())
    private lateinit var palette: Palette
    private lateinit var statusView: TextView
    private lateinit var listView: LinearLayout
    private lateinit var allButton: Button
    private lateinit var imageButton: Button
    private lateinit var fileButton: Button
    private var filter = Filter.All
    private var items: List<HistoryItem> = emptyList()
    private val thumbnails = mutableMapOf<String, Bitmap?>()
    private val loadingThumbnails = mutableSetOf<String>()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        palette = Palette.from(this)
        setContentView(createContentView())
        loadItems()
    }

    private fun createContentView(): ScrollView {
        val scroll = ScrollView(this).apply {
            setBackgroundColor(palette.bg)
            isFillViewport = true
        }
        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(16), dp(16), dp(16), dp(22))
        }
        root.addView(TextView(this).apply {
            text = "图片和文件"
            setTextColor(palette.text)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 24f)
            typeface = Typeface.DEFAULT_BOLD
        }, matchWrap())
        root.addView(TextView(this).apply {
            text = "最近从电脑同步的图片和文件"
            setTextColor(palette.muted)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 14f)
        }, topMargin(matchWrap(), 2))

        statusView = TextView(this).apply {
            text = "正在加载..."
            setTextColor(palette.muted)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 14f)
        }
        root.addView(statusView, topMargin(matchWrap(), 12))

        allButton = filterButton("全部") { setFilter(Filter.All) }
        imageButton = filterButton("图片") { setFilter(Filter.Images) }
        fileButton = filterButton("文件") { setFilter(Filter.Files) }
        root.addView(buttonRow(allButton, imageButton, fileButton), topMargin(matchWrap(), 12))

        listView = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
        }
        root.addView(listView, topMargin(matchWrap(), 12))
        scroll.addView(root)
        render()
        return scroll
    }

    private fun loadItems() {
        statusView.text = "正在加载..."
        thread(name = "zsclip-media-history") {
            val result = runCatching {
                when {
                    LanPrefs.hasPairing(this) ->
                        LanClient.fetchMobileHistoryItems(this, 50).map { HistoryItem.fromLan(it) } to
                            "局域网历史：最多显示最近 50 条"
                    LanPrefs.hasWebDavConfig(this) ->
                        loadWebDavLatest()
                    else ->
                        emptyList<HistoryItem>() to "请先完成配对或配置 WebDAV"
                }
            }
            main.post {
                result.fold(
                    onSuccess = { (loaded, message) ->
                        items = loaded
                        statusView.text = if (loaded.isEmpty()) message else "$message，${loaded.size} 条"
                        render()
                    },
                    onFailure = {
                        items = emptyList()
                        val message = "加载失败：${it.message}"
                        statusView.text = message
                        LanPrefs.saveSyncStatus(this, false, message)
                        render()
                    }
                )
            }
        }
    }

    private fun loadWebDavLatest(): Pair<List<HistoryItem>, String> {
        val clip = LanClient.fetchWebDavMultiSyncClip(this)
        return when {
            clip == null ->
                emptyList<HistoryItem>() to "WebDAV 最新清单暂无记录"
            clip.kind == "image" && clip.hasData ->
                listOf(HistoryItem.fromWebDav(clip)) to "WebDAV 仅显示最新图片"
            else ->
                emptyList<HistoryItem>() to "WebDAV 最新记录不是可预览图片"
        }
    }

    private fun setFilter(next: Filter) {
        filter = next
        render()
    }

    private fun render() {
        if (!::listView.isInitialized) {
            return
        }
        renderFilterButtons()
        listView.removeAllViews()
        val filtered = when (filter) {
            Filter.All -> items
            Filter.Images -> items.filter { it.kind == "image" }
            Filter.Files -> items.filter { it.kind == "files" }
        }
        if (filtered.isEmpty()) {
            listView.addView(emptyView(), matchWrap())
            return
        }
        filtered.forEach { item ->
            val row = if (item.kind == "image") imageRow(item) else fileRow(item)
            listView.addView(row, topMargin(matchWrap(), 10))
        }
    }

    private fun renderFilterButtons() {
        if (!::allButton.isInitialized) {
            return
        }
        allButton.text = "全部 ${items.size}"
        imageButton.text = "图片 ${items.count { it.kind == "image" }}"
        fileButton.text = "文件 ${items.count { it.kind == "files" }}"
        listOf(allButton to Filter.All, imageButton to Filter.Images, fileButton to Filter.Files).forEach { (button, value) ->
            val active = value == filter
            button.setTextColor(if (active) palette.onPrimary else palette.text)
            button.background = rounded(if (active) palette.primary else palette.button)
        }
    }

    private fun imageRow(item: HistoryItem): LinearLayout =
        card().apply {
            val preview = item.preview.ifBlank { "图片 ${item.id}" }
            addView(rowTitle(preview), matchWrap())
            addView(rowMeta(item.metaText()), topMargin(matchWrap(), 4))
            val image = ImageView(this@MediaActivity).apply {
                setBackgroundColor(palette.input)
                scaleType = ImageView.ScaleType.CENTER_CROP
                contentDescription = preview
                setOnClickListener { openImagePreview(item) }
            }
            addView(image, topMargin(LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                dp(160)
            ), 10))
            addView(actionButton("预览图片", primary = true) { openImagePreview(item) }, topMargin(matchWrap(), 10))
            loadThumbnail(item, image)
        }

    private fun fileRow(item: HistoryItem): LinearLayout =
        card().apply {
            addView(rowTitle(item.preview.ifBlank { "文件 ${item.id}" }), matchWrap())
            addView(rowMeta(item.metaText()), topMargin(matchWrap(), 4))
            item.files.forEach { file ->
                addView(fileActionRow(item, file), topMargin(matchWrap(), 10))
            }
        }

    private fun fileActionRow(item: HistoryItem, file: LanClient.MobileFileItem): LinearLayout {
        val row = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
        }
        row.addView(rowMeta("${file.name}  ${formatSize(file.size)}"), matchWrap())
        row.addView(buttonRow(
            actionButton("打开") { downloadAndUse(item, file, share = false) },
            actionButton("分享", primary = true) { downloadAndUse(item, file, share = true) }
        ), topMargin(matchWrap(), 6))
        return row
    }

    private fun loadThumbnail(item: HistoryItem, imageView: ImageView) {
        val key = item.thumbKey
        if (thumbnails.containsKey(key)) {
            imageView.setImageBitmap(thumbnails[key])
            return
        }
        if (!loadingThumbnails.add(key)) {
            return
        }
        thread(name = "zsclip-thumb") {
            val bitmap = runCatching {
                val bytes = if (item.webDav) {
                    LanClient.fetchLatestWebDavImageBytes(this).first
                } else {
                    LanClient.fetchMobileImageBytes(this, item.id)
                }
                decodeBitmap(bytes, 900)
            }.getOrNull()
            main.post {
                thumbnails[key] = bitmap
                loadingThumbnails.remove(key)
                imageView.setImageBitmap(bitmap)
            }
        }
    }

    private fun openImagePreview(item: HistoryItem) {
        val intent = if (item.webDav) {
            ImagePreviewActivity.webDavIntent(this, item.preview.ifBlank { "WebDAV 图片" })
        } else {
            ImagePreviewActivity.lanIntent(this, item.id, item.preview.ifBlank { "图片 ${item.id}" })
        }
        startActivity(intent)
    }

    private fun downloadAndUse(item: HistoryItem, file: LanClient.MobileFileItem, share: Boolean) {
        statusView.text = "正在下载：${file.name}"
        thread(name = "zsclip-file-download") {
            val result = runCatching { LanClient.downloadMobileFile(this, item.id, file) }
            main.post {
                result.fold(
                    onSuccess = { downloaded ->
                        statusView.text = "已下载：${downloaded.name}"
                        if (share) {
                            AndroidFileActions.shareFile(this, downloaded)
                        } else {
                            AndroidFileActions.openFile(this, downloaded)
                        }
                    },
                    onFailure = {
                        val message = "文件下载失败：${it.message}"
                        statusView.text = message
                        LanPrefs.saveSyncStatus(this, false, message)
                    }
                )
            }
        }
    }

    private fun decodeBitmap(bytes: ByteArray, maxSide: Int): Bitmap? {
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeByteArray(bytes, 0, bytes.size, bounds)
        var sample = 1
        while (bounds.outWidth / sample > maxSide || bounds.outHeight / sample > maxSide) {
            sample *= 2
        }
        return BitmapFactory.decodeByteArray(
            bytes,
            0,
            bytes.size,
            BitmapFactory.Options().apply { inSampleSize = sample }
        )
    }

    private fun emptyView(): TextView =
        rowMeta(
            when (filter) {
                Filter.All -> "暂无可显示记录"
                Filter.Images -> "暂无图片记录"
                Filter.Files -> "暂无文件记录"
            }
        ).apply {
            gravity = Gravity.CENTER
            setPadding(dp(12), dp(28), dp(12), dp(28))
            background = rounded(palette.surface)
        }

    private fun card(): LinearLayout =
        LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(14), dp(14), dp(14), dp(14))
            background = rounded(palette.surface)
            elevation = dp(1).toFloat()
        }

    private fun rowTitle(text: String): TextView =
        TextView(this).apply {
            this.text = text
            setTextColor(palette.text)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 16f)
            typeface = Typeface.DEFAULT_BOLD
            maxLines = 2
        }

    private fun rowMeta(text: String): TextView =
        TextView(this).apply {
            this.text = text
            setTextColor(palette.muted)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 13f)
            setLineSpacing(dp(2).toFloat(), 1.0f)
        }

    private fun actionButton(text: String, primary: Boolean = false, onClick: () -> Unit): Button =
        Button(this).apply {
            this.text = text
            setAllCaps(false)
            minHeight = dp(44)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 14f)
            setTextColor(if (primary) palette.onPrimary else palette.text)
            background = rounded(if (primary) palette.primary else palette.button)
            setOnClickListener { onClick() }
        }

    private fun filterButton(text: String, onClick: () -> Unit): Button =
        actionButton(text, onClick = onClick)

    private fun buttonRow(left: Button, right: Button): LinearLayout =
        LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            addView(left, LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f).apply {
                marginEnd = dp(6)
            })
            addView(right, LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f).apply {
                marginStart = dp(6)
            })
        }

    private fun buttonRow(first: Button, second: Button, third: Button): LinearLayout =
        LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            addView(first, LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f).apply {
                marginEnd = dp(5)
            })
            addView(second, LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f).apply {
                marginStart = dp(5)
                marginEnd = dp(5)
            })
            addView(third, LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f).apply {
                marginStart = dp(5)
            })
        }

    private fun rounded(color: Int): GradientDrawable =
        GradientDrawable().apply {
            setColor(color)
            cornerRadius = dp(8).toFloat()
            setStroke(dp(1), palette.outline)
        }

    private fun matchWrap() = LinearLayout.LayoutParams(
        ViewGroup.LayoutParams.MATCH_PARENT,
        ViewGroup.LayoutParams.WRAP_CONTENT
    )

    private fun topMargin(params: LinearLayout.LayoutParams, top: Int): LinearLayout.LayoutParams =
        params.apply { topMargin = dp(top) }

    private fun dp(value: Int): Int =
        (value * resources.displayMetrics.density + 0.5f).toInt()

    private fun formatSize(bytes: Long): String =
        when {
            bytes >= 1024L * 1024L -> "%.1f MB".format(bytes / 1024.0 / 1024.0)
            bytes >= 1024L -> "%.1f KB".format(bytes / 1024.0)
            else -> "$bytes B"
        }

    private enum class Filter {
        All,
        Images,
        Files
    }

    private data class HistoryItem(
        val id: Long,
        val kind: String,
        val preview: String,
        val sourceApp: String,
        val createdAt: String,
        val size: Long,
        val width: Int?,
        val height: Int?,
        val files: List<LanClient.MobileFileItem>,
        val webDav: Boolean
    ) {
        val thumbKey: String = if (webDav) "webdav-latest" else "lan-$id"

        fun metaText(): String {
            val parts = mutableListOf<String>()
            if (sourceApp.isNotBlank()) parts += sourceApp
            if (createdAt.isNotBlank()) parts += createdAt
            if (size > 0) parts += formatSizeStatic(size)
            if (width != null && height != null) parts += "${width}x$height"
            if (webDav) parts += "WebDAV 最新项"
            return parts.joinToString(" · ")
        }

        companion object {
            fun fromLan(item: LanClient.MobileHistoryItem): HistoryItem =
                HistoryItem(
                    id = item.id,
                    kind = item.kind,
                    preview = item.preview,
                    sourceApp = item.sourceApp,
                    createdAt = item.createdAt,
                    size = item.size,
                    width = item.width,
                    height = item.height,
                    files = item.files,
                    webDav = false
                )

            fun fromWebDav(clip: LanProtocol.MultiSyncClip): HistoryItem =
                HistoryItem(
                    id = -1,
                    kind = "image",
                    preview = clip.preview.ifBlank { clip.dataName ?: "WebDAV 图片" },
                    sourceApp = "WebDAV",
                    createdAt = "",
                    size = clip.size,
                    width = null,
                    height = null,
                    files = emptyList(),
                    webDav = true
                )

            private fun formatSizeStatic(bytes: Long): String =
                when {
                    bytes >= 1024L * 1024L -> "%.1f MB".format(bytes / 1024.0 / 1024.0)
                    bytes >= 1024L -> "%.1f KB".format(bytes / 1024.0)
                    else -> "$bytes B"
                }
        }
    }

    private data class Palette(
        val bg: Int,
        val surface: Int,
        val input: Int,
        val text: Int,
        val muted: Int,
        val primary: Int,
        val onPrimary: Int,
        val button: Int,
        val outline: Int
    ) {
        companion object {
            fun from(activity: Activity): Palette {
                val dark = (activity.resources.configuration.uiMode and
                    Configuration.UI_MODE_NIGHT_MASK) == Configuration.UI_MODE_NIGHT_YES
                val primary = if (Build.VERSION.SDK_INT >= 31) {
                    activity.systemColor("system_accent1_600", if (dark) 0xFF8AB4F8.toInt() else 0xFF006DCC.toInt())
                } else if (dark) 0xFF8AB4F8.toInt() else 0xFF006DCC.toInt()
                return if (dark) {
                    Palette(0xFF101418.toInt(), 0xFF1B2026.toInt(), 0xFF222831.toInt(), 0xFFE6E9EF.toInt(), 0xFFB6BDC7.toInt(), primary, 0xFF081018.toInt(), 0xFF253140.toInt(), 0xFF343D49.toInt())
                } else {
                    Palette(0xFFF7F8FC.toInt(), 0xFFFFFFFF.toInt(), 0xFFF3F6FA.toInt(), 0xFF1D1B20.toInt(), 0xFF5F6368.toInt(), primary, Color.WHITE, 0xFFE7F0FA.toInt(), 0xFFDCE2EA.toInt())
                }
            }

            private fun Activity.systemColor(name: String, fallback: Int): Int {
                val id = resources.getIdentifier(name, "color", "android")
                return if (id != 0) runCatching { getColor(id) }.getOrDefault(fallback) else fallback
            }
        }
    }
}
