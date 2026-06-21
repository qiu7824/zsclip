package com.zsclip.lan

import android.Manifest
import android.app.Activity
import android.app.AlertDialog
import android.app.StatusBarManager
import android.content.ClipData
import android.content.ClipboardManager
import android.content.ComponentName
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
import android.graphics.drawable.Icon
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.provider.OpenableColumns
import android.text.InputType
import android.text.TextUtils
import android.util.TypedValue
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.view.WindowInsets
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import java.io.ByteArrayOutputStream
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.concurrent.thread

class SettingsActivity : Activity() {
    private val main = Handler(Looper.getMainLooper())
    private val logLines = ArrayDeque<String>()
    private val clipboardAutoPushRunning = AtomicBoolean(false)
    private var logExpanded = false
    private var webDavExpanded = false
    private var toolsExpanded = false
    private lateinit var palette: AppPalette

    private lateinit var hostEdit: EditText
    private lateinit var webDavSummaryView: TextView
    private lateinit var webDavDetails: LinearLayout
    private lateinit var webDavToggleButton: Button
    private lateinit var toolsDetails: LinearLayout
    private lateinit var toolsToggleButton: Button
    private lateinit var logView: TextView
    private lateinit var logToggleButton: Button
    private lateinit var autoSyncButton: Button
    private lateinit var notificationStatusButton: Button
    private lateinit var webDavUrlEdit: EditText
    private lateinit var webDavUserEdit: EditText
    private lateinit var webDavPassEdit: EditText
    private lateinit var webDavDirEdit: EditText
    private var clipboardListener: ClipboardManager.OnPrimaryClipChangedListener? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        palette = AppPalette.from(this)
        requestNotificationPermission()
        configureSystemBars()
        setContentView(createContentView())
        installClipboardAutoPushListener()
        append("设置已打开。")
        handleLaunchIntent(intent)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        handleLaunchIntent(intent)
    }

    override fun onResume() {
        super.onResume()
        LanAutoSyncService.resumeIfEnabled(this)?.let { append(it) }
        autoPushClipboardIfNeeded()
        refreshStatus()
        main.postDelayed({ refreshStatus() }, 500)
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

        root.addView(TextView(this).apply {
            text = "设置"
            setTextColor(palette.text)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 24f)
            typeface = Typeface.DEFAULT_BOLD
        }, matchWrap())
        root.addView(TextView(this).apply {
            text = "剪贴板同步"
            setTextColor(palette.muted)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 14f)
        }, topMargin(matchWrap(), 2))

        root.addView(bindingCard(), topMargin(matchWrap(), 14))
        root.addView(webDavCard(), topMargin(matchWrap(), 12))
        root.addView(moreCard(), topMargin(matchWrap(), 12))
        scroll.addView(root)
        refreshStatus()
        return scroll
    }

    private fun bindingCard(): LinearLayout =
        card().apply {
            addView(sectionTitle("连接电脑"), matchWrap())
            hostEdit = EditText(this@SettingsActivity).apply {
                hint = "192.168.1.10:38473"
                setText(LanPrefs.displayHost(this@SettingsActivity))
                setSingleLine(true)
                textSize = 16f
                styleInput(this)
            }
            addView(hostEdit, topMargin(matchWrap(), 8))
            addView(buttonRow(
                actionButton("发现设备", iconRes = android.R.drawable.ic_menu_search) { discover() },
                actionButton("连接", iconRes = android.R.drawable.ic_menu_send, primary = true) { pair() }
            ), topMargin(matchWrap(), 10))
            addView(actionButton("清除配对", iconRes = android.R.drawable.ic_menu_delete) { clearPairing() }, topMargin(matchWrap(), 8))
        }

    private fun webDavCard(): LinearLayout =
        card().apply {
            val config = LanPrefs.webDavConfig(this@SettingsActivity)
            addView(sectionTitle("WebDAV"), matchWrap())
            webDavSummaryView = body("可选，用于外网同步和局域网失败时兜底。")
            addView(webDavSummaryView, topMargin(matchWrap(), 6))
            webDavToggleButton = actionButton("WebDAV 设置", iconRes = android.R.drawable.ic_menu_manage) {
                toggleWebDavDetails()
            }
            addView(webDavToggleButton, topMargin(matchWrap(), 10))
            webDavDetails = LinearLayout(this@SettingsActivity).apply {
                orientation = LinearLayout.VERTICAL
                visibility = View.GONE
            }
            webDavUrlEdit = EditText(this@SettingsActivity).apply {
                hint = "https://dav.example.com/root"
                setText(config.url)
                setSingleLine(true)
                textSize = 15f
                styleInput(this)
            }
            webDavDetails.addView(webDavUrlEdit, topMargin(matchWrap(), 10))
            webDavUserEdit = EditText(this@SettingsActivity).apply {
                hint = "WebDAV 用户名（可空）"
                setText(config.user)
                setSingleLine(true)
                textSize = 15f
                styleInput(this)
            }
            webDavDetails.addView(webDavUserEdit, topMargin(matchWrap(), 8))
            webDavPassEdit = EditText(this@SettingsActivity).apply {
                hint = "WebDAV 密码或应用密码（可空）"
                setText(config.pass)
                setSingleLine(true)
                inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_PASSWORD
                textSize = 15f
                styleInput(this)
            }
            webDavDetails.addView(webDavPassEdit, topMargin(matchWrap(), 8))
            webDavDirEdit = EditText(this@SettingsActivity).apply {
                hint = "远程目录，默认 ZS Clip"
                setText(config.remoteDir)
                setSingleLine(true)
                textSize = 15f
                styleInput(this)
            }
            webDavDetails.addView(webDavDirEdit, topMargin(matchWrap(), 8))
            webDavDetails.addView(buttonRow(
                actionButton("保存设置", iconRes = android.R.drawable.ic_menu_save) { saveWebDavConfig() },
                actionButton("保存并检查", iconRes = android.R.drawable.ic_dialog_info, primary = true) { checkWebDavMultiSyncStatus() }
            ), topMargin(matchWrap(), 10))
            addView(webDavDetails, matchWrap())
        }

    private fun moreCard(): LinearLayout =
        card().apply {
            addView(sectionTitle("更多"), matchWrap())
            autoSyncButton = actionButton("开启自动同步", iconRes = R.drawable.ic_sync_tile) { toggleAutoSync() }
            addView(autoSyncButton, topMargin(matchWrap(), 10))
            notificationStatusButton = actionButton("关闭通知栏状态", iconRes = android.R.drawable.ic_dialog_info) {
                toggleSyncNotification()
            }
            addView(notificationStatusButton, topMargin(matchWrap(), 8))
            toolsToggleButton = actionButton("通知栏快捷开关", iconRes = android.R.drawable.ic_menu_manage) {
                toggleToolsDetails()
            }
            addView(toolsToggleButton, topMargin(matchWrap(), 8))
            toolsDetails = LinearLayout(this@SettingsActivity).apply {
                orientation = LinearLayout.VERTICAL
                visibility = View.GONE
            }
            toolsDetails.addView(buttonRow(
                actionButton("添加：推送到电脑", iconRes = android.R.drawable.ic_menu_add) {
                    requestTile(PushToComputerTileService::class.java, "推送到电脑", R.drawable.ic_pc_tile)
                },
                actionButton("添加：拉取到手机", iconRes = android.R.drawable.ic_menu_add) {
                    requestTile(PushToPhoneTileService::class.java, "拉取到手机", R.drawable.ic_phone_tile)
                }
            ), topMargin(matchWrap(), 10))
            toolsDetails.addView(actionButton("添加：多端自动同步", iconRes = android.R.drawable.ic_menu_add) {
                requestTile(AutoSyncTileService::class.java, "多端自动同步", R.drawable.ic_sync_tile)
            }, topMargin(matchWrap(), 8))
            toolsDetails.addView(buttonRow(
                actionButton("检查状态", iconRes = android.R.drawable.ic_dialog_info) { checkMultiSyncStatus() },
                actionButton("关于", iconRes = android.R.drawable.ic_menu_info_details) { showAbout() }
            ), topMargin(matchWrap(), 8))
            toolsDetails.addView(actionButton("复制诊断", iconRes = android.R.drawable.ic_menu_save) { copyDiagnostics() }, topMargin(matchWrap(), 8))
            logView = body("")
            logView.maxLines = 6
            logView.ellipsize = TextUtils.TruncateAt.END
            toolsDetails.addView(logView, topMargin(matchWrap(), 10))
            logToggleButton = actionButton("展开日志") {
                logExpanded = !logExpanded
                renderLog()
            }
            toolsDetails.addView(logToggleButton, topMargin(matchWrap(), 8))
            addView(toolsDetails, matchWrap())
        }

    private fun toggleWebDavDetails() {
        webDavExpanded = !webDavExpanded
        webDavDetails.visibility = if (webDavExpanded) View.VISIBLE else View.GONE
        webDavToggleButton.text = if (webDavExpanded) "收起 WebDAV 设置" else "WebDAV 设置"
    }

    private fun toggleToolsDetails() {
        toolsExpanded = !toolsExpanded
        toolsDetails.visibility = if (toolsExpanded) View.VISIBLE else View.GONE
        toolsToggleButton.text = if (toolsExpanded) "收起通知栏快捷开关" else "通知栏快捷开关"
    }

    private fun discover() = thread {
        append("开始发现 Windows 设备...")
        try {
            val result = LanClient.discover()
            LanPrefs.saveCandidate(this, result.host, result.name)
            main.post {
                if (!LanPrefs.hasPairing(this)) {
                    hostEdit.setText(result.host)
                }
                refreshStatus()
            }
            append("发现设备：${result.name} ${result.host}")
        } catch (e: Exception) {
            append("未发现设备：${e.message}")
        }
    }

    private fun pair() {
        val enteredHost = hostEdit.text.toString()
        thread {
            val host = LanClient.normalizedHost(enteredHost)
                .ifBlank { LanPrefs.candidateHost(this) }
                .ifBlank { LanPrefs.pairedHost(this) }
            if (host.isBlank()) {
                append("请先输入或发现 Windows IP")
                return@thread
            }
            LanPrefs.saveCandidate(this, host, LanPrefs.candidateName(this))
            try {
                val (pairId, code) = LanClient.requestPair(this, host)
                append("配对请求已发送，请在 Windows 设置 -> 局域网页点击允许。安全码：$code")
                if (LanClient.pollPair(this, host, pairId)) {
                    append("配对成功，绑定已保存")
                    main.post {
                        hostEdit.setText(LanPrefs.pairedHost(this))
                        refreshStatus()
                    }
                } else {
                    append("配对被拒绝或超时")
                }
            } catch (e: Exception) {
                append("配对失败：${e.message}")
            }
        }
    }

    private fun clearPairing() {
        LanAutoSyncService.stop(this)
        LanPrefs.clearPairing(this)
        hostEdit.setText(LanPrefs.candidateHost(this))
        append("已清除配对凭据，可重新请求配对")
        refreshStatus()
    }

    private fun pullToPhone() {
        saveWebDavDraftIfPresent()
        thread {
            try {
                val result = LanClient.pullAvailableTransportToClipboard(this, force = true)
                append(result)
            } catch (e: Exception) {
                val message = "拉取到手机失败：${e.message}"
                LanPrefs.saveSyncStatus(this, false, message)
                append(message)
            }
        }
    }

    private fun openMediaHistory() {
        saveWebDavDraftIfPresent()
        if (!LanPrefs.hasPairing(this) && !LanPrefs.hasWebDavConfig(this)) {
            append("请先完成配对或配置 WebDAV 后再打开图片和文件")
            refreshStatus()
            return
        }
        try {
            startActivity(Intent(this, MediaActivity::class.java))
            append("已打开图片和文件")
        } catch (e: Exception) {
            append("打开图片和文件失败：${e.message}")
        }
    }

    private fun openMobileSetupPage() {
        val host = LanPrefs.pairedHost(this)
            .ifBlank { LanPrefs.candidateHost(this) }
            .ifBlank { hostEdit.text.toString() }
        try {
            val url = LanClient.mobileSetupUrl(host)
            startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)))
            append("已打开多端同步入口")
        } catch (e: Exception) {
            append("打开多端同步入口失败：${e.message}")
        }
    }

    private fun checkMultiSyncStatus() {
        saveWebDavDraftIfPresent()
        thread {
            if (!LanPrefs.hasPairing(this) && !LanPrefs.hasWebDavConfig(this)) {
                append("请先完成配对或配置 WebDAV 后再检查多端同步状态")
                return@thread
            }
            try {
                val result = LanClient.checkAvailableTransportStatus(this)
                append(result.message)
                result.detail?.let { append(it) }
            } catch (e: Exception) {
                val message = "检查多端同步失败：${e.message}"
                LanPrefs.saveSyncStatus(this, false, message)
                append(message)
            }
        }
    }

    private fun saveWebDavConfig() {
        LanPrefs.saveWebDavConfig(
            this,
            webDavUrlEdit.text.toString(),
            webDavUserEdit.text.toString(),
            webDavPassEdit.text.toString(),
            webDavDirEdit.text.toString()
        )
        append("WebDAV 多端同步配置已保存")
    }

    private fun saveWebDavDraftIfPresent(): Boolean {
        if (!::webDavUrlEdit.isInitialized) {
            return LanPrefs.hasWebDavConfig(this)
        }
        val url = webDavUrlEdit.text.toString()
        if (url.isBlank()) {
            return LanPrefs.hasWebDavConfig(this)
        }
        LanPrefs.saveWebDavConfig(
            this,
            url,
            webDavUserEdit.text.toString(),
            webDavPassEdit.text.toString(),
            webDavDirEdit.text.toString()
        )
        refreshStatus()
        return true
    }

    private fun checkWebDavMultiSyncStatus() {
        saveWebDavConfig()
        thread {
            try {
                val config = LanPrefs.webDavConfig(this)
                val clip = LanClient.fetchWebDavMultiSyncClip(this)
                val message = LanProtocol.multiSyncStatusMessage(clip)
                LanPrefs.saveSyncStatus(this, true, "WebDAV：$message")
                append("WebDAV：$message")
                append("清单：${LanClient.webDavManifestUrl(config.url, config.remoteDir)}")
                if (clip?.kind == "image" && clip.dataName != null) {
                    append("图片数据：${LanClient.webDavDataUrl(config.url, config.remoteDir, clip.dataName)}")
                }
            } catch (e: Exception) {
                val message = "检查 WebDAV 多端同步失败：${e.message}"
                LanPrefs.saveSyncStatus(this, false, message)
                append(message)
            }
        }
    }

    private fun pullWebDavToPhone() {
        saveWebDavConfig()
        thread {
            try {
                append(LanClient.pullWebDavToClipboard(this))
            } catch (e: Exception) {
                val message = "拉取 WebDAV 到手机失败：${e.message}"
                LanPrefs.saveSyncStatus(this, false, message)
                append(message)
            }
        }
    }

    private fun downloadWebDavImage() {
        saveWebDavConfig()
        thread {
            try {
                append(LanClient.enqueueLatestWebDavImageDownload(this))
            } catch (e: Exception) {
                val message = "下载 WebDAV 图片失败：${e.message}"
                LanPrefs.saveSyncStatus(this, false, message)
                append(message)
            }
        }
    }

    private fun toggleAutoSync() {
        saveWebDavDraftIfPresent()
        if (LanAutoSyncService.isEnabled(this)) {
            LanAutoSyncService.stop(this)
            append("多端自动同步已关闭")
        } else {
            val error = LanAutoSyncService.start(this)
            if (error == null) {
                append("多端自动同步正在启动")
            } else {
                append(error)
            }
        }
        refreshStatus()
        main.postDelayed({ refreshStatus() }, 500)
    }

    private fun toggleSyncNotification() {
        val next = !LanAutoSyncService.isNotificationEnabled(this)
        val message = LanAutoSyncService.setNotificationEnabled(this, next)
        append(message)
        refreshStatus()
        main.postDelayed({ refreshStatus() }, 500)
    }

    private fun requestTile(serviceClass: Class<*>, label: String, iconRes: Int) {
        if (Build.VERSION.SDK_INT < 33) {
            append("当前 Android 版本需要手动添加快捷开关，请在系统快捷面板中编辑添加“$label”。")
            return
        }
        val manager = getSystemService(StatusBarManager::class.java)
        manager.requestAddTileService(
            ComponentName(this, serviceClass),
            label,
            Icon.createWithResource(this, iconRes),
            mainExecutor
        ) { result ->
            append("快捷开关“$label”添加请求完成：$result")
        }
    }

    private fun copyDiagnostics() {
        val text = buildString {
            append(currentStatusText())
            append("\n\n")
            append(logLines.joinToString("\n"))
        }
        val clipboard = getSystemService(CLIPBOARD_SERVICE) as ClipboardManager
        clipboard.setPrimaryClip(ClipData.newPlainText("ZSClip 多端同步诊断", text))
        append("诊断信息已复制")
    }

    private fun showAbout() {
        val version = packageManager.getPackageInfo(packageName, 0).versionName ?: "0.9.9"
        AlertDialog.Builder(this)
            .setTitle("剪贴板同步")
            .setMessage(
                "版本：$version\n\n" +
                    "用于手机与电脑之间同步剪贴板文本、图片和文件。\n\n" +
                    "开源地址：https://github.com/qiu7824/zsclip/\n" +
                    "发布页：https://github.com/qiu7824/zsclip/releases\n" +
                    "许可：见仓库 LICENSE"
            )
            .setPositiveButton("确定", null)
            .show()
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
        if (!LanAutoSyncService.isEnabled(this)) {
            return
        }
        if (!LanPrefs.hasPairing(this) && !LanPrefs.hasWebDavConfig(this)) {
            return
        }
        if (!clipboardAutoPushRunning.compareAndSet(false, true)) {
            return
        }
        thread(name = "zsclip-auto-clipboard-push") {
            try {
                val message = LanClient.pushChangedClipboardTextToAvailableTransport(this)
                if (!LanProtocol.isAutoSyncNoopMessage(message)) {
                    append(message)
                }
            } catch (e: Exception) {
                val message = "手机剪贴板自动推送失败：${e.message}"
                LanPrefs.saveSyncStatus(this, false, message)
                append(message)
            } finally {
                clipboardAutoPushRunning.set(false)
            }
        }
    }

    private fun append(text: String) {
        main.post {
            logLines.addLast(text)
            while (logLines.size > 80) {
                logLines.removeFirst()
            }
            renderLog()
            refreshStatus()
        }
    }

    private fun renderLog() {
        logView.maxLines = if (logExpanded) Int.MAX_VALUE else 6
        logView.ellipsize = if (logExpanded) null else TextUtils.TruncateAt.END
        logView.text = logLines.joinToString("\n")
        logToggleButton.text = if (logExpanded) "收起日志" else "展开日志"
    }

    private fun handleLaunchIntent(intent: Intent?) {
        val message = intent?.getStringExtra(LanUi.EXTRA_STATUS_MESSAGE)
        if (!message.isNullOrBlank()) {
            append(message)
        }
        if (handlePairLink(intent)) {
            return
        }
    }

    private fun handlePairLink(intent: Intent?): Boolean {
        val host = LanProtocol.pairHostFromLink(intent?.data?.toString()) ?: return false
        val normalized = LanClient.normalizedHost(host)
        if (normalized.isBlank()) {
            append("二维码里的 Windows 地址无效")
            return true
        }
        LanPrefs.saveCandidate(this, normalized, "Windows")
        hostEdit.setText(normalized)
        append("已从二维码读取 Windows 地址，正在请求配对；请在电脑端允许")
        refreshStatus()
        pair()
        return true
    }

    private fun extractSharedText(intent: Intent?): String {
        if (intent == null) {
            return ""
        }
        val explicit = intent.getStringExtra(LanUi.EXTRA_SHARED_TEXT)
        if (!explicit.isNullOrBlank()) {
            return LanProtocol.cleanShareText(explicit)
        }
        if (intent.action != Intent.ACTION_SEND) {
            return ""
        }
        val type = intent.type.orEmpty()
        if (!type.startsWith("text/")) {
            return ""
        }
        return LanProtocol.cleanShareText(intent.getCharSequenceExtra(Intent.EXTRA_TEXT)?.toString())
    }

    private fun extractSharedImageUris(intent: Intent?): List<Uri> {
        if (intent == null) {
            return emptyList()
        }
        val type = intent.type.orEmpty()
        if (!type.startsWith("image/")) {
            return emptyList()
        }
        return when (intent.action) {
            Intent.ACTION_SEND -> {
                @Suppress("DEPRECATION")
                val uri = intent.getParcelableExtra<Uri>(Intent.EXTRA_STREAM)
                listOfNotNull(uri)
            }
            Intent.ACTION_SEND_MULTIPLE -> {
                @Suppress("DEPRECATION")
                intent.getParcelableArrayListExtra<Uri>(Intent.EXTRA_STREAM)?.toList().orEmpty()
            }
            else -> emptyList()
        }
    }

    private fun handleSharedImages(uris: List<Uri>) {
        if (!LanPrefs.hasPairing(this) && !LanPrefs.hasWebDavConfig(this)) {
            append("收到 ${uris.size} 张分享图片，尚未配对；也未配置 WebDAV。")
            return
        }
        val target = if (LanPrefs.hasPairing(this)) "电脑" else "WebDAV"
        append("收到 ${uris.size} 张分享图片，正在推送到$target...")
        pushImages(uris)
    }

    private fun pushImages(uris: List<Uri>) = thread {
        if (!LanPrefs.hasPairing(this) && !LanPrefs.hasWebDavConfig(this)) {
            val message = "请先完成配对或配置 WebDAV 后再推送图片"
            LanPrefs.saveSyncStatus(this, false, message)
            append(message)
            return@thread
        }
        var sent = 0
        var skipped = 0
        for (uri in uris) {
            val name = displayNameForUri(uri)
            val pngBytes = try {
                loadSharedImageAsPng(uri)
            } catch (e: Exception) {
                append("图片读取失败：${name ?: uri} ${e.message}")
                null
            }
            if (pngBytes == null) {
                skipped += 1
                continue
            }
            val base64Chars = ((pngBytes.size + 2) / 3) * 4
            val error = LanProtocol.validateMobileImagePayload(pngBytes.size, base64Chars)
            if (error != null) {
                skipped += 1
                append("${name ?: "图片"}：$error")
                continue
            }
            try {
                append(LanClient.pushImageToAvailableTransport(this, pngBytes, name))
                sent += 1
            } catch (e: Exception) {
                skipped += 1
                val label = name ?: uri.toString()
                val message = "图片推送失败：$label ${e.message}"
                LanPrefs.saveSyncStatus(this, false, message)
                append(message)
            }
        }
        val summary = "图片分享完成：成功 $sent 张，跳过 $skipped 张"
        LanPrefs.saveSyncStatus(this, sent > 0, summary)
        append(summary)
    }

    private fun loadSharedImageAsPng(uri: Uri): ByteArray? {
        val bounds = BitmapFactory.Options().apply {
            inJustDecodeBounds = true
        }
        contentResolver.openInputStream(uri).use { input ->
            if (input == null) {
                return null
            }
            BitmapFactory.decodeStream(input, null, bounds)
        }
        LanProtocol.validateMobileImageDimensions(bounds.outWidth, bounds.outHeight)?.let { error ->
            throw IllegalArgumentException(error)
        }
        val bitmap = contentResolver.openInputStream(uri).use { input ->
            if (input == null) {
                null
            } else {
                BitmapFactory.decodeStream(input)
            }
        } ?: return null
        return try {
            ByteArrayOutputStream().use { out ->
                if (!bitmap.compress(Bitmap.CompressFormat.PNG, 100, out)) {
                    null
                } else {
                    out.toByteArray()
                }
            }
        } finally {
            bitmap.recycle()
        }
    }

    private fun displayNameForUri(uri: Uri): String? {
        var cursor: Cursor? = null
        return try {
            cursor = contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)
            if (cursor != null && cursor.moveToFirst()) {
                cursor.getString(0)
            } else {
                uri.lastPathSegment
            }
        } catch (_: Exception) {
            uri.lastPathSegment
        } finally {
            cursor?.close()
        }
    }

    private fun refreshStatus() {
        val paired = LanPrefs.currentPairing(this)
        val hasWebDav = LanPrefs.hasWebDavConfig(this)
        val webDavConfig = LanPrefs.webDavConfig(this)
        val autoSyncEnabled = LanAutoSyncService.isEnabled(this)
        val syncNotificationEnabled = LanAutoSyncService.isNotificationEnabled(this)
        hostEdit.setText(
            paired?.host
                ?: LanPrefs.candidateHost(this)
        )
        webDavSummaryView.text = if (hasWebDav) {
            "已配置：${webDavConfig.remoteDir}"
        } else {
            "可选，用于外网同步和局域网失败时兜底"
        }
        autoSyncButton.text = if (autoSyncEnabled) "自动同步：开" else "自动同步：关"
        paintToggleButton(autoSyncButton, autoSyncEnabled)
        notificationStatusButton.text = if (syncNotificationEnabled) {
            "通知栏状态：开"
        } else {
            "通知栏状态：关"
        }
        paintToggleButton(notificationStatusButton, syncNotificationEnabled)
    }

    private fun currentStatusText(): String =
        "多端自动同步：${when {
            LanAutoSyncService.isRunning(this) -> "运行中"
            LanAutoSyncService.isEnabled(this) -> "后台运行"
            else -> "关闭"
        }}\n手机复制：App 前台自动推送，后台请用快捷磁贴或分享入口\n通知栏状态：${if (LanAutoSyncService.isNotificationEnabled(this)) "开启" else "关闭"}\n" +
            LanPrefs.lastSyncStatusText(this)

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
            applyLegacyInsets(view, insets)
            insets
        }
    }

    @Suppress("DEPRECATION")
    private fun applyLegacyInsets(view: View, insets: WindowInsets) {
        view.setPadding(
            dp(16),
            dp(16) + insets.systemWindowInsetTop,
            dp(16),
            dp(22) + insets.systemWindowInsetBottom
        )
    }

    private fun card(): LinearLayout =
        LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(16), dp(16), dp(16), dp(16))
            background = rounded(palette.surface, dp(8).toFloat(), palette.outline, 1)
            elevation = dp(1).toFloat()
        }

    private fun sectionTitle(text: String): TextView =
        TextView(this).apply {
            this.text = text
            setTextColor(palette.text)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 17f)
            typeface = Typeface.DEFAULT_BOLD
        }

    private fun title(text: String): TextView =
        TextView(this).apply {
            this.text = text
            setTextColor(palette.text)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 20f)
            typeface = Typeface.DEFAULT_BOLD
        }

    private fun body(text: String): TextView =
        TextView(this).apply {
            this.text = text
            setTextColor(palette.muted)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 14f)
            setLineSpacing(dp(2).toFloat(), 1.0f)
        }

    private fun styleInput(edit: EditText) {
        edit.setTextColor(palette.text)
        edit.setHintTextColor(palette.muted)
        edit.background = rounded(palette.input, dp(8).toFloat(), palette.outline, 1)
        edit.setPadding(dp(12), dp(8), dp(12), dp(8))
    }

    private fun divider(): View =
        View(this).apply {
            setBackgroundColor(palette.outline)
        }.also {
            it.layoutParams = LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                dp(1)
            )
        }

    private fun actionButton(
        text: String,
        iconRes: Int? = null,
        primary: Boolean = false,
        onClick: () -> Unit
    ): Button =
        Button(this).apply {
            this.text = text
            setAllCaps(false)
            minHeight = dp(48)
            minimumHeight = dp(48)
            gravity = Gravity.CENTER
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 15f)
            setTextColor(if (primary) palette.onPrimary else palette.text)
            background = rounded(
                if (primary) palette.primary else palette.button,
                dp(8).toFloat(),
                if (primary) palette.primary else palette.outline,
                1
            )
            iconRes?.let {
                setCompoundDrawablesWithIntrinsicBounds(
                    iconDrawable(it, if (primary) palette.onPrimary else palette.text),
                    null,
                    null,
                    null
                )
                compoundDrawablePadding = dp(8)
            }
            setOnClickListener { onClick() }
        }

    private fun paintToggleButton(button: Button, enabled: Boolean) {
        button.setTextColor(if (enabled) palette.onPrimary else palette.text)
        button.background = rounded(
            if (enabled) palette.primary else palette.button,
            dp(8).toFloat(),
            if (enabled) palette.primary else palette.outline,
            1
        )
    }

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

    private fun iconDrawable(iconRes: Int, color: Int): Drawable? =
        runCatching {
            getDrawable(iconRes)?.mutate()?.apply {
                setTint(color)
            }
        }.getOrNull()

    private fun rounded(
        color: Int,
        radius: Float,
        strokeColor: Int? = null,
        strokeDp: Int = 0
    ): GradientDrawable =
        GradientDrawable().apply {
            setColor(color)
            cornerRadius = radius
            if (strokeColor != null && strokeDp > 0) {
                setStroke(dp(strokeDp), strokeColor)
            }
        }

    private fun matchWrap() = LinearLayout.LayoutParams(
        ViewGroup.LayoutParams.MATCH_PARENT,
        ViewGroup.LayoutParams.WRAP_CONTENT
    )

    private fun topMargin(params: LinearLayout.LayoutParams, top: Int): LinearLayout.LayoutParams =
        params.apply { topMargin = dp(top) }

    private fun dp(value: Int): Int =
        (value * resources.displayMetrics.density + 0.5f).toInt()

    private fun requestNotificationPermission() {
        if (Build.VERSION.SDK_INT >= 33 &&
            checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
        ) {
            requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), 100)
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
                    AppPalette(
                        dark = true,
                        bg = 0xFF101418.toInt(),
                        surface = 0xFF1B2026.toInt(),
                        input = 0xFF222831.toInt(),
                        text = 0xFFE6E9EF.toInt(),
                        muted = 0xFFB6BDC7.toInt(),
                        primary = primary,
                        onPrimary = 0xFF081018.toInt(),
                        button = 0xFF253140.toInt(),
                        outline = 0xFF343D49.toInt()
                    )
                } else {
                    AppPalette(
                        dark = false,
                        bg = 0xFFF7F8FC.toInt(),
                        surface = 0xFFFFFFFF.toInt(),
                        input = 0xFFF3F6FA.toInt(),
                        text = 0xFF1D1B20.toInt(),
                        muted = 0xFF5F6368.toInt(),
                        primary = primary,
                        onPrimary = Color.WHITE,
                        button = 0xFFE7F0FA.toInt(),
                        outline = 0xFFDCE2EA.toInt()
                    )
                }
            }

            private fun Activity.systemColor(name: String, fallback: Int): Int {
                val id = resources.getIdentifier(name, "color", "android")
                return if (id != 0) runCatching { getColor(id) }.getOrDefault(fallback) else fallback
            }
        }
    }
}
