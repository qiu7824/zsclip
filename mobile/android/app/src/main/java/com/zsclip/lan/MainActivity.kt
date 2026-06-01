package com.zsclip.lan

import android.Manifest
import android.app.Activity
import android.app.StatusBarManager
import android.content.ClipData
import android.content.ClipboardManager
import android.content.ComponentName
import android.content.Intent
import android.content.pm.PackageManager
import android.database.Cursor
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.graphics.drawable.Icon
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
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import java.io.ByteArrayOutputStream
import kotlin.concurrent.thread

class MainActivity : Activity() {
    private val main = Handler(Looper.getMainLooper())
    private val logLines = ArrayDeque<String>()
    private var logExpanded = false

    private lateinit var hostEdit: EditText
    private lateinit var textEdit: EditText
    private lateinit var connectionTitle: TextView
    private lateinit var connectionMeta: TextView
    private lateinit var syncStatusView: TextView
    private lateinit var logView: TextView
    private lateinit var logToggleButton: Button
    private lateinit var autoSyncButton: Button

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        requestNotificationPermission()
        configureSystemBars()
        setContentView(createContentView())
        append("先发现或输入 Windows IP，点击请求配对，然后在 Windows 设置 -> 局域网页允许。")
        handleLaunchIntent(intent)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        handleLaunchIntent(intent)
    }

    override fun onResume() {
        super.onResume()
        refreshStatus()
    }

    private fun createContentView(): View {
        val scroll = ScrollView(this).apply {
            setBackgroundColor(COLOR_BG)
            isFillViewport = true
        }
        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(18), dp(18), dp(18), dp(22))
        }
        applyInsets(root)

        root.addView(TextView(this).apply {
            text = "ZSClip LAN"
            setTextColor(COLOR_TEXT)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 26f)
            typeface = Typeface.DEFAULT_BOLD
        }, matchWrap())
        root.addView(TextView(this).apply {
            text = "手机与电脑的局域网剪贴板同步"
            setTextColor(COLOR_MUTED)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 14f)
        }, topMargin(matchWrap(), 2))

        root.addView(connectionCard(), topMargin(matchWrap(), 18))
        root.addView(bindingCard(), topMargin(matchWrap(), 12))
        root.addView(actionsCard(), topMargin(matchWrap(), 12))
        root.addView(tileCard(), topMargin(matchWrap(), 12))
        root.addView(statusCard(), topMargin(matchWrap(), 12))
        root.addView(logCard(), topMargin(matchWrap(), 12))
        scroll.addView(root)
        refreshStatus()
        return scroll
    }

    private fun connectionCard(): LinearLayout =
        card().apply {
            connectionTitle = title("未绑定 Windows")
            connectionMeta = body("请先发现或输入 Windows IP，再请求配对。")
            addView(rowLabel("连接状态", active = LanPrefs.hasPairing(this@MainActivity)), matchWrap())
            addView(connectionTitle, topMargin(matchWrap(), 8))
            addView(connectionMeta, topMargin(matchWrap(), 6))
        }

    private fun bindingCard(): LinearLayout =
        card().apply {
            addView(sectionTitle("绑定 Windows"), matchWrap())
            hostEdit = EditText(this@MainActivity).apply {
                hint = "192.168.1.10:38473"
                setText(LanPrefs.displayHost(this@MainActivity))
                setSingleLine(true)
                textSize = 18f
                setTextColor(COLOR_TEXT)
                setHintTextColor(COLOR_MUTED)
            }
            addView(hostEdit, topMargin(matchWrap(), 8))
            addView(buttonRow(
                actionButton("发现设备") { discover() },
                actionButton("请求配对", primary = true) { pair() }
            ), topMargin(matchWrap(), 10))
            addView(actionButton("清除配对") { clearPairing() }, topMargin(matchWrap(), 8))
        }

    private fun actionsCard(): LinearLayout =
        card().apply {
            addView(sectionTitle("同步操作"), matchWrap())
            textEdit = EditText(this@MainActivity).apply {
                hint = "输入或从其他 App 分享文本后，点击推送到电脑"
                minLines = 3
                maxLines = 5
                textSize = 15f
                setTextColor(COLOR_TEXT)
                setHintTextColor(COLOR_MUTED)
            }
            addView(textEdit, topMargin(matchWrap(), 8))
            addView(actionButton("推送到电脑", primary = true) { pushTextToComputer() }, topMargin(matchWrap(), 10))
            addView(buttonRow(
                actionButton("拉取到手机") { pullToPhone() },
                actionButton("图片下载页") { openImagesPage() }
            ), topMargin(matchWrap(), 8))
            addView(actionButton("复制诊断") { copyDiagnostics() }, topMargin(matchWrap(), 8))
        }

    private fun tileCard(): LinearLayout =
        card().apply {
            addView(sectionTitle("通知栏快捷开关"), matchWrap())
            addView(body("建议添加三个开关：推送到电脑、拉取到手机、局域网自动同步。自动同步只使用已保存配对，不会重新匹配。"), topMargin(matchWrap(), 6))
            addView(buttonRow(
                actionButton("添加：推送到电脑") {
                    requestTile(PushToComputerTileService::class.java, "推送到电脑", R.drawable.ic_pc_tile)
                },
                actionButton("添加：拉取到手机") {
                    requestTile(PushToPhoneTileService::class.java, "拉取到手机", R.drawable.ic_phone_tile)
                }
            ), topMargin(matchWrap(), 10))
            addView(actionButton("添加：局域网自动同步") {
                requestTile(AutoSyncTileService::class.java, "局域网自动同步", R.drawable.ic_sync_tile)
            }, topMargin(matchWrap(), 8))
            autoSyncButton = actionButton("开启局域网自动同步") { toggleAutoSync() }
            addView(autoSyncButton, topMargin(matchWrap(), 8))
        }

    private fun statusCard(): LinearLayout =
        card().apply {
            addView(sectionTitle("最近同步"), matchWrap())
            syncStatusView = body("最近同步：暂无")
            addView(syncStatusView, topMargin(matchWrap(), 8))
        }

    private fun logCard(): LinearLayout =
        card().apply {
            addView(sectionTitle("诊断日志"), matchWrap())
            logView = body("")
            logView.maxLines = 6
            logView.ellipsize = TextUtils.TruncateAt.END
            addView(logView, topMargin(matchWrap(), 8))
            logToggleButton = actionButton("展开日志") {
                logExpanded = !logExpanded
                renderLog()
            }
            addView(logToggleButton, topMargin(matchWrap(), 8))
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

    private fun pair() = thread {
        val host = LanClient.normalizedHost(hostEdit.text.toString())
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
                if (textEdit.text.toString().isNotBlank()) {
                    append("文本已保留，可点击“推送到电脑”发送。")
                }
            } else {
                append("配对被拒绝或超时")
            }
        } catch (e: Exception) {
            append("配对失败：${e.message}")
        }
    }

    private fun clearPairing() {
        LanAutoSyncService.stop(this)
        LanPrefs.clearPairing(this)
        hostEdit.setText(LanPrefs.candidateHost(this))
        append("已清除配对凭据，可重新请求配对")
        refreshStatus()
    }

    private fun pushTextToComputer() = thread {
        val text = textEdit.text.toString()
        if (!LanPrefs.hasPairing(this)) {
            append("请先完成配对后再推送到电脑")
            return@thread
        }
        if (text.isBlank()) {
            append("请输入要推送到电脑的文本，或从其他 App 分享文本到 ZSClip")
            return@thread
        }
        try {
            LanClient.pushText(this, LanPrefs.pairedHost(this), text)
            append("已推送到电脑")
        } catch (e: Exception) {
            val message = "推送到电脑失败：${e.message}"
            LanPrefs.saveSyncStatus(this, false, message)
            append(message)
        }
    }

    private fun pullToPhone() = thread {
        try {
            val result = LanClient.pullLatestToClipboard(this, force = true)
            append(result)
        } catch (e: Exception) {
            val message = "拉取到手机失败：${e.message}"
            LanPrefs.saveSyncStatus(this, false, message)
            append(message)
        }
    }

    private fun openImagesPage() {
        if (!LanPrefs.hasPairing(this)) {
            append("请先完成配对后再打开图片下载页")
            refreshStatus()
            return
        }
        try {
            val url = LanClient.mobileImagesUrl(
                LanPrefs.pairedHost(this),
                LanPrefs.deviceId(this),
                LanPrefs.token(this)
            )
            startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)))
            append("已打开图片下载页")
        } catch (e: Exception) {
            append("打开图片下载页失败：${e.message}")
        }
    }

    private fun toggleAutoSync() {
        if (LanAutoSyncService.isRunning(this)) {
            LanAutoSyncService.stop(this)
            append("局域网自动同步已关闭")
        } else {
            if (!LanPrefs.hasPairing(this)) {
                append("请先完成配对后再开启局域网自动同步")
                refreshStatus()
                return
            }
            LanAutoSyncService.start(this)
            append("局域网自动同步已开启")
        }
        refreshStatus()
    }

    private fun requestTile(serviceClass: Class<*>, label: String, iconRes: Int) {
        if (Build.VERSION.SDK_INT < 33) {
            append("当前 Android 版本需要手动下拉快捷设置，点击编辑后添加“$label”。")
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
        clipboard.setPrimaryClip(ClipData.newPlainText("ZSClip LAN diagnostics", text))
        append("诊断信息已复制")
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
        val sharedText = extractSharedText(intent)
        if (sharedText.isBlank()) {
            val imageUris = extractSharedImageUris(intent)
            if (imageUris.isNotEmpty()) {
                handleSharedImages(imageUris)
            }
            return
        }
        textEdit.setText(sharedText)
        if (LanPrefs.hasPairing(this)) {
            append("收到分享文本，正在推送到电脑...")
            pushTextToComputer()
        } else {
            append("收到分享文本，尚未配对；配对完成后点击“推送到电脑”发送。")
        }
        val imageUris = extractSharedImageUris(intent)
        if (imageUris.isNotEmpty()) {
            handleSharedImages(imageUris)
        }
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
        if (!LanPrefs.hasPairing(this)) {
            append("收到 ${uris.size} 张分享图片，尚未配对；请配对后重新分享图片。")
            return
        }
        append("收到 ${uris.size} 张分享图片，正在推送到电脑...")
        pushImages(uris)
    }

    private fun pushImages(uris: List<Uri>) = thread {
        val host = LanPrefs.pairedHost(this)
        if (!LanPrefs.hasPairing(this)) {
            val message = "请先完成配对后再推送图片"
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
                LanClient.pushImage(this, host, pngBytes, name)
                sent += 1
                append("已推送图片到电脑：${name ?: "未命名图片"}")
            } catch (e: Exception) {
                skipped += 1
                val message = "图片推送失败：${name ?: uri} ${e.message}"
                LanPrefs.saveSyncStatus(this, false, message)
                append(message)
            }
        }
        val summary = "图片分享完成：成功 $sent 张，跳过 $skipped 张"
        LanPrefs.saveSyncStatus(this, sent > 0, summary)
        append(summary)
    }

    private fun loadSharedImageAsPng(uri: Uri): ByteArray? {
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
        if (paired == null) {
            val candidate = LanPrefs.candidateHost(this)
            connectionTitle.text = "未绑定 Windows"
            connectionMeta.text = if (candidate.isBlank()) {
                "请先发现或输入 Windows IP，再请求配对。"
            } else {
                "候选设备：${LanPrefs.candidateName(this).ifBlank { "Windows" }}  $candidate"
            }
        } else {
            connectionTitle.text = "已绑定 ${paired.deviceName.ifBlank { "Windows" }}"
            connectionMeta.text = "目标：${paired.host}\n设备 ID：${paired.deviceId.ifBlank { "未记录" }}"
        }
        syncStatusView.text = currentStatusText()
        autoSyncButton.text = if (LanAutoSyncService.isRunning(this)) {
            "关闭局域网自动同步"
        } else {
            "开启局域网自动同步"
        }
    }

    private fun currentStatusText(): String =
        "自动同步：${if (LanAutoSyncService.isRunning(this)) "开启" else "关闭"}\n" +
            LanPrefs.lastSyncStatusText(this)

    private fun configureSystemBars() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.LOLLIPOP) {
            window.statusBarColor = COLOR_BG
            window.navigationBarColor = COLOR_BG
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            window.decorView.systemUiVisibility = View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR
        }
    }

    private fun applyInsets(root: LinearLayout) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.KITKAT_WATCH) {
            root.setOnApplyWindowInsetsListener { view, insets ->
                view.setPadding(
                    dp(18),
                    dp(18) + insets.systemWindowInsetTop,
                    dp(18),
                    dp(22) + insets.systemWindowInsetBottom
                )
                insets
            }
        }
    }

    private fun card(): LinearLayout =
        LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(16), dp(16), dp(16), dp(16))
            background = rounded(COLOR_CARD, dp(20).toFloat())
            elevation = dp(1).toFloat()
        }

    private fun rowLabel(text: String, active: Boolean): TextView =
        TextView(this).apply {
            this.text = text
            setTextColor(if (active) COLOR_PRIMARY else COLOR_MUTED)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 13f)
            typeface = Typeface.DEFAULT_BOLD
        }

    private fun sectionTitle(text: String): TextView =
        TextView(this).apply {
            this.text = text
            setTextColor(COLOR_TEXT)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 17f)
            typeface = Typeface.DEFAULT_BOLD
        }

    private fun title(text: String): TextView =
        TextView(this).apply {
            this.text = text
            setTextColor(COLOR_TEXT)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 20f)
            typeface = Typeface.DEFAULT_BOLD
        }

    private fun body(text: String): TextView =
        TextView(this).apply {
            this.text = text
            setTextColor(COLOR_MUTED)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 14f)
            setLineSpacing(dp(2).toFloat(), 1.0f)
        }

    private fun actionButton(text: String, primary: Boolean = false, onClick: () -> Unit): Button =
        Button(this).apply {
            this.text = text
            setAllCaps(false)
            minHeight = dp(48)
            gravity = Gravity.CENTER
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 15f)
            setTextColor(if (primary) Color.WHITE else COLOR_TEXT)
            background = rounded(if (primary) COLOR_PRIMARY else COLOR_BUTTON, dp(16).toFloat())
            setOnClickListener { onClick() }
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

    private fun rounded(color: Int, radius: Float): GradientDrawable =
        GradientDrawable().apply {
            setColor(color)
            cornerRadius = radius
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

    companion object {
        private const val COLOR_BG = 0xFFF7F8FC.toInt()
        private const val COLOR_CARD = 0xFFFFFFFF.toInt()
        private const val COLOR_TEXT = 0xFF1D1B20.toInt()
        private const val COLOR_MUTED = 0xFF5F6368.toInt()
        private const val COLOR_PRIMARY = 0xFF006DCC.toInt()
        private const val COLOR_BUTTON = 0xFFE7F0FA.toInt()
    }
}
