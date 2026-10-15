package app.faradayoffline.companion

import android.app.KeyguardManager
import android.content.Context
import android.os.Build
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyPermanentlyInvalidatedException
import android.security.keystore.KeyProperties
import android.security.keystore.UserNotAuthenticatedException
import java.io.File
import java.security.KeyStore
import java.security.SecureRandom
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec
import javax.crypto.spec.SecretKeySpec

/**
 * Wallet descriptions (docs/COMPANION.md §7, D4): the
 * bytes the core hands over in `Core.CMD_KEEP_KEPT`, kept behind the
 * phone's own unlock.
 *
 * Three files in the app's private files:
 *  - `kept`: the store, AES-256-GCM under a data key (the IV, then the
 *    ciphertext);
 *  - `kept.key`: the data key, a random 256-bit key, wrapped with
 *    AES-256-GCM by a Keystore key that works only after the phone's
 *    unlock (the IV, then the wrapped key);
 *  - `kept.counts`: the store's version and how many descriptions it holds
 *    ("v2 3"), outside the encryption, so Home can show the count while the
 *    store is locked. A store of version 1 held BitLocker keys too (F33): it
 *    has two numbers there, and [counts] reports version 1,
 *    which has the core delete it at the first start.
 *
 * The Keystore key is bound to the unlock. On Android 11 and later it
 * needs one for every use (`setUserAuthenticationParameters(0, strong
 * biometric or device credential)`), given through `BiometricPrompt` with
 * the cipher as its `CryptoObject`; on Android 8 to 10 it may be used for
 * 30 seconds after the system's confirm-credential screen. Either way one
 * prompt unwraps the data key, which then stays in memory, and every write
 * while the store is open uses it without asking again. [lock] wipes it;
 * the activity calls that on `onStop`.
 *
 * The first open makes the data key and wraps it, under the same one
 * prompt, so the first keep asks nothing more. A phone with no screen lock
 * cannot make or use such a Keystore key: [begin] answers
 * [Begin.NoScreenLock] and nothing is kept. Removing the screen lock
 * invalidates the key for good, so a store left from before is
 * [Begin.Unreadable] until the person deletes it.
 *
 * Nothing here is logged.
 */
internal class KeptStore(private val context: Context) {

    private val store = File(context.filesDir, STORE_NAME)
    private val storeTemp = File(context.filesDir, "$STORE_NAME.tmp")
    private val wrapped = File(context.filesDir, KEY_NAME)
    private val wrappedTemp = File(context.filesDir, "$KEY_NAME.tmp")
    private val countsFile = File(context.filesDir, COUNTS_NAME)
    private val countsTemp = File(context.filesDir, "$COUNTS_NAME.tmp")

    private val keystore: KeyStore? = try {
        KeyStore.getInstance(PROVIDER).apply { load(null) }
    } catch (_: Exception) {
        null
    }

    /** The data key, while the store is open; wiped by [lock]. */
    private var dataKey: ByteArray? = null

    /** What [begin] found. */
    sealed class Begin {
        /** The phone has no screen lock. */
        object NoScreenLock : Begin()

        /** The store cannot be opened: its key is gone or damaged. */
        object Unreadable : Begin()

        /** The Keystore key needs an unlock it has not had (Android 8 to 10). */
        object NotAuthenticated : Begin()

        /**
         * The cipher to authorise with the unlock: it unwraps the data key,
         * or, the first time ([first]), wraps a new one.
         */
        class Ready(val cipher: Cipher, val first: Boolean) : Begin()
    }

    /** Whether the phone has a screen lock, which the Keystore key needs. */
    fun deviceSecure(): Boolean =
        (context.getSystemService(Context.KEYGUARD_SERVICE) as? KeyguardManager)?.isDeviceSecure == true

    /**
     * The version and the count kept beside the store, or null when none
     * are known (nothing kept, or the counts file is not there). "v2 3" is
     * version 2 and 3 descriptions; the two numbers a version 1 store
     * wrote ("3 1") are version 1. The version is the core's: it is the
     * store's own header byte, copied by [keep].
     */
    fun counts(): Pair<Int, Int>? = try {
        val text = if (countsFile.exists()) countsFile.readText().trim() else null
        val parts = text?.split(' ')
        if (text == null || parts == null || parts.size != 2) {
            null
        } else if (text.startsWith("v")) {
            Pair(parts[0].substring(1).toInt(), parts[1].toInt().coerceAtLeast(0))
        } else {
            parts[0].toInt()
            parts[1].toInt()
            Pair(1, 0)
        }
    } catch (_: Exception) {
        null
    }

    /**
     * The first half of opening the store: the cipher the unlock
     * authorises. On Android 8 to 10 it is called after the
     * confirm-credential screen, inside the key's 30 seconds.
     */
    fun begin(): Begin {
        if (!deviceSecure()) return Begin.NoScreenLock
        val ks = keystore ?: return Begin.Unreadable
        return try {
            if (wrapped.exists()) {
                val sealed = wrapped.readBytes()
                val key = ks.getKey(ALIAS, null) as? SecretKey ?: return Begin.Unreadable
                if (sealed.size <= IV_LEN) return Begin.Unreadable
                val cipher = Cipher.getInstance(TRANSFORMATION)
                cipher.init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(TAG_BITS, sealed, 0, IV_LEN))
                Begin.Ready(cipher, first = false)
            } else if (store.exists()) {
                // A store whose data key is gone can never be opened.
                Begin.Unreadable
            } else {
                // The first open: a Keystore key of its own, made now.
                if (ks.containsAlias(ALIAS)) ks.deleteEntry(ALIAS)
                val key = createKey() ?: return Begin.Unreadable
                val cipher = Cipher.getInstance(TRANSFORMATION)
                cipher.init(Cipher.ENCRYPT_MODE, key)
                Begin.Ready(cipher, first = true)
            }
        } catch (_: UserNotAuthenticatedException) {
            Begin.NotAuthenticated
        } catch (_: KeyPermanentlyInvalidatedException) {
            Begin.Unreadable
        } catch (_: Exception) {
            Begin.Unreadable
        }
    }

    /**
     * The second half, after the unlock authorised [ready]'s cipher: the
     * data key is unwrapped (or made and wrapped) and held, and the store's
     * bytes are returned, empty when nothing is kept. `null` when the store
     * cannot be read; nothing is held then.
     */
    fun finish(ready: Begin.Ready, cipher: Cipher = ready.cipher): ByteArray? {
        lock()
        return try {
            if (ready.first) {
                val key = ByteArray(DATA_KEY_LEN).also { SecureRandom().nextBytes(it) }
                val sealed = cipher.doFinal(key)
                if (!replace(wrapped, wrappedTemp, cipher.iv + sealed)) {
                    key.fill(0)
                    return null
                }
                dataKey = key
                ByteArray(0)
            } else {
                val sealed = wrapped.readBytes()
                val key = cipher.doFinal(sealed, IV_LEN, sealed.size - IV_LEN)
                if (!store.exists()) {
                    dataKey = key
                    return ByteArray(0)
                }
                val bytes = open(key, store.readBytes())
                if (bytes == null) key.fill(0) else dataKey = key
                bytes
            }
        } catch (_: Exception) {
            null
        }
    }

    /**
     * Keeps `bytes` as the store, with its count, under the data key held
     * since the unlock. Empty bytes delete the store, its wrapped key, the
     * Keystore key and the counts. Nothing is written while the store is
     * locked.
     */
    fun keep(bytes: ByteArray, descriptions: Int): Boolean {
        if (bytes.isEmpty()) {
            delete()
            return true
        }
        if (bytes.size <= VERSION_AT) return false
        val key = dataKey ?: return false
        return try {
            val iv = ByteArray(IV_LEN).also { SecureRandom().nextBytes(it) }
            val cipher = Cipher.getInstance(TRANSFORMATION)
            cipher.init(Cipher.ENCRYPT_MODE, SecretKeySpec(key, "AES"), GCMParameterSpec(TAG_BITS, iv))
            val sealed = cipher.doFinal(bytes)
            replace(store, storeTemp, iv + sealed) &&
                replace(countsFile, countsTemp, "v${bytes[VERSION_AT]} $descriptions".toByteArray())
        } catch (_: Exception) {
            false
        }
    }

    /** The store is locked: the data key in memory is wiped. */
    fun lock() {
        dataKey?.fill(0)
        dataKey = null
    }

    /** Everything kept here goes: the files and the Keystore key. */
    private fun delete() {
        lock()
        for (f in listOf(store, storeTemp, wrapped, wrappedTemp, countsFile, countsTemp)) {
            try {
                f.delete()
            } catch (_: Exception) {
            }
        }
        try {
            keystore?.deleteEntry(ALIAS)
        } catch (_: Exception) {
        }
    }

    /** Decrypts the store's file under the data key; `null` when it does not open. */
    private fun open(key: ByteArray, sealed: ByteArray): ByteArray? = try {
        if (sealed.size <= IV_LEN) {
            null
        } else {
            val cipher = Cipher.getInstance(TRANSFORMATION)
            cipher.init(Cipher.DECRYPT_MODE, SecretKeySpec(key, "AES"), GCMParameterSpec(TAG_BITS, sealed, 0, IV_LEN))
            cipher.doFinal(sealed, IV_LEN, sealed.size - IV_LEN)
        }
    } catch (_: Exception) {
        null
    }

    /** Writes the temporary file and renames it over `file`. */
    private fun replace(file: File, temp: File, bytes: ByteArray): Boolean = try {
        temp.writeBytes(bytes)
        temp.renameTo(file) || run { temp.delete(); false }
    } catch (_: Exception) {
        temp.delete()
        false
    }

    private fun createKey(): SecretKey? = try {
        val spec = KeyGenParameterSpec.Builder(
            ALIAS,
            KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
        )
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .setKeySize(KEY_BITS)
            .setUserAuthenticationRequired(true)
            // A fingerprint added later is the same person's phone; it
            // must not make the store unreadable.
            .setInvalidatedByBiometricEnrollment(false)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            spec.setUserAuthenticationParameters(
                0,
                KeyProperties.AUTH_BIOMETRIC_STRONG or KeyProperties.AUTH_DEVICE_CREDENTIAL,
            )
        } else {
            @Suppress("DEPRECATION")
            spec.setUserAuthenticationValidityDurationSeconds(VALID_SECONDS)
        }
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, PROVIDER)
        generator.init(spec.build())
        generator.generateKey()
    } catch (_: Exception) {
        null
    }

    private companion object {
        const val STORE_NAME = "kept"
        const val KEY_NAME = "kept.key"
        const val COUNTS_NAME = "kept.counts"

        /** Where the core's store format keeps its version byte, after the 4-byte magic. */
        const val VERSION_AT = 4
        const val PROVIDER = "AndroidKeyStore"
        const val ALIAS = "companion.kept"
        const val TRANSFORMATION = "AES/GCM/NoPadding"
        const val KEY_BITS = 256
        const val DATA_KEY_LEN = 32
        const val IV_LEN = 12
        const val TAG_BITS = 128

        /** Android 8 to 10: how long the key works after the confirm-credential screen. */
        const val VALID_SECONDS = 30
    }
}
