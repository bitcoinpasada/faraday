package app.opensigner

import android.Manifest
import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.hardware.biometrics.BiometricManager
import android.hardware.biometrics.BiometricPrompt
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.CancellationSignal
import android.os.VibrationEffect
import android.os.Vibrator
import android.os.VibratorManager
import android.view.WindowInsets
import android.view.WindowManager
import android.widget.FrameLayout
import android.window.OnBackInvokedDispatcher
import java.io.ByteArrayOutputStream

/**
 * The one Activity. It owns a [SignerView] and answers the things the view
 * cannot do for itself: the document picker, the document creator, the
 * camera and its permission, haptics, the secure element behind a key kept
 * on this device, and leaving.
 *
 * Nothing the core produces is logged (docs/PLANNING.md §5.3), and no
 * state is written anywhere: the manifest disables backups, and this class
 * saves nothing in `onSaveInstanceState`. Rotation is locked to portrait
 * and every configuration change the activity might see is declared in the
 * manifest, so the process is never recreated under a loaded key.
 */
class MainActivity : Activity(), SignerView.Host, Camera.Listener {

    private lateinit var view: SignerView

    /** The camera, open only while the core has asked for it and we are resumed. */
    private lateinit var camera: Camera

    /** Whether the core's last camera command was on. */
    private var wantsCamera = false

    /** Whether the permission dialog is in front of the user. */
    private var askingCamera = false

    /** The kind of the file request now in front of the user. */
    private var awaitingFile = Core.FILE_NONE

    /** The bytes waiting for a place to be written to. */
    private var awaitingWrite: ByteArray? = null

    /** The kind of the write the core is waiting on an answer for. */
    private var awaitingWriteKind = Core.FILE_NONE

    /** The Keystore keys and the blob they protect. */
    private lateinit var kept: KeptSecret

    /**
     * The authentication prompt in front of the person, if one is.
     * Cancelling it is how a prompt the system dismissed without a
     * callback is cleared away when the core asks again.
     */
    private var prompting: CancellationSignal? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Before any content exists, so that no frame of this window can
        // ever be screenshotted, recorded, or shown in the recents
        // thumbnail (§4.6, §5.3).
        window.setFlags(
            WindowManager.LayoutParams.FLAG_SECURE,
            WindowManager.LayoutParams.FLAG_SECURE,
        )

        // Before the view, which asks it what backs the secure element as
        // soon as it has a size: that answer is fixed for the session.
        kept = KeptSecret(this)
        view = SignerView(this, this)
        camera = Camera(this, this)
        val root = FrameLayout(this)
        root.addView(view)
        setContentView(root)

        // Android 15 draws every app edge to edge with no way to opt out,
        // so the frame is inset by hand: the core gets the area that is
        // actually visible, and the black window background fills the rest.
        root.setOnApplyWindowInsetsListener { padded, insets ->
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                val bars = insets.getInsets(
                    WindowInsets.Type.systemBars() or WindowInsets.Type.displayCutout(),
                )
                padded.setPadding(bars.left, bars.top, bars.right, bars.bottom)
            } else {
                @Suppress("DEPRECATION")
                padded.setPadding(
                    insets.systemWindowInsetLeft,
                    insets.systemWindowInsetTop,
                    insets.systemWindowInsetRight,
                    insets.systemWindowInsetBottom,
                )
            }
            insets
        }

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            onBackInvokedDispatcher.registerOnBackInvokedCallback(
                OnBackInvokedDispatcher.PRIORITY_DEFAULT,
            ) { view.back() }
        }
    }

    /** The back gesture below API 33, where the manifest opt-in has no effect. */
    @Deprecated("Superseded by OnBackInvokedDispatcher above API 32")
    @Suppress("DEPRECATION")
    override fun onBackPressed() {
        view.back()
    }

    /**
     * A backgrounded app must not hold the camera. The core is told nothing:
     * as far as it knows the camera is still on, and [onResume] reopens it if
     * it still wants one.
     */
    override fun onPause() {
        camera.off()
        super.onPause()
    }

    override fun onResume() {
        super.onResume()
        if (wantsCamera && hasCameraPermission()) camera.on()
    }

    override fun onStart() {
        super.onStart()
        view.foreground()
    }

    /**
     * Off screen the app stops ticking, because a ticker nobody can see is
     * battery, and locks nothing. The auto-lock timer says how long an
     * untouched session may stand, and a session left for another app is
     * held to that same window, not a shorter one: switching away and back
     * costs the state only once the timer has run out. `onStart` hands the
     * core the elapsed time on the wall clock and the core applies its own
     * timers to it, so an app left overnight comes back locked, or wiped,
     * exactly as the settings asked.
     */
    override fun onStop() {
        view.background()
        super.onStop()
    }

    override fun onDestroy() {
        camera.off()
        view.destroy()
        super.onDestroy()
    }

    // --- SignerView.Host ---

    override fun requestFile(kind: Int) {
        awaitingFile = kind
        // ACTION_OPEN_DOCUMENT rather than a path: the app has no storage
        // permission and never browses the filesystem. It gets exactly the
        // one document the user pointed at, for as long as it takes to
        // read it (§5.3: no file access the user did not trigger).
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT)
            .addCategory(Intent.CATEGORY_OPENABLE)
            .setType("*/*")
        try {
            startActivityForResult(intent, REQUEST_OPEN)
        } catch (_: ActivityNotFoundException) {
            answerFile(null)
        }
    }

    /**
     * The clipboard, read on the main thread, which is where Android
     * requires it: this runs inside the view's command drain, on that
     * thread. Nothing listens to the clipboard and nothing polls it; the
     * only read is this one, for a tap on "Paste".
     */
    override fun requestClipboard(kind: Int) {
        val manager = getSystemService(Context.CLIPBOARD_SERVICE) as? ClipboardManager
        val text = manager?.primaryClip
            ?.takeIf { it.itemCount > 0 }
            ?.getItemAt(0)
            ?.coerceToText(this)
            ?.toString()
            ?.takeIf { it.isNotEmpty() }
        view.clipboardAnswer(kind, text)
    }

    /**
     * "Copy", on the same thread. The core sends this only for public
     * strings (docs/DESIGN.md §4.10), so nothing secret reaches the
     * clipboard; the label is what a clipboard viewer shows beside it.
     */
    override fun writeClipboard(kind: Int, text: String?) {
        if (text == null) {
            view.copyAnswer(kind, false)
            return
        }
        val manager = getSystemService(Context.CLIPBOARD_SERVICE) as? ClipboardManager
        if (manager == null) {
            view.copyAnswer(kind, false)
            return
        }
        val written = try {
            manager.setPrimaryClip(ClipData.newPlainText(CLIP_LABEL, text))
            true
        } catch (_: RuntimeException) {
            // A clipboard service that refuses the transaction (a very
            // large string, a device policy) is a copy that did not
            // happen, which the screen states.
            false
        }
        view.copyAnswer(kind, written)
    }

    override fun writeFile(kind: Int, name: String?, bytes: ByteArray?) {
        if (bytes == null) {
            view.writeAnswer(kind, false)
            return
        }
        awaitingWrite = bytes
        awaitingWriteKind = kind
        val intent = Intent(Intent.ACTION_CREATE_DOCUMENT)
            .addCategory(Intent.CATEGORY_OPENABLE)
            .setType(if (kind == Core.FILE_PNG) "image/png" else "application/octet-stream")
            .putExtra(Intent.EXTRA_TITLE, name ?: DEFAULT_NAME)
        try {
            startActivityForResult(intent, REQUEST_CREATE)
        } catch (_: ActivityNotFoundException) {
            awaitingWrite = null
            answerWrite(false)
        }
    }

    override fun cameraOn() {
        wantsCamera = true
        if (hasCameraPermission()) {
            camera.on()
        } else if (!askingCamera) {
            // The framework API, not AndroidX: minSdk is 26 and the shell has
            // no dependencies. The dialog pauses the activity, so the camera
            // is opened from the result or from onResume, whichever is first.
            askingCamera = true
            requestPermissions(arrayOf(Manifest.permission.CAMERA), REQUEST_CAMERA)
        }
    }

    override fun cameraOff() {
        wantsCamera = false
        camera.off()
    }

    override fun vibrate(ms: Int) {
        if (ms <= 0) return
        // The manifest declares no VIBRATE permission (§11.2: the camera is
        // the only one), so this is a no-op on any device that guards the
        // vibrator. Whether haptics are worth a permission is a question for
        // a later milestone; the handler exists so the command is never
        // silently dropped in code.
        val vibrator = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            getSystemService(VibratorManager::class.java)?.defaultVibrator
        } else {
            @Suppress("DEPRECATION")
            getSystemService(Vibrator::class.java)
        }
        try {
            vibrator?.vibrate(VibrationEffect.createOneShot(ms.toLong(), VibrationEffect.DEFAULT_AMPLITUDE))
        } catch (_: SecurityException) {
            // No VIBRATE permission. Nothing to do and nothing to say.
        }
    }

    // --- a key kept on this device ---

    override fun secureLevel(): Int = kept.secure

    /**
     * What key attestation says about the boot, as the three answers the
     * core acts on.
     *
     * A locked bootloader with a verified or a self-signed state is the
     * device this app is written for. Self-signed with the bootloader
     * locked is GrapheneOS or CalyxOS: the system is signed by a key the
     * owner installed and the bootloader verifies it on every boot,
     * which is the same promise the manufacturer's own key makes, so it
     * is fine.
     *
     * An unlocked bootloader is not, whatever the state beside it says:
     * the system can be replaced, and with it every promise about the
     * sandbox, the screen and the element. So is a failed verification.
     * The core refuses to run on either.
     *
     * No attestation at all — an old device, a keystore that attests
     * nothing — is unknown, and the app runs with the tier saying what
     * it can and cannot promise.
     */
    override fun bootState(): Int {
        val state = kept.verifiedBoot
        if (state == KeptSecret.BOOT_UNKNOWN) return Core.BOOT_UNKNOWN
        val signed = state == KeptSecret.BOOT_VERIFIED || state == KeptSecret.BOOT_SELF_SIGNED
        return if (kept.deviceLocked && signed) Core.BOOT_VERIFIED else Core.BOOT_UNVERIFIED
    }

    override fun secretKept(): Boolean = kept.kept()

    /**
     * One authentication, one tag. The prompt takes a strong biometric or
     * the device credential, whichever the person has, and the `Mac` it
     * authorises is the only thing that can produce the tag: the key it
     * holds requires an authentication for every single use, which is what
     * puts a price on each guess at the stored key's PIN.
     */
    override fun secureMac(salt: ByteArray?) {
        // The core asks once per exchange, so a prompt still standing
        // from an earlier request is one whose callback never came. It
        // is cancelled and its answer, if it ever arrives, is dropped:
        // one stale prompt must not wedge every attempt after it.
        prompting?.cancel()
        prompting = null
        val usable = salt != null &&
            salt.size == Core.MAC_LEN &&
            Build.VERSION.SDK_INT >= Build.VERSION_CODES.R
        val mac = if (usable) kept.beginMac() else null
        if (salt == null || mac == null) {
            // No usable key. When that is because the element's key was
            // invalidated and the blob went with it, the core hears that
            // nothing is kept, so the pad asking for its PIN goes.
            if (!kept.kept()) view.keptAnswer(false)
            view.secureMacAnswer(null)
            return
        }
        val signal = CancellationSignal()
        prompting = signal
        var answered = false
        val answer = { tag: ByteArray? ->
            if (!answered) {
                answered = true
                if (prompting === signal) {
                    prompting = null
                    view.secureMacAnswer(tag)
                }
            }
        }
        val prompt = BiometricPrompt.Builder(this)
            // The system dialog wants a title and the app has exactly one
            // string of its own; everything a person reads about keeping a
            // key is on the core's screen behind this prompt (UX.md §6).
            .setTitle(getString(R.string.app_name))
            .setAllowedAuthenticators(
                BiometricManager.Authenticators.BIOMETRIC_STRONG or
                    BiometricManager.Authenticators.DEVICE_CREDENTIAL,
            )
            .build()
        prompt.authenticate(
            BiometricPrompt.CryptoObject(mac),
            signal,
            mainExecutor,
            object : BiometricPrompt.AuthenticationCallback() {
                override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                    answer(kept.finishMac(result.cryptoObject?.mac ?: mac, salt))
                }

                override fun onAuthenticationError(code: Int, message: CharSequence) {
                    // Cancelled, timed out, or locked out. The core reads
                    // all three the same way: no tag, so no unlock.
                    answer(null)
                }

                override fun onAuthenticationFailed() {
                    // A finger the element did not recognise. The prompt is
                    // still up and the person can try again; an answer now
                    // would be an answer too early.
                }
            },
        )
    }

    override fun storeSecret(blob: ByteArray?) {
        view.storeAnswer(blob != null && kept.store(blob))
    }

    override fun loadSecret() {
        view.secretAnswer(kept.load())
    }

    override fun forgetSecret() {
        kept.forget()
        view.forgetAnswer()
    }

    override fun exit() {
        // The task goes with it, so the app does not sit in recents with a
        // wiped session behind it.
        finishAndRemoveTask()
    }

    // --- Camera.Listener ---

    override fun cameraFrame(width: Int, height: Int, luma: ByteArray, chroma: ByteArray) {
        view.cameraFrame(width, height, luma, chroma)
        // The core has copied what it needs. The frame may picture a SeedQR,
        // so neither of the shell's copies lingers in a reused buffer or a
        // heap dump.
        luma.fill(0)
        chroma.fill(0)
    }

    override fun cameraUnavailable() {
        view.cameraUnavailable()
    }

    // --- the camera permission ---

    private fun hasCameraPermission(): Boolean =
        checkSelfPermission(Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        if (requestCode != REQUEST_CAMERA) return
        askingCamera = false
        // A CameraOff that arrived while the dialog was up wins: the core has
        // left the scanner and is not waiting for an answer.
        if (!wantsCamera) return
        val granted = grantResults.isNotEmpty() &&
            grantResults[0] == PackageManager.PERMISSION_GRANTED
        if (granted) camera.on() else view.cameraUnavailable()
    }

    // --- picker results ---

    @Deprecated("The AndroidX result API would be a dependency; this is the framework one")
    @Suppress("DEPRECATION")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        val uri = if (resultCode == RESULT_OK) data?.data else null
        when (requestCode) {
            // RESULT_CANCELED is the person backing out of the picker, which
            // is not the same fact as a device with no picker at all: the
            // core is told which, and says nothing about the first.
            REQUEST_OPEN -> answerFile(uri?.let(::readAll), resultCode != RESULT_OK)
            REQUEST_CREATE -> {
                val bytes = awaitingWrite
                awaitingWrite = null
                answerWrite(uri != null && bytes != null && writeAll(uri, bytes))
            }
        }
    }

    private fun answerFile(bytes: ByteArray?, cancelled: Boolean = false) {
        val kind = awaitingFile
        awaitingFile = Core.FILE_NONE
        if (kind != Core.FILE_NONE) view.fileAnswer(kind, bytes, cancelled)
    }

    /** Exactly one answer per write the core asked for. */
    private fun answerWrite(written: Boolean) {
        val kind = awaitingWriteKind
        awaitingWriteKind = Core.FILE_NONE
        if (kind != Core.FILE_NONE) view.writeAnswer(kind, written)
    }

    /**
     * Reads the whole document, up to [MAX_FILE]. A PSBT is kilobytes; the
     * cap is there so that pointing the picker at a video cannot take the
     * process down.
     */
    private fun readAll(uri: Uri): ByteArray? = try {
        contentResolver.openInputStream(uri)?.use { input ->
            val out = ByteArrayOutputStream()
            val chunk = ByteArray(1 shl 16)
            while (true) {
                val read = input.read(chunk)
                if (read < 0) break
                if (out.size() + read > MAX_FILE) return null
                out.write(chunk, 0, read)
            }
            out.toByteArray()
        }
    } catch (_: Exception) {
        // A read that fails is a file that did not arrive, which the core
        // already knows how to handle.
        null
    }

    /** Writes the whole document, and says whether it is there. */
    private fun writeAll(uri: Uri, bytes: ByteArray): Boolean = try {
        contentResolver.openOutputStream(uri, "wt")?.use { it.write(bytes) } != null
    } catch (_: Exception) {
        // A write that fails is a file that never appeared, which the
        // core's result screen says instead of "saved as".
        false
    }

    private companion object {
        const val REQUEST_OPEN = 1
        const val REQUEST_CREATE = 2
        const val REQUEST_CAMERA = 3
        const val MAX_FILE = 4 * 1024 * 1024
        const val DEFAULT_NAME = "opensigner.bin"
        const val CLIP_LABEL = "OpenSigner"
    }
}
