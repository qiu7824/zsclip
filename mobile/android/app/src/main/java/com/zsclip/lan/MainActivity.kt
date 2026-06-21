package com.zsclip.lan

import android.Manifest
import android.app.Activity
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Intent
import android.content.pm.PackageManager
import android.content.res.Configuration
import android.database.Cursor
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.provider.OpenableColumns
import android.text.TextUtils
import android.util.TypedValue
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.view.WindowInsets
import android.widget.Button
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import java.io.ByteArrayOutputStream
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.concurrent.thread

class MainActivity : Activity() {
    private val main = Handler(Looper.getMainLooper())
    private val clipboardAutoPushRunning = AtomicBoolean(false)
    private lateinit var palette: AppPalette
    private lateinit var statusView: TextView
    private lateinit var listView: LinearLayout
    private lateinit var connectionCardView: LinearLayout
    private lateinit var allButton: Button
    private lateinit var textButton: Button
    private lateinit var imageButton: Button
    private lateinit var fileButton: Button
    private var filter = Filter.All
    private var items: List<HistoryItem> = emptyList()
    private val thumbnails = mutableMapOf<String, Bitmap?>()
    private val loadingThumbnails = mutableSetOf<String>()
    private var clipboardListener: ClipboardManager.OnPrimaryClipChangedListener? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        palette = AppPalette.from(this)
        requestNotificationPermission()
        configureSystemBars()
        setContentView(createContentView())
        installClipboardAutoPushListener()
        handleLaunchIntent(intent)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        handleLaunchIntent(intent)
    }

    override fun onResume() {
        super.onResume()
        LanAutoSyncService.resumeIfEnabled(this)
        autoPushClipboardIfNeeded()
        loadItems()
    }

    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (hasFocus) {
            autoPushClipboardIfNeeded()
        }
    }

    override fun onDestroy() {
        removeClipboardAutoPushListener()
        super.onDestroy()
    }

    private fun createContentView(): View {
        val scroll = ScrollView(this).apply {
            setBackgroundColor(palette.bg)
            isFillViewport = true
        }
        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(16), dp(16), dp(16), dp(22))
        }
        applyInsets(root)

        val header = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
        }
        header.addView(LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            addView(TextView(this@MainActivity).apply {
                text = "剪贴板记录"
                setTextColor(palette.text)
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 24f)
                typeface = Typeface.DEFAULT_BOLD
            }, matchWrap())
            addView(TextView(this@MainActivity).apply {
                text = "文本、图片和文件"
                setTextColor(palette.muted)
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 14f)
            }, topMargin(matchWrap(), 2))
        }, LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f))
        header.addView(settingsIconButton {
            openSettings()
        }, LinearLayout.LayoutParams(dp(44), dp(44)))
        root.addView(header, matchWrap())

        connectionCardView = compactConnectionCard()
        root.addView(connectionCardView, topMargin(matchWrap(), 12))

        statusView = TextView(this).apply {
            setTextColor(palette.muted)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 14f)
        }
        root.addView(statusView, topMargin(matchWrap(), 12))

        allButton = filterButton("全部") { setFilter(Filter.All) }
        textButton = filterButton("文本") { setFilter(Filter.Text) }
        imageButton = filterButton("图片") { setFilter(Filter.Images) }
        fileButton = filterButton("文件") { setFilter(Filter.Files) }
        root.addView(filterRow(allButton, textButton, imageButton, fileButton), topMargin(matchWrap(), 10))

        listView = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
        }
        root.addView(listView, topMargin(matchWrap(), 10))
        scroll.addView(root)
        render()
        return scroll
    }

    private fun compactConnectionCard(): LinearLayout =
        card().apply {
            addView(rowTitle("连接电脑"), matchWrap())
            addView(rowMeta("连接后显示电脑剪贴板记录。手机后台复制请用快捷磁贴或分享入口推送。"), topMargin(matchWrap(), 4))
            addView(actionButton("连接设置") { openSettings() }, topMargin(matchWrap(), 8))
        }

    private fun loadItems() {
        connectionCardView.visibility = if (LanPrefs.hasPairing(this)) View.GONE else View.VISIBLE
        statusView.text = "正在加载..."
        thread(name = "zsclip-history-home") {
            val result = runCatching {
                when {
                    LanPrefs.hasPairing(this) ->
                        LanClient.fetchMobileHistoryItems(this, 80).map { HistoryItem.fromLan(it) } to
                            "最近记录"
                    LanPrefs.hasWebDavConfig(this) ->
                        loadWebDavLatest()
                    else ->
                        emptyList<HistoryItem>() to "请先在设置中连接电脑"
                }
            }
            main.post {
                result.fold(
                    onSuccess = { (loaded, message) ->
                        items = loaded
                        statusView.text = if (loaded.isEmpty()) message else "$message · ${loaded.size} 条"
                        render()
                    },
                    onFailure = {
                        statusView.text = "加载失败：${it.message}"
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
            clip.kind == "text" ->
                listOf(HistoryItem.fromWebDavText(clip)) to "WebDAV 最新文本"
            clip.kind == "image" && clip.hasData ->
                listOf(HistoryItem.fromWebDavImage(clip)) to "WebDAV 最新图片"
            else ->
                emptyList<HistoryItem>() to "WebDAV 最新记录暂不支持预览"
        }
    }

    private fun setFilter(next: Filter) {
        filter = next
        render()
    }

    private fun render() {
        if (!::listView.isInitialized) return
        renderFilterButtons()
        listView.removeAllViews()
        val filtered = when (filter) {
            Filter.All -> items
            Filter.Text -> items.filter { it.kind == "text" }
            Filter.Images -> items.filter { it.kind == "image" }
            Filter.Files -> items.filter { it.kind == "files" }
        }
        if (filtered.isEmpty()) {
            listView.addView(emptyView(), matchWrap())
            return
        }
        filtered.forEach { item ->
            val row = when (item.kind) {
                "text" -> textRow(item)
                "image" -> imageRow(item)
                else -> fileRow(item)
            }
            listView.addView(row, topMargin(matchWrap(), 10))
        }
    }

    private fun renderFilterButtons() {
        if (!::allButton.isInitialized) return
        allButton.text = "全部 ${items.size}"
        textButton.text = "文本 ${items.count { it.kind == "text" }}"
        imageButton.text = "图片 ${items.count { it.kind == "image" }}"
        fileButton.text = "文件 ${items.count { it.kind == "files" }}"
        listOf(
            allButton to Filter.All,
            textButton to Filter.Text,
            imageButton to Filter.Images,
            fileButton to Filter.Files
        ).forEach { (button, value) ->
            val active = value == filter
            button.setTextColor(if (active) palette.onPrimary else palette.text)
            button.background = rounded(if (active) palette.primary else palette.button)
        }
    }

    private fun textRow(item: HistoryItem): LinearLayout =
        card().apply {
            val text = item.text.ifBlank { item.preview }
            addView(rowTitle(item.preview.ifBlank { text.take(40).ifBlank { "文本 ${item.id}" } }), matchWrap())
            addView(rowMeta(item.metaText()), topMargin(matchWrap(), 4))
            addView(TextView(this@MainActivity).apply {
                this.text = text
                setTextColor(palette.text)
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 14f)
                setLineSpacing(dp(3).toFloat(), 1.0f)
                maxLines = 5
                ellipsize = TextUtils.TruncateAt.END
            }, topMargin(matchWrap(), 8))
            addView(actionButton("复制到手机", primary = true) { copyTextItem(item) }, topMargin(matchWrap(), 10))
            setOnClickListener { copyTextItem(item) }
        }

    private fun imageRow(item: HistoryItem): LinearLayout =
        card().apply {
            val preview = item.preview.ifBlank { "图片 ${item.id}" }
            addView(rowTitle(preview), matchWrap())
            addView(rowMeta(item.metaText()), topMargin(matchWrap(), 4))
            val image = ImageView(this@MainActivity).apply {
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

    private fun fileActionRow(item: HistoryItem, file: LanClient.MobileFileItem): LinearLayout =
        LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            addView(rowMeta("${file.name}  ${formatSize(file.size)}"), matchWrap())
            addView(buttonRow(
                actionButton("打开") { downloadAndUse(item, file, share = false) },
                actionButton("分享", primary = true) { downloadAndUse(item, file, share = true) }
            ), topMargin(matchWrap(), 6))
        }

    private fun copyTextItem(item: HistoryItem) {
        val text = item.text.ifBlank { item.preview }
        if (text.isBlank()) {
            return
        }
        val signature = LanProtocol.clipboardTextSignature(text)
        LanPrefs.saveLastRemoteClipboardSignature(this, signature)
        LanPrefs.saveLastLocalClipboardSignature(this, signature)
        val clipboard = getSystemService(CLIPBOARD_SERVICE) as ClipboardManager
        clipboard.setPrimaryClip(ClipData.newPlainText("剪贴板同步", text))
        statusView.text = "已复制到手机剪贴板"
    }

    private fun loadThumbnail(item: HistoryItem, imageView: ImageView) {
        val key = item.thumbKey
        if (thumbnails.containsKey(key)) {
            imageView.setImageBitmap(thumbnails[key])
            return
        }
        if (!loadingThumbnails.add(key)) return
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
                        statusView.text = "文件下载失败：${it.message}"
                    }
                )
            }
        }
    }

    private fun handleLaunchIntent(intent: Intent?) {
        if (handlePairLink(intent)) return
        val sharedText = extractSharedText(intent)
        if (sharedText.isNotBlank()) {
            pushSharedText(sharedText)
        }
        val imageUris = extractSharedImageUris(intent)
        if (imageUris.isNotEmpty()) {
            pushImages(imageUris)
        }
    }

    private fun handlePairLink(intent: Intent?): Boolean {
        val host = LanProtocol.pairHostFromLink(intent?.data?.toString()) ?: return false
        LanPrefs.saveCandidate(this, LanClient.normalizedHost(host), "Windows")
        startActivity(Intent(this, SettingsActivity::class.java).apply {
            action = intent?.action
            data = intent?.data
        })
        return true
    }

    private fun pushSharedText(text: String) {
        statusView.text = "正在推送分享文本..."
        thread(name = "zsclip-share-text") {
            val message = try {
                LanClient.pushTextToAvailableTransport(this, text)
            } catch (e: Exception) {
                "文本分享失败：${e.message}"
            }
            main.post {
                statusView.text = message
                loadItems()
            }
        }
    }

    private fun pushImages(uris: List<Uri>) {
        if (!LanPrefs.hasPairing(this) && !LanPrefs.hasWebDavConfig(this)) {
            statusView.text = "请先在设置中连接电脑或配置 WebDAV"
            openSettings()
            return
        }
        statusView.text = "正在推送 ${uris.size} 张图片..."
        thread(name = "zsclip-share-images") {
            var sent = 0
            var skipped = 0
            for (uri in uris) {
                val name = displayNameForUri(uri)
                val pngBytes = runCatching { loadSharedImageAsPng(uri) }.getOrNull()
                if (pngBytes == null) {
                    skipped += 1
                    continue
                }
                try {
                    LanClient.pushImageToAvailableTransport(this, pngBytes, name)
                    sent += 1
                } catch (_: Exception) {
                    skipped += 1
                }
            }
            main.post {
                statusView.text = "图片分享完成：成功 $sent 张，跳过 $skipped 张"
                loadItems()
            }
        }
    }

    private fun extractSharedText(intent: Intent?): String {
        if (intent == null || intent.action != Intent.ACTION_SEND) return ""
        val type = intent.type.orEmpty()
        if (!type.startsWith("text/")) return ""
        return LanProtocol.cleanShareText(intent.getCharSequenceExtra(Intent.EXTRA_TEXT)?.toString())
    }

    private fun extractSharedImageUris(intent: Intent?): List<Uri> {
        if (intent == null) return emptyList()
        val type = intent.type.orEmpty()
        if (!type.startsWith("image/")) return emptyList()
        return when (intent.action) {
            Intent.ACTION_SEND -> {
                @Suppress("DEPRECATION")
                listOfNotNull(intent.getParcelableExtra<Uri>(Intent.EXTRA_STREAM))
            }
            Intent.ACTION_SEND_MULTIPLE -> {
                @Suppress("DEPRECATION")
                intent.getParcelableArrayListExtra<Uri>(Intent.EXTRA_STREAM)?.toList().orEmpty()
            }
            else -> emptyList()
        }
    }

    private fun loadSharedImageAsPng(uri: Uri): ByteArray? {
        val bitmap = contentResolver.openInputStream(uri).use { input ->
            input?.let { BitmapFactory.decodeStream(it) }
        } ?: return null
        return try {
            ByteArrayOutputStream().use { out ->
                if (!bitmap.compress(Bitmap.CompressFormat.PNG, 100, out)) null else out.toByteArray()
            }
        } finally {
            bitmap.recycle()
        }
    }

    private fun displayNameForUri(uri: Uri): String? {
        var cursor: Cursor? = null
        return try {
            cursor = contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)
            if (cursor != null && cursor.moveToFirst()) cursor.getString(0) else uri.lastPathSegment
        } catch (_: Exception) {
            uri.lastPathSegment
        } finally {
            cursor?.close()
        }
    }

    private fun installClipboardAutoPushListener() {
        val clipboard = getSystemService(CLIPBOARD_SERVICE) as ClipboardManager
        val listener = ClipboardManager.OnPrimaryClipChangedListener {
            autoPushClipboardIfNeeded()
        }
        clipboard.addPrimaryClipChangedListener(listener)
        clipboardListener = listener
    }

    private fun removeClipboardAutoPushListener() {
        val listener = clipboardListener ?: return
        val clipboard = getSystemService(CLIPBOARD_SERVICE) as ClipboardManager
        clipboard.removePrimaryClipChangedListener(listener)
        clipboardListener = null
    }

    private fun autoPushClipboardIfNeeded() {
        if (!LanAutoSyncService.isEnabled(this)) return
        if (!LanPrefs.hasPairing(this) && !LanPrefs.hasWebDavConfig(this)) return
        if (!clipboardAutoPushRunning.compareAndSet(false, true)) return
        thread(name = "zsclip-auto-clipboard-push") {
            try {
                LanClient.pushChangedClipboardTextToAvailableTransport(this)
            } catch (_: Exception) {
            } finally {
                clipboardAutoPushRunning.set(false)
            }
        }
    }

    private fun openSettings() {
        startActivity(Intent(this, SettingsActivity::class.java))
    }

    private fun decodeBitmap(bytes: ByteArray, maxSide: Int): Bitmap? {
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeByteArray(bytes, 0, bytes.size, bounds)
        var sample = 1
        while (bounds.outWidth / sample > maxSide || bounds.outHeight / sample > maxSide) sample *= 2
        return BitmapFactory.decodeByteArray(bytes, 0, bytes.size, BitmapFactory.Options().apply {
            inSampleSize = sample
        })
    }

    private fun emptyView(): TextView =
        rowMeta(
            when (filter) {
                Filter.All -> "暂无剪贴板记录"
                Filter.Text -> "暂无文本记录"
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

    private fun settingsIconButton(onClick: () -> Unit): ImageButton =
        ImageButton(this).apply {
            setImageDrawable(iconDrawable(android.R.drawable.ic_menu_manage, palette.muted))
            background = subtleRounded(palette.bg)
            contentDescription = "设置"
            scaleType = ImageView.ScaleType.CENTER
            setPadding(dp(10), dp(10), dp(10), dp(10))
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

    private fun filterRow(first: Button, second: Button, third: Button, fourth: Button): LinearLayout =
        LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            listOf(first, second, third, fourth).forEachIndexed { index, button ->
                addView(button, LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f).apply {
                    if (index > 0) marginStart = dp(4)
                    if (index < 3) marginEnd = dp(4)
                })
            }
        }

    private fun iconDrawable(iconRes: Int, color: Int): Drawable? =
        runCatching {
            getDrawable(iconRes)?.mutate()?.apply { setTint(color) }
        }.getOrNull()

    private fun rounded(color: Int): GradientDrawable =
        GradientDrawable().apply {
            setColor(color)
            cornerRadius = dp(8).toFloat()
            setStroke(dp(1), palette.outline)
        }

    private fun subtleRounded(color: Int): GradientDrawable =
        GradientDrawable().apply {
            setColor(color)
            cornerRadius = dp(8).toFloat()
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

    @Suppress("DEPRECATION")
    private fun configureSystemBars() {
        window.statusBarColor = palette.bg
        window.navigationBarColor = palette.bg
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            window.decorView.systemUiVisibility =
                if (palette.dark) 0 else View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR
        }
    }

    private fun applyInsets(root: LinearLayout) {
        root.setOnApplyWindowInsetsListener { view, insets ->
            @Suppress("DEPRECATION")
            view.setPadding(
                dp(16),
                dp(16) + insets.systemWindowInsetTop,
                dp(16),
                dp(22) + insets.systemWindowInsetBottom
            )
            insets
        }
    }

    private fun requestNotificationPermission() {
        if (Build.VERSION.SDK_INT >= 33 &&
            checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
        ) {
            requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), 100)
        }
    }

    private enum class Filter {
        All,
        Text,
        Images,
        Files
    }

    private data class HistoryItem(
        val id: Long,
        val kind: String,
        val preview: String,
        val text: String,
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
                    text = item.text.orEmpty(),
                    sourceApp = item.sourceApp,
                    createdAt = item.createdAt,
                    size = item.size,
                    width = item.width,
                    height = item.height,
                    files = item.files,
                    webDav = false
                )

            fun fromWebDavText(clip: LanProtocol.MultiSyncClip): HistoryItem =
                HistoryItem(
                    id = -1,
                    kind = "text",
                    preview = clip.preview.ifBlank { clip.content?.take(40).orEmpty() },
                    text = clip.content.orEmpty(),
                    sourceApp = "WebDAV",
                    createdAt = "",
                    size = clip.content.orEmpty().toByteArray().size.toLong(),
                    width = null,
                    height = null,
                    files = emptyList(),
                    webDav = true
                )

            fun fromWebDavImage(clip: LanProtocol.MultiSyncClip): HistoryItem =
                HistoryItem(
                    id = -1,
                    kind = "image",
                    preview = clip.preview.ifBlank { clip.dataName ?: "WebDAV 图片" },
                    text = "",
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

    private data class AppPalette(
        val dark: Boolean,
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
            fun from(activity: Activity): AppPalette {
                val dark = (activity.resources.configuration.uiMode and
                    Configuration.UI_MODE_NIGHT_MASK) == Configuration.UI_MODE_NIGHT_YES
                val primary = if (Build.VERSION.SDK_INT >= 31) {
                    activity.systemColor("system_accent1_600", if (dark) 0xFF8AB4F8.toInt() else 0xFF006DCC.toInt())
                } else if (dark) {
                    0xFF8AB4F8.toInt()
                } else {
                    0xFF006DCC.toInt()
                }
                return if (dark) {
                    AppPalette(true, 0xFF101418.toInt(), 0xFF1B2026.toInt(), 0xFF222831.toInt(), 0xFFE6E9EF.toInt(), 0xFFB6BDC7.toInt(), primary, 0xFF081018.toInt(), 0xFF253140.toInt(), 0xFF343D49.toInt())
                } else {
                    AppPalette(false, 0xFFF7F8FC.toInt(), 0xFFFFFFFF.toInt(), 0xFFF3F6FA.toInt(), 0xFF1D1B20.toInt(), 0xFF5F6368.toInt(), primary, Color.WHITE, 0xFFE7F0FA.toInt(), 0xFFDCE2EA.toInt())
                }
            }

            private fun Activity.systemColor(name: String, fallback: Int): Int {
                val id = resources.getIdentifier(name, "color", "android")
                return if (id != 0) runCatching { getColor(id) }.getOrDefault(fallback) else fallback
            }
        }
    }
}
