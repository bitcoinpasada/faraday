package app.opensigner

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.graphics.ImageFormat
import android.hardware.camera2.CameraAccessException
import android.hardware.camera2.CameraCaptureSession
import android.hardware.camera2.CameraCharacteristics
import android.hardware.camera2.CameraDevice
import android.hardware.camera2.CameraManager
import android.hardware.camera2.CaptureRequest
import android.media.Image
import android.media.ImageReader
import android.os.Handler
import android.os.HandlerThread
import android.os.Looper
import android.os.SystemClock
import android.util.Size
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.math.abs

/**
 * The camera channel: the platform Camera2 API and nothing else.
 *
 * The core decodes the luma itself (`core/osk-shell-api/src/lib.rs`),
 * which is exactly the Y plane of a `YUV_420_888` [ImageReader], and draws
 * the viewfinder's preview from the U and V planes beside it. So there is
 * no camera library here — no CameraX, no preview surface, no view in the
 * hierarchy. The reader is the only target of the repeating request, its
 * images never reach the screen, and the shell never looks at what a frame
 * contains.
 *
 * The contract this implements (docs/PLANNING.md §16.33):
 *
 *  - [on] while on is a no-op; [off] while off is harmless.
 *  - Every failure — no camera, no permission, a device error, a
 *    disconnect — is exactly one [Listener.cameraUnavailable] for that
 *    [on], after which the class is off and a later [on] tries again.
 *  - Frames arrive on the main thread, at most [MAX_FPS] a second, and a
 *    new one is dropped rather than queued while the last is still being
 *    decoded.
 *  - Frames come out the way the user is holding the phone. Camera2
 *    delivers the sensor's own orientation, landscape on nearly every
 *    device, while the activity is locked to portrait, so the Y plane is
 *    turned clockwise by `SENSOR_ORIENTATION` as it is copied. A front
 *    camera's frames are turned, never mirrored: the core reads codes,
 *    and a mirrored code does not decode.
 *
 * Nothing here is logged, not even a size or a camera id (§5.3).
 */
internal class Camera(private val context: Context, private val listener: Listener) {

    /** Where frames and the one refusal go. Both are called on the main thread. */
    interface Listener {
        /**
         * One frame: 8-bit luma, row-major, `width * height` bytes, and its
         * NV12 chroma plane, `ceil(width / 2) * ceil(height / 2)` pairs of
         * interleaved U and V.
         */
        fun cameraFrame(width: Int, height: Int, luma: ByteArray, chroma: ByteArray)

        /** No frames will come for the [on] that is in flight. */
        fun cameraUnavailable()
    }

    private val main = Handler(Looper.getMainLooper())

    // Touched on the main thread only.
    private var thread: HandlerThread? = null
    private var background: Handler? = null
    private var device: CameraDevice? = null
    private var session: CameraCaptureSession? = null
    private var reader: ImageReader? = null
    private var characteristics: CameraCharacteristics? = null

    /** Degrees the sensor's frames are turned from the device's own upright. */
    private var sensorOrientation = 0

    /** [on] has been called and neither [off] nor a failure has undone it. */
    private var running = false

    /** Whether this [on] has already spent its one refusal. */
    private var reported = false

    /** A frame is on its way to the main thread and has not been consumed. */
    private val busy = AtomicBoolean(false)

    /** The next moment a frame may be sent, on the [SystemClock.uptimeMillis] clock. */
    @Volatile
    private var nextFrameAt = 0L

    /**
     * Opens the camera and starts streaming. Idempotent while on. Any
     * failure, immediate or later, is one [Listener.cameraUnavailable].
     */
    fun on() {
        if (running) return
        running = true
        reported = false
        busy.set(false)
        nextFrameAt = 0L
        try {
            start()
        } catch (_: CameraAccessException) {
            fail()
        } catch (_: SecurityException) {
            // The CAMERA permission is not held. The activity asks before
            // it gets here, so this is the race where it was revoked.
            fail()
        } catch (_: IllegalArgumentException) {
            fail()
        } catch (_: IllegalStateException) {
            fail()
        }
    }

    /**
     * Stops streaming and releases the device. Harmless when already off,
     * and silent: the core is never told about a stop it asked for, or
     * about the activity being backgrounded.
     */
    fun off() {
        running = false
        reported = false
        teardown()
    }

    // --- opening -----------------------------------------------------------

    private fun start() {
        // The activity asks before it gets here; this is the second check,
        // for the race where the permission was revoked in between.
        if (context.checkSelfPermission(Manifest.permission.CAMERA) !=
            PackageManager.PERMISSION_GRANTED
        ) {
            fail()
            return
        }
        val manager = context.getSystemService(CameraManager::class.java)
        if (manager == null) {
            fail()
            return
        }
        val id = pick(manager)
        if (id == null) {
            fail()
            return
        }
        val traits = manager.getCameraCharacteristics(id)
        val map = traits.get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP)
        val size = map?.getOutputSizes(ImageFormat.YUV_420_888)?.let(::choose)
        if (size == null) {
            fail()
            return
        }
        characteristics = traits
        sensorOrientation = traits.get(CameraCharacteristics.SENSOR_ORIENTATION) ?: 0

        val worker = HandlerThread(THREAD_NAME)
        worker.start()
        val handler = Handler(worker.looper)
        thread = worker
        background = handler

        // maxImages 2: one being copied, one the device may fill. More would
        // only build a queue of frames that are stale by the time they land.
        val images = ImageReader.newInstance(size.width, size.height, ImageFormat.YUV_420_888, 2)
        images.setOnImageAvailableListener(onImage, handler)
        reader = images

        manager.openCamera(id, deviceCallback, handler)
    }

    /** The first back-facing camera, else the first camera of any facing. */
    private fun pick(manager: CameraManager): String? {
        val ids = manager.cameraIdList
        for (id in ids) {
            val facing = manager.getCameraCharacteristics(id).get(CameraCharacteristics.LENS_FACING)
            if (facing == CameraCharacteristics.LENS_FACING_BACK) return id
        }
        return ids.firstOrNull()
    }

    private val deviceCallback = object : CameraDevice.StateCallback() {
        override fun onOpened(camera: CameraDevice) {
            main.post {
                if (!running) {
                    camera.close()
                    return@post
                }
                device = camera
                configure(camera)
            }
        }

        override fun onDisconnected(camera: CameraDevice) {
            main.post { fail() }
        }

        override fun onError(camera: CameraDevice, error: Int) {
            main.post { fail() }
        }
    }

    private fun configure(camera: CameraDevice) {
        val surface = reader?.surface
        if (surface == null) {
            fail()
            return
        }
        try {
            // The SessionConfiguration overload is API 28 and the shell's
            // minSdk is 26; this is the one every supported version has.
            @Suppress("DEPRECATION")
            camera.createCaptureSession(listOf(surface), sessionCallback, background)
        } catch (_: CameraAccessException) {
            fail()
        } catch (_: IllegalStateException) {
            fail()
        }
    }

    private val sessionCallback = object : CameraCaptureSession.StateCallback() {
        override fun onConfigured(configured: CameraCaptureSession) {
            main.post {
                if (!running) {
                    configured.close()
                    return@post
                }
                session = configured
                stream(configured)
            }
        }

        override fun onConfigureFailed(configured: CameraCaptureSession) {
            main.post { fail() }
        }
    }

    private fun stream(configured: CameraCaptureSession) {
        val camera = device
        val surface = reader?.surface
        if (camera == null || surface == null) {
            fail()
            return
        }
        try {
            val request = camera.createCaptureRequest(CameraDevice.TEMPLATE_PREVIEW)
            request.addTarget(surface)
            if (continuousFocus()) {
                request.set(
                    CaptureRequest.CONTROL_AF_MODE,
                    CaptureRequest.CONTROL_AF_MODE_CONTINUOUS_PICTURE,
                )
            }
            configured.setRepeatingRequest(request.build(), null, background)
        } catch (_: CameraAccessException) {
            fail()
        } catch (_: IllegalStateException) {
            fail()
        }
    }

    /** Whether the device can hold focus by itself; a fixed-focus one cannot. */
    private fun continuousFocus(): Boolean {
        val modes = characteristics?.get(CameraCharacteristics.CONTROL_AF_AVAILABLE_MODES)
        return modes != null &&
            modes.any { it == CameraCharacteristics.CONTROL_AF_MODE_CONTINUOUS_PICTURE }
    }

    // --- frames ------------------------------------------------------------

    private val onImage = ImageReader.OnImageAvailableListener { source ->
        val image = try {
            source.acquireLatestImage()
        } catch (_: IllegalStateException) {
            null
        } ?: return@OnImageAvailableListener
        try {
            val now = SystemClock.uptimeMillis()
            // A frame the main thread has not finished with, or one inside
            // the rate cap, is worth less than the next one will be.
            if (now < nextFrameAt) return@OnImageAvailableListener
            if (!busy.compareAndSet(false, true)) return@OnImageAvailableListener
            val width = image.width
            val height = image.height
            val factor = subsample(width)
            val sampledWidth = (width + factor - 1) / factor
            val sampledHeight = (height + factor - 1) / factor
            // A quarter turn either way hands the core a portrait frame.
            val turned = sensorOrientation == 90 || sensorOrientation == 270
            val outWidth = if (turned) sampledHeight else sampledWidth
            val outHeight = if (turned) sampledWidth else sampledHeight
            val planes = try {
                copyPlanes(image, factor, sampledWidth, sampledHeight)
            } catch (_: RuntimeException) {
                busy.set(false)
                return@OnImageAvailableListener
            }
            nextFrameAt = if (nextFrameAt < now) now + FRAME_MS else nextFrameAt + FRAME_MS
            main.post { deliver(outWidth, outHeight, planes.first, planes.second) }
        } finally {
            image.close()
        }
    }

    private fun deliver(width: Int, height: Int, luma: ByteArray, chroma: ByteArray) {
        if (!running) {
            // A frame captured just before the camera was stopped. It may
            // picture a seed, so it is wiped rather than dropped.
            luma.fill(0)
            chroma.fill(0)
            busy.set(false)
            return
        }
        listener.cameraFrame(width, height, luma, chroma)
        busy.set(false)
    }

    /**
     * Copies the Y plane and the two chroma planes, taking every `factor`th
     * pixel of every `factor`th row and turning both clockwise by
     * [sensorOrientation] in the same pass. Picking rather than averaging:
     * a QR is high contrast and an average across a module edge is a grey
     * the decoder has to threshold back out.
     *
     * [sampledWidth] and [sampledHeight] are the size of the subsampled
     * frame before the turn; a quarter turn either way makes the result
     * `sampledHeight × sampledWidth`. The chroma is half that in each
     * direction, rounded up, and comes out in NV12 order: one U and V pair
     * per 2 x 2 block of the luma the core is given.
     *
     * There are two buffers, the two outputs: the turn is a different
     * destination index for the same source byte.
     */
    private fun copyPlanes(
        image: Image,
        factor: Int,
        sampledWidth: Int,
        sampledHeight: Int,
    ): Pair<ByteArray, ByteArray> {
        val luma = copyLuma(image, factor, sampledWidth, sampledHeight)
        val chroma = copyChroma(image, factor, sampledWidth, sampledHeight)
        return Pair(luma, chroma)
    }

    /** Where a sampled pixel `(x, y)` lands once the frame is turned. */
    private fun turned(x: Int, y: Int, width: Int, height: Int): Int = when (sensorOrientation) {
        90 -> x * height + (height - 1 - y)
        180 -> (height - 1 - y) * width + (width - 1 - x)
        270 -> (width - 1 - x) * height + y
        else -> y * width + x
    }

    /**
     * The Y plane, subsampled and turned. There is one buffer, the output.
     */
    private fun copyLuma(
        image: Image,
        factor: Int,
        sampledWidth: Int,
        sampledHeight: Int,
    ): ByteArray {
        val plane = image.planes[0]
        val buffer = plane.buffer
        val rowStride = plane.rowStride
        // Documented as 1 for the Y plane of YUV_420_888, but the format
        // permits more and reading it costs nothing.
        val pixelStride = plane.pixelStride
        val luma = ByteArray(sampledWidth * sampledHeight)
        val row = ByteArray(rowStride)
        for (y in 0 until sampledHeight) {
            val start = y * factor * rowStride
            if (start >= buffer.limit()) break
            buffer.position(start)
            val got = minOf(rowStride, buffer.remaining())
            buffer.get(row, 0, got)
            for (x in 0 until sampledWidth) {
                val at = x * factor * pixelStride
                val value = if (at < got) row[at] else 0
                luma[turned(x, y, sampledWidth, sampledHeight)] = value
            }
        }
        return luma
    }

    /**
     * The U and V planes as one NV12 chroma plane for the luma above,
     * subsampled and turned with it.
     *
     * Both planes are read through their own row and pixel strides, which
     * is what makes the two layouts Camera2 permits one piece of code. A
     * pixel stride of 2 means the device has already interleaved them, and
     * one plane is the other offset by a byte; a stride of 1 means two
     * separate planar buffers. Either way the pair for a 2 x 2 block of
     * luma sits at `(y / 2) * rowStride + (x / 2) * pixelStride`.
     */
    private fun copyChroma(
        image: Image,
        factor: Int,
        sampledWidth: Int,
        sampledHeight: Int,
    ): ByteArray {
        // The pairs of the subsampled frame, before the turn. A quarter
        // turn swaps them, and either way `ceil(w / 2)` of the frame the
        // core is handed is one of these two.
        val cw = (sampledWidth + 1) / 2
        val ch = (sampledHeight + 1) / 2
        val chroma = ByteArray(cw * ch * 2)
        val u = image.planes[1]
        val v = image.planes[2]
        val uBuffer = u.buffer
        val vBuffer = v.buffer
        // Every second sampled pixel of every second sampled row is one
        // 2 x 2 block of the frame the core is given.
        for (y in 0 until sampledHeight step 2) {
            for (x in 0 until sampledWidth step 2) {
                // The source pixel this block starts at, in the full frame.
                val sx = x * factor
                val sy = y * factor
                val at = (sy / 2) * u.rowStride + (sx / 2) * u.pixelStride
                if (at >= uBuffer.limit() || at >= vBuffer.limit()) continue
                val to = turned(x / 2, y / 2, cw, ch) * 2
                if (to + 1 >= chroma.size) continue
                chroma[to] = uBuffer.get(at)
                chroma[to + 1] = vBuffer.get(at)
            }
        }
        return chroma
    }

    // --- stopping ----------------------------------------------------------

    /** The one refusal for this [on], and then off. */
    private fun fail() {
        val speak = running && !reported
        reported = true
        running = false
        teardown()
        if (speak) listener.cameraUnavailable()
    }

    private fun teardown() {
        try {
            session?.stopRepeating()
        } catch (_: Exception) {
            // The session is being closed either way.
        }
        try {
            session?.close()
        } catch (_: Exception) {
            // As above.
        }
        session = null
        device?.close()
        device = null
        reader?.setOnImageAvailableListener(null, null)
        reader?.close()
        reader = null
        characteristics = null
        sensorOrientation = 0
        thread?.quitSafely()
        thread = null
        background = null
        busy.set(false)
        nextFrameAt = 0L
    }

    private companion object {
        const val THREAD_NAME = "opensigner-camera"

        /** What the core wants: 640 × 480 is more than QR detection needs. */
        const val TARGET_WIDTH = 640
        const val TARGET_HEIGHT = 480
        const val TARGET_AREA = TARGET_WIDTH * TARGET_HEIGHT

        /** Above this the luma is subsampled before it is copied. */
        const val MAX_WIDTH = 800

        /** Ten frames a second, as the V4L2 capture thread caps at. */
        const val MAX_FPS = 10
        const val FRAME_MS = 1000L / MAX_FPS

        /**
         * The output size closest to 640 × 480 by area, preferring one at
         * least that large: downscaling is free and upscaling is not.
         */
        fun choose(sizes: Array<Size>): Size? {
            if (sizes.isEmpty()) return null
            val big = sizes.filter { it.width >= TARGET_WIDTH && it.height >= TARGET_HEIGHT }
            val from = if (big.isNotEmpty()) big else sizes.toList()
            return from.minByOrNull { abs(it.width * it.height - TARGET_AREA) }
        }

        /** The smallest integer factor that brings `width` to [MAX_WIDTH] or under. */
        fun subsample(width: Int): Int =
            if (width <= MAX_WIDTH) 1 else (width + MAX_WIDTH - 1) / MAX_WIDTH
    }
}
