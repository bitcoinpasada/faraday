package app.faradayoffline.companion

/**
 * Faraday Companion's Rust core, as seen from Kotlin. Copied from
 * upstream's `opensigner/shells/android` at d26b84e and cut to the
 * Companion's calls.
 *
 * Every declaration here has a matching
 * `Java_app_faradayoffline_companion_Core_*` in
 * `faraday/faraday-companion-ffi/src/jni.rs`; changing a name or a
 * signature on one side without the other produces an
 * `UnsatisfiedLinkError` at the first call, not a silent mismatch.
 *
 * The shell holds a `handle`, never a pointer: the Rust side keeps a table
 * and answers a handle it does not recognise by doing nothing.
 *
 * Nothing here is logged. What crosses this boundary is pixels, camera
 * frames, the files and text the person chose or shared in, and the two
 * stores; none of it belongs in logcat.
 */
internal object Core {

    init {
        System.loadLibrary("faraday_companion")
    }

    // --- command codes returned by pollCommand ---

    /** Nothing is pending; stop draining. */
    const val CMD_NONE = 0

    /** Blit the frame. */
    const val CMD_DRAW = 1

    /** The core is done. */
    const val CMD_EXIT = 2

    /** Ask the person for a file; the kind is in [commandKind]. */
    const val CMD_REQUEST_FILE = 3

    /** Store [commandBytes] where the person picks, starting from [commandName]. */
    const val CMD_WRITE_FILE = 4

    /** Start the camera. */
    const val CMD_CAMERA_ON = 6

    /** Stop the camera. */
    const val CMD_CAMERA_OFF = 7

    /** Read the clipboard, for a tap on Paste; the kind is in [commandKind]. */
    const val CMD_REQUEST_CLIPBOARD = 14

    /** Keep [commandBytes] as the received files' store; hand them back at start. */
    const val CMD_KEEP_RECEIVED = 16

    /** Keep the screen on, or let it sleep: [commandOn]. */
    const val CMD_KEEP_SCREEN_ON = 17

    /** Keep the theme [commandText]; hand it back at start. */
    const val CMD_KEEP_THEME = 18

    /**
     * Keep [commandText], the computer's id, a newline and the maker's
     * page id (either may be empty); hand it back with [start] at start.
     */
    const val CMD_KEEP_START = 22

    /**
     * Keep [commandText], a guide page's id, a newline and its ticked step
     * numbers separated by commas (none: drop what is kept for the page);
     * hand each page's text back with [steps] at start.
     */
    const val CMD_KEEP_STEPS = 23

    /**
     * Ask the phone's unlock and open the kept store; the prompt's title is
     * [commandText]. Answer with exactly one [kept] or [keptClosed].
     */
    const val CMD_UNLOCK_KEPT = 19

    /**
     * Keep [commandBytes] as the kept store, with [commandDescriptions] and
     * [commandKeys] beside it; empty bytes delete the store and its key.
     */
    const val CMD_KEEP_KEPT = 20

    /** Share [commandBytes] as [commandName], of type [commandText]. */
    const val CMD_SHARE = 21

    /** The handle names nothing. */
    const val CMD_ERROR = -1

    // --- why the kept store stayed closed ---

    const val CLOSED_CANCELLED = 0
    const val CLOSED_NO_SCREEN_LOCK = 1
    const val CLOSED_UNREADABLE = 2

    // --- file kinds ---

    const val FILE_PSBT = 0
    const val FILE_ANY = 1
    const val FILE_TEXT = 2
    const val FILE_PNG = 3
    const val FILE_NONE = -1

    // --- touch phases ---

    const val TOUCH_DOWN = 0
    const val TOUCH_MOVE = 1
    const val TOUCH_UP = 2

    /** Back. The only key this shell sends. */
    const val KEY_ESCAPE = 3

    /** How many bytes the signing certificate's digest is. */
    const val CERT_LEN = 32

    /**
     * Creates the core for a `width` x `height` display at `dpi` and sends
     * it the display. Returns a handle, or `0` if the core refused the
     * size. The view is padded by the system bars before it is measured,
     * so both insets are zero.
     */
    external fun new(width: Int, height: Int, dpi: Int, insetBottom: Int, insetTop: Int): Long

    /** Drops the core and zeroizes everything it held. */
    external fun free(handle: Long)

    /** A contact at a frame pixel; `phase` is one of the `TOUCH_*` codes. */
    external fun touch(handle: Long, x: Int, y: Int, phase: Int)

    /** A scroll of `dy` frame pixels at a frame pixel. */
    external fun scroll(handle: Long, x: Int, y: Int, dy: Int)

    /** Monotonic milliseconds; also hands over what the scanner read. */
    external fun tick(handle: Long, nowMs: Long)

    /** A key press: [KEY_ESCAPE]. */
    external fun key(handle: Long, code: Int)

    /** The answer to [CMD_REQUEST_FILE]: the bytes, and the name the provider gave. */
    external fun file(handle: Long, kind: Int, name: String?, bytes: ByteArray)

    /** The answer to [CMD_REQUEST_FILE] when no file came. */
    external fun fileUnavailable(handle: Long, kind: Int)

    /** The answer to [CMD_REQUEST_FILE] when the person closed the picker. */
    external fun fileCancelled(handle: Long, kind: Int)

    /** The answer to [CMD_WRITE_FILE]: the bytes are stored. */
    external fun fileWritten(handle: Long, kind: Int)

    /** The answer to [CMD_WRITE_FILE] when the person cancelled or the write failed. */
    external fun fileNotWritten(handle: Long, kind: Int)

    /** The answer to [CMD_REQUEST_CLIPBOARD]: what the clipboard holds. */
    external fun clipboard(handle: Long, kind: Int, text: String)

    /** The answer to [CMD_REQUEST_CLIPBOARD] when no text comes. */
    external fun clipboardUnavailable(handle: Long, kind: Int)

    /**
     * One camera frame: 8-bit luma, `width * height` bytes, and its NV12
     * chroma plane, or null for a grey preview.
     */
    external fun cameraFrame(handle: Long, width: Int, height: Int, luma: ByteArray, chroma: ByteArray?)

    /** The answer to [CMD_CAMERA_ON] when no frames will come. */
    external fun cameraUnavailable(handle: Long)

    /** The wall clock: seconds since 1970 UTC and the offset from UTC now. */
    external fun time(handle: Long, unixS: Long, offsetS: Int)

    /** The received files' store kept from [CMD_KEEP_RECEIVED], at start. */
    external fun received(handle: Long, bytes: ByteArray)

    /** The theme kept from [CMD_KEEP_THEME], at start. */
    external fun theme(handle: Long, id: String)

    /** The text kept from [CMD_KEEP_START], at start. */
    external fun start(handle: Long, text: String)

    /** One page's text kept from [CMD_KEEP_STEPS], at start. */
    external fun steps(handle: Long, text: String)

    /**
     * About's facts from the package manager, at start: the version name
     * and code, the signing certificate's SHA-256 ([CERT_LEN] bytes), and
     * the permissions the app holds, one a line.
     */
    external fun system(handle: Long, build: String, certificate: ByteArray, permissions: String)

    /** The answer to [CMD_UNLOCK_KEPT]: the kept store, decrypted; empty when nothing is kept. */
    external fun kept(handle: Long, bytes: ByteArray)

    /** The answer to [CMD_UNLOCK_KEPT] when the store stayed closed: a `CLOSED_*` code. */
    external fun keptClosed(handle: Long, why: Int)

    /**
     * The app went to the background: the core wipes what it held of the
     * kept store. Never while the system's own unlock prompt stopped us.
     */
    external fun locked(handle: Long)

    /** The two counts kept beside the kept store, at start. */
    external fun keptCounts(handle: Long, descriptions: Int, keys: Int)

    /** One file (with its name) or one text (null name, UTF-8) shared to the app. */
    external fun sharedIn(handle: Long, name: String?, bytes: ByteArray)

    /** Takes the next command as a `CMD_*` code and parks its payload. */
    external fun pollCommand(handle: Long): Int

    /** The file kind of the last polled command, or [FILE_NONE]. */
    external fun commandKind(handle: Long): Int

    /** The bytes of the last polled command. Answers once; then null. */
    external fun commandBytes(handle: Long): ByteArray?

    /** The suggested file name of the last polled command, once. */
    external fun commandName(handle: Long): String?

    /** The text of the last polled command ([CMD_KEEP_THEME]), once. */
    external fun commandText(handle: Long): String?

    /** [CMD_KEEP_KEPT]'s count of wallet descriptions. */
    external fun commandDescriptions(handle: Long): Int

    /** [CMD_KEEP_KEPT]'s count of BitLocker keys. */
    external fun commandKeys(handle: Long): Int

    /** Whether the last polled command says on ([CMD_KEEP_SCREEN_ON]). */
    external fun commandOn(handle: Long): Boolean

    /** The page's colour in the theme on show, opaque ARGB. */
    external fun pageColor(handle: Long): Int

    /** Whether the theme on show is dark text on a light page. */
    external fun pageLight(handle: Long): Boolean

    /**
     * A direct buffer over the core's framebuffer: premultiplied RGBA8888,
     * `width * height * 4` bytes, laid out as an ARGB_8888 bitmap wants.
     * The memory belongs to Rust and is valid until the next call on this
     * handle: read it, copy out of it, and drop it.
     */
    external fun frame(handle: Long): java.nio.ByteBuffer?
}
