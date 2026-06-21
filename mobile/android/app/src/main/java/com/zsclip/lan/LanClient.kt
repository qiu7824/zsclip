package com.zsclip.lan

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.app.DownloadManager
import android.net.Uri
import android.os.Environment
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.io.FileOutputStream
import java.io.OutputStreamWriter
import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.HttpURLConnection
import java.net.InetSocketAddress
import java.net.Socket
import java.net.URLEncoder
import java.net.URL
import java.util.Base64
import java.util.concurrent.atomic.AtomicBoolean
import javax.net.ssl.SSLSocketFactory

object LanClient {
    data class DiscoveryResult(val name: String, val host: String, val deviceId: String)
    data class LatestResult(
        val key: String,
        val messageId: String,
        val kind: String,
        val hash: String,
        val text: String?,
        val preview: String,
        val originDeviceId: String,
        val originSeq: Long
    )
    data class LanTextPushResult(
        val message: String,
        val key: String,
        val signature: String,
        val originDeviceId: String,
        val originSeq: Long
    )
    data class WebDavPushResult(val message: String, val key: String)
    data class MultiSyncStatusResult(
        val clip: LanProtocol.MultiSyncClip?,
        val message: String,
        val detail: String?
    )
    data class WebDavImageDownloadRequest(
        val url: String,
        val fileName: String,
        val authHeader: String?
    )
    data class MobileFileItem(
        val index: Int,
        val name: String,
        val size: Long
    )
    data class MobileHistoryItem(
        val id: Long,
        val kind: String,
        val preview: String,
        val text: String?,
        val sourceApp: String,
        val createdAt: String,
        val size: Long,
        val width: Int?,
        val height: Int?,
        val files: List<MobileFileItem>
    )
    data class PendingRemoteText(
        val key: String,
        val preview: String,
        val transport: String
    )
    private var lastOriginSeq: Long = 0
    private val autoClipboardPushInFlight = AtomicBoolean(false)
    private const val RECENT_OWN_PUSH_SKIP_MS = 10 * 60 * 1000L

    fun normalizedHost(raw: String): String {
        val host = raw.trim().removePrefix("http://").removePrefix("https://").substringBefore("/")
        if (host.isBlank()) return ""
        return if (host.contains(":")) host else "$host:38473"
    }

    fun discover(timeoutMs: Int = 6000): DiscoveryResult {
        DatagramSocket(38472).use { socket ->
            socket.soTimeout = timeoutMs
            val buf = ByteArray(4096)
            val packet = DatagramPacket(buf, buf.size)
            socket.receive(packet)
            val text = String(packet.data, 0, packet.length, Charsets.UTF_8)
            val json = JSONObject(text)
            if (json.optString("magic") != "ZSCLIP_LAN_V1") {
                throw IllegalStateException("不是 ZSClip 发现包")
            }
            val host = "${packet.address.hostAddress}:${json.optInt("tcp_port", 38473)}"
            return DiscoveryResult(
                json.optString("name", "ZSClip"),
                host,
                json.optString("device_id")
            )
        }
    }

    fun requestPair(context: Context, host: String): Pair<String, String> {
        val body = LanProtocol.pairRequestBody(LanPrefs.deviceId(context))
        val json = http(context, "POST", host, "/v1/pair/request", body, false)
        return json.optString("pair_id") to json.optString("code")
    }

    fun pollPair(context: Context, host: String, pairId: String): Boolean {
        repeat(90) {
            Thread.sleep(1000)
            val status = http(context, "GET", host, "/v1/pair/status?id=$pairId", null, false)
            when (status.optString("status")) {
                "accepted" -> {
                    LanPrefs.savePairing(
                        context = context,
                        host = host,
                        token = status.optString("token"),
                        deviceId = status.optString("device_id"),
                        deviceName = status.optString("name", "Windows")
                    )
                    return true
                }
                "rejected" -> return false
            }
        }
        return false
    }

    fun pushText(context: Context, host: String, text: String): LanTextPushResult {
        val targetHost = normalizedHost(host).ifBlank { LanPrefs.pairedHost(context) }
        if (!LanProtocol.hasPairing(targetHost, LanPrefs.token(context))) {
            throw IllegalStateException("请先完成配对")
        }
        val normalized = LanProtocol.cleanShareText(text)
        if (normalized.isBlank()) {
            throw IllegalStateException("请输入要推送的文本")
        }
        val seq = nextOriginSeq()
        val deviceId = LanPrefs.deviceId(context)
        val body = LanProtocol.mobileTextEnvelopeBody(deviceId, normalized, seq)
        val messageId = "$deviceId-$seq"
        val hash = LanProtocol.mobileMessageHash(deviceId, seq)
        val key = LanProtocol.lanLatestKey(
            host = targetHost,
            messageId = messageId,
            originDeviceId = deviceId,
            originSeq = seq,
            hash = hash
        )
        val signature = LanProtocol.clipboardTextSignature(normalized)
        http(context, "POST", targetHost, "/v1/clip", body, true)
        LanPrefs.saveLastOwnPush(context, key, signature)
        LanPrefs.saveSyncStatus(context, true, "已推送文本到电脑", key)
        return LanTextPushResult(
            message = "已推送文本到电脑",
            key = key,
            signature = signature,
            originDeviceId = deviceId,
            originSeq = seq
        )
    }

    fun pushImage(context: Context, host: String, imagePngBytes: ByteArray, displayName: String?) {
        val targetHost = normalizedHost(host).ifBlank { LanPrefs.pairedHost(context) }
        if (!LanProtocol.hasPairing(targetHost, LanPrefs.token(context))) {
            throw IllegalStateException("请先完成配对")
        }
        val seq = nextOriginSeq()
        val deviceId = LanPrefs.deviceId(context)
        val body = LanProtocol.mobileImageEnvelopeBody(
            deviceId,
            imagePngBytes,
            seq,
            displayName
        )
        http(context, "POST", targetHost, "/v1/clip", body, true)
        LanPrefs.saveSyncStatus(
            context,
            true,
            "已推送图片到电脑：${displayName?.takeIf { it.isNotBlank() } ?: "未命名图片"}",
            LanProtocol.mobileMessageHash(deviceId, seq)
        )
    }

    fun latest(context: Context): LatestResult? {
        val host = normalizedHost(LanPrefs.pairedHost(context))
        if (host.isBlank()) throw IllegalStateException("未配置 Windows 设备")
        val json = http(context, "GET", host, "/v1/latest", null, true)
        val clip = json.optJSONObject("clip") ?: return null
        val messageId = clip.optString("message_id")
        val hash = clip.optString("hash")
        val key = LanProtocol.lanLatestKey(
            host = host,
            messageId = messageId,
            originDeviceId = clip.optString("origin_device_id"),
            originSeq = clip.optLong("origin_seq"),
            hash = hash
        )
        return LatestResult(
            key = key,
            messageId = messageId,
            kind = clip.optString("kind"),
            hash = hash,
            text = clip.optString("text").takeIf { it.isNotEmpty() },
            preview = clip.optString("preview"),
            originDeviceId = clip.optString("origin_device_id"),
            originSeq = clip.optLong("origin_seq")
        )
    }

    fun mobileImagesUrl(host: String, deviceId: String, token: String): String {
        val targetHost = normalizedHost(host)
        if (!LanProtocol.hasPairing(targetHost, token)) {
            throw IllegalStateException("请先完成配对")
        }
        val encodedDevice = URLEncoder.encode(deviceId, "UTF-8")
        val encodedToken = URLEncoder.encode(token, "UTF-8")
        return "http://$targetHost/mobile/images?device=$encodedDevice&token=$encodedToken"
    }

    fun mobileItemsPath(limit: Int = 50): String =
        "/v1/mobile/items?limit=${limit.coerceIn(1, 100)}"

    fun mobileItemImagePath(itemId: Long): String {
        if (itemId <= 0) {
            throw IllegalArgumentException("图片记录无效")
        }
        return "/v1/mobile/items/$itemId/image"
    }

    fun mobileItemFilePath(itemId: Long, fileIndex: Int): String {
        if (itemId <= 0 || fileIndex < 0) {
            throw IllegalArgumentException("文件记录无效")
        }
        return "/v1/mobile/items/$itemId/file/$fileIndex"
    }

    fun mobileSetupUrl(host: String): String {
        val targetHost = normalizedHost(host)
        if (targetHost.isBlank()) {
            throw IllegalStateException("请先填写或发现 Windows 地址")
        }
        return "http://$targetHost/mobile/setup"
    }

    fun multiSyncManifestUrl(host: String, deviceId: String, token: String): String {
        val targetHost = normalizedHost(host)
        if (!LanProtocol.hasPairing(targetHost, token)) {
            throw IllegalStateException("请先完成配对")
        }
        val encodedDevice = URLEncoder.encode(deviceId, "UTF-8")
        val encodedToken = URLEncoder.encode(token, "UTF-8")
        return "http://$targetHost/zsSyncClipboard.json?device=$encodedDevice&token=$encodedToken"
    }

    fun multiSyncDataUrl(host: String, deviceId: String, token: String, dataName: String): String {
        val targetHost = normalizedHost(host)
        if (!LanProtocol.hasPairing(targetHost, token)) {
            throw IllegalStateException("请先完成配对")
        }
        val safeName = LanProtocol.safeMultiSyncDataName(dataName)
            ?: throw IllegalArgumentException("图片数据名无效")
        val encodedDevice = URLEncoder.encode(deviceId, "UTF-8")
        val encodedToken = URLEncoder.encode(token, "UTF-8")
        return "http://$targetHost/file/$safeName?device=$encodedDevice&token=$encodedToken"
    }

    fun webDavManifestUrl(baseUrl: String, remoteDir: String): String {
        val base = webDavBaseUrl(baseUrl, remoteDir)
        return "$base/zsSyncClipboard.json"
    }

    fun webDavBaseUrl(baseUrl: String, remoteDir: String): String {
        val base = baseUrl.trim().trimEnd('/')
        if (base.isBlank()) {
            throw IllegalStateException("请先填写 WebDAV 地址")
        }
        val dir = remoteDir.trim().ifBlank { "ZS Clip" }
        return "$base/${encodePathSegments(dir)}"
    }

    fun webDavDataUrl(baseUrl: String, remoteDir: String, dataName: String): String {
        val safeName = LanProtocol.safeMultiSyncDataName(dataName)
            ?: throw IllegalArgumentException("图片数据名无效")
        return "${webDavBaseUrl(baseUrl, remoteDir)}/file/$safeName"
    }

    fun webDavImageUrlFromClip(
        baseUrl: String,
        remoteDir: String,
        clip: LanProtocol.MultiSyncClip?
    ): String {
        if (clip?.kind != "image" || clip.dataName.isNullOrBlank()) {
            throw IllegalStateException("WebDAV 最新记录不是可下载图片")
        }
        val safeName = LanProtocol.safeMultiSyncDataName(clip.dataName)
            ?: throw IllegalArgumentException("图片数据名无效")
        return webDavDataUrl(baseUrl, remoteDir, safeName)
    }

    fun latestWebDavImageUrl(context: Context): String {
        val config = LanPrefs.webDavConfig(context)
        val clip = fetchWebDavMultiSyncClip(context)
        return webDavImageUrlFromClip(config.url, config.remoteDir, clip)
    }

    fun webDavImageDownloadRequest(
        baseUrl: String,
        remoteDir: String,
        user: String,
        pass: String,
        clip: LanProtocol.MultiSyncClip?
    ): WebDavImageDownloadRequest {
        if (clip?.kind != "image" || clip.dataName.isNullOrBlank()) {
            throw IllegalStateException("WebDAV 最新记录不是可下载图片")
        }
        val safeName = LanProtocol.safeMultiSyncDataName(clip.dataName)
            ?: throw IllegalArgumentException("图片数据名无效")
        return WebDavImageDownloadRequest(
            url = webDavDataUrl(baseUrl, remoteDir, safeName),
            fileName = safeName,
            authHeader = basicAuthHeader(user, pass)
        )
    }

    fun enqueueLatestWebDavImageDownload(context: Context): String {
        val config = LanPrefs.webDavConfig(context)
        val clip = fetchWebDavMultiSyncClip(context)
        val requestInfo = webDavImageDownloadRequest(
            config.url,
            config.remoteDir,
            config.user,
            config.pass,
            clip
        )
        val request = DownloadManager.Request(Uri.parse(requestInfo.url))
            .setTitle("ZSClip ${requestInfo.fileName}")
            .setDescription("WebDAV 多端同步图片")
            .setMimeType("image/png")
            .setNotificationVisibility(DownloadManager.Request.VISIBILITY_VISIBLE_NOTIFY_COMPLETED)
            .setDestinationInExternalFilesDir(
                context,
                Environment.DIRECTORY_DOWNLOADS,
                requestInfo.fileName
            )
        requestInfo.authHeader?.let { request.addRequestHeader("Authorization", it) }
        val manager = context.getSystemService(Context.DOWNLOAD_SERVICE) as DownloadManager
        manager.enqueue(request)
        val message = "已开始下载 WebDAV 图片：${requestInfo.fileName}"
        LanPrefs.saveSyncStatus(context, true, message, LanProtocol.multiSyncClipKey(clip!!))
        return message
    }

    fun fetchMobileHistoryItems(context: Context, limit: Int = 50): List<MobileHistoryItem> {
        val host = normalizedHost(LanPrefs.pairedHost(context))
        if (!LanProtocol.hasPairing(host, LanPrefs.token(context))) {
            throw IllegalStateException("请先完成配对")
        }
        val json = http(context, "GET", host, mobileItemsPath(limit), null, true)
        val items = json.optJSONArray("items") ?: JSONArray()
        return (0 until items.length()).mapNotNull { index ->
            val item = items.optJSONObject(index) ?: return@mapNotNull null
            val filesJson = item.optJSONArray("files") ?: JSONArray()
            MobileHistoryItem(
                id = item.optLong("id"),
                kind = item.optString("kind"),
                preview = item.optString("preview"),
                text = item.optString("text").takeIf { it.isNotEmpty() },
                sourceApp = item.optString("source_app"),
                createdAt = item.optString("created_at"),
                size = item.optLong("size"),
                width = item.optInt("width").takeIf { !item.isNull("width") && it > 0 },
                height = item.optInt("height").takeIf { !item.isNull("height") && it > 0 },
                files = (0 until filesJson.length()).mapNotNull { fileIndex ->
                    val file = filesJson.optJSONObject(fileIndex) ?: return@mapNotNull null
                    MobileFileItem(
                        index = file.optInt("index"),
                        name = file.optString("name", "file.bin"),
                        size = file.optLong("size")
                    )
                }
            )
        }
    }

    fun fetchMobileImageBytes(context: Context, itemId: Long): ByteArray {
        val host = normalizedHost(LanPrefs.pairedHost(context))
        if (!LanProtocol.hasPairing(host, LanPrefs.token(context))) {
            throw IllegalStateException("请先完成配对")
        }
        return httpBytes(context, host, mobileItemImagePath(itemId), true)
    }

    fun fetchLatestWebDavImageBytes(context: Context): Pair<ByteArray, String> {
        val config = LanPrefs.webDavConfig(context)
        val clip = fetchWebDavMultiSyncClip(context)
        val requestInfo = webDavImageDownloadRequest(
            config.url,
            config.remoteDir,
            config.user,
            config.pass,
            clip
        )
        return httpBytesUrl(requestInfo.url, config.user, config.pass) to requestInfo.fileName
    }

    fun downloadMobileFile(context: Context, itemId: Long, file: MobileFileItem): File {
        val host = normalizedHost(LanPrefs.pairedHost(context))
        if (!LanProtocol.hasPairing(host, LanPrefs.token(context))) {
            throw IllegalStateException("请先完成配对")
        }
        val bytes = httpBytes(context, host, mobileItemFilePath(itemId, file.index), true)
        val out = File(AndroidFileActions.downloadsDir(context), safeDownloadName(file.name))
        FileOutputStream(out).use { stream -> stream.write(bytes) }
        return out
    }

    fun fetchMultiSyncClip(context: Context): LanProtocol.MultiSyncClip? {
        val host = normalizedHost(LanPrefs.pairedHost(context))
        val token = LanPrefs.token(context)
        if (!LanProtocol.hasPairing(host, token)) {
            throw IllegalStateException("请先完成配对")
        }
        val path = multiSyncManifestUrl(host, LanPrefs.deviceId(context), token)
            .removePrefix("http://$host")
        val json = http(context, "GET", host, path, null, false)
        return LanProtocol.parseMultiSyncClip(json)
    }

    fun fetchWebDavMultiSyncClip(context: Context): LanProtocol.MultiSyncClip? {
        val config = LanPrefs.webDavConfig(context)
        return fetchWebDavMultiSyncClipForConfig(
            baseUrl = config.url,
            remoteDir = config.remoteDir,
            user = config.user,
            pass = config.pass
        )
    }

    internal fun fetchWebDavMultiSyncClipForConfig(
        baseUrl: String,
        remoteDir: String,
        user: String,
        pass: String
    ): LanProtocol.MultiSyncClip? {
        val url = webDavManifestUrl(baseUrl, remoteDir)
        val json = httpOptionalJsonUrl(url, user, pass) ?: return null
        return LanProtocol.parseMultiSyncClip(json)
    }

    fun checkAvailableTransportStatus(context: Context): MultiSyncStatusResult {
        val hasPairing = LanPrefs.hasPairing(context)
        val hasWebDav = LanPrefs.hasWebDavConfig(context)
        if (hasPairing) {
            try {
                return buildLanStatusResult(context, fetchMultiSyncClip(context))
            } catch (e: Exception) {
                if (!hasWebDav) {
                    throw e
                }
                val webDavResult = buildWebDavStatusResult(context, fetchWebDavMultiSyncClip(context))
                val fallbackMessage = LanProtocol.checkWebDavFallbackMessage(e.message, webDavResult.message)
                LanPrefs.updateSyncStatusMessage(context, true, fallbackMessage)
                return webDavResult.copy(message = fallbackMessage)
            }
        }
        if (hasWebDav) {
            return buildWebDavStatusResult(context, fetchWebDavMultiSyncClip(context))
        }
        throw IllegalStateException("请先完成配对或配置 WebDAV")
    }

    private fun buildLanStatusResult(
        context: Context,
        clip: LanProtocol.MultiSyncClip?
    ): MultiSyncStatusResult {
        val message = LanProtocol.multiSyncStatusMessage(clip)
        LanPrefs.saveSyncStatus(context, true, message, clip?.let { LanProtocol.multiSyncClipKey(it) }.orEmpty())
        return MultiSyncStatusResult(
            clip = clip,
            message = message,
            detail = if (clip?.kind == "image" && !clip.dataName.isNullOrBlank()) {
                "图片数据：${clip.dataName}，可从图片入口查看或下载"
            } else {
                null
            }
        )
    }

    private fun buildWebDavStatusResult(
        context: Context,
        clip: LanProtocol.MultiSyncClip?
    ): MultiSyncStatusResult {
        val config = LanPrefs.webDavConfig(context)
        val message = "WebDAV：${LanProtocol.multiSyncStatusMessage(clip)}"
        LanPrefs.saveSyncStatus(context, true, message, clip?.let { LanProtocol.multiSyncClipKey(it) }.orEmpty())
        return MultiSyncStatusResult(
            clip = clip,
            message = message,
            detail = if (clip?.kind == "image" && !clip.dataName.isNullOrBlank()) {
                "图片数据：${webDavDataUrl(config.url, config.remoteDir, clip.dataName)}"
            } else {
                null
            }
        )
    }

    fun pullWebDavToClipboard(context: Context, force: Boolean = true): String {
        val clip = fetchWebDavMultiSyncClip(context)
        if (clip == null) {
            val message = "WebDAV 多端同步清单暂无记录"
            LanPrefs.saveSyncStatus(context, true, message)
            return message
        }
        val key = LanProtocol.multiSyncClipKey(clip)
        if (LanProtocol.isOwnMultiSyncClip(clip, LanPrefs.deviceId(context))) {
            val message = "跳过手机自己推送的 WebDAV 记录"
            LanPrefs.saveLastClipKey(context, key)
            LanPrefs.saveSyncStatus(context, true, message, key)
            return message
        }
        if (!force && key == LanPrefs.lastClipKey(context)) {
            val message = "没有新 WebDAV 记录"
            LanPrefs.saveSyncStatus(context, true, message, key)
            return message
        }
        val text = LanProtocol.clipboardTextForMultiSync(clip)
        if (text.isNullOrBlank()) {
            val message = LanProtocol.multiSyncStatusMessage(clip)
            LanPrefs.saveLastClipKey(context, key)
            LanPrefs.saveSyncStatus(context, true, "WebDAV：$message", key)
            return "WebDAV：$message"
        }
        val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        val signature = LanProtocol.clipboardTextSignature(text)
        LanPrefs.saveLastRemoteClipboardSignature(context, signature)
        LanPrefs.saveLastLocalClipboardSignature(context, signature)
        clipboard.setPrimaryClip(ClipData.newPlainText("ZSClip WebDAV", text))
        val message = "WebDAV 已写入手机剪贴板：${text.take(30)}"
        LanPrefs.saveLastClipKey(context, key)
        LanPrefs.saveSyncStatus(context, true, message, key)
        return message
    }

    fun pullAvailableTransportToClipboard(context: Context, force: Boolean): String {
        val hasPairing = LanPrefs.hasPairing(context)
        val hasWebDav = LanPrefs.hasWebDavConfig(context)
        if (hasPairing) {
            try {
                return pullLatestToClipboard(context, force)
            } catch (e: Exception) {
                if (!hasWebDav) {
                    throw e
                }
                val webDavMessage = pullWebDavToClipboard(context, force)
                val fallbackMessage = LanProtocol.autoSyncWebDavFallbackMessage(e.message, webDavMessage)
                LanPrefs.updateSyncStatusMessage(context, true, fallbackMessage)
                return fallbackMessage
            }
        }
        if (hasWebDav) {
            return pullWebDavToClipboard(context, force)
        }
        throw IllegalStateException("请先完成配对或配置 WebDAV")
    }

    fun pushWebDavText(context: Context, text: String): String {
        val config = LanPrefs.webDavConfig(context)
        val seq = nextOriginSeq()
        val result = pushWebDavTextForConfig(
            baseUrl = config.url,
            remoteDir = config.remoteDir,
            user = config.user,
            pass = config.pass,
            deviceId = LanPrefs.deviceId(context),
            text = text,
            seq = seq
        )
        LanPrefs.saveSyncStatus(context, true, result.message, result.key)
        return result.message
    }

    fun pushTextToAvailableTransport(context: Context, text: String): String {
        val normalized = LanProtocol.cleanShareText(text)
        if (normalized.isBlank()) {
            throw IllegalStateException("请输入要推送的文本")
        }
        val hasPairing = LanPrefs.hasPairing(context)
        val hasWebDav = LanPrefs.hasWebDavConfig(context)
        if (hasPairing) {
            try {
                pushText(context, LanPrefs.pairedHost(context), normalized)
                return "已推送到电脑：${normalized.take(30)}"
            } catch (e: Exception) {
                if (!hasWebDav) {
                    throw e
                }
                val webDavMessage = pushWebDavText(context, normalized)
                val fallbackMessage = LanProtocol.pushWebDavFallbackMessage(e.message, webDavMessage)
                LanPrefs.updateSyncStatusMessage(context, true, fallbackMessage)
                return fallbackMessage
            }
        }
        if (hasWebDav) {
            return pushWebDavText(context, normalized)
        }
        throw IllegalStateException("请先完成配对或配置 WebDAV")
    }

    fun pushClipboardTextToAvailableTransport(context: Context): String {
        val text = clipboardText(context)
        if (text.isBlank()) {
            throw IllegalStateException("手机剪贴板没有可推送的文本")
        }
        val message = pushTextToAvailableTransport(context, text)
        LanPrefs.saveLastLocalClipboardSignature(context, LanProtocol.clipboardTextSignature(text))
        return message
    }

    fun pushChangedClipboardTextToAvailableTransport(context: Context): String {
        if (!autoClipboardPushInFlight.compareAndSet(false, true)) {
            return "手机剪贴板自动推送正在进行"
        }
        return try {
            val text = clipboardText(context)
            val signature = LanProtocol.clipboardTextSignature(text)
            when {
                text.isBlank() -> "手机剪贴板暂无可自动推送文本"
                signature == LanPrefs.lastLocalClipboardSignature(context) -> "手机剪贴板无新文本"
                signature == LanPrefs.lastRemoteClipboardSignature(context) -> {
                    LanPrefs.saveLastLocalClipboardSignature(context, signature)
                    "手机剪贴板是刚同步到手机的内容"
                }
                else -> {
                    val message = pushTextToAvailableTransport(context, text)
                    LanPrefs.saveLastLocalClipboardSignature(context, signature)
                    message.replace("已推送", "已自动推送")
                }
            }
        } finally {
            autoClipboardPushInFlight.set(false)
        }
    }

    fun pendingRemoteTextForAutoSync(context: Context): PendingRemoteText? {
        val hasPairing = LanPrefs.hasPairing(context)
        val hasWebDav = LanPrefs.hasWebDavConfig(context)
        if (hasPairing) {
            try {
                return pendingLanRemoteText(context)
            } catch (e: Exception) {
                if (!hasWebDav) {
                    throw e
                }
                return pendingWebDavRemoteText(context)
            }
        }
        if (hasWebDav) {
            return pendingWebDavRemoteText(context)
        }
        throw IllegalStateException("请先完成配对或配置 WebDAV")
    }

    fun autoSyncAvailableTransport(context: Context, pullRemote: Boolean = true): String {
        val messages = mutableListOf<String>()

        runCatching {
            pushChangedClipboardTextToAvailableTransport(context)
        }.fold(
            onSuccess = { message ->
                if (!LanProtocol.isAutoSyncNoopMessage(message)) {
                    messages += message
                }
            },
            onFailure = { error ->
                messages += "手机自动推送失败：${error.message}"
            }
        )

        if (pullRemote) {
            runCatching {
                pullAvailableTransportToClipboard(context, force = false)
            }.fold(
                onSuccess = { message ->
                    if (!LanProtocol.isAutoSyncNoopMessage(message)) {
                        messages += message
                    }
                },
                onFailure = { error ->
                    messages += "电脑自动拉取失败：${error.message}"
                }
            )
        }

        val message = messages.joinToString("\n").ifBlank { "没有新内容" }
        LanPrefs.updateSyncStatusMessage(context, messages.none { it.contains("失败") }, message)
        return message
    }

    private fun pendingLanRemoteText(context: Context): PendingRemoteText? {
        val latest = latest(context)
        if (latest == null) {
            return null
        }
        if (LanProtocol.isOwnLatest(latest.originDeviceId, LanPrefs.deviceId(context))) {
            LanPrefs.saveLastClipKey(context, latest.key)
            LanPrefs.saveSyncStatus(context, true, "跳过手机自己推送的记录", latest.key)
            return null
        }
        if (latest.key == LanPrefs.lastClipKey(context)) {
            return null
        }
        val text = LanProtocol.clipboardTextForLatest(latest.kind, latest.text)
        if (text.isNullOrBlank()) {
            LanPrefs.saveLastClipKey(context, latest.key)
            LanPrefs.saveSyncStatus(context, true, LanProtocol.latestNonTextMessage(latest.kind), latest.key)
            return null
        }
        if (isRecentOwnLanPush(context, latest, text)) {
            LanPrefs.saveLastClipKey(context, latest.key)
            LanPrefs.saveSyncStatus(context, true, "跳过手机刚推送的记录", latest.key)
            return null
        }
        return PendingRemoteText(
            key = latest.key,
            preview = text.take(30),
            transport = "电脑"
        )
    }

    private fun pendingWebDavRemoteText(context: Context): PendingRemoteText? {
        val clip = fetchWebDavMultiSyncClip(context) ?: return null
        val key = LanProtocol.multiSyncClipKey(clip)
        if (LanProtocol.isOwnMultiSyncClip(clip, LanPrefs.deviceId(context))) {
            LanPrefs.saveLastClipKey(context, key)
            LanPrefs.saveSyncStatus(context, true, "跳过手机自己推送的 WebDAV 记录", key)
            return null
        }
        if (key == LanPrefs.lastClipKey(context)) {
            return null
        }
        val text = LanProtocol.clipboardTextForMultiSync(clip)
        if (text.isNullOrBlank()) {
            LanPrefs.saveLastClipKey(context, key)
            LanPrefs.saveSyncStatus(context, true, "WebDAV：${LanProtocol.multiSyncStatusMessage(clip)}", key)
            return null
        }
        return PendingRemoteText(
            key = key,
            preview = text.take(30),
            transport = "WebDAV"
        )
    }

    fun pushWebDavImage(context: Context, imagePngBytes: ByteArray, displayName: String?): String {
        val config = LanPrefs.webDavConfig(context)
        val seq = nextOriginSeq()
        val result = pushWebDavImageForConfig(
            baseUrl = config.url,
            remoteDir = config.remoteDir,
            user = config.user,
            pass = config.pass,
            deviceId = LanPrefs.deviceId(context),
            imagePngBytes = imagePngBytes,
            displayName = displayName,
            seq = seq
        )
        LanPrefs.saveSyncStatus(context, true, result.message, result.key)
        return result.message
    }

    fun pushImageToAvailableTransport(context: Context, imagePngBytes: ByteArray, displayName: String?): String {
        val hasPairing = LanPrefs.hasPairing(context)
        val hasWebDav = LanPrefs.hasWebDavConfig(context)
        if (hasPairing) {
            try {
                pushImage(context, LanPrefs.pairedHost(context), imagePngBytes, displayName)
                val label = displayName?.takeIf { it.isNotBlank() } ?: "未命名图片"
                return "已推送图片到电脑：${label.take(30)}"
            } catch (e: Exception) {
                if (!hasWebDav) {
                    throw e
                }
                val webDavMessage = pushWebDavImage(context, imagePngBytes, displayName)
                val fallbackMessage = LanProtocol.pushWebDavFallbackMessage(e.message, webDavMessage)
                LanPrefs.updateSyncStatusMessage(context, true, fallbackMessage)
                return fallbackMessage
            }
        }
        if (hasWebDav) {
            return pushWebDavImage(context, imagePngBytes, displayName)
        }
        throw IllegalStateException("请先完成配对或配置 WebDAV")
    }

    internal fun pushWebDavTextForConfig(
        baseUrl: String,
        remoteDir: String,
        user: String,
        pass: String,
        deviceId: String,
        text: String,
        seq: Long
    ): WebDavPushResult {
        val normalized = LanProtocol.cleanShareText(text)
        if (normalized.isBlank()) {
            throw IllegalStateException("请输入要推送到 WebDAV 的文本")
        }
        val body = LanProtocol.webDavTextManifestBody(deviceId, normalized, seq)
        val clip = LanProtocol.parseMultiSyncClip(JSONObject(body))
            ?: throw IllegalStateException("WebDAV 清单为空")
        ensureWebDavBaseDir(baseUrl, remoteDir, user, pass)
        httpPutJson(webDavManifestUrl(baseUrl, remoteDir), user, pass, body)
        return WebDavPushResult(
            message = "已推送 WebDAV 文本：${normalized.take(30)}",
            key = LanProtocol.multiSyncClipKey(clip)
        )
    }

    internal fun pushWebDavImageForConfig(
        baseUrl: String,
        remoteDir: String,
        user: String,
        pass: String,
        deviceId: String,
        imagePngBytes: ByteArray,
        displayName: String?,
        seq: Long
    ): WebDavPushResult {
        val body = LanProtocol.webDavImageManifestBody(deviceId, imagePngBytes, seq, displayName)
        val clip = LanProtocol.parseMultiSyncClip(JSONObject(body))
            ?: throw IllegalStateException("WebDAV 图片清单为空")
        val dataName = clip.dataName ?: throw IllegalStateException("WebDAV 图片数据名为空")
        ensureWebDavBaseDir(baseUrl, remoteDir, user, pass)
        webDavMkcol("${webDavBaseUrl(baseUrl, remoteDir)}/file", user, pass)
        httpPutBytes(webDavDataUrl(baseUrl, remoteDir, dataName), user, pass, imagePngBytes, "image/png")
        httpPutJson(webDavManifestUrl(baseUrl, remoteDir), user, pass, body)
        val label = displayName?.takeIf { it.isNotBlank() } ?: dataName
        return WebDavPushResult(
            message = "已推送 WebDAV 图片：${label.take(30)}",
            key = LanProtocol.multiSyncClipKey(clip)
        )
    }

    fun pullLatestToClipboard(context: Context, force: Boolean): String {
        val latest = latest(context)
        if (latest == null) {
            val message = "Windows 暂无记录"
            LanPrefs.saveSyncStatus(context, true, message)
            return message
        }
        if (LanProtocol.isOwnLatest(latest.originDeviceId, LanPrefs.deviceId(context))) {
            val message = "跳过手机自己推送的记录"
            LanPrefs.saveLastClipKey(context, latest.key)
            LanPrefs.saveSyncStatus(context, true, message, latest.key)
            return message
        }
        if (!force && latest.key == LanPrefs.lastClipKey(context)) {
            val message = "没有新记录"
            LanPrefs.saveSyncStatus(context, true, message, latest.key)
            return message
        }
        val text = LanProtocol.clipboardTextForLatest(latest.kind, latest.text)
        if (text.isNullOrBlank()) {
            LanPrefs.saveLastClipKey(context, latest.key)
            val message = LanProtocol.latestNonTextMessage(latest.kind)
            LanPrefs.saveSyncStatus(context, true, message, latest.key)
            return message
        }
        if (isRecentOwnLanPush(context, latest, text)) {
            val message = "跳过手机刚推送的记录"
            LanPrefs.saveLastClipKey(context, latest.key)
            LanPrefs.saveSyncStatus(context, true, message, latest.key)
            return message
        }
        val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        val signature = LanProtocol.clipboardTextSignature(text)
        LanPrefs.saveLastRemoteClipboardSignature(context, signature)
        LanPrefs.saveLastLocalClipboardSignature(context, signature)
        clipboard.setPrimaryClip(ClipData.newPlainText("ZSClip", text))
        LanPrefs.saveLastClipKey(context, latest.key)
        val message = "已写入手机剪贴板：${text.take(30)}"
        LanPrefs.saveSyncStatus(context, true, message, latest.key)
        return message
    }

    fun pushClipboardTextToComputer(context: Context): String {
        if (!LanPrefs.hasPairing(context)) {
            throw IllegalStateException("请先完成配对")
        }
        val text = clipboardText(context)
        if (text.isBlank()) {
            throw IllegalStateException("手机剪贴板没有可推送的文本")
        }
        val result = pushText(context, LanPrefs.pairedHost(context), text)
        LanPrefs.saveLastLocalClipboardSignature(context, result.signature)
        return "已推送到电脑：${text.take(30)}"
    }

    private fun isRecentOwnLanPush(context: Context, latest: LatestResult, text: String): Boolean {
        if (latest.key == LanPrefs.lastOwnPushKey(context)) {
            return true
        }
        val signature = LanProtocol.clipboardTextSignature(text)
        if (signature.isBlank() || signature != LanPrefs.lastOwnPushSignature(context)) {
            return false
        }
        val pushedAt = LanPrefs.lastOwnPushAt(context)
        return pushedAt > 0 && System.currentTimeMillis() - pushedAt <= RECENT_OWN_PUSH_SKIP_MS
    }

    private fun clipboardText(context: Context): String {
        val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        val raw = clipboard.primaryClip
            ?.takeIf { it.itemCount > 0 }
            ?.getItemAt(0)
            ?.coerceToText(context)
            ?.toString()
            .orEmpty()
        return LanProtocol.cleanShareText(raw)
    }

    private fun http(
        context: Context,
        method: String,
        host: String,
        path: String,
        body: String?,
        auth: Boolean
    ): JSONObject {
        try {
            val conn = URL("http://${normalizedHost(host)}$path").openConnection() as HttpURLConnection
            conn.requestMethod = method
            conn.connectTimeout = 5000
            conn.readTimeout = 8000
            conn.setRequestProperty("Content-Type", "application/json")
            if (auth) {
                val token = LanPrefs.token(context).trim()
                if (token.isBlank()) {
                    throw IllegalStateException("请先完成配对")
                }
                conn.setRequestProperty("X-ZSClip-Device", LanPrefs.deviceId(context))
                conn.setRequestProperty("X-ZSClip-Token", token)
            }
            if (body != null) {
                conn.doOutput = true
                OutputStreamWriter(conn.outputStream, Charsets.UTF_8).use { it.write(body) }
            }
            val code = conn.responseCode
            val stream = if (code in 200..299) conn.inputStream else conn.errorStream
            val text = stream?.bufferedReader(Charsets.UTF_8)?.readText().orEmpty()
            if (code !in 200..299) {
                throw IllegalStateException(text.ifBlank { "HTTP $code" })
            }
            return JSONObject(text.ifBlank { "{}" })
        } catch (e: Exception) {
            if (e is IllegalStateException && e.message?.startsWith("请先完成配对") == true) {
                throw e
            }
            throw IllegalStateException(networkAccessMessage(e), e)
        }
    }

    private fun networkAccessMessage(error: Throwable): String {
        val raw = error.message.orEmpty()
        val normalized = raw.lowercase()
        return when {
            normalized.contains("unexpected end of stream") ||
                normalized.contains("failed to connect") ||
                normalized.contains("connection refused") ||
                normalized.contains("connect timed out") ||
                error is java.net.UnknownHostException ||
                error is java.net.SocketTimeoutException ||
                error is java.net.ConnectException ->
                "电脑当前不可达，请检查手机是否连接到与电脑同一网络，或 VPN 是否允许访问局域网"
            raw.isNotBlank() -> raw
            else -> "网络请求失败"
        }
    }

    private fun httpJsonUrl(url: String, user: String, pass: String): JSONObject {
        return httpOptionalJsonUrl(url, user, pass)
            ?: throw IllegalStateException("HTTP 404")
    }

    private fun httpOptionalJsonUrl(url: String, user: String, pass: String): JSONObject? {
        val conn = URL(url).openConnection() as HttpURLConnection
        conn.requestMethod = "GET"
        conn.connectTimeout = 8000
        conn.readTimeout = 12000
        setBasicAuth(conn, user, pass)
        val code = conn.responseCode
        val stream = if (code in 200..299) conn.inputStream else conn.errorStream
        val text = stream?.bufferedReader(Charsets.UTF_8)?.readText().orEmpty()
        if (code == 404) {
            return null
        }
        if (code !in 200..299) {
            throw IllegalStateException(text.ifBlank { "HTTP $code" })
        }
        if (text.isBlank()) {
            return null
        }
        return JSONObject(text.ifBlank { "{}" })
    }

    private fun httpBytes(context: Context, host: String, path: String, auth: Boolean): ByteArray {
        val conn = URL("http://${normalizedHost(host)}$path").openConnection() as HttpURLConnection
        conn.requestMethod = "GET"
        conn.connectTimeout = 8000
        conn.readTimeout = 20000
        if (auth) {
            val token = LanPrefs.token(context).trim()
            if (token.isBlank()) {
                throw IllegalStateException("请先完成配对")
            }
            conn.setRequestProperty("X-ZSClip-Device", LanPrefs.deviceId(context))
            conn.setRequestProperty("X-ZSClip-Token", token)
        }
        val code = conn.responseCode
        val stream = if (code in 200..299) conn.inputStream else conn.errorStream
        val bytes = stream?.readBytes() ?: ByteArray(0)
        if (code !in 200..299) {
            throw IllegalStateException(bytes.toString(Charsets.UTF_8).ifBlank { "HTTP $code" })
        }
        return bytes
    }

    private fun httpBytesUrl(url: String, user: String, pass: String): ByteArray {
        val conn = URL(url).openConnection() as HttpURLConnection
        conn.requestMethod = "GET"
        conn.connectTimeout = 8000
        conn.readTimeout = 20000
        setBasicAuth(conn, user, pass)
        val code = conn.responseCode
        val stream = if (code in 200..299) conn.inputStream else conn.errorStream
        val bytes = stream?.readBytes() ?: ByteArray(0)
        if (code !in 200..299) {
            throw IllegalStateException(bytes.toString(Charsets.UTF_8).ifBlank { "HTTP $code" })
        }
        return bytes
    }

    private fun httpPutJson(url: String, user: String, pass: String, body: String) {
        val conn = URL(url).openConnection() as HttpURLConnection
        conn.requestMethod = "PUT"
        conn.connectTimeout = 8000
        conn.readTimeout = 12000
        conn.doOutput = true
        conn.setRequestProperty("Content-Type", "application/json; charset=utf-8")
        setBasicAuth(conn, user, pass)
        OutputStreamWriter(conn.outputStream, Charsets.UTF_8).use { it.write(body) }
        val code = conn.responseCode
        if (code !in listOf(200, 201, 204)) {
            val text = conn.errorStream?.bufferedReader(Charsets.UTF_8)?.readText().orEmpty()
            throw IllegalStateException(text.ifBlank { "HTTP $code" })
        }
    }

    private fun httpPutBytes(url: String, user: String, pass: String, body: ByteArray, contentType: String) {
        val conn = URL(url).openConnection() as HttpURLConnection
        conn.requestMethod = "PUT"
        conn.connectTimeout = 8000
        conn.readTimeout = 12000
        conn.doOutput = true
        conn.setRequestProperty("Content-Type", contentType)
        setBasicAuth(conn, user, pass)
        conn.outputStream.use { it.write(body) }
        val code = conn.responseCode
        if (code !in listOf(200, 201, 204)) {
            val text = conn.errorStream?.bufferedReader(Charsets.UTF_8)?.readText().orEmpty()
            throw IllegalStateException(text.ifBlank { "HTTP $code" })
        }
    }

    private fun ensureWebDavBaseDir(baseUrl: String, remoteDir: String, user: String, pass: String) {
        val base = baseUrl.trim().trimEnd('/')
        if (base.isBlank()) {
            throw IllegalStateException("请先填写 WebDAV 地址")
        }
        var current = base
        val segments = remoteDir.trim().ifBlank { "ZS Clip" }
            .split('/')
            .map { it.trim() }
            .filter { it.isNotEmpty() }
        for (segment in segments) {
            current = "$current/${encodePathSegment(segment)}"
            webDavMkcol(current, user, pass)
        }
    }

    private fun webDavMkcol(url: String, user: String, pass: String) {
        val code = rawWebDavMkcol(url, user, pass)
        if (code !in listOf(200, 201, 204, 301, 302, 405, 409)) {
            throw IllegalStateException("HTTP $code")
        }
    }

    private fun rawWebDavMkcol(rawUrl: String, user: String, pass: String): Int {
        val url = URL(rawUrl)
        val secure = url.protocol.equals("https", ignoreCase = true)
        val port = if (url.port > 0) url.port else if (secure) 443 else 80
        val requestTarget = requestTarget(url)
        val hostHeader = hostHeader(url.host, port, secure)
        val baseSocket = Socket()
        baseSocket.connect(InetSocketAddress(url.host, port), 8000)
        val socket = if (secure) {
            (SSLSocketFactory.getDefault() as SSLSocketFactory)
                .createSocket(baseSocket, url.host, port, true)
        } else {
            baseSocket
        }
        socket.soTimeout = 12000
        socket.use { conn ->
            val authHeader = basicAuthHeader(user, pass)
                ?.let { "Authorization: $it\r\n" }
                .orEmpty()
            val request = buildString {
                append("MKCOL ")
                append(requestTarget)
                append(" HTTP/1.1\r\n")
                append("Host: ")
                append(hostHeader)
                append("\r\n")
                append(authHeader)
                append("Content-Length: 0\r\n")
                append("Connection: close\r\n\r\n")
            }
            conn.getOutputStream().write(request.toByteArray(Charsets.UTF_8))
            conn.getOutputStream().flush()
            val statusLine = conn.getInputStream()
                .bufferedReader(Charsets.UTF_8)
                .readLine()
                .orEmpty()
            return statusLine.split(' ').getOrNull(1)?.toIntOrNull() ?: 0
        }
    }

    private fun setBasicAuth(conn: HttpURLConnection, user: String, pass: String) {
        basicAuthHeader(user, pass)?.let { conn.setRequestProperty("Authorization", it) }
    }

    private fun basicAuthHeader(user: String, pass: String): String? {
        if (user.trim().isEmpty() && pass.isEmpty()) return null
        val raw = "${user.trim()}:$pass"
        val auth = Base64.getEncoder().encodeToString(raw.toByteArray(Charsets.UTF_8))
        return "Basic $auth"
    }

    private fun requestTarget(url: URL): String =
        (url.path.takeIf { it.isNotBlank() } ?: "/") +
            url.query?.let { "?$it" }.orEmpty()

    private fun hostHeader(host: String, port: Int, secure: Boolean): String {
        val bracketedHost = if (host.contains(':') && !host.startsWith("[")) "[$host]" else host
        val defaultPort = if (secure) 443 else 80
        return if (port == defaultPort) bracketedHost else "$bracketedHost:$port"
    }

    private fun encodePathSegments(value: String): String =
        value.split('/')
            .map { it.trim() }
            .filter { it.isNotEmpty() }
            .joinToString("/") { encodePathSegment(it) }

    private fun encodePathSegment(value: String): String =
        URLEncoder.encode(value, "UTF-8").replace("+", "%20")

    private fun safeDownloadName(value: String): String {
        val cleaned = value
            .replace(Regex("""[<>:"/\\|?*\p{Cntrl}]"""), "_")
            .trim()
            .trim('.')
        return cleaned.ifBlank { "file.bin" }.take(120)
    }

    @Synchronized
    private fun nextOriginSeq(): Long {
        val now = System.currentTimeMillis()
        val next = if (now <= lastOriginSeq) lastOriginSeq + 1 else now
        lastOriginSeq = next
        return next
    }
}
