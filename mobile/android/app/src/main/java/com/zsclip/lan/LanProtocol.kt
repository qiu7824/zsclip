package com.zsclip.lan

import org.json.JSONArray
import org.json.JSONObject
import java.net.URI
import java.net.URLDecoder
import java.util.Base64
import java.security.MessageDigest

object LanProtocol {
    const val MOBILE_IMAGE_MAX_BYTES = 10 * 1024 * 1024
    const val MOBILE_IMAGE_MAX_BASE64_CHARS = MOBILE_IMAGE_MAX_BYTES * 2
    const val MOBILE_IMAGE_MAX_PIXELS = 32_000_000
    const val MULTI_SYNC_PROTOCOL = "ZSCLIP_MULTI_SYNC_V1"
    private val androidCapabilities = listOf("text", "image", "latest", "client_only", "pull_only")

    data class MultiSyncClip(
        val id: String,
        val kind: String,
        val hash: String,
        val preview: String,
        val content: String?,
        val dataName: String?,
        val hasData: Boolean,
        val size: Long,
        val transport: String = "unknown"
    )

    fun hasPairing(host: String, token: String): Boolean =
        host.trim().isNotEmpty() && token.trim().isNotEmpty()

    fun tileStateLabel(host: String, token: String, running: Boolean): String =
        when {
            !hasPairing(host, token) -> "需要配对"
            running -> "运行中"
            else -> "可用"
        }

    fun multiAutoSyncStateLabel(hasPairing: Boolean, hasWebDav: Boolean, running: Boolean): String =
        when {
            running -> "运行中"
            hasPairing -> "局域网可用"
            hasWebDav -> "WebDAV 可用"
            else -> "需配对或配置"
        }

    fun multiPullStateLabel(hasPairing: Boolean, hasWebDav: Boolean, running: Boolean): String =
        when {
            running -> "运行中"
            hasPairing && hasWebDav -> "局域网 / WebDAV 可用"
            hasPairing -> "局域网可用"
            hasWebDav -> "WebDAV 可用"
            else -> "需配对或配置"
        }

    fun multiPushStateLabel(hasPairing: Boolean, hasWebDav: Boolean, running: Boolean): String =
        when {
            running -> "运行中"
            hasPairing && hasWebDav -> "局域网 / WebDAV 可用"
            hasPairing -> "局域网可用"
            hasWebDav -> "WebDAV 可用"
            else -> "需配对或配置"
        }

    fun autoSyncWebDavFallbackMessage(lanError: String?, webDavMessage: String): String {
        val reason = lanError?.trim().orEmpty()
        return if (reason.isEmpty()) {
            "已改用 WebDAV：$webDavMessage"
        } else {
            "局域网同步失败，已改用 WebDAV：$webDavMessage"
        }
    }

    fun pushWebDavFallbackMessage(lanError: String?, webDavMessage: String): String {
        val reason = lanError?.trim().orEmpty()
        return if (reason.isEmpty()) {
            "已改用 WebDAV：$webDavMessage"
        } else {
            "局域网推送失败，已改用 WebDAV：$webDavMessage"
        }
    }

    fun checkWebDavFallbackMessage(lanError: String?, webDavMessage: String): String {
        val reason = lanError?.trim().orEmpty()
        return if (reason.isEmpty()) {
            "已改用 WebDAV：$webDavMessage"
        } else {
            "局域网检查失败，已改用 WebDAV：$webDavMessage"
        }
    }

    fun sharedTextRoute(hasPairing: Boolean, hasWebDav: Boolean): String =
        when {
            hasPairing -> "lan"
            hasWebDav -> "webdav"
            else -> "manual"
        }

    fun cleanShareText(text: String?): String =
        normalizeCapturedText(text.orEmpty())

    fun mobileMessageHash(deviceId: String, seq: Long): String =
        "msg:${deviceId.trim()}:$seq"

    fun clipboardTextSignature(text: String): String {
        val normalized = normalizeCapturedText(text)
        return if (normalized.isBlank()) "" else "text:md5:${md5Hex(normalized)}"
    }

    fun isOwnLatest(originDeviceId: String, deviceId: String): Boolean =
        originDeviceId.trim().isNotEmpty() && originDeviceId.trim() == deviceId.trim()

    fun isOwnMultiSyncClip(clip: MultiSyncClip?, deviceId: String): Boolean {
        val id = clip?.id?.trim().orEmpty()
        val prefix = "android:"
        val suffix = ":${deviceId.trim()}:"
        return id.startsWith(prefix) && id.contains(suffix)
    }

    fun isAutoSyncNoopMessage(message: String): Boolean {
        val value = message.trim()
        return value.isBlank() ||
            value == "没有新内容" ||
            value == "没有新记录" ||
            value == "没有新 WebDAV 记录" ||
            value == "Windows 暂无记录" ||
            value == "手机剪贴板无新文本" ||
            value == "手机剪贴板自动推送正在进行" ||
            value == "手机剪贴板暂无可自动推送文本" ||
            value == "手机剪贴板是刚同步到手机的内容" ||
            value == "跳过手机自己推送的记录" ||
            value == "跳过手机自己推送的 WebDAV 记录" ||
            value.endsWith("暂无记录")
    }

    fun pairRequestBody(deviceId: String, name: String = "Android"): String =
        JSONObject()
            .put("device_id", deviceId)
            .put("name", name)
            .put("tcp_port", 0)
            .put("capabilities", JSONArray(androidCapabilities))
            .toString()

    fun mobileTextEnvelopeBody(deviceId: String, text: String, seq: Long): String {
        val normalized = normalizeCapturedText(text)
        require(normalized.isNotBlank()) { "文本为空" }
        return baseEnvelope(deviceId, seq, "text", normalized.take(80))
            .put("message_id", "$deviceId-$seq")
            .put("origin_device_id", deviceId)
            .put("origin_seq", seq)
            .put("kind", "text")
            .put("hash", mobileMessageHash(deviceId, seq))
            .put("created_at_ms", seq)
            .put("preview", normalized.take(80))
            .put("text", normalized)
            .put("image_png_base64", JSONObject.NULL)
            .put("file_meta", JSONArray())
            .toString()
    }

    fun webDavTextManifestBody(deviceId: String, text: String, seq: Long): String {
        val normalized = normalizeCapturedText(text)
        require(normalized.isNotBlank()) { "文本为空" }
        val preview = normalized.take(80)
        val clip = JSONObject()
            .put("id", "android:text:${deviceId.trim()}:$seq")
            .put("type", "text")
            .put("hash", "md5:${md5Hex(normalized)}")
            .put("preview", preview)
            .put("content", normalized)
            .put("hasData", false)
            .put("size", normalized.toByteArray(Charsets.UTF_8).size)
            .put("source_app", "Android")
            .put("created_at", seq.toString())
        return JSONObject()
            .put("protocol", MULTI_SYNC_PROTOCOL)
            .put("version", 1)
            .put("transport", "webdav")
            .put("clip", clip)
            .toString()
    }

    fun webDavImageManifestBody(
        deviceId: String,
        imagePngBytes: ByteArray,
        seq: Long,
        displayName: String? = null
    ): String {
        val estimatedBase64Chars = ((imagePngBytes.size + 2) / 3) * 4
        validateMobileImagePayload(imagePngBytes.size, estimatedBase64Chars)?.let { error ->
            throw IllegalArgumentException(error)
        }
        val preview = displayName
            ?.trim()
            ?.takeIf { it.isNotEmpty() }
            ?.take(80)
            ?: "Android 图片"
        val dataName = "zsclip_image_$seq.png"
        val clip = JSONObject()
            .put("id", "android:image:${deviceId.trim()}:$seq")
            .put("type", "image")
            .put("hash", "md5:${md5Hex(imagePngBytes)}")
            .put("preview", preview)
            .put("content", JSONObject.NULL)
            .put("hasData", true)
            .put("dataName", dataName)
            .put("size", imagePngBytes.size)
            .put("source_app", "Android")
            .put("created_at", seq.toString())
        return JSONObject()
            .put("protocol", MULTI_SYNC_PROTOCOL)
            .put("version", 1)
            .put("transport", "webdav")
            .put("clip", clip)
            .toString()
    }

    fun mobileImageEnvelopeBody(
        deviceId: String,
        imagePngBytes: ByteArray,
        seq: Long,
        displayName: String? = null
    ): String {
        val estimatedBase64Chars = ((imagePngBytes.size + 2) / 3) * 4
        validateMobileImagePayload(imagePngBytes.size, estimatedBase64Chars)?.let { error ->
            throw IllegalArgumentException(error)
        }
        val encoded = Base64.getEncoder().encodeToString(imagePngBytes)
        val preview = displayName
            ?.trim()
            ?.takeIf { it.isNotEmpty() }
            ?.take(80)
            ?: "Android 图片"
        return baseEnvelope(deviceId, seq, "image", preview)
            .put("text", JSONObject.NULL)
            .put("image_png_base64", encoded)
            .put("file_meta", JSONArray())
            .toString()
    }

    fun validateMobileImagePayload(byteCount: Int, base64CharCount: Int): String? =
        when {
            byteCount <= 0 -> "图片为空"
            byteCount > MOBILE_IMAGE_MAX_BYTES -> "图片超过 10MB，已跳过"
            base64CharCount > MOBILE_IMAGE_MAX_BASE64_CHARS -> "图片编码后过大，已跳过"
            else -> null
        }

    fun validateMobileImageDimensions(width: Int, height: Int): String? {
        if (width <= 0 || height <= 0) {
            return "图片无法解码，已跳过"
        }
        val pixels = width.toLong() * height.toLong()
        return if (pixels > MOBILE_IMAGE_MAX_PIXELS) {
            "图片尺寸过大，已跳过"
        } else {
            null
        }
    }

    fun clipboardTextForLatest(kind: String, text: String?): String? =
        text?.takeIf { kind == "text" && it.isNotBlank() }

    fun latestNonTextMessage(kind: String): String =
        if (kind == "image") {
            "最新记录是图片，可在图片入口查看或下载"
        } else {
            "最新记录不是可写入手机剪贴板的文本"
        }

    fun parseMultiSyncClip(manifest: JSONObject): MultiSyncClip? {
        val protocol = manifest.optString("protocol")
        require(protocol == MULTI_SYNC_PROTOCOL) { "不是 ZSClip 多端同步清单" }
        val clip = manifest.optJSONObject("clip") ?: return null
        return MultiSyncClip(
            id = clip.optString("id"),
            kind = clip.optString("type"),
            hash = clip.optString("hash"),
            preview = clip.optString("preview"),
            content = clip.optString("content").takeIf { it.isNotEmpty() && !clip.isNull("content") },
            dataName = clip.optString("dataName").takeIf { it.isNotEmpty() },
            hasData = clip.optBoolean("hasData", false),
            size = clip.optLong("size", 0L),
            transport = manifest.optString("transport", "unknown").ifBlank { "unknown" }
        )
    }

    fun multiSyncStatusMessage(clip: MultiSyncClip?): String =
        when {
            clip == null -> "多端同步清单暂无记录"
            clip.kind == "text" && !clip.content.isNullOrBlank() ->
                "多端同步最新文本：${clip.content.take(30)}"
            clip.kind == "image" && clip.dataName != null ->
                "多端同步最新记录是图片，可在图片入口查看或下载"
            else -> "多端同步最新记录暂不支持直接写入手机剪贴板"
        }

    fun clipboardTextForMultiSync(clip: MultiSyncClip?): String? =
        clip?.content?.takeIf { clip.kind == "text" && it.isNotBlank() }

    fun multiSyncClipKey(transport: String, clip: MultiSyncClip): String =
        listOf("multi", transport.trim().ifBlank { "unknown" }, clip.id, clip.hash, clip.size.toString())
            .joinToString(":")

    fun multiSyncClipKey(clip: MultiSyncClip): String =
        multiSyncClipKey(clip.transport, clip)

    fun lanLatestKey(
        host: String,
        messageId: String,
        originDeviceId: String,
        originSeq: Long,
        hash: String
    ): String =
        listOf(
            "lan",
            host.trim().ifBlank { "unknown" },
            messageId,
            originDeviceId,
            originSeq.toString(),
            hash
        ).joinToString(":")

    fun lanPairingIdentity(host: String, deviceId: String): String =
        "${host.trim()}|${deviceId.trim()}"

    fun webDavConfigIdentity(url: String, remoteDir: String): String {
        val normalizedUrl = url.trim().trimEnd('/')
        val normalizedDir = remoteDir.trim().ifBlank { "ZS Clip" }
        return "$normalizedUrl|$normalizedDir"
    }

    fun safeMultiSyncDataName(name: String): String? {
        val value = name.trim()
        if (!Regex("""zsclip_image_\d+\.png""").matches(value)) {
            return null
        }
        return value
    }

    fun pairHostFromLink(raw: String?): String? {
        val value = raw?.trim().orEmpty()
        if (value.isBlank()) return null
        val uri = runCatching { URI(value) }.getOrNull() ?: return null
        if (!uri.scheme.equals("zsclip", ignoreCase = true)) return null
        if (!uri.host.equals("pair", ignoreCase = true)) return null
        return queryParam(uri.rawQuery, "host")
            ?.trim()
            ?.takeIf { it.isNotEmpty() }
    }

    fun normalizeCapturedText(raw: String): String {
        var normalized = raw
            .replace("\r\n", "\n")
            .replace('\r', '\n')
            .filterNot { ch ->
                ch == '\u200B' || ch == '\u200C' || ch == '\u200D' || ch == '\uFEFF'
            }
            .lines()
            .joinToString("\n") { it.trimEnd() }
            .trim()
        if (normalized.contains('\n')) {
            return normalized
        }
        if (
            normalized.startsWith("http://") ||
            normalized.startsWith("https://") ||
            normalized.startsWith("www.")
        ) {
            normalized = normalized.split(Regex("\\s+")).firstOrNull().orEmpty()
        }
        return normalized
    }

    private fun queryParam(rawQuery: String?, key: String): String? {
        if (rawQuery.isNullOrBlank()) return null
        return rawQuery.split('&')
            .mapNotNull { part ->
                val index = part.indexOf('=')
                if (index <= 0) null else part.substring(0, index) to part.substring(index + 1)
            }
            .firstOrNull { (name, _) -> name == key }
            ?.second
            ?.let { URLDecoder.decode(it, "UTF-8") }
    }

    private fun baseEnvelope(deviceId: String, seq: Long, kind: String, preview: String): JSONObject =
        JSONObject()
            .put("message_id", "$deviceId-$seq")
            .put("origin_device_id", deviceId)
            .put("origin_seq", seq)
            .put("kind", kind)
            .put("hash", mobileMessageHash(deviceId, seq))
            .put("created_at_ms", seq)
            .put("preview", preview)

    private fun md5Hex(value: String): String {
        return md5Hex(value.toByteArray(Charsets.UTF_8))
    }

    private fun md5Hex(bytes: ByteArray): String {
        val digest = MessageDigest.getInstance("MD5").digest(bytes)
        return digest.joinToString("") { "%02x".format(it) }
    }
}
