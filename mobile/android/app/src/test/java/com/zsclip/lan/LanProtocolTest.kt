package com.zsclip.lan

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.net.ServerSocket
import java.net.Socket
import java.util.Base64
import java.util.Collections
import kotlin.concurrent.thread

class LanProtocolTest {
    private data class RecordedRequest(
        val method: String,
        val path: String,
        val host: String,
        val auth: String,
        val body: String
    )

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
    fun bdd_multi_auto_sync_state_accepts_lan_or_webdav() {
        assertEquals("运行中", LanProtocol.multiAutoSyncStateLabel(hasPairing = false, hasWebDav = false, running = true))
        assertEquals("局域网可用", LanProtocol.multiAutoSyncStateLabel(hasPairing = true, hasWebDav = false, running = false))
        assertEquals("WebDAV 可用", LanProtocol.multiAutoSyncStateLabel(hasPairing = false, hasWebDav = true, running = false))
        assertEquals("需配对或配置", LanProtocol.multiAutoSyncStateLabel(hasPairing = false, hasWebDav = false, running = false))
    }

    @Test
    fun bdd_multi_pull_tile_accepts_webdav_without_pairing() {
        assertEquals("运行中", LanProtocol.multiPullStateLabel(hasPairing = false, hasWebDav = false, running = true))
        assertEquals("局域网 / WebDAV 可用", LanProtocol.multiPullStateLabel(hasPairing = true, hasWebDav = true, running = false))
        assertEquals("WebDAV 可用", LanProtocol.multiPullStateLabel(hasPairing = false, hasWebDav = true, running = false))
        assertEquals("需配对或配置", LanProtocol.multiPullStateLabel(hasPairing = false, hasWebDav = false, running = false))
    }

    @Test
    fun bdd_multi_push_tile_accepts_webdav_without_pairing() {
        assertEquals("运行中", LanProtocol.multiPushStateLabel(hasPairing = false, hasWebDav = false, running = true))
        assertEquals("局域网 / WebDAV 可用", LanProtocol.multiPushStateLabel(hasPairing = true, hasWebDav = true, running = false))
        assertEquals("局域网可用", LanProtocol.multiPushStateLabel(hasPairing = true, hasWebDav = false, running = false))
        assertEquals("WebDAV 可用", LanProtocol.multiPushStateLabel(hasPairing = false, hasWebDav = true, running = false))
        assertEquals("需配对或配置", LanProtocol.multiPushStateLabel(hasPairing = false, hasWebDav = false, running = false))
    }

    @Test
    fun bdd_multi_auto_sync_fallback_message_mentions_webdav() {
        assertEquals(
            "局域网同步失败，已改用 WebDAV：WebDAV 已写入手机剪贴板：hello",
            LanProtocol.autoSyncWebDavFallbackMessage(
                "connect timed out",
                "WebDAV 已写入手机剪贴板：hello"
            )
        )
    }

    @Test
    fun bdd_multi_push_fallback_message_mentions_webdav() {
        assertEquals(
            "局域网推送失败，已改用 WebDAV：已推送 WebDAV 文本：hello",
            LanProtocol.pushWebDavFallbackMessage(
                "connect timed out",
                "已推送 WebDAV 文本：hello"
            )
        )
    }

    @Test
    fun bdd_multi_check_fallback_message_mentions_webdav() {
        assertEquals(
            "局域网检查失败，已改用 WebDAV：WebDAV：多端同步最新文本：hello",
            LanProtocol.checkWebDavFallbackMessage(
                "connect timed out",
                "WebDAV：多端同步最新文本：hello"
            )
        )
    }

    @Test
    fun bdd_shared_text_route_prefers_lan_then_webdav_then_manual() {
        assertEquals("lan", LanProtocol.sharedTextRoute(hasPairing = true, hasWebDav = true))
        assertEquals("lan", LanProtocol.sharedTextRoute(hasPairing = true, hasWebDav = false))
        assertEquals("webdav", LanProtocol.sharedTextRoute(hasPairing = false, hasWebDav = true))
        assertEquals("manual", LanProtocol.sharedTextRoute(hasPairing = false, hasWebDav = false))
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
    fun bdd_android_webdav_image_manifest_uses_lazy_data_name() {
        val png = "PNGDATA".toByteArray()
        val manifest = JSONObject(LanProtocol.webDavImageManifestBody("android-1", png, 99, "a.png"))
        val clip = manifest.getJSONObject("clip")

        assertEquals("ZSCLIP_MULTI_SYNC_V1", manifest.getString("protocol"))
        assertEquals("webdav", manifest.getString("transport"))
        assertEquals("android:image:android-1:99", clip.getString("id"))
        assertEquals("image", clip.getString("type"))
        assertEquals("a.png", clip.getString("preview"))
        assertTrue(clip.getBoolean("hasData"))
        assertEquals("zsclip_image_99.png", clip.getString("dataName"))
        assertEquals(png.size.toLong(), clip.getLong("size"))
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
    fun bdd_android_image_dimension_validation_blocks_bad_or_huge_decode() {
        assertNull(LanProtocol.validateMobileImageDimensions(3840, 2160))
        assertEquals("图片无法解码，已跳过", LanProtocol.validateMobileImageDimensions(0, 2160))
        assertEquals("图片无法解码，已跳过", LanProtocol.validateMobileImageDimensions(3840, -1))
        assertEquals("图片尺寸过大，已跳过", LanProtocol.validateMobileImageDimensions(9000, 4000))
    }

    @Test
    fun bdd_non_text_latest_is_not_written_to_android_clipboard() {
        assertNull(LanProtocol.clipboardTextForLatest("image", "preview text"))
        assertNull(LanProtocol.clipboardTextForLatest("files", "file.txt"))
        assertEquals("hello", LanProtocol.clipboardTextForLatest("text", "hello"))
        assertEquals("最新记录是图片，可在图片入口查看或下载", LanProtocol.latestNonTextMessage("image"))
    }

    @Test
    fun bdd_lan_latest_key_includes_windows_target_for_dedupe() {
        val first = LanProtocol.lanLatestKey(
            host = "192.168.1.2:38473",
            messageId = "m1",
            originDeviceId = "pc",
            originSeq = 7,
            hash = "md5:abc"
        )
        val second = LanProtocol.lanLatestKey(
            host = "192.168.1.3:38473",
            messageId = "m1",
            originDeviceId = "pc",
            originSeq = 7,
            hash = "md5:abc"
        )

        assertEquals("lan:192.168.1.2:38473:m1:pc:7:md5:abc", first)
        assertFalse(first == second)
    }

    @Test
    fun bdd_lan_pairing_identity_normalizes_for_dedupe_reset() {
        assertEquals(
            "192.168.1.2:38473|pc-1",
            LanProtocol.lanPairingIdentity(" 192.168.1.2:38473 ", " pc-1 ")
        )
        assertFalse(
            LanProtocol.lanPairingIdentity("192.168.1.2:38473", "pc-1") ==
                LanProtocol.lanPairingIdentity("192.168.1.3:38473", "pc-1")
        )
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
    fun bdd_auto_sync_signs_clipboard_text_for_duplicate_guard() {
        val first = LanProtocol.clipboardTextSignature(" hello  ")
        val second = LanProtocol.clipboardTextSignature("hello")

        assertEquals(first, second)
        assertTrue(first.startsWith("text:md5:"))
        assertEquals("", LanProtocol.clipboardTextSignature("  "))
    }

    @Test
    fun bdd_auto_sync_recognizes_records_originated_from_this_android_device() {
        assertTrue(LanProtocol.isOwnLatest("android-1", "android-1"))
        assertFalse(LanProtocol.isOwnLatest("pc-1", "android-1"))
        val clip = LanProtocol.MultiSyncClip(
            id = "android:text:android-1:42",
            kind = "text",
            hash = "md5:abc",
            preview = "hello",
            content = "hello",
            dataName = null,
            hasData = false,
            size = 5,
            transport = "webdav"
        )

        assertTrue(LanProtocol.isOwnMultiSyncClip(clip, "android-1"))
        assertFalse(LanProtocol.isOwnMultiSyncClip(clip, "android-2"))
    }

    @Test
    fun bdd_auto_sync_noop_messages_are_hidden_from_foreground_log() {
        assertTrue(LanProtocol.isAutoSyncNoopMessage("手机剪贴板无新文本"))
        assertTrue(LanProtocol.isAutoSyncNoopMessage("手机剪贴板自动推送正在进行"))
        assertTrue(LanProtocol.isAutoSyncNoopMessage("没有新记录"))
        assertFalse(LanProtocol.isAutoSyncNoopMessage("已自动推送到电脑：hello"))
    }

    @Test
    fun bdd_mobile_images_url_uses_pairing_and_query_auth() {
        assertEquals(
            "http://192.168.1.2:38473/mobile/images?device=android+phone&token=tok%2Ben",
            LanClient.mobileImagesUrl("192.168.1.2", "android phone", "tok+en")
        )
    }

    @Test
    fun bdd_mobile_media_json_paths_are_stable_and_bounded() {
        assertEquals("/v1/mobile/items?limit=50", LanClient.mobileItemsPath())
        assertEquals("/v1/mobile/items?limit=1", LanClient.mobileItemsPath(0))
        assertEquals("/v1/mobile/items?limit=100", LanClient.mobileItemsPath(500))
        assertEquals("/v1/mobile/items/42/image", LanClient.mobileItemImagePath(42))
        assertEquals("/v1/mobile/items/42/file/3", LanClient.mobileItemFilePath(42, 3))
    }

    @Test
    fun bdd_multi_sync_manifest_url_uses_syncclipboard_entrypoint() {
        assertEquals(
            "http://192.168.1.2:38473/zsSyncClipboard.json?device=android+phone&token=tok%2Ben",
            LanClient.multiSyncManifestUrl("192.168.1.2", "android phone", "tok+en")
        )
    }

    @Test
    fun bdd_multi_sync_manifest_text_is_parsed_for_android_status() {
        val clip = LanProtocol.parseMultiSyncClip(
            JSONObject(
                """
                {
                  "protocol": "ZSCLIP_MULTI_SYNC_V1",
                  "version": 1,
                  "transport": "lan",
                  "clip": {
                    "id": "db:text:7",
                    "type": "text",
                    "hash": "md5:abc",
                    "preview": "hello",
                    "content": "hello world",
                    "hasData": false,
                    "size": 11,
                    "source_app": "notepad.exe",
                    "created_at": "2026-06-02 10:00:00"
                  }
                }
                """.trimIndent()
            )
        )

        assertEquals("db:text:7", clip?.id)
        assertEquals("text", clip?.kind)
        assertEquals("md5:abc", clip?.hash)
        assertEquals("lan", clip?.transport)
        assertEquals("hello world", clip?.content)
        assertEquals("hello world", LanProtocol.clipboardTextForMultiSync(clip))
        assertEquals("多端同步最新文本：hello world", LanProtocol.multiSyncStatusMessage(clip))
        assertEquals(
            "multi:lan:db:text:7:md5:abc:11",
            LanProtocol.multiSyncClipKey(clip!!)
        )
    }

    @Test
    fun bdd_multi_sync_manifest_null_clip_is_empty_android_state() {
        val clip = LanProtocol.parseMultiSyncClip(
            JSONObject(
                """
                {
                  "protocol": "ZSCLIP_MULTI_SYNC_V1",
                  "version": 1,
                  "transport": "webdav",
                  "clip": null
                }
                """.trimIndent()
            )
        )

        assertNull(clip)
        assertEquals("多端同步清单暂无记录", LanProtocol.multiSyncStatusMessage(clip))
    }

    @Test
    fun bdd_android_webdav_text_manifest_uses_shared_contract() {
        val body = JSONObject(LanProtocol.webDavTextManifestBody("android-1", " hello\r\ncloud ", 42))
        val clip = LanProtocol.parseMultiSyncClip(body)

        assertEquals("ZSCLIP_MULTI_SYNC_V1", body.getString("protocol"))
        assertEquals("webdav", body.getString("transport"))
        assertEquals("android:text:android-1:42", clip?.id)
        assertEquals("text", clip?.kind)
        assertEquals("webdav", clip?.transport)
        assertEquals("hello\ncloud", clip?.content)
        assertTrue(clip?.hash.orEmpty().startsWith("md5:"))
        assertEquals("hello\ncloud", LanProtocol.clipboardTextForMultiSync(clip))
    }

    @Test
    fun bdd_webdav_fetch_reads_shared_manifest_with_basic_auth() {
        val manifest = LanProtocol.webDavTextManifestBody("android-1", "cloud text", 77)
        FakeWebDavServer(getManifestBody = manifest).use { server ->
            val clip = LanClient.fetchWebDavMultiSyncClipForConfig(
                baseUrl = "http://127.0.0.1:${server.port}/root",
                remoteDir = "team/ZS Clip",
                user = "alice",
                pass = "secret"
            )

            assertEquals("android:text:android-1:77", clip?.id)
            assertEquals("webdav", clip?.transport)
            assertEquals("cloud text", clip?.content)
            assertEquals("multi:webdav:android:text:android-1:77:${clip?.hash}:${clip?.size}", LanProtocol.multiSyncClipKey(clip!!))

            assertEquals(1, server.requests.size)
            assertEquals("GET", server.requests.first().method)
            assertEquals("/root/team/ZS%20Clip/zsSyncClipboard.json", server.requests.first().path)
            assertEquals("Basic YWxpY2U6c2VjcmV0", server.requests.first().auth)
        }
    }

    @Test
    fun bdd_webdav_fetch_missing_shared_manifest_is_empty_state() {
        FakeWebDavServer().use { server ->
            val clip = LanClient.fetchWebDavMultiSyncClipForConfig(
                baseUrl = "http://127.0.0.1:${server.port}/root",
                remoteDir = "team/ZS Clip",
                user = "alice",
                pass = "secret"
            )

            assertNull(clip)
            assertEquals(1, server.requests.size)
            assertEquals("GET", server.requests.first().method)
            assertEquals("/root/team/ZS%20Clip/zsSyncClipboard.json", server.requests.first().path)
            assertEquals("Basic YWxpY2U6c2VjcmV0", server.requests.first().auth)
        }
    }

    @Test
    fun bdd_webdav_fetch_empty_shared_manifest_body_is_empty_state() {
        FakeWebDavServer(getManifestBody = "").use { server ->
            val clip = LanClient.fetchWebDavMultiSyncClipForConfig(
                baseUrl = "http://127.0.0.1:${server.port}/root",
                remoteDir = "team/ZS Clip",
                user = "alice",
                pass = "secret"
            )

            assertNull(clip)
            assertEquals(1, server.requests.size)
            assertEquals("GET", server.requests.first().method)
            assertEquals("/root/team/ZS%20Clip/zsSyncClipboard.json", server.requests.first().path)
        }
    }

    @Test
    fun bdd_webdav_text_push_creates_dirs_and_puts_shared_manifest_with_basic_auth() {
        FakeWebDavServer().use { server ->
            val base = "http://127.0.0.1:${server.port}/root"
            val result = LanClient.pushWebDavTextForConfig(
                baseUrl = base,
                remoteDir = "team/ZS Clip",
                user = "alice",
                pass = "secret",
                deviceId = "android-1",
                text = " hello\r\ncloud ",
                seq = 42
            )

            assertEquals(
                listOf("MKCOL", "MKCOL", "PUT"),
                server.requests.map { it.method }
            )
            assertEquals(
                listOf(
                    "/root/team",
                    "/root/team/ZS%20Clip",
                    "/root/team/ZS%20Clip/zsSyncClipboard.json"
                ),
                server.requests.map { it.path }
            )
            val auth = "Basic ${Base64.getEncoder().encodeToString("alice:secret".toByteArray(Charsets.UTF_8))}"
            assertTrue(server.requests.all { it.auth == auth })
            assertTrue(server.requests.all { it.host == "127.0.0.1:${server.port}" })

            val manifest = JSONObject(server.requests.last().body)
            val clip = LanProtocol.parseMultiSyncClip(manifest)!!
            assertEquals("webdav", manifest.getString("transport"))
            assertEquals("android:text:android-1:42", clip.id)
            assertEquals("hello\ncloud", clip.content)
            assertEquals("已推送 WebDAV 文本：hello\ncloud", result.message)
            assertEquals(LanProtocol.multiSyncClipKey("webdav", clip), result.key)
        }
    }

    @Test
    fun bdd_webdav_image_push_uploads_file_before_shared_manifest() {
        FakeWebDavServer().use { server ->
            val png = "PNGDATA".toByteArray()
            val result = LanClient.pushWebDavImageForConfig(
                baseUrl = "http://127.0.0.1:${server.port}/root",
                remoteDir = "team/ZS Clip",
                user = "alice",
                pass = "secret",
                deviceId = "android-1",
                imagePngBytes = png,
                displayName = "shot.png",
                seq = 99
            )

            val putRequests = server.requests.filter { it.method == "PUT" }
            assertEquals(
                listOf(
                    "/root/team/ZS%20Clip/file/zsclip_image_99.png",
                    "/root/team/ZS%20Clip/zsSyncClipboard.json"
                ),
                putRequests.map { it.path }
            )
            val imagePut = putRequests.first()
            assertEquals("Basic YWxpY2U6c2VjcmV0", imagePut.auth)
            assertEquals("PNGDATA", imagePut.body)
            val manifest = JSONObject(putRequests.last().body)
            val clip = manifest.getJSONObject("clip")
            assertEquals("image", clip.getString("type"))
            assertEquals("zsclip_image_99.png", clip.getString("dataName"))
            assertEquals("已推送 WebDAV 图片：shot.png", result.message)
        }
    }

    @Test
    fun bdd_multi_sync_manifest_image_points_to_safe_lazy_download() {
        val clip = LanProtocol.parseMultiSyncClip(
            JSONObject(
                """
                {
                  "protocol": "ZSCLIP_MULTI_SYNC_V1",
                  "version": 1,
                  "transport": "lan",
                  "clip": {
                    "id": "db:image:9",
                    "type": "image",
                    "hash": "image:9:2048",
                    "preview": "shot.png",
                    "content": null,
                    "hasData": true,
                    "dataName": "zsclip_image_9.png",
                    "size": 2048,
                    "source_app": "snippingtool.exe",
                    "created_at": "2026-06-02 10:00:00",
                    "width": 100,
                    "height": 80
                  }
                }
                """.trimIndent()
            )
        )

        assertEquals("image", clip?.kind)
        assertEquals("zsclip_image_9.png", clip?.dataName)
        assertNull(LanProtocol.clipboardTextForMultiSync(clip))
        assertEquals("多端同步最新记录是图片，可在图片入口查看或下载", LanProtocol.multiSyncStatusMessage(clip))
        assertEquals(
            "http://192.168.1.2:38473/file/zsclip_image_9.png?device=android+phone&token=tok%2Ben",
            LanClient.multiSyncDataUrl("192.168.1.2", "android phone", "tok+en", "zsclip_image_9.png")
        )
    }

    @Test
    fun bdd_multi_sync_data_name_rejects_path_traversal() {
        assertEquals("zsclip_image_9.png", LanProtocol.safeMultiSyncDataName("zsclip_image_9.png"))
        assertNull(LanProtocol.safeMultiSyncDataName("../zsclip_image_9.png"))
        assertNull(LanProtocol.safeMultiSyncDataName("zsclip_image_9.png/evil"))
    }

    @Test
    fun bdd_webdav_config_identity_normalizes_target_for_dedupe_reset() {
        assertEquals(
            "https://dav.example.com/root|ZS Clip",
            LanProtocol.webDavConfigIdentity(" https://dav.example.com/root/ ", "")
        )
        assertEquals(
            LanProtocol.webDavConfigIdentity("https://dav.example.com/root", "ZS Clip"),
            LanProtocol.webDavConfigIdentity("https://dav.example.com/root/", " ZS Clip ")
        )
        assertFalse(
            LanProtocol.webDavConfigIdentity("https://dav.example.com/root", "ZS Clip") ==
                LanProtocol.webDavConfigIdentity("https://dav.example.com/root", "Team")
        )
    }

    @Test
    fun bdd_webdav_multi_sync_urls_share_desktop_layout() {
        assertEquals(
            "https://dav.example.com/root/ZS%20Clip/zsSyncClipboard.json",
            LanClient.webDavManifestUrl("https://dav.example.com/root/", "ZS Clip")
        )
        assertEquals(
            "https://dav.example.com/root/team/ZS%20Clip/zsSyncClipboard.json",
            LanClient.webDavManifestUrl("https://dav.example.com/root/", "team/ZS Clip")
        )
        assertEquals(
            "https://dav.example.com/root/ZS%20Clip/file/zsclip_image_9.png",
            LanClient.webDavDataUrl("https://dav.example.com/root/", "ZS Clip", "zsclip_image_9.png")
        )
    }

    @Test
    fun bdd_webdav_image_action_opens_latest_lazy_data_url() {
        val clip = LanProtocol.parseMultiSyncClip(
            JSONObject(
                """
                {
                  "protocol": "ZSCLIP_MULTI_SYNC_V1",
                  "version": 1,
                  "transport": "webdav",
                  "clip": {
                    "id": "db:image:9",
                    "type": "image",
                    "hash": "image:9:2048",
                    "preview": "shot.png",
                    "content": null,
                    "hasData": true,
                    "dataName": "zsclip_image_9.png",
                    "size": 2048
                  }
                }
                """.trimIndent()
            )
        )

        assertEquals(
            "https://dav.example.com/root/ZS%20Clip/file/zsclip_image_9.png",
            LanClient.webDavImageUrlFromClip("https://dav.example.com/root/", "ZS Clip", clip)
        )
    }

    @Test
    fun bdd_webdav_image_download_uses_auth_header_without_leaking_credentials_to_url() {
        val clip = LanProtocol.MultiSyncClip(
            id = "db:image:9",
            kind = "image",
            hash = "image:9:2048",
            preview = "shot.png",
            content = null,
            dataName = "zsclip_image_9.png",
            hasData = true,
            size = 2048
        )

        val request = LanClient.webDavImageDownloadRequest(
            baseUrl = "https://dav.example.com/root/",
            remoteDir = "ZS Clip",
            user = "alice",
            pass = "secret",
            clip = clip
        )

        assertEquals("https://dav.example.com/root/ZS%20Clip/file/zsclip_image_9.png", request.url)
        assertEquals("zsclip_image_9.png", request.fileName)
        assertEquals("Basic YWxpY2U6c2VjcmV0", request.authHeader)
        assertFalse(request.url.contains("alice"))
        assertFalse(request.url.contains("secret"))
    }

    @Test(expected = IllegalArgumentException::class)
    fun bdd_webdav_image_action_rejects_unsafe_data_name() {
        val clip = LanProtocol.MultiSyncClip(
            id = "db:image:9",
            kind = "image",
            hash = "image:9:2048",
            preview = "shot.png",
            content = null,
            dataName = "../zsclip_image_9.png",
            hasData = true,
            size = 2048,
            transport = "webdav"
        )

        LanClient.webDavImageUrlFromClip("https://dav.example.com/root/", "ZS Clip", clip)
    }

    @Test(expected = IllegalStateException::class)
    fun bdd_webdav_image_action_rejects_non_image_latest() {
        val clip = LanProtocol.MultiSyncClip(
            id = "android:text:1",
            kind = "text",
            hash = "md5:abc",
            preview = "hello",
            content = "hello",
            dataName = null,
            hasData = false,
            size = 5
        )

        LanClient.webDavImageUrlFromClip("https://dav.example.com/root/", "ZS Clip", clip)
    }

    @Test(expected = IllegalArgumentException::class)
    fun bdd_webdav_multi_sync_data_url_rejects_unsafe_name() {
        LanClient.webDavDataUrl("https://dav.example.com/root/", "ZS Clip", "../zsclip_image_9.png")
    }

    @Test
    fun bdd_mobile_setup_url_opens_shared_mobile_entrypoint() {
        assertEquals(
            "http://192.168.1.2:38473/mobile/setup",
            LanClient.mobileSetupUrl("192.168.1.2")
        )
        assertEquals(
            "http://192.168.1.2:38473/mobile/setup",
            LanClient.mobileSetupUrl("http://192.168.1.2:38473/mobile/setup")
        )
    }

    @Test
    fun bdd_android_pair_link_extracts_host_from_qr_payload() {
        assertEquals(
            "192.168.1.2:38473",
            LanProtocol.pairHostFromLink("zsclip://pair?host=192.168.1.2%3A38473")
        )
        assertEquals(
            "http://192.168.1.2:38473/mobile/setup",
            LanProtocol.pairHostFromLink("zsclip://pair?host=http%3A%2F%2F192.168.1.2%3A38473%2Fmobile%2Fsetup")
        )
        assertNull(LanProtocol.pairHostFromLink("https://example.com/pair?host=192.168.1.2"))
        assertNull(LanProtocol.pairHostFromLink("zsclip://pair"))
    }

    @Test(expected = IllegalStateException::class)
    fun bdd_mobile_images_url_requires_pairing() {
        LanClient.mobileImagesUrl("", "android-1", "token")
    }

    private class FakeWebDavServer(
        private val getManifestBody: String? = null
    ) : AutoCloseable {
        private val socket = ServerSocket(0)
        private var running = true
        val port: Int = socket.localPort
        val requests: MutableList<RecordedRequest> =
            Collections.synchronizedList(mutableListOf())
        private val worker = thread(start = true) {
            while (running && requests.size < 8) {
                runCatching { socket.accept().use { handle(it) } }
            }
        }

        private fun handle(client: Socket) {
            val reader = client.getInputStream().bufferedReader(Charsets.UTF_8)
            val requestLine = reader.readLine().orEmpty()
            val parts = requestLine.split(' ')
            val method = parts.getOrElse(0) { "" }
            val path = parts.getOrElse(1) { "" }
            val headers = mutableMapOf<String, String>()
            while (true) {
                val line = reader.readLine() ?: break
                if (line.isEmpty()) break
                val idx = line.indexOf(':')
                if (idx > 0) {
                    headers[line.substring(0, idx).trim().lowercase()] =
                        line.substring(idx + 1).trim()
                }
            }
            val contentLength = headers["content-length"]?.toIntOrNull() ?: 0
            val body = if (contentLength > 0) {
                val chars = CharArray(contentLength)
                var read = 0
                while (read < contentLength) {
                    val n = reader.read(chars, read, contentLength - read)
                    if (n <= 0) break
                    read += n
                }
                String(chars, 0, read)
            } else {
                ""
            }
            requests.add(
                RecordedRequest(
                    method = method,
                    path = path,
                    host = headers["host"].orEmpty(),
                    auth = headers["authorization"].orEmpty(),
                    body = body
                )
            )
            val responseBody = if (method == "GET" && path.endsWith("/zsSyncClipboard.json")) {
                getManifestBody.orEmpty()
            } else {
                ""
            }
            val code = when {
                method == "GET" && path.endsWith("/zsSyncClipboard.json") && getManifestBody != null -> 200
                method == "MKCOL" -> 201
                method == "PUT" -> 204
                else -> 404
            }
            client.getOutputStream().write(
                "HTTP/1.1 $code OK\r\nContent-Length: ${responseBody.toByteArray(Charsets.UTF_8).size}\r\nConnection: close\r\n\r\n$responseBody"
                    .toByteArray(Charsets.UTF_8)
            )
        }

        override fun close() {
            running = false
            socket.close()
            worker.join(1000)
        }
    }
}
