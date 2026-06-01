package com.zsclip.lan

import org.json.JSONArray
import org.json.JSONObject
import java.util.Base64

object LanProtocol {
    const val MOBILE_IMAGE_MAX_BYTES = 10 * 1024 * 1024
    const val MOBILE_IMAGE_MAX_BASE64_CHARS = MOBILE_IMAGE_MAX_BYTES * 2
    private val androidCapabilities = listOf("text", "image", "latest", "client_only", "pull_only")

    fun hasPairing(host: String, token: String): Boolean =
        host.trim().isNotEmpty() && token.trim().isNotEmpty()

    fun tileStateLabel(host: String, token: String, running: Boolean): String =
        when {
            !hasPairing(host, token) -> "需要配对"
            running -> "运行中"
            else -> "可用"
        }

    fun cleanShareText(text: String?): String =
        normalizeCapturedText(text.orEmpty())

    fun mobileMessageHash(deviceId: String, seq: Long): String =
        "msg:${deviceId.trim()}:$seq"

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

    fun clipboardTextForLatest(kind: String, text: String?): String? =
        text?.takeIf { kind == "text" && it.isNotBlank() }

    fun latestNonTextMessage(kind: String): String =
        if (kind == "image") {
            "最新记录是图片，可在图片下载页查看"
        } else {
            "最新记录不是可写入手机剪贴板的文本"
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

    private fun baseEnvelope(deviceId: String, seq: Long, kind: String, preview: String): JSONObject =
        JSONObject()
            .put("message_id", "$deviceId-$seq")
            .put("origin_device_id", deviceId)
            .put("origin_seq", seq)
            .put("kind", kind)
            .put("hash", mobileMessageHash(deviceId, seq))
            .put("created_at_ms", seq)
            .put("preview", preview)
}
