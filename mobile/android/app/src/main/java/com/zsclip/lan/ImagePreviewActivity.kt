package com.zsclip.lan

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.content.res.Configuration
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.util.TypedValue
import android.view.Gravity
import android.view.ViewGroup
import android.widget.Button
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import kotlin.concurrent.thread

class ImagePreviewActivity : Activity() {
    private val main = Handler(Looper.getMainLooper())
    private lateinit var titleView: TextView
    private lateinit var statusView: TextView
    private lateinit var imageView: ImageView
    private lateinit var saveButton: Button
    private lateinit var shareButton: Button
    private lateinit var palette: Palette
    private var imageBytes: ByteArray? = null
    private var imageName = "zsclip_image.png"

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        palette = Palette.from(this)
        setContentView(createContentView())
        loadImage()
    }

    private fun createContentView(): LinearLayout =
        LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(16), dp(16), dp(16), dp(16))
            setBackgroundColor(palette.bg)

            titleView = TextView(this@ImagePreviewActivity).apply {
                text = intent.getStringExtra(EXTRA_TITLE).orEmpty().ifBlank { "图片预览" }
                setTextColor(palette.text)
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 20f)
                typeface = Typeface.DEFAULT_BOLD
                maxLines = 2
            }
            addView(titleView, matchWrap())

            statusView = TextView(this@ImagePreviewActivity).apply {
                text = "正在加载..."
                setTextColor(palette.muted)
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 14f)
            }
            addView(statusView, topMargin(matchWrap(), 6))

            imageView = ImageView(this@ImagePreviewActivity).apply {
                setBackgroundColor(palette.panel)
                scaleType = ImageView.ScaleType.FIT_CENTER
                adjustViewBounds = true
                contentDescription = "图片预览"
            }
            addView(imageView, LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                0,
                1f
            ).apply {
                topMargin = dp(14)
                bottomMargin = dp(12)
            })

            addView(buttonRow(
                actionButton("保存") { saveImage() },
                actionButton("分享", primary = true) { shareImage() }
            ), matchWrap())
        }

    private fun loadImage() {
        saveButton.isEnabled = false
        shareButton.isEnabled = false
        val itemId = intent.getLongExtra(EXTRA_ITEM_ID, 0L)
        val webDav = intent.getBooleanExtra(EXTRA_WEBDAV, false)
        thread(name = "zsclip-image-preview") {
            val result = runCatching {
                if (webDav) {
                    val (bytes, name) = LanClient.fetchLatestWebDavImageBytes(this)
                    bytes to name
                } else {
                    LanClient.fetchMobileImageBytes(this, itemId) to "zsclip_image_$itemId.png"
                }
            }
            main.post {
                result.fold(
                    onSuccess = { (bytes, name) ->
                        imageBytes = bytes
                        imageName = AndroidFileActions.safeFileName(name)
                        imageView.setImageBitmap(decodeBitmap(bytes, 2200))
                        statusView.text = "${formatSize(bytes.size.toLong())}  $imageName"
                        saveButton.isEnabled = true
                        shareButton.isEnabled = true
                    },
                    onFailure = {
                        statusView.text = "图片加载失败：${it.message}"
                        LanPrefs.saveSyncStatus(this, false, statusView.text.toString())
                    }
                )
            }
        }
    }

    private fun saveImage() {
        val bytes = imageBytes ?: return
        val file = AndroidFileActions.writeDownloadsFile(this, imageName, bytes)
        LanUi.showToast(this, "已保存：${file.name}")
    }

    private fun shareImage() {
        val bytes = imageBytes ?: return
        val file = AndroidFileActions.writeShareFile(this, imageName, bytes)
        AndroidFileActions.shareFile(this, file)
    }

    private fun decodeBitmap(bytes: ByteArray, maxSide: Int): Bitmap? {
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeByteArray(bytes, 0, bytes.size, bounds)
        var sample = 1
        while (bounds.outWidth / sample > maxSide || bounds.outHeight / sample > maxSide) {
            sample *= 2
        }
        return BitmapFactory.decodeByteArray(
            bytes,
            0,
            bytes.size,
            BitmapFactory.Options().apply { inSampleSize = sample }
        )
    }

    private fun actionButton(text: String, primary: Boolean = false, onClick: () -> Unit): Button =
        Button(this).apply {
            this.text = text
            setAllCaps(false)
            minHeight = dp(48)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 15f)
            setTextColor(if (primary) palette.onPrimary else palette.text)
            background = rounded(if (primary) palette.primary else palette.button)
            setOnClickListener { onClick() }
        }.also {
            if (text == "保存") saveButton = it
            if (text == "分享") shareButton = it
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

    private fun rounded(color: Int): GradientDrawable =
        GradientDrawable().apply {
            setColor(color)
            cornerRadius = dp(8).toFloat()
            setStroke(dp(1), palette.outline)
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

    private data class Palette(
        val bg: Int,
        val panel: Int,
        val text: Int,
        val muted: Int,
        val primary: Int,
        val onPrimary: Int,
        val button: Int,
        val outline: Int
    ) {
        companion object {
            fun from(activity: Activity): Palette {
                val dark = (activity.resources.configuration.uiMode and
                    Configuration.UI_MODE_NIGHT_MASK) == Configuration.UI_MODE_NIGHT_YES
                val primary = if (Build.VERSION.SDK_INT >= 31) {
                    activity.systemColor("system_accent1_600", if (dark) 0xFF8AB4F8.toInt() else 0xFF006DCC.toInt())
                } else if (dark) 0xFF8AB4F8.toInt() else 0xFF006DCC.toInt()
                return if (dark) {
                    Palette(0xFF101418.toInt(), 0xFF1B2026.toInt(), 0xFFE6E9EF.toInt(), 0xFFB6BDC7.toInt(), primary, 0xFF081018.toInt(), 0xFF253140.toInt(), 0xFF343D49.toInt())
                } else {
                    Palette(0xFFF7F8FC.toInt(), 0xFFFFFFFF.toInt(), 0xFF1D1B20.toInt(), 0xFF5F6368.toInt(), primary, Color.WHITE, 0xFFE7F0FA.toInt(), 0xFFDCE2EA.toInt())
                }
            }

            private fun Activity.systemColor(name: String, fallback: Int): Int {
                val id = resources.getIdentifier(name, "color", "android")
                return if (id != 0) runCatching { getColor(id) }.getOrDefault(fallback) else fallback
            }
        }
    }

    companion object {
        private const val EXTRA_ITEM_ID = "com.zsclip.lan.IMAGE_ITEM_ID"
        private const val EXTRA_WEBDAV = "com.zsclip.lan.IMAGE_WEBDAV"
        private const val EXTRA_TITLE = "com.zsclip.lan.IMAGE_TITLE"

        fun lanIntent(context: Context, itemId: Long, title: String): Intent =
            Intent(context, ImagePreviewActivity::class.java)
                .putExtra(EXTRA_ITEM_ID, itemId)
                .putExtra(EXTRA_TITLE, title)

        fun webDavIntent(context: Context, title: String): Intent =
            Intent(context, ImagePreviewActivity::class.java)
                .putExtra(EXTRA_WEBDAV, true)
                .putExtra(EXTRA_TITLE, title)
    }
}
