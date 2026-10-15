package app.faradayoffline.companion

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
import java.util.TimeZone

/**
 * The whole user interface: one view that owns the core, feeds it input and
 * blits its frame. Copied from upstream's `SignerView.kt`
 * (`opensigner/shells/android` at d26b84e) and cut to what the Companion
 * asks of a shell.
 *
 * The core draws its own pixels, so there is no view hierarchy, no layout,
 * no text field and no system keyboard here. What this class does:
 *
 *  - create the core once, at the first settled size, hand it what was
 *    kept (the received files, the kept store's count, the theme, the
 *    Start from the stick answers and each guide page's ticked steps), the
 *    wall clock, About's facts and anything shared in at start, and free
 *    it on destroy;
 *  - turn touches into `Touch` events in frame pixels, and Back into Escape;
 *  - tick every [TICK_MS] while the activity is on screen, which moves an
 *    animated code on and hands the core what the scanner read;
 *  - drain the commands each event produces and act on them, keeping the
 *    theme here and handing documents, the clipboard, the camera, the
 *    stores, the unlock, Share and the screen to the [Host].
 */
internal class CompanionView(context: Context, private val host: Host) : View(context) {

    /** What the shell needs an Activity for. */
    interface Host {
        /** Ask the person for a file; answer with [fileAnswer] exactly once. */
        fun requestFile(kind: Int)

        /**
         * Store `bytes` under a name the person picks, starting from `name`;
         * answer with [writeAnswer] exactly once.
         */
        fun writeFile(kind: Int, name: String?, bytes: ByteArray?)

        /** Read the clipboard, for a tap on Paste; answer with [clipboardAnswer]. */
        fun requestClipboard(kind: Int)

        /** Start the camera, asking for the permission the first time. */
        fun cameraOn()

        /** Stop the camera. Harmless when it is off. */
        fun cameraOff()

        /** Keep the received files' store, encrypted. */
        fun keepReceived(bytes: ByteArray?)

        /** The received files' store kept before, for the core at start. */
        fun loadReceived(): ByteArray?

        /** Keep the screen on, or let it sleep. */
        fun keepScreenOn(on: Boolean)

        /**
         * Ask the phone's unlock and open the kept store; answer with
         * [keptAnswer] or [keptClosed] exactly once.
         */
        fun unlockKept(title: String?)

        /** Keep the kept store, encrypted, with its count; empty bytes delete it. */
        fun keepKept(bytes: ByteArray?, descriptions: Int)

        /** The version and the count kept beside the kept store, for the core at start. */
        fun keptCounts(): Pair<Int, Int>?

        /** Hand one file to the system's share sheet. */
        fun share(name: String?, mime: String?, bytes: ByteArray?)

        /** The window behind the frame, in the theme's page colour. */
        fun pageColor(color: Int, light: Boolean)

        /** About's facts: the version, the certificate's digest, the permissions. */
        fun system(): Triple<String, ByteArray, String>?

        /** The core is done. */
        fun exit()
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

    /** The theme chosen on About, in the app's private storage. */
    private val themeFile = File(context.filesDir, THEME_NAME)
    private val themeTemp = File(context.filesDir, "$THEME_NAME.tmp")

    /** The computer and the maker chosen on Start from the stick, beside it. */
    private val startFile = File(context.filesDir, START_NAME)
    private val startTemp = File(context.filesDir, "$START_NAME.tmp")

    /** The steps ticked on each guide page: a file a page, named by its id. */
    private val stepsDir = File(context.filesDir, STEPS_NAME)

    /**
     * Events waiting to go in, delivered in order rather than recursively:
     * an event's commands can produce further events.
     */
    private val pending = ArrayDeque<() -> Unit>()
    private var draining = false
    private var needsDraw = false
    private var needsColor = false

    private val ticker = Handler(Looper.getMainLooper())

    /** Whether the activity is between `onStart` and `onStop`. */
    private var onScreen = false

    /** What was shared in before the core existed, for it at start. */
    private var sharedWaiting: Pair<String?, ByteArray>? = null

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
        // The view is already padded by the system bars and the display
        // cutout (MainActivity), so every pixel of this frame is usable.
        handle = Core.new(width, height, resources.displayMetrics.densityDpi, 0, 0)
        check(handle != 0L) { "the core refused a ${width}x$height display" }
        frameWidth = width
        frameHeight = height
        bitmap = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
        // The display is in. What was kept goes in behind it, before any
        // input can change it, with the clock and About's facts.
        val received = host.loadReceived()
        val counts = host.keptCounts()
        val theme = readTheme()
        val start = readStart()
        val steps = readSteps()
        val system = host.system()
        send {
            received?.let {
                Core.received(handle, it)
                // The core holds its own copy; this one should not linger.
                it.fill(0)
            }
        }
        send { counts?.let { Core.keptCounts(handle, it.first, it.second) } }
        send { theme?.let { Core.theme(handle, it) } }
        send { start?.let { Core.start(handle, it) } }
        for (text in steps) send { Core.steps(handle, text) }
        send { system?.let { (build, cert, permissions) -> Core.system(handle, build, cert, permissions) } }
        sendTime()
        // In the theme the core now has, the kept one or the default.
        host.pageColor(Core.pageColor(handle), Core.pageLight(handle))
        if (onScreen) ticker.postDelayed(tick, TICK_MS)
        sharedWaiting?.let { (name, bytes) ->
            sharedWaiting = null
            sharedIn(name, bytes)
        }
    }

    /** `onStart`: the clock again, and the ticker runs. */
    fun foreground() {
        onScreen = true
        if (handle == 0L) return
        sendTime()
        ticker.removeCallbacks(tick)
        send { Core.tick(handle, SystemClock.elapsedRealtime()) }
        ticker.postDelayed(tick, TICK_MS)
    }

    /**
     * About's facts again: a permission granted or taken away (in the
     * camera's dialog, or in the phone's settings while this app was in
     * the background) shows there as it now is.
     */
    fun systemChanged() {
        if (handle == 0L) return
        val system = host.system()
        send { system?.let { (build, cert, permissions) -> Core.system(handle, build, cert, permissions) } }
    }

    /** `onStop`: the ticker stops, so a backgrounded app costs no battery. */
    fun background() {
        onScreen = false
        ticker.removeCallbacks(tick)
    }

    /** The wall clock now, and the phone's offset from UTC at this moment. */
    private fun sendTime() {
        val now = System.currentTimeMillis()
        val offset = TimeZone.getDefault().getOffset(now) / 1000
        send { Core.time(handle, now / 1000, offset) }
    }

    init {
        isFocusable = true
        setWillNotDraw(false)
    }

    override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) {
        super.onSizeChanged(w, h, oldw, oldh)
        // The size is fixed for the session: a later change is absorbed by
        // scaling in onDraw, not by rebuilding the core.
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
            // A cancelled gesture releases whatever it was holding.
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
        send { Core.key(handle, Core.KEY_ESCAPE) }
    }

    /**
     * The answer to a [Host.requestFile]: the bytes and the name the
     * provider gave, or nothing. `cancelled` is the person closing the
     * picker, which the core says nothing about.
     */
    fun fileAnswer(kind: Int, name: String?, bytes: ByteArray?, cancelled: Boolean) {
        if (handle == 0L) return
        send {
            when {
                bytes != null -> Core.file(handle, kind, name, bytes)
                cancelled -> Core.fileCancelled(handle, kind)
                else -> Core.fileUnavailable(handle, kind)
            }
        }
    }

    /** The answer to a [Host.requestClipboard]: the text, or `null` for none. */
    fun clipboardAnswer(kind: Int, text: String?) {
        if (handle == 0L) return
        send {
            if (text != null) Core.clipboard(handle, kind, text)
            else Core.clipboardUnavailable(handle, kind)
        }
    }

    /** The answer to a [Host.writeFile]: whether the bytes were stored. */
    fun writeAnswer(kind: Int, written: Boolean) {
        if (handle == 0L) return
        send {
            if (written) Core.fileWritten(handle, kind) else Core.fileNotWritten(handle, kind)
        }
    }

    /** The answer to a [Host.unlockKept]: the store's bytes, or `null` when they cannot be read. */
    fun keptAnswer(bytes: ByteArray?) {
        if (handle == 0L) return
        send {
            if (bytes != null) {
                Core.kept(handle, bytes)
                bytes.fill(0)
            } else {
                Core.keptClosed(handle, Core.CLOSED_UNREADABLE)
            }
        }
    }

    /** The answer to a [Host.unlockKept] when the store stays closed: a `CLOSED_*` code. */
    fun keptClosed(why: Int) {
        if (handle == 0L) return
        send { Core.keptClosed(handle, why) }
    }

    /** The app went to the background: the core lets go of the kept store. */
    fun locked() {
        if (handle == 0L) return
        send { Core.locked(handle) }
    }

    /**
     * One file or text shared in; held until the core exists when it
     * arrives with the app's start.
     */
    fun sharedIn(name: String?, bytes: ByteArray) {
        if (handle == 0L) {
            sharedWaiting?.second?.fill(0)
            sharedWaiting = Pair(name, bytes)
            return
        }
        send {
            Core.sharedIn(handle, name, bytes)
            bytes.fill(0)
        }
    }

    /** One camera frame after a [Host.cameraOn], through the same queue. */
    fun cameraFrame(width: Int, height: Int, luma: ByteArray, chroma: ByteArray) {
        if (handle == 0L) return
        send { Core.cameraFrame(handle, width, height, luma, chroma) }
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
        sharedWaiting?.second?.fill(0)
        sharedWaiting = null
        val dying = handle
        handle = 0L
        bitmap?.recycle()
        bitmap = null
        if (dying != 0L) Core.free(dying)
    }

    /**
     * Delivers one event and then acts on every command it produced, and on
     * every command produced by the events those commands triggered, until
     * the core has nothing left to say. One redraw at the end.
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
        if (needsColor && handle != 0L) {
            needsColor = false
            host.pageColor(Core.pageColor(handle), Core.pageLight(handle))
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
                Core.CMD_CAMERA_ON -> host.cameraOn()
                Core.CMD_CAMERA_OFF -> host.cameraOff()
                Core.CMD_KEEP_RECEIVED -> host.keepReceived(Core.commandBytes(handle))
                Core.CMD_KEEP_SCREEN_ON -> host.keepScreenOn(Core.commandOn(handle))
                Core.CMD_UNLOCK_KEPT -> host.unlockKept(Core.commandText(handle))
                Core.CMD_KEEP_KEPT -> host.keepKept(
                    Core.commandBytes(handle),
                    Core.commandDescriptions(handle),
                )
                Core.CMD_SHARE ->
                    host.share(Core.commandName(handle), Core.commandText(handle), Core.commandBytes(handle))
                Core.CMD_KEEP_THEME -> {
                    writeTheme(Core.commandText(handle))
                    needsColor = true
                }
                Core.CMD_KEEP_START -> writeStart(Core.commandText(handle))
                Core.CMD_KEEP_STEPS -> writeSteps(Core.commandText(handle))
                // CMD_NONE, and CMD_ERROR after the core has been freed.
                else -> return
            }
        }
    }

    /** The theme kept last time, or `null` for none. */
    private fun readTheme(): String? = try {
        if (themeFile.exists()) themeFile.readText().trim().takeIf { it.isNotEmpty() } else null
    } catch (_: Exception) {
        null
    }

    /**
     * Keeps the theme for the next run: the temporary file is written and
     * the rename puts it in place. A failure is silent; the theme is worth
     * no dialog.
     */
    private fun writeTheme(id: String?) {
        if (id == null) return
        try {
            themeTemp.writeText(id)
            if (!themeTemp.renameTo(themeFile)) themeTemp.delete()
        } catch (_: Exception) {
            themeTemp.delete()
        }
    }

    /** The start answers kept last time, or `null` for none. */
    private fun readStart(): String? = try {
        if (startFile.exists()) startFile.readText().takeIf { it.isNotBlank() } else null
    } catch (_: Exception) {
        null
    }

    /** Keeps the start answers as the theme is kept; a failure is silent. */
    private fun writeStart(text: String?) {
        if (text == null) return
        try {
            startTemp.writeText(text)
            if (!startTemp.renameTo(startFile)) startTemp.delete()
        } catch (_: Exception) {
            startTemp.delete()
        }
    }

    /** Each guide page's ticked steps kept last time, as [Core.steps] takes them. */
    private fun readSteps(): List<String> = try {
        stepsDir.listFiles()
            ?.filter { PAGE_ID.matches(it.name) }
            ?.mapNotNull { f -> f.readText().trim().takeIf { it.isNotEmpty() }?.let { "${f.name}\n$it" } }
            ?: emptyList()
    } catch (_: Exception) {
        emptyList()
    }

    /**
     * Keeps one page's ticked steps as the start answers are kept, or
     * drops the page's file when none is ticked; a failure is silent.
     */
    private fun writeSteps(text: String?) {
        if (text == null) return
        val page = text.substringBefore('\n').trim()
        val done = text.substringAfter('\n', "").trim()
        if (!PAGE_ID.matches(page)) return
        val file = File(stepsDir, page)
        val temp = File(stepsDir, "$page.tmp")
        try {
            if (done.isEmpty()) {
                file.delete()
                return
            }
            stepsDir.mkdirs()
            temp.writeText(done)
            if (!temp.renameTo(file)) temp.delete()
        } catch (_: Exception) {
            temp.delete()
        }
    }

    private companion object {
        /** The theme's file, in `filesDir`. */
        const val THEME_NAME = "theme"

        /** The start answers' file, in `filesDir`. */
        const val START_NAME = "start"

        /** The directory of ticked steps, in `filesDir`. */
        const val STEPS_NAME = "steps"

        /** A guide page's id, the only file name kept in [STEPS_NAME]. */
        val PAGE_ID = Regex("[a-z0-9-]{1,64}")

        /** 20 Hz: enough for an animated code's fastest rate. */
        const val TICK_MS = 50L

        /** View pixel to frame pixel along one axis, nearest neighbour. */
        fun toFrame(position: Float, viewLength: Int, frameLength: Int): Int {
            if (viewLength <= 0 || frameLength <= 0) return 0
            val scaled = (position * frameLength / viewLength).toInt()
            return scaled.coerceIn(0, frameLength - 1)
        }
    }
}
