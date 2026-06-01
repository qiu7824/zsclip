package com.zsclip.lan

import android.content.Context
import java.text.DateFormat
import java.util.Date
import java.util.UUID

object LanPrefs {
    private const val PREF = "lan"
    private const val KEY_DEVICE_ID = "device_id"
    private const val KEY_LEGACY_HOST = "host"
    private const val KEY_TOKEN = "token"
    private const val KEY_PAIRED_HOST = "paired_host"
    private const val KEY_PAIRED_DEVICE_ID = "paired_device_id"
    private const val KEY_PAIRED_DEVICE_NAME = "paired_device_name"
    private const val KEY_CANDIDATE_HOST = "candidate_host"
    private const val KEY_CANDIDATE_NAME = "candidate_name"
    private const val KEY_LAST_CLIP = "last_clip_key"
    private const val KEY_AUTO_SYNC = "auto_sync_enabled"

    data class Pairing(
        val host: String,
        val token: String,
        val deviceId: String,
        val deviceName: String
    )

    fun deviceId(context: Context): String {
        val prefs = context.getSharedPreferences(PREF, Context.MODE_PRIVATE)
        return prefs.getString(KEY_DEVICE_ID, null) ?: UUID.randomUUID().toString().also {
            prefs.edit().putString(KEY_DEVICE_ID, it).apply()
        }
    }

    fun host(context: Context): String = pairedHost(context)

    fun pairedHost(context: Context): String {
        migrateLegacyPairing(context)
        return prefs(context).getString(KEY_PAIRED_HOST, "") ?: ""
    }

    fun pairedDeviceId(context: Context): String {
        migrateLegacyPairing(context)
        return prefs(context).getString(KEY_PAIRED_DEVICE_ID, "") ?: ""
    }

    fun pairedDeviceName(context: Context): String {
        migrateLegacyPairing(context)
        return prefs(context).getString(KEY_PAIRED_DEVICE_NAME, "") ?: ""
    }

    fun candidateHost(context: Context): String =
        prefs(context).getString(KEY_CANDIDATE_HOST, "") ?: ""

    fun candidateName(context: Context): String =
        prefs(context).getString(KEY_CANDIDATE_NAME, "") ?: ""

    fun displayHost(context: Context): String =
        pairedHost(context).ifBlank { candidateHost(context) }

    fun token(context: Context): String {
        migrateLegacyPairing(context)
        return prefs(context).getString(KEY_TOKEN, "") ?: ""
    }

    fun hasPairing(context: Context): Boolean =
        LanProtocol.hasPairing(pairedHost(context), token(context))

    fun saveHost(context: Context, host: String) {
        saveCandidate(context, host, candidateName(context))
    }

    fun saveCandidate(context: Context, host: String, name: String = "") {
        prefs(context)
            .edit()
            .putString(KEY_CANDIDATE_HOST, LanClient.normalizedHost(host))
            .putString(KEY_CANDIDATE_NAME, name)
            .putString(KEY_LEGACY_HOST, LanClient.normalizedHost(host))
            .apply()
    }

    fun saveToken(context: Context, token: String) {
        prefs(context)
            .edit()
            .putString(KEY_TOKEN, token)
            .apply()
    }

    fun savePairing(
        context: Context,
        host: String,
        token: String,
        deviceId: String,
        deviceName: String
    ) {
        val normalized = LanClient.normalizedHost(host)
        prefs(context)
            .edit()
            .putString(KEY_PAIRED_HOST, normalized)
            .putString(KEY_PAIRED_DEVICE_ID, deviceId)
            .putString(KEY_PAIRED_DEVICE_NAME, deviceName.ifBlank { "Windows" })
            .putString(KEY_TOKEN, token)
            .putString(KEY_CANDIDATE_HOST, normalized)
            .putString(KEY_CANDIDATE_NAME, deviceName)
            .putString(KEY_LEGACY_HOST, normalized)
            .apply()
    }

    fun currentPairing(context: Context): Pairing? {
        val host = pairedHost(context)
        val token = token(context)
        if (!LanProtocol.hasPairing(host, token)) {
            return null
        }
        return Pairing(
            host = host,
            token = token,
            deviceId = pairedDeviceId(context),
            deviceName = pairedDeviceName(context).ifBlank { host }
        )
    }

    fun clearPairing(context: Context) {
        prefs(context)
            .edit()
            .remove(KEY_TOKEN)
            .remove(KEY_PAIRED_HOST)
            .remove(KEY_PAIRED_DEVICE_ID)
            .remove(KEY_PAIRED_DEVICE_NAME)
            .remove(KEY_LAST_CLIP)
            .putBoolean(KEY_AUTO_SYNC, false)
            .apply()
    }

    fun saveLastClipKey(context: Context, key: String) {
        prefs(context)
            .edit()
            .putString(KEY_LAST_CLIP, key)
            .apply()
    }

    fun lastClipKey(context: Context): String =
        prefs(context).getString(KEY_LAST_CLIP, "") ?: ""

    fun saveAutoSync(context: Context, enabled: Boolean) {
        prefs(context)
            .edit()
            .putBoolean(KEY_AUTO_SYNC, enabled)
            .apply()
    }

    fun autoSync(context: Context): Boolean =
        prefs(context).getBoolean(KEY_AUTO_SYNC, false)

    fun saveSyncStatus(context: Context, success: Boolean, message: String, latestKey: String = "") {
        prefs(context)
            .edit()
            .putBoolean("last_sync_success", success)
            .putString("last_sync_message", message)
            .putString("last_sync_key", latestKey)
            .putLong("last_sync_at", System.currentTimeMillis())
            .apply()
    }

    fun lastSyncStatusText(context: Context): String {
        val prefs = prefs(context)
        val at = prefs.getLong("last_sync_at", 0L)
        if (at <= 0L) {
            return "最近同步：暂无"
        }
        val ok = prefs.getBoolean("last_sync_success", false)
        val message = prefs.getString("last_sync_message", "") ?: ""
        val key = prefs.getString("last_sync_key", "") ?: ""
        val time = DateFormat.getDateTimeInstance(DateFormat.SHORT, DateFormat.MEDIUM)
            .format(Date(at))
        return buildString {
            append("最近同步：")
            append(if (ok) "成功" else "失败")
            append("  ")
            append(time)
            if (message.isNotBlank()) {
                append("\n")
                append(message)
            }
            if (key.isNotBlank()) {
                append("\nkey: ")
                append(key.take(48))
                if (key.length > 48) {
                    append("...")
                }
            }
        }
    }

    private fun migrateLegacyPairing(context: Context) {
        val prefs = prefs(context)
        val paired = prefs.getString(KEY_PAIRED_HOST, "").orEmpty()
        val legacyHost = prefs.getString(KEY_LEGACY_HOST, "").orEmpty()
        val token = prefs.getString(KEY_TOKEN, "").orEmpty()
        if (paired.isBlank() && legacyHost.isNotBlank() && token.isNotBlank()) {
            prefs.edit()
                .putString(KEY_PAIRED_HOST, LanClient.normalizedHost(legacyHost))
                .putString(KEY_PAIRED_DEVICE_NAME, "Windows")
                .putString(KEY_CANDIDATE_HOST, LanClient.normalizedHost(legacyHost))
                .apply()
        }
    }

    private fun prefs(context: Context) =
        context.getSharedPreferences(PREF, Context.MODE_PRIVATE)
}
