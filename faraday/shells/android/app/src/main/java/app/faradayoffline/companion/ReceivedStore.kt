package app.faradayoffline.companion

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import java.io.File
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * The received files' store (docs/COMPANION.md §7, Received): the bytes the
 * core hands over in `Core.CMD_KEEP_RECEIVED`, encrypted with AES-256-GCM
 * under an Android Keystore key of the Companion's own, in the app's
 * private files.
 *
 * The Keystore handling is upstream's `KeptSecret.kt` wrapping key, copied
 * from `opensigner/shells/android` at d26b84e: the IV beside the
 * ciphertext, a temporary file renamed into place. What is not copied is
 * everything that keeps a signing key: the MAC key, the authentication
 * prompt, attestation and the blob. This key needs nothing from the
 * person, as §7 says Received's does; it never leaves the phone's keystore,
 * so the file is useless on any other device.
 *
 * Nothing here is logged.
 */
internal class ReceivedStore(context: Context) {

    /** The store: the GCM IV, then the ciphertext. */
    private val file = File(context.filesDir, STORE_NAME)

    /** Written first and renamed over [file], so an interrupted write
     *  leaves the previous store rather than half of the new one. */
    private val temp = File(context.filesDir, "$STORE_NAME.tmp")

    private val keystore: KeyStore? = try {
        KeyStore.getInstance(PROVIDER).apply { load(null) }
    } catch (_: Exception) {
        null
    }

    /**
     * What is kept, for the core at start:
     *  - `null` when nothing is kept, and the core starts empty;
     *  - the store's bytes, as the core handed them over;
     *  - an empty array when a store is there and cannot be opened (its key
     *    is gone, or the file is damaged). The core reads that as a store it
     *    cannot read, says so on Received, and asks for nothing to be
     *    written over it until the person deletes it.
     */
    fun load(): ByteArray? {
        val exists = try {
            file.exists()
        } catch (_: Exception) {
            false
        }
        if (!exists) return null
        val key = existingKey() ?: return ByteArray(0)
        return try {
            val bytes = file.readBytes()
            if (bytes.size <= IV_LEN) return ByteArray(0)
            val cipher = Cipher.getInstance(TRANSFORMATION)
            cipher.init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(TAG_BITS, bytes, 0, IV_LEN))
            cipher.doFinal(bytes, IV_LEN, bytes.size - IV_LEN)
        } catch (_: Exception) {
            ByteArray(0)
        }
    }

    /** Encrypts `bytes` and keeps them in place of what was kept. */
    fun store(bytes: ByteArray): Boolean {
        val key = existingKey() ?: createKey() ?: return false
        return try {
            val cipher = Cipher.getInstance(TRANSFORMATION)
            cipher.init(Cipher.ENCRYPT_MODE, key)
            val sealed = cipher.doFinal(bytes)
            temp.writeBytes(cipher.iv + sealed)
            temp.renameTo(file) || run { temp.delete(); false }
        } catch (_: Exception) {
            temp.delete()
            false
        }
    }

    private fun existingKey(): SecretKey? = try {
        keystore?.getKey(ALIAS, null) as? SecretKey
    } catch (_: Exception) {
        null
    }

    private fun createKey(): SecretKey? = try {
        val spec = KeyGenParameterSpec.Builder(
            ALIAS,
            KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
        )
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .setKeySize(KEY_BITS)
            .build()
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, PROVIDER)
        generator.init(spec)
        generator.generateKey()
    } catch (_: Exception) {
        null
    }

    private companion object {
        /** The store, in `filesDir`. */
        const val STORE_NAME = "received"
        const val PROVIDER = "AndroidKeyStore"
        const val ALIAS = "companion.received"
        const val TRANSFORMATION = "AES/GCM/NoPadding"
        const val KEY_BITS = 256

        /** GCM's own IV length, which the platform generates and we store. */
        const val IV_LEN = 12
        const val TAG_BITS = 128
    }
}
