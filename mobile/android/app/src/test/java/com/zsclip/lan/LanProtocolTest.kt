package com.zsclip.lan

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.util.Base64

class LanProtocolTest {
    @Test
    fun bdd_mobile_message_hash_is_message_scoped_not_content_crc() {
        assertEquals("msg:android-1:42", LanProtocol.mobileMessageHash("android-1", 42))
    }

    @Test
    fun bdd_android_pair_request_declares_pull_only_capabilities() {
        val body = JSONObject(LanProtocol.pairRequestBody("android-1"))
        val caps = body.getJSONArray("capabilities")
        val values = (0 until caps.length()).map { caps.getString(it) }.toSet()

        assertEquals(0, body.getInt("tcp_port"))
        assertTrue(values.contains("text"))
        assertTrue(values.contains("image"))
        assertTrue(values.contains("latest"))
        assertTrue(values.contains("client_only"))
        assertTrue(values.contains("pull_only"))
        assertFalse(values.contains("receive_clip"))
    }

    @Test
    fun bdd_android_text_envelope_uses_unified_mobile_message_hash() {
        val body = JSONObject(LanProtocol.mobileTextEnvelopeBody("android-1", " hello\r\nworld ", 42))

        assertEquals("android-1-42", body.getString("message_id"))
        assertEquals("text", body.getString("kind"))
        assertEquals("msg:android-1:42", body.getString("hash"))
        assertEquals("hello\nworld", body.getString("text"))
    }

    @Test
    fun bdd_android_image_envelope_uses_same_mobile_message_shape() {
        val png = byteArrayOf(1, 2, 3, 4)
        val body = JSONObject(LanProtocol.mobileImageEnvelopeBody("android-1", png, 99, "a.png"))

        assertEquals("android-1-99", body.getString("message_id"))
        assertEquals("image", body.getString("kind"))
        assertEquals("msg:android-1:99", body.getString("hash"))
        assertEquals("a.png", body.getString("preview"))
        assertTrue(body.isNull("text"))
        assertEquals(0, body.getJSONArray("file_meta").length())
        assertEquals(png.toList(), Base64.getDecoder().decode(body.getString("image_png_base64")).toList())
    }

    @Test
    fun bdd_android_image_payload_validation_blocks_empty_and_oversized_images() {
        assertNull(LanProtocol.validateMobileImagePayload(100, 140))
        assertEquals("图片为空", LanProtocol.validateMobileImagePayload(0, 0))
        assertEquals(
            "图片超过 10MB，已跳过",
            LanProtocol.validateMobileImagePayload(LanProtocol.MOBILE_IMAGE_MAX_BYTES + 1, 1)
        )
        assertEquals(
            "图片编码后过大，已跳过",
            LanProtocol.validateMobileImagePayload(100, LanProtocol.MOBILE_IMAGE_MAX_BASE64_CHARS + 1)
        )
    }

    @Test
    fun bdd_non_text_latest_is_not_written_to_android_clipboard() {
        assertNull(LanProtocol.clipboardTextForLatest("image", "preview text"))
        assertNull(LanProtocol.clipboardTextForLatest("files", "file.txt"))
        assertEquals("hello", LanProtocol.clipboardTextForLatest("text", "hello"))
        assertEquals("最新记录是图片，可在图片下载页查看", LanProtocol.latestNonTextMessage("image"))
    }

    @Test
    fun bdd_tile_requires_pairing_before_network_actions() {
        assertFalse(LanProtocol.hasPairing("", "token"))
        assertFalse(LanProtocol.hasPairing("192.168.1.2:38473", ""))
        assertTrue(LanProtocol.hasPairing("192.168.1.2:38473", "token"))
    }

    @Test
    fun bdd_tile_state_label_distinguishes_pairing_and_running_state() {
        assertEquals("需要配对", LanProtocol.tileStateLabel("", "", false))
        assertEquals("可用", LanProtocol.tileStateLabel("192.168.1.2:38473", "token", false))
        assertEquals("运行中", LanProtocol.tileStateLabel("192.168.1.2:38473", "token", true))
    }

    @Test
    fun bdd_shared_text_is_cleaned_before_push() {
        assertEquals(
            "https://example.com/a",
            LanProtocol.cleanShareText(" https://example.com/a copied from browser ")
        )
        assertEquals("hello\nworld", LanProtocol.cleanShareText(" hello\r\nworld \uFEFF"))
    }

    @Test
    fun bdd_mobile_images_url_uses_pairing_and_query_auth() {
        assertEquals(
            "http://192.168.1.2:38473/mobile/images?device=android+phone&token=tok%2Ben",
            LanClient.mobileImagesUrl("192.168.1.2", "android phone", "tok+en")
        )
    }

    @Test(expected = IllegalStateException::class)
    fun bdd_mobile_images_url_requires_pairing() {
        LanClient.mobileImagesUrl("", "android-1", "token")
    }
}
