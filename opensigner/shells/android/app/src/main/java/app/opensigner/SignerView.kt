package app.opensigner

import android.app.ActivityManager
import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Rect
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.view.MotionEvent
import android.view.View
import java.io.File
import java.security.SecureRandom

/**
 * The whole user interface: one view that owns the core, feeds it input and
 * blits its frame.
 *
 * The core renders its own pixels, so there is no view hierarchy, no
 * layout, no text field and no system keyboard here (docs/PLANNING.md §4.5).
 * What this class does is the shell's entire job:
 *
 *  - create the core once, at the first settled size, and free it on
 *    destroy;
 *  - turn touches into `Touch` events in frame pixels;
 *  - tick every [TICK_MS] while the activity is on screen, so the core's
 *    holds, masking, auto-lock and auto-wipe timers run whether or not
 *    anything is happening; stop ticking off screen ([background]), and on
 *    the way back hand the core the elapsed time in one tick ([foreground]),
 *    which is what makes those timers run at the same rate behind another
 *    app as in front of it;
 *  - drain the commands each event produces and act on them, answering
 *    entropy and the settings file here and handing documents, the camera,
 *    haptics and exit to the [Host].
 *
 * Two files are all this shell writes down, both in the app's private
 * storage, which no other app can read and the OS does not back up: the
 * settings the core keeps between sessions (§6), written here, and the
 * kept key's blob, which goes through the [Host] to [KeptSecret] because
 * it is wrapped by a hardware key on the way.
 */
internal class SignerView(context: Context, private val host: Host) : View(context) {

    /** What the shell needs an Activity for. */
    interface Host {
        /** Ask the user for a file; answer with [fileAnswer] exactly once. */
        fun requestFile(kind: Int)

        /**
         * Store `bytes` under a name the user picks, starting from `name`;
         * answer with [writeAnswer] exactly once.
         */
        fun writeFile(kind: Int, name: String?, bytes: ByteArray?)

        /**
         * Read the clipboard; answer with [clipboardAnswer] exactly once.
         * Called only for a tap on "Paste".
         */
        fun requestClipboard(kind: Int)

        /**
         * Put `text` on the clipboard; answer with [copyAnswer] exactly
         * once. Called only for a tap on "Copy", and never with a secret.
         */
        fun writeClipboard(kind: Int, text: String?)

        /**
         * Start the camera, asking for the permission if this is the first
         * time; answer with [cameraFrame]s or one [cameraUnavailable].
         */
        fun cameraOn()

        /** Stop the camera. Harmless when it is off. */
        fun cameraOff()

        /** Vibrate, if the device and the manifest allow it. */
        fun vibrate(ms: Int)

        /** The core is done. */
        fun exit()

        /**
         * What backs this device's secure element, as one of the
         * `Core.SECURE_*` codes. Fixed for the session: it is what the
         * core is created with.
         */
        fun secureLevel(): Int

        /**
         * What the platform's attestation says about this device's boot,
         * as one of the `Core.BOOT_*` codes. Fixed for the session, as
         * [secureLevel] is.
         */
        fun bootState(): Int

        /** Whether a blob is kept on this device. */
        fun secretKept(): Boolean

        /**
         * Authenticate `salt` inside the secure element, which asks the
         * person for a biometric or the device credential; answer with
         * [secureMacAnswer] exactly once.
         */
        fun secureMac(salt: ByteArray?)

        /** Keep `blob`; answer with [storeAnswer] exactly once. */
        fun storeSecret(blob: ByteArray?)

        /** Hand the kept blob back; answer with [secretAnswer] exactly once. */
        fun loadSecret()

        /**
         * Delete the blob and the hardware keys behind it; answer with
         * [forgetAnswer] exactly once.
         */
        fun forgetSecret()
    }

    private var handle = 0L
    private var frameWidth = 0
    private var frameHeight = 0
    private var bitmap: Bitmap? = null

    /** Nearest neighbour: the core's pixels are exact and must not blur. */
    private val paint = Paint().apply {
        isFilterBitmap = false
        isDither = false
    }
    private val destination = Rect()

    private val random = SecureRandom()

    /** What the core kept last time, in the app's private storage. */
    private val settingsFile = File(context.filesDir, SETTINGS_NAME)

    /** Written first and renamed over [settingsFile], so an interrupted
     *  write leaves the old settings rather than half of the new ones. */
    private val settingsTemp = File(context.filesDir, "$SETTINGS_NAME.tmp")

    /**
     * Events waiting to go in. An event's commands can produce further
     * events (an entropy request is answered at once), so events are queued
     * and delivered in order rather than recursively.
     */
    private val pending = ArrayDeque<() -> Unit>()
    private var draining = false
    private var needsDraw = false

    private val ticker = Handler(Looper.getMainLooper())

    /** Whether the activity is between `onStart` and `onStop`. */
    private var onScreen = false

    /**
     * The core's clock is [SystemClock.elapsedRealtime]: milliseconds since
     * boot, including every one the device spent asleep.
     * [SystemClock.uptimeMillis] stops while the device sleeps, so a phone
     * put down for an hour would come back with an auto-wipe timer that had
     * barely moved, and a seed the person told the app to forget would still
     * be there. Auto-lock and auto-wipe are wall-clock promises, so the
     * clock has to be one.
     */
    private val tick = object : Runnable {
        override fun run() {
            if (handle == 0L) return
            send { Core.tick(handle, SystemClock.elapsedRealtime()) }
            ticker.postDelayed(this, TICK_MS)
        }
    }

    /**
     * Creating the core fixes the frame size for the session, so it waits
     * until layout has settled: `onSizeChanged` can fire more than once
     * while window insets are applied, and only the last size counts.
     */
    private val start = Runnable {
        if (handle != 0L || width <= 0 || height <= 0) return@Runnable
        // The camera code turns every frame upright from the sensor's
        // orientation, so there is nothing for a person to turn.
        // The view is already padded by the system bars and the display
        // cutout (MainActivity), so every pixel of this frame is usable
        // and the core is told to add nothing at either edge.
        // A device whose secure element wraps the key it keeps is Tier B;
        // one with no element to wrap it with stays Tier C, and the core
        // then offers no way to keep a key at all.
        val secure = host.secureLevel()
        val tier = if (secure == Core.SECURE_NONE) Core.TIER_C else Core.TIER_B
        handle = Core.new(
            width,
            height,
            resources.displayMetrics.densityDpi,
            0,
            0,
            tier,
            BuildConfig.VERSION_NAME,
            true,
            secure,
            host.bootState(),
            totalMemoryMib(),
        )
        check(handle != 0L) { "the core refused a ${width}x$height display" }
        frameWidth = width
        frameHeight = height
        bitmap = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
        // The display event is already in. What was kept last time goes in
        // behind it, before any input can change it, and this drains what
        // all three produced.
        val settings = readSettings()
        val kept = host.secretKept()
        send { settings?.let { Core.settings(handle, it) } }
        send { Core.secretKept(handle, kept) }
        if (onScreen) ticker.postDelayed(tick, TICK_MS)
    }

    /**
     * `onStart`: the ticker runs again, and the first tick goes in before
     * anything is drawn, so the time the app spent away has already been
     * applied when the person sees the screen. Past the auto-lock deadline
     * the first frame is the lock screen; past the auto-wipe deadline the
     * key is gone; short of both, the screen is the one the person left.
     */
    fun foreground() {
        onScreen = true
        if (handle == 0L) return
        ticker.removeCallbacks(tick)
        send { Core.tick(handle, SystemClock.elapsedRealtime()) }
        ticker.postDelayed(tick, TICK_MS)
    }

    /**
     * `onStop`: the ticker stops, so a backgrounded app costs no battery.
     * The session is not locked. How long an untouched session may stand is
     * the auto-lock timer, and a session left for another app is held to the
     * same rule as a session left on the table; [foreground] hands the core
     * the elapsed time and the core decides.
     */
    fun background() {
        onScreen = false
        ticker.removeCallbacks(tick)
    }

    init {
        isFocusable = true
        setWillNotDraw(false)
    }

    override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) {
        super.onSizeChanged(w, h, oldw, oldh)
        // The size is fixed for the session (§4.3): a later change is
        // absorbed by scaling in onDraw, not by rebuilding the core.
        if (handle != 0L) return
        removeCallbacks(start)
        post(start)
    }

    override fun onDraw(canvas: Canvas) {
        val bitmap = bitmap ?: return
        if (handle == 0L) return
        val frame = Core.frame(handle) ?: return
        bitmap.copyPixelsFromBuffer(frame)
        destination.set(0, 0, width, height)
        canvas.drawBitmap(bitmap, null, destination, paint)
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        if (handle == 0L) return false
        val phase = when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> Core.TOUCH_DOWN
            MotionEvent.ACTION_MOVE -> Core.TOUCH_MOVE
            MotionEvent.ACTION_UP -> Core.TOUCH_UP
            // A cancelled gesture must release whatever it was holding, or
            // a hold-to-confirm would stay pressed with no finger on it.
            MotionEvent.ACTION_CANCEL -> Core.TOUCH_UP
            else -> return false
        }
        val x = toFrame(event.x, width, frameWidth)
        val y = toFrame(event.y, height, frameHeight)
        send { Core.touch(handle, x, y, phase) }
        return true
    }

    /** The system back gesture. The core reads Escape as "one step back". */
    fun back() {
        if (handle == 0L) return
        send { Core.key(handle, Core.KEY_ESCAPE, 0) }
    }

    /**
     * The answer to a [Host.requestFile]. `cancelled` is the person closing
     * the picker, which the core says nothing about; `null` bytes without it
     * are a device with no file channel or a read that failed, which it
     * does say.
     */
    fun fileAnswer(kind: Int, bytes: ByteArray?, cancelled: Boolean) {
        if (handle == 0L) return
        send {
            when {
                bytes != null -> Core.file(handle, kind, bytes)
                cancelled -> Core.fileCancelled(handle, kind)
                else -> Core.fileUnavailable(handle, kind)
            }
        }
    }

    /**
     * The answer to a [Host.requestClipboard]. `null` is nothing on the
     * clipboard, no clipboard, or content that is not text; the core says
     * the same thing about all three.
     */
    fun clipboardAnswer(kind: Int, text: String?) {
        if (handle == 0L) return
        send {
            if (text != null) Core.clipboard(handle, kind, text)
            else Core.clipboardUnavailable(handle, kind)
        }
    }

    /** The answer to a [Host.writeClipboard]: whether the text went there. */
    fun copyAnswer(kind: Int, written: Boolean) {
        if (handle == 0L) return
        send {
            if (written) Core.clipboardWritten(handle, kind)
            else Core.clipboardNotWritten(handle, kind)
        }
    }

    /** The answer to a [Host.writeFile]: whether the bytes were stored. */
    fun writeAnswer(kind: Int, written: Boolean) {
        if (handle == 0L) return
        send {
            if (written) Core.fileWritten(handle, kind) else Core.fileNotWritten(handle, kind)
        }
    }

    /**
     * One camera frame after a [Host.cameraOn]: 8-bit luma, `width * height`
     * bytes, and the NV12 chroma the preview is drawn in colour from.
     * Delivered through the same queue as everything else, so a frame cannot
     * arrive in the middle of a touch and its commands are drained.
     */
    fun cameraFrame(width: Int, height: Int, luma: ByteArray, chroma: ByteArray) {
        if (handle == 0L) return
        send { Core.cameraFrame(handle, width, height, luma, chroma) }
    }

    /**
     * The answer to a [Host.secureMac]: the 32-byte tag, or `null` when
     * the person cancelled, the hardware refused, or there is none.
     */
    fun secureMacAnswer(mac: ByteArray?) {
        if (handle == 0L) return
        send {
            if (mac == null || mac.size != Core.MAC_LEN) {
                Core.secureUnavailable(handle)
            } else {
                Core.secureMac(handle, mac)
                // The tag is one half of the key that opens the blob. The
                // core has taken it; this copy should not outlive the call.
                mac.fill(0)
            }
        }
    }

    /** The answer to a [Host.storeSecret]: whether the blob is kept. */
    fun storeAnswer(stored: Boolean) {
        if (handle == 0L) return
        send {
            if (stored) Core.secretStored(handle) else Core.secretNotStored(handle)
        }
    }

    /** The answer to a [Host.loadSecret]: the blob, or `null` for none. */
    fun secretAnswer(blob: ByteArray?) {
        if (handle == 0L) return
        send {
            if (blob == null) Core.secretUnavailable(handle) else Core.secret(handle, blob)
        }
    }

    /**
     * Whether a blob is kept, when that changed under the core: the shell
     * found its key invalidated and deleted the blob with it.
     */
    fun keptAnswer(kept: Boolean) {
        if (handle == 0L) return
        send { Core.secretKept(handle, kept) }
    }

    /** The answer to a [Host.forgetSecret]: nothing is kept any more. */
    fun forgetAnswer() {
        if (handle == 0L) return
        send { Core.secretForgotten(handle) }
    }

    /** The answer to a [Host.cameraOn] when no frames will come. */
    fun cameraUnavailable() {
        if (handle == 0L) return
        send { Core.cameraUnavailable(handle) }
    }

    /** Frees the core. The view is unusable afterwards. */
    fun destroy() {
        ticker.removeCallbacksAndMessages(null)
        removeCallbacks(start)
        pending.clear()
        val dying = handle
        handle = 0L
        bitmap?.recycle()
        bitmap = null
        if (dying != 0L) Core.free(dying)
    }

    /**
     * Delivers one event and then acts on every command it produced, and on
     * every command produced by the events those commands triggered, until
     * the core has nothing left to say. One redraw at the end, however many
     * `Draw` commands came out.
     */
    private fun send(event: () -> Unit) {
        pending.addLast(event)
        if (draining) return
        draining = true
        try {
            while (pending.isNotEmpty()) {
                pending.removeFirst()()
                drainCommands()
            }
        } finally {
            draining = false
        }
        if (needsDraw) {
            needsDraw = false
            invalidate()
        }
    }

    private fun drainCommands() {
        while (handle != 0L) {
            when (Core.pollCommand(handle)) {
                Core.CMD_DRAW -> needsDraw = true
                Core.CMD_EXIT -> host.exit()
                Core.CMD_REQUEST_FILE -> host.requestFile(Core.commandKind(handle))
                Core.CMD_WRITE_FILE ->
                    host.writeFile(Core.commandKind(handle), Core.commandName(handle), Core.commandBytes(handle))
                Core.CMD_REQUEST_CLIPBOARD -> host.requestClipboard(Core.commandKind(handle))
                Core.CMD_WRITE_CLIPBOARD ->
                    host.writeClipboard(Core.commandKind(handle), Core.commandText(handle))
                Core.CMD_REQUEST_ENTROPY -> {
                    val bytes = ByteArray(Core.ENTROPY_LEN)
                    random.nextBytes(bytes)
                    pending.addLast {
                        Core.entropy(handle, bytes)
                        // Consumed into the core's session key; the shell's
                        // copy is worth nothing and should not linger.
                        bytes.fill(0)
                    }
                }
                Core.CMD_CAMERA_ON -> host.cameraOn()
                Core.CMD_CAMERA_OFF -> host.cameraOff()
                Core.CMD_VIBRATE -> host.vibrate(Core.commandMs(handle))
                Core.CMD_STORE_SETTINGS -> writeSettings(Core.commandBytes(handle))
                Core.CMD_SECURE_MAC -> host.secureMac(Core.commandBytes(handle))
                Core.CMD_STORE_SECRET -> host.storeSecret(Core.commandBytes(handle))
                Core.CMD_LOAD_SECRET -> host.loadSecret()
                Core.CMD_FORGET_SECRET -> host.forgetSecret()
                // CMD_NONE, and CMD_ERROR after the core has been freed.
                else -> return
            }
        }
    }

    /**
     * The settings of an earlier session, or `null` when there are none and
     * when the file cannot be read, which is a device with its defaults.
     */
    private fun readSettings(): ByteArray? = try {
        if (settingsFile.exists()) settingsFile.readBytes() else null
    } catch (_: Exception) {
        null
    }

    /**
     * Keeps `bytes` for the next run: the temporary file is written and the
     * rename puts it in place in one step. A failure is silent; the core has
     * moved on and the settings are worth no dialog.
     */
    private fun writeSettings(bytes: ByteArray?) {
        if (bytes == null) return
        try {
            settingsTemp.writeBytes(bytes)
            if (!settingsTemp.renameTo(settingsFile)) settingsTemp.delete()
        } catch (_: Exception) {
            settingsTemp.delete()
        }
    }

    /**
     * This device's total memory in MiB, from `ActivityManager`, or 0
     * where it cannot be read. The core uses it only to recommend an
     * Argon2id memory cost for an encrypted backup.
     */
    private fun totalMemoryMib(): Int = try {
        val manager = context.getSystemService(Context.ACTIVITY_SERVICE) as ActivityManager
        val info = ActivityManager.MemoryInfo()
        manager.getMemoryInfo(info)
        (info.totalMem / (1024L * 1024L)).coerceIn(0L, Int.MAX_VALUE.toLong()).toInt()
    } catch (_: Exception) {
        0
    }

    private companion object {
        /** The only file this shell writes, in `filesDir`. */
        const val SETTINGS_NAME = "settings"

        /** 20 Hz: enough for a hold's progress ring and the 500 ms mask. */
        const val TICK_MS = 50L

        /** View pixel to frame pixel along one axis, nearest neighbour. */
        fun toFrame(position: Float, viewLength: Int, frameLength: Int): Int {
            if (viewLength <= 0 || frameLength <= 0) return 0
            val scaled = (position * frameLength / viewLength).toInt()
            return scaled.coerceIn(0, frameLength - 1)
        }
    }
}
