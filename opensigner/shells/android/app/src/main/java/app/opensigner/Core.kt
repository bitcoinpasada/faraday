package app.opensigner

/**
 * The Rust core, as seen from Kotlin.
 *
 * Every declaration here has a matching `Java_app_opensigner_Core_*` in
 * `opensigner/opensigner-ffi/src/jni.rs`; changing a name or a signature on
 * one side without the other produces an `UnsatisfiedLinkError` at the
 * first call, not a silent mismatch.
 *
 * The shell holds a `handle`, never a pointer: the Rust side keeps a table
 * and answers a handle it does not recognise by doing nothing.
 *
 * Nothing here is logged. The bytes that cross this boundary are pixels,
 * files the user chose, and entropy; none of them belongs in logcat
 * (docs/PLANNING.md §5.3).
 */
internal object Core {

    init {
        System.loadLibrary("opensigner")
    }

    // --- command codes returned by pollCommand ---

    /** Nothing is pending; stop draining. */
    const val CMD_NONE = 0

    /** Blit the frame. */
    const val CMD_DRAW = 1

    /** The core is done. */
    const val CMD_EXIT = 2

    /** Ask the user for a file; the kind is in [commandKind]. */
    const val CMD_REQUEST_FILE = 3

    /** Store [commandBytes] under [commandName]. */
    const val CMD_WRITE_FILE = 4

    /** 32 random bytes, please. */
    const val CMD_REQUEST_ENTROPY = 5

    /** Start the camera. */
    const val CMD_CAMERA_ON = 6

    /** Stop the camera. */
    const val CMD_CAMERA_OFF = 7

    /** Vibrate for [commandMs] milliseconds. */
    const val CMD_VIBRATE = 8

    /** Keep [commandBytes] for the next run; hand them back at start. */
    const val CMD_STORE_SETTINGS = 9

    /**
     * Authenticate the 32-byte salt in [commandBytes] inside the secure
     * element; answer with one [secureMac] or [secureUnavailable].
     *
     * The salt is a challenge the core derives from the PIN it is
     * trying, so it differs between one attempt and the next on the same
     * stored key, and the tag that comes back is worth nothing for any
     * other PIN.
     */
    const val CMD_SECURE_MAC = 10

    /**
     * Keep the blob in [commandBytes], replacing whatever was kept;
     * answer with one [secretStored] or [secretNotStored].
     */
    const val CMD_STORE_SECRET = 11

    /** Hand the blob back: one [secret] or [secretUnavailable]. */
    const val CMD_LOAD_SECRET = 12

    /** Delete the blob and its hardware keys; answer [secretForgotten]. */
    const val CMD_FORGET_SECRET = 13

    /**
     * Read the clipboard; the kind is in [commandKind]. Answer with one
     * [clipboard] or [clipboardUnavailable]. Sent only for a tap on
     * "Paste": the shell never reads the clipboard unasked.
     */
    const val CMD_REQUEST_CLIPBOARD = 14

    /**
     * Put [commandText] on the clipboard; the kind is in [commandKind].
     * Answer with one [clipboardWritten] or [clipboardNotWritten]. Sent
     * only for a tap on "Copy", and never with a secret in it.
     */
    const val CMD_WRITE_CLIPBOARD = 15

    /** The handle names no app. */
    const val CMD_ERROR = -1

    // --- file kinds ---

    /** The core wants a PSBT. */
    const val FILE_PSBT = 0

    /** The core wants any file. */
    const val FILE_ANY = 1

    /** Text, which is what a clipboard carries. */
    const val FILE_TEXT = 2

    /** A PNG image. */
    const val FILE_PNG = 3

    /** The last command carried no file kind. */
    const val FILE_NONE = -1

    // --- touch phases ---

    /** A contact began. */
    const val TOUCH_DOWN = 0

    /** A contact moved. */
    const val TOUCH_MOVE = 1

    /** A contact ended. */
    const val TOUCH_UP = 2

    // --- keys ---

    /** Cancel / back. The only key this shell ever sends: text goes
     *  through the core's own on-screen keyboard (§4.5). */
    const val KEY_ESCAPE = 3

    // --- assurance tiers (docs/PLANNING.md §3) ---

    /** A phone whose secure element wraps the key it keeps. */
    const val TIER_B = 1

    /** Stock Android: the OS is trusted, there is no hardware key wrap. */
    const val TIER_C = 2

    // --- what backs the secure element (docs/PLANNING.md §6) ---

    /** No secure element: no key can be kept on this device. */
    const val SECURE_NONE = 0

    /** A trusted execution environment. */
    const val SECURE_TEE = 1

    /** A separate chip. */
    const val SECURE_STRONGBOX = 2

    // --- the device's verified boot (docs/PLANNING.md §6) ---

    /** The platform said nothing about how the device booted. */
    const val BOOT_UNKNOWN = 0

    /** The bootloader is locked and it verified the running system,
     *  against the manufacturer's key or one the owner installed. */
    const val BOOT_VERIFIED = 1

    /** It did not: the bootloader is unlocked, or verification ran and
     *  failed. The core refuses to run. */
    const val BOOT_UNVERIFIED = 2

    /** How many bytes an entropy answer carries. */
    const val ENTROPY_LEN = 32

    /** How many bytes a [CMD_SECURE_MAC] salt and its tag carry. */
    const val MAC_LEN = 32

    /**
     * Creates an app for a `width` x `height` display at `dpi` and sends it
     * the one-off display event. Returns a handle, or `0` if the core
     * refused the parameters. `cameraFixed` says the frames this shell
     * sends are always upright.
     *
     * `insetBottom` and `insetTop` are the strips of the frame, in pixels,
     * the person cannot use. This shell pads its view by the system bars
     * and the display cutout before it creates the core, so the frame is
     * already the visible area and both are zero.
     *
     * `secure` is one of the `SECURE_*` codes: what backs this device's
     * secure element, and so whether a key may be kept on it at all.
     * `boot` is one of the `BOOT_*` codes: what key attestation said
     * about the device's boot, which About states and on which
     * [BOOT_UNVERIFIED] makes the core show a refusal instead of the app.
     *
     * `memoryMib` is this device's total memory in MiB, from
     * `ActivityManager.MemoryInfo.totalMem`, or 0 where it cannot be
     * read. It decides which Argon2id memory cost Settings recommends
     * for an encrypted backup.
     */
    external fun new(
        width: Int,
        height: Int,
        dpi: Int,
        insetBottom: Int,
        insetTop: Int,
        tier: Int,
        version: String?,
        cameraFixed: Boolean,
        secure: Int,
        boot: Int,
        memoryMib: Int,
    ): Long

    /** Drops the app and zeroizes everything it held. */
    external fun free(handle: Long)

    /** A contact at a frame pixel; `phase` is one of the `TOUCH_*` codes. */
    external fun touch(handle: Long, x: Int, y: Int, phase: Int)

    /** A scroll of `dy` frame pixels at a frame pixel. */
    external fun scroll(handle: Long, x: Int, y: Int, dy: Int)

    /** Monotonic milliseconds; drives holds, masking and the session timers. */
    external fun tick(handle: Long, nowMs: Long)

    /** A key press; `ch` is the code point and is ignored except for text. */
    external fun key(handle: Long, code: Int, ch: Int)

    /** The answer to [CMD_REQUEST_FILE]: the file's bytes, unmodified. */
    external fun file(handle: Long, kind: Int, bytes: ByteArray)

    /** The answer to [CMD_REQUEST_FILE] when this device has no file channel. */
    external fun fileUnavailable(handle: Long, kind: Int)

    /** The answer to [CMD_REQUEST_FILE] when the user closed the picker. */
    external fun fileCancelled(handle: Long, kind: Int)

    /** The app left the foreground: lock the session now. */
    external fun lock(handle: Long)

    /** The answer to [CMD_WRITE_FILE]: the bytes are stored. */
    external fun fileWritten(handle: Long, kind: Int)

    /** The answer to [CMD_WRITE_FILE] when the user cancelled or the write failed. */
    external fun fileNotWritten(handle: Long, kind: Int)

    /** The answer to [CMD_REQUEST_CLIPBOARD]: what the clipboard holds. */
    external fun clipboard(handle: Long, kind: Int, text: String)

    /**
     * The answer to [CMD_REQUEST_CLIPBOARD] when no text comes: nothing on
     * the clipboard, no clipboard, or content that is not text.
     */
    external fun clipboardUnavailable(handle: Long, kind: Int)

    /** The answer to [CMD_WRITE_CLIPBOARD]: the text is on the clipboard. */
    external fun clipboardWritten(handle: Long, kind: Int)

    /** The answer to [CMD_WRITE_CLIPBOARD] when nothing was put there. */
    external fun clipboardNotWritten(handle: Long, kind: Int)

    /** The answer to [CMD_REQUEST_ENTROPY]: exactly [ENTROPY_LEN] bytes. */
    external fun entropy(handle: Long, bytes: ByteArray)

    /**
     * The settings kept from an earlier session, exactly as the core wrote
     * them in [CMD_STORE_SETTINGS]. Sent once, after the display and before
     * any input; a shell with nothing kept sends nothing.
     */
    external fun settings(handle: Long, bytes: ByteArray)

    /**
     * One camera frame: 8-bit luma, `width * height` bytes, and its NV12
     * chroma plane, `ceil(width / 2) * ceil(height / 2)` pairs of
     * interleaved U and V. The preview is drawn from the chroma and the
     * decode reads the luma alone; a null chroma leaves the preview grey.
     */
    external fun cameraFrame(
        handle: Long,
        width: Int,
        height: Int,
        luma: ByteArray,
        chroma: ByteArray?,
    )

    /** The answer to [CMD_CAMERA_ON] when there is no camera. */
    external fun cameraUnavailable(handle: Long)

    /**
     * Whether a blob is on this device. Sent once, after the display and
     * the settings and before any input; the core offers the way back to a
     * stored key only when this said `true`.
     */
    external fun secretKept(handle: Long, kept: Boolean)

    /** The answer to [CMD_SECURE_MAC]: exactly [MAC_LEN] bytes. */
    external fun secureMac(handle: Long, mac: ByteArray)

    /**
     * The answer to [CMD_SECURE_MAC] when no tag comes: no secure
     * hardware, an authentication the person cancelled, or a key that is
     * gone.
     */
    external fun secureUnavailable(handle: Long)

    /** The answer to [CMD_STORE_SECRET]: the bytes are kept. */
    external fun secretStored(handle: Long)

    /** The answer to [CMD_STORE_SECRET] when nothing was kept. */
    external fun secretNotStored(handle: Long)

    /** The answer to [CMD_LOAD_SECRET]: the blob, as it was stored. */
    external fun secret(handle: Long, blob: ByteArray)

    /** The answer to [CMD_LOAD_SECRET] when no blob comes back. */
    external fun secretUnavailable(handle: Long)

    /** The answer to [CMD_FORGET_SECRET]: the blob and its keys are gone. */
    external fun secretForgotten(handle: Long)

    /** Takes the next command as a `CMD_*` code and parks its payload. */
    external fun pollCommand(handle: Long): Int

    /** The file kind of the last polled command, or [FILE_NONE]. */
    external fun commandKind(handle: Long): Int

    /** The bytes of the last polled command. Answers once; then null. */
    external fun commandBytes(handle: Long): ByteArray?

    /** The suggested file name of the last polled command. */
    external fun commandName(handle: Long): String?

    /** The duration in milliseconds of the last polled command. */
    /** The last command's text ([CMD_WRITE_CLIPBOARD]), once. */
    external fun commandText(handle: Long): String?

    external fun commandMs(handle: Long): Int

    /**
     * A direct buffer over the core's framebuffer: premultiplied RGBA8888,
     * `width * height * 4` bytes, laid out exactly as an ARGB_8888
     * [android.graphics.Bitmap] wants them, so `copyPixelsFromBuffer` is
     * the only copy in the whole path.
     *
     * The memory belongs to Rust and is valid until the next call on this
     * handle. Read it, copy out of it, and drop it; never keep it across a
     * frame.
     */
    external fun frame(handle: Long): java.nio.ByteBuffer?
}
