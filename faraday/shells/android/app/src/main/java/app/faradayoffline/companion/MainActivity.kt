package app.faradayoffline.companion

import android.Manifest
import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.content.pm.PackageInfo
import android.content.pm.PackageManager
import android.graphics.drawable.ColorDrawable
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.OpenableColumns
import android.view.View
import android.view.WindowInsets
import android.view.WindowInsetsController
import android.view.WindowManager
import android.widget.FrameLayout
import android.window.OnBackInvokedDispatcher
import java.io.ByteArrayOutputStream
import java.security.MessageDigest

/**
 * The one Activity. Copied from upstream's `MainActivity.kt`
 * (`opensigner/shells/android` at d26b84e) and cut to what the Companion
 * asks for: it owns a [CompanionView] and answers what the view cannot do
 * for itself: the document picker and creator, the clipboard's read for
 * Paste, the camera and its permission, the received files' store, the
 * screen kept on while a code shows, the window's colour, About's facts
 * from the package manager, and leaving.
 *
 * Nothing the core produces is logged. The app never writes the clipboard
 * and never opens a link (docs/COMPANION.md §0, §6.3).
 */
class MainActivity : Activity(), CompanionView.Host, Camera.Listener {

    private lateinit var view: CompanionView

    /** The camera, open only while the core has asked for it and we are resumed. */
    private lateinit var camera: Camera

    /** The received files, encrypted under a Keystore key of their own. */
    private lateinit var store: ReceivedStore

    /** Whether the core's last camera command was on. */
    private var wantsCamera = false

    /** Whether the permission dialog is in front of the person. */
    private var askingCamera = false

    /** The kind of the file request now in front of the person. */
    private var awaitingFile = Core.FILE_NONE

    /** The bytes waiting for a place to be written to. */
    private var awaitingWrite: ByteArray? = null

    /** The kind of the write the core is waiting on an answer for. */
    private var awaitingWriteKind = Core.FILE_NONE

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Before any content exists, so that no frame of this window can
        // be screenshotted, recorded, or shown in the recents thumbnail
        // (docs/COMPANION.md §7).
        window.setFlags(
            WindowManager.LayoutParams.FLAG_SECURE,
            WindowManager.LayoutParams.FLAG_SECURE,
        )

        store = ReceivedStore(this)
        view = CompanionView(this, this)
        camera = Camera(this, this)
        val root = FrameLayout(this)
        root.addView(view)
        setContentView(root)

        // Android 15 draws every app edge to edge, so the frame is inset by
        // hand: the core gets the area that is visible, and the window's
        // page colour fills the rest.
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

    /** A backgrounded app does not hold the camera; [onResume] reopens it. */
    override fun onPause() {
        camera.off()
        super.onPause()
    }

    override fun onResume() {
        super.onResume()
        view.systemChanged()
        if (wantsCamera && hasCameraPermission()) camera.on()
    }

    override fun onStart() {
        super.onStart()
        view.foreground()
    }

    override fun onStop() {
        view.background()
        super.onStop()
    }

    override fun onDestroy() {
        camera.off()
        view.destroy()
        super.onDestroy()
    }

    // --- CompanionView.Host ---

    override fun requestFile(kind: Int) {
        awaitingFile = kind
        // ACTION_OPEN_DOCUMENT rather than a path: the app has no storage
        // permission and never browses the filesystem. It gets the one
        // document the person pointed at, for as long as it takes to read.
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT)
            .addCategory(Intent.CATEGORY_OPENABLE)
            .setType("*/*")
        try {
            startActivityForResult(intent, REQUEST_OPEN)
        } catch (_: ActivityNotFoundException) {
            answerFile(null, null)
        }
    }

    /**
     * The clipboard, read on the main thread, for a tap on Paste. Nothing
     * listens to the clipboard and nothing polls it, and nothing here ever
     * writes it.
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

    override fun writeFile(kind: Int, name: String?, bytes: ByteArray?) {
        if (bytes == null) {
            view.writeAnswer(kind, false)
            return
        }
        awaitingWrite = bytes
        awaitingWriteKind = kind
        val type = when (kind) {
            Core.FILE_PNG -> "image/png"
            Core.FILE_TEXT -> "text/plain"
            else -> "application/octet-stream"
        }
        val intent = Intent(Intent.ACTION_CREATE_DOCUMENT)
            .addCategory(Intent.CATEGORY_OPENABLE)
            .setType(type)
            .putExtra(Intent.EXTRA_TITLE, name ?: DEFAULT_NAME)
        try {
            startActivityForResult(intent, REQUEST_CREATE)
        } catch (_: ActivityNotFoundException) {
            awaitingWrite?.fill(0)
            awaitingWrite = null
            answerWrite(false)
        }
    }

    override fun cameraOn() {
        wantsCamera = true
        if (hasCameraPermission()) {
            camera.on()
        } else if (!askingCamera) {
            // The framework API, not AndroidX. The dialog pauses the
            // activity, so the camera opens from the result or from
            // onResume, whichever is first.
            askingCamera = true
            requestPermissions(arrayOf(Manifest.permission.CAMERA), REQUEST_CAMERA)
        }
    }

    override fun cameraOff() {
        wantsCamera = false
        camera.off()
    }

    override fun keepReceived(bytes: ByteArray?) {
        if (bytes == null) return
        store.store(bytes)
        // Encrypted and written; the plain copy goes.
        bytes.fill(0)
    }

    override fun loadReceived(): ByteArray? = store.load()

    override fun keepScreenOn(on: Boolean) {
        if (on) {
            window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        } else {
            window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        }
    }

    /**
     * The window behind the frame, and the system bars over it, in the
     * page colour of the theme on show, with dark bar icons on a light
     * page. The bars' colours are ignored from Android 15, where the
     * window's background shows through them instead.
     */
    override fun pageColor(color: Int, light: Boolean) {
        window.setBackgroundDrawable(ColorDrawable(color))
        @Suppress("DEPRECATION")
        window.statusBarColor = color
        @Suppress("DEPRECATION")
        window.navigationBarColor = color
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            val bars = WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS or
                WindowInsetsController.APPEARANCE_LIGHT_NAVIGATION_BARS
            window.insetsController?.setSystemBarsAppearance(if (light) bars else 0, bars)
        } else {
            val flags = View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR or View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR
            @Suppress("DEPRECATION")
            val now = window.decorView.systemUiVisibility
            @Suppress("DEPRECATION")
            window.decorView.systemUiVisibility = if (light) now or flags else now and flags.inv()
        }
    }

    /**
     * About's facts, read from the package manager at run time
     * (docs/COMPANION.md §4.8): the version name and code; the SHA-256 of
     * the certificate the installed APK is signed with, the digest
     * `apksigner verify --print-certs` prints; and each permission the
     * manifest asks for with whether it is granted, as `NAME\t1` or
     * `NAME\t0`, one a line.
     */
    override fun system(): Triple<String, ByteArray, String>? = try {
        val info = packageInfo()
        val code = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            info.longVersionCode
        } else {
            @Suppress("DEPRECATION")
            info.versionCode.toLong()
        }
        val certificate = signingCertificate(info) ?: ByteArray(0)
        val digest = MessageDigest.getInstance("SHA-256").digest(certificate)
        val asked = info.requestedPermissions.orEmpty()
        val flags = info.requestedPermissionsFlags ?: IntArray(asked.size)
        val permissions = asked.indices.joinToString("") { i ->
            val granted = flags.getOrElse(i) { 0 } and PackageInfo.REQUESTED_PERMISSION_GRANTED != 0
            "${asked[i]}\t${if (granted) 1 else 0}\n"
        }
        Triple("${info.versionName} ($code)", digest, permissions)
    } catch (_: Exception) {
        null
    }

    @Suppress("DEPRECATION")
    private fun packageInfo(): PackageInfo {
        val what = PackageManager.GET_PERMISSIONS or if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            PackageManager.GET_SIGNING_CERTIFICATES
        } else {
            PackageManager.GET_SIGNATURES
        }
        return packageManager.getPackageInfo(packageName, what)
    }

    /** The encoded certificate the installed APK's contents are signed with. */
    private fun signingCertificate(info: PackageInfo): ByteArray? =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            info.signingInfo?.apkContentsSigners?.firstOrNull()?.toByteArray()
        } else {
            @Suppress("DEPRECATION")
            info.signatures?.firstOrNull()?.toByteArray()
        }

    override fun exit() {
        finishAndRemoveTask()
    }

    // --- Camera.Listener ---

    override fun cameraFrame(width: Int, height: Int, luma: ByteArray, chroma: ByteArray) {
        view.cameraFrame(width, height, luma, chroma)
        // The core has copied what it needs; neither of the shell's copies
        // lingers in a reused buffer or a heap dump.
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
        view.systemChanged()
        // A CameraOff that arrived while the dialog was up wins.
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
            // is not the same fact as a phone with no picker at all.
            REQUEST_OPEN -> answerFile(uri?.let(::displayName), uri?.let(::readAll), resultCode != RESULT_OK)
            REQUEST_CREATE -> {
                val bytes = awaitingWrite
                awaitingWrite = null
                answerWrite(uri != null && bytes != null && writeAll(uri, bytes))
                bytes?.fill(0)
            }
        }
    }

    private fun answerFile(name: String?, bytes: ByteArray?, cancelled: Boolean = false) {
        val kind = awaitingFile
        awaitingFile = Core.FILE_NONE
        if (kind != Core.FILE_NONE) view.fileAnswer(kind, name, bytes, cancelled)
    }

    /** Exactly one answer per write the core asked for. */
    private fun answerWrite(written: Boolean) {
        val kind = awaitingWriteKind
        awaitingWriteKind = Core.FILE_NONE
        if (kind != Core.FILE_NONE) view.writeAnswer(kind, written)
    }

    /** The name the document's provider gives it, or `null`. */
    private fun displayName(uri: Uri): String? = try {
        contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { c ->
            if (c.moveToFirst() && !c.isNull(0)) c.getString(0) else null
        }
    } catch (_: Exception) {
        null
    }

    /**
     * Reads the whole document, up to [MAX_FILE]: a file over the
     * envelope's 256 KiB is refused by the core, and the cap keeps a video
     * from taking the process down.
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
        null
    }

    /** Writes the whole document, and says whether it is there. */
    private fun writeAll(uri: Uri, bytes: ByteArray): Boolean = try {
        contentResolver.openOutputStream(uri, "wt")?.use { it.write(bytes) } != null
    } catch (_: Exception) {
        false
    }

    private companion object {
        const val REQUEST_OPEN = 1
        const val REQUEST_CREATE = 2
        const val REQUEST_CAMERA = 3
        const val MAX_FILE = 4 * 1024 * 1024
        const val DEFAULT_NAME = "received.bin"
    }
}
