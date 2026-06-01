package com.zsclip.lan

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import org.json.JSONObject
import java.io.OutputStreamWriter
import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.HttpURLConnection
import java.net.URLEncoder
import java.net.URL

object LanClient {
    data class DiscoveryResult(val name: String, val host: String, val deviceId: String)
    data class LatestResult(val key: String, val kind: String, val text: String?, val preview: String)
    private var lastOriginSeq: Long = 0

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

    fun pushText(context: Context, host: String, text: String) {
        val targetHost = normalizedHost(host).ifBlank { LanPrefs.pairedHost(context) }
        if (!LanProtocol.hasPairing(targetHost, LanPrefs.token(context))) {
            throw IllegalStateException("请先完成配对")
        }
        val seq = nextOriginSeq()
        val deviceId = LanPrefs.deviceId(context)
        val body = LanProtocol.mobileTextEnvelopeBody(deviceId, text, seq)
        http(context, "POST", targetHost, "/v1/clip", body, true)
        LanPrefs.saveSyncStatus(context, true, "已推送文本到电脑", LanProtocol.mobileMessageHash(deviceId, seq))
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
        val key = listOf(
            clip.optString("message_id"),
            clip.optString("origin_device_id"),
            clip.optLong("origin_seq").toString(),
            clip.optString("hash")
        ).joinToString(":")
        return LatestResult(
            key = key,
            kind = clip.optString("kind"),
            text = clip.optString("text").takeIf { it.isNotEmpty() },
            preview = clip.optString("preview")
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

    fun pullLatestToClipboard(context: Context, force: Boolean): String {
        val latest = latest(context)
        if (latest == null) {
            val message = "Windows 暂无记录"
            LanPrefs.saveSyncStatus(context, true, message)
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
        val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
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
        val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        val raw = clipboard.primaryClip
            ?.takeIf { it.itemCount > 0 }
            ?.getItemAt(0)
            ?.coerceToText(context)
            ?.toString()
            .orEmpty()
        val text = LanProtocol.cleanShareText(raw)
        if (text.isBlank()) {
            throw IllegalStateException("手机剪贴板没有可推送的文本")
        }
        pushText(context, LanPrefs.pairedHost(context), text)
        return "已推送到电脑：${text.take(30)}"
    }

    private fun http(
        context: Context,
        method: String,
        host: String,
        path: String,
        body: String?,
        auth: Boolean
    ): JSONObject {
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
    }

    @Synchronized
    private fun nextOriginSeq(): Long {
        val now = System.currentTimeMillis()
        val next = if (now <= lastOriginSeq) lastOriginSeq + 1 else now
        lastOriginSeq = next
        return next
    }
}
