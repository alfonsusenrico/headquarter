package com.alfonsusenrico.headquarter.ui

import android.content.Context
import android.graphics.Bitmap
import android.graphics.ImageDecoder
import android.net.Uri
import java.io.ByteArrayOutputStream
import java.nio.ByteBuffer
import kotlin.math.max
import kotlin.math.roundToInt

/** The largest image the computer takes (add-image-upload D3). */
const val MAX_IMAGE_BYTES = 10L * 1024 * 1024

/** The longer side a converted image is scaled to at most (D6). */
private const val MAX_SIDE = 4096

/** JPEG quality of a converted image (D6). */
private const val JPEG_QUALITY = 90

/** Whether the computer takes [bytes] as an image: PNG, JPEG, GIF or WebP by its first bytes. */
fun agentsRead(bytes: ByteArray): Boolean {
    fun starts(vararg head: Int) = bytes.size >= head.size && head.indices.all { bytes[it] == head[it].toByte() }
    val webp =
        bytes.size >= 12 &&
            starts('R'.code, 'I'.code, 'F'.code, 'F'.code) &&
            String(bytes, 8, 4, Charsets.US_ASCII) == "WEBP"
    return starts(0x89, 'P'.code, 'N'.code, 'G'.code, 0x0D, 0x0A, 0x1A, 0x0A) ||
        starts(0xFF, 0xD8, 0xFF) ||
        starts('G'.code, 'I'.code, 'F'.code, '8'.code) ||
        webp
}

/** Whether an image must be converted to JPEG before it is sent (D6). */
fun needsConversion(bytes: ByteArray): Boolean = !agentsRead(bytes) || bytes.size > MAX_IMAGE_BYTES

/** The scale that brings the longer of [width] and [height] to at most [MAX_SIDE]. */
fun scaleFor(
    width: Int,
    height: Int,
): Float = (MAX_SIDE.toFloat() / max(width, height)).coerceAtMost(1f)

/** Why an image could not be prepared, for the owner. */
class ImageProblem(
    message: String,
) : Exception(message)

/**
 * The image at [uri] as the computer takes it: as it is when the agents read it and it fits,
 * otherwise converted to JPEG and scaled down until it fits. Held in memory only, never written
 * to the phone's storage (android-app "The phone keeps no content").
 */
fun prepareImage(
    context: Context,
    uri: Uri,
): ByteArray {
    val bytes =
        context.contentResolver.openInputStream(uri)?.use { it.readBytes() }
            ?: throw ImageProblem("The image couldn't be opened")
    return prepareImage(bytes)
}

fun prepareImage(bytes: ByteArray): ByteArray {
    if (!needsConversion(bytes)) return bytes
    var scale = 1f
    repeat(CONVERSION_TRIES) {
        val jpeg = toJpeg(bytes, scale)
        if (jpeg.size <= MAX_IMAGE_BYTES) return jpeg
        scale *= 0.75f
    }
    throw ImageProblem("The image is too large to send")
}

private const val CONVERSION_TRIES = 6

private fun toJpeg(
    bytes: ByteArray,
    extraScale: Float,
): ByteArray {
    val source = ImageDecoder.createSource(ByteBuffer.wrap(bytes))
    val bitmap =
        try {
            ImageDecoder.decodeBitmap(source) { decoder, info, _ ->
                val scale = scaleFor(info.size.width, info.size.height) * extraScale
                if (scale < 1f) {
                    decoder.setTargetSize(
                        (info.size.width * scale).roundToInt().coerceAtLeast(1),
                        (info.size.height * scale).roundToInt().coerceAtLeast(1),
                    )
                }
                decoder.allocator = ImageDecoder.ALLOCATOR_SOFTWARE
            }
        } catch (_: Exception) {
            throw ImageProblem("Only images can be sent")
        }
    return ByteArrayOutputStream().use { out ->
        bitmap.compress(Bitmap.CompressFormat.JPEG, JPEG_QUALITY, out)
        bitmap.recycle()
        out.toByteArray()
    }
}
