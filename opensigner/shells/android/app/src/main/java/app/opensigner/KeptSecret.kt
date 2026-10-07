package app.opensigner

import android.content.Context
import android.os.Build
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyInfo
import android.security.keystore.KeyPermanentlyInvalidatedException
import android.security.keystore.KeyProperties
import android.security.keystore.StrongBoxUnavailableException
import java.io.File
import java.security.KeyPairGenerator
import java.security.KeyStore
import java.security.SecureRandom
import java.security.cert.X509Certificate
import java.security.spec.ECGenParameterSpec
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.Mac
import javax.crypto.SecretKey
import javax.crypto.SecretKeyFactory
import javax.crypto.spec.GCMParameterSpec

/**
 * The two Android Keystore keys behind a key kept on this device
 * (`docs/PLANNING.md` §6, §15 item 32), and the one file they protect.
 *
 * The core encrypts everything secret before it ever reaches the shell, so
 * what this class holds is an opaque blob. What it adds is the half of the
 * threat model the core cannot provide itself:
 *
 *  - **The MAC key** answers `Core.CMD_SECURE_MAC`. It is HMAC-SHA256,
 *    it never leaves the secure element, and it requires the person's
 *    authentication for *every* use. The core derives what it sends
 *    from the PIN being tried, so a tag answers for that PIN and no
 *    other: a copy of the blob cannot be attacked offline, and each
 *    guess costs one authentication on this one chip.
 *  - **The wrapping key** encrypts the blob at rest with AES-256-GCM. It
 *    needs no authentication of its own — the MAC key already gates every
 *    real use — but it too never leaves the element, so the bytes in
 *    `filesDir` are useless on any other device.
 *
 * Both are created together on first use and deleted together by
 * [forget], which is what makes a stolen copy of the file permanently
 * worthless.
 *
 * Everything here needs Android 11 (API 30): `setUnlockedDeviceRequired`,
 * per-use authentication that accepts either a strong biometric or the
 * device credential, and a `BiometricPrompt` that can offer both. Below
 * that, and on a device with no secure lock screen — a key that requires
 * authentication cannot be generated without one — [secure] stays
 * [Core.SECURE_NONE] and the app runs as Tier C, exactly as it did before
 * this class existed.
 *
 * Nothing here is logged (§5.3).
 */
internal class KeptSecret(context: Context) {

    /** The kept blob: the GCM IV, then the ciphertext. */
    private val file = File(context.filesDir, KEPT_NAME)

    /** Written first and renamed over [file], so an interrupted write
     *  leaves the previous blob rather than half of the new one. */
    private val temp = File(context.filesDir, "$KEPT_NAME.tmp")

    /**
     * What backs the two keys: one of [Core.SECURE_STRONGBOX],
     * [Core.SECURE_TEE] and [Core.SECURE_NONE]. This is what the core is
     * told at `Core.new`, and so whether a key may be kept at all.
     */
    var secure = Core.SECURE_NONE
        private set

    /**
     * The device's verified-boot state, from the same attestation
     * extension the security level comes from: one of the `BOOT_*`
     * constants. Read with [deviceLocked], which is the half that says
     * whether the state means anything: see [MainActivity.bootState].
     */
    var verifiedBoot = BOOT_UNKNOWN
        private set

    /**
     * Whether the bootloader is locked, from the same root of trust.
     * A locked bootloader is what makes a verified-boot state a promise:
     * with it unlocked the system can be replaced, and the attestation
     * says so. False until an attestation is read.
     */
    var deviceLocked = false
        private set

    private val keystore: KeyStore? = try {
        KeyStore.getInstance(PROVIDER).apply { load(null) }
    } catch (_: Exception) {
        null
    }

    init {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) create()
    }

    /** Whether a blob is on the device, which the core asks once at start. */
    fun kept(): Boolean = try {
        file.exists()
    } catch (_: Exception) {
        false
    }

    /**
     * A `Mac` initialised under the MAC key, for a `BiometricPrompt` to
     * authorise, or `null` when there is no usable key. The tag comes out
     * of [finishMac] once the person has authenticated; calling
     * `doFinal` without that throws, which is the whole point of the key.
     */
    fun beginMac(): Mac? {
        val key = secretKey(MAC_ALIAS) ?: return null
        return try {
            Mac.getInstance(MAC_ALGORITHM).apply { init(key) }
        } catch (_: KeyPermanentlyInvalidatedException) {
            // The one exception that says the key is gone for good: a
            // changed screen lock or a new enrolment invalidated it, so
            // nothing can ever be unlocked with it again and the blob it
            // protected goes too.
            forget()
            null
        } catch (_: Exception) {
            // Anything else is this attempt failing, not the key dying.
            // The core gets `SecureUnavailable` and the person can try
            // again; a transient Keystore error must not delete the key.
            null
        }
    }

    /** The 32-byte tag for `salt`, or `null` if the operation failed. */
    fun finishMac(mac: Mac, salt: ByteArray): ByteArray? = try {
        mac.doFinal(salt).takeIf { it.size == Core.MAC_LEN }
    } catch (_: Exception) {
        null
    }

    /** Encrypts `blob` under the wrapping key and keeps it. */
    fun store(blob: ByteArray): Boolean {
        val key = secretKey(WRAP_ALIAS) ?: return false
        return try {
            val cipher = Cipher.getInstance(WRAP_TRANSFORMATION)
            cipher.init(Cipher.ENCRYPT_MODE, key)
            val sealed = cipher.doFinal(blob)
            val iv = cipher.iv
            temp.writeBytes(iv + sealed)
            temp.renameTo(file) || run { temp.delete(); false }
        } catch (_: Exception) {
            temp.delete()
            false
        }
    }

    /** The blob as it was stored, or `null` when none comes back. */
    fun load(): ByteArray? {
        val key = secretKey(WRAP_ALIAS) ?: return null
        return try {
            val bytes = file.readBytes()
            if (bytes.size <= IV_LEN) return null
            val cipher = Cipher.getInstance(WRAP_TRANSFORMATION)
            cipher.init(
                Cipher.DECRYPT_MODE,
                key,
                GCMParameterSpec(GCM_TAG_BITS, bytes, 0, IV_LEN),
            )
            cipher.doFinal(bytes, IV_LEN, bytes.size - IV_LEN)
        } catch (_: Exception) {
            null
        }
    }

    /**
     * Deletes the blob and both keys. After this no copy of the bytes can
     * ever be tried again, on this device or any other.
     */
    fun forget() {
        try {
            file.delete()
            temp.delete()
        } catch (_: Exception) {
            // A file that will not delete is still a file no key opens.
        }
        delete(MAC_ALIAS)
        delete(WRAP_ALIAS)
    }

    /**
     * Both keys exist afterwards, or [secure] stays [Core.SECURE_NONE].
     *
     * StrongBox is asked for first and the TEE is the fallback, as the
     * platform requires: a device without a separate chip answers
     * `StrongBoxUnavailableException` rather than quietly doing the
     * lesser thing.
     */
    private fun create() {
        val store = keystore ?: return
        try {
            val fresh = !store.containsAlias(MAC_ALIAS) || !store.containsAlias(WRAP_ALIAS)
            if (fresh) {
                // Half a pair is no pair: a previous run that made one key
                // and died leaves nothing usable behind.
                forget()
                if (!generate(strongBox = true)) {
                    delete(MAC_ALIAS)
                    delete(WRAP_ALIAS)
                    if (!generate(strongBox = false)) return
                }
            }
            secure = level()
        } catch (_: Exception) {
            // No secure lock screen, no keystore, or a platform that
            // refuses one of the properties above. There is then no key to
            // keep on this device, which is what SECURE_NONE says.
            secure = Core.SECURE_NONE
        }
    }

    /** Both keys, or neither. `false` when StrongBox was asked for and refused. */
    private fun generate(strongBox: Boolean): Boolean = try {
        generateMac(strongBox)
        generateWrap(strongBox)
        true
    } catch (_: StrongBoxUnavailableException) {
        false
    }

    private fun generateMac(strongBox: Boolean) {
        val spec = KeyGenParameterSpec.Builder(MAC_ALIAS, KeyProperties.PURPOSE_SIGN)
            // Every use, with no validity window: one authentication buys
            // one tag, so one PIN guess against the blob costs one
            // authentication on this chip.
            .setUserAuthenticationRequired(true)
            // A newly enrolled fingerprint is a new person as far as this
            // key is concerned, and invalidates it.
            .setInvalidatedByBiometricEnrollment(true)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            spec.setUserAuthenticationParameters(
                0,
                KeyProperties.AUTH_BIOMETRIC_STRONG or KeyProperties.AUTH_DEVICE_CREDENTIAL,
            )
            // A locked device is a device whose kept key cannot be used,
            // however the request reaches the element.
            spec.setUnlockedDeviceRequired(true)
        }
        if (strongBox) spec.setIsStrongBoxBacked(true)
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_HMAC_SHA256, PROVIDER)
        generator.init(spec.build())
        generator.generateKey()
    }

    private fun generateWrap(strongBox: Boolean) {
        val spec = KeyGenParameterSpec.Builder(
            WRAP_ALIAS,
            KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
        )
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .setKeySize(WRAP_KEY_BITS)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            spec.setUnlockedDeviceRequired(true)
        }
        if (strongBox) spec.setIsStrongBoxBacked(true)
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, PROVIDER)
        generator.init(spec.build())
        generator.generateKey()
    }

    private fun secretKey(alias: String): SecretKey? {
        if (!ensure()) return null
        return try {
            keystore?.getKey(alias, null) as? SecretKey
        } catch (_: Exception) {
            null
        }
    }

    /**
     * Both keys are there afterwards, or the answer is `false`.
     *
     * [forget] deletes them, so the next key a person keeps is kept under
     * new ones. A blob that outlived its keys is a blob nothing will ever
     * open, and goes with them rather than waiting to fail.
     */
    private fun ensure(): Boolean {
        val store = keystore ?: return false
        if (secure == Core.SECURE_NONE) return false
        return try {
            if (store.containsAlias(MAC_ALIAS) && store.containsAlias(WRAP_ALIAS)) return true
            delete(MAC_ALIAS)
            delete(WRAP_ALIAS)
            file.delete()
            generate(strongBox = secure == Core.SECURE_STRONGBOX)
        } catch (_: Exception) {
            false
        }
    }

    private fun delete(alias: String) {
        try {
            keystore?.deleteEntry(alias)
        } catch (_: Exception) {
            // Nothing to delete, or a keystore that is already gone.
        }
    }

    /**
     * What the element the keys live in reports about itself.
     *
     * Attestation is the statement the chip signs, so it is asked first,
     * and it also carries the verified-boot state. A symmetric key has no
     * certificate chain to carry an attestation, so the challenge goes on
     * a throwaway EC key generated the same way and deleted at once; what
     * it attests to is the keystore both it and the two real keys were
     * made in. Where attestation is unavailable, [KeyInfo] answers the
     * narrower question — which element this particular key is in — and
     * verified boot goes unread.
     */
    private fun level(): Int {
        attested()?.let { return it }
        return keyInfoLevel()
    }

    /** The keymaster security level from a fresh attestation, or `null`. */
    private fun attested(): Int? {
        val store = keystore ?: return null
        return try {
            val challenge = ByteArray(CHALLENGE_LEN).also { SecureRandom().nextBytes(it) }
            if (!generateAttestation(challenge, strongBox = true)) {
                generateAttestation(challenge, strongBox = false)
            }
            val leaf = store.getCertificateChain(ATTEST_ALIAS)?.firstOrNull() as? X509Certificate
            leaf?.getExtensionValue(ATTESTATION_OID)?.let(::readAttestation)
        } catch (_: Exception) {
            null
        } finally {
            delete(ATTEST_ALIAS)
        }
    }

    private fun generateAttestation(challenge: ByteArray, strongBox: Boolean): Boolean = try {
        val spec = KeyGenParameterSpec.Builder(ATTEST_ALIAS, KeyProperties.PURPOSE_SIGN)
            .setAlgorithmParameterSpec(ECGenParameterSpec(ATTEST_CURVE))
            .setDigests(KeyProperties.DIGEST_SHA256)
            .setAttestationChallenge(challenge)
        if (strongBox) spec.setIsStrongBoxBacked(true)
        val generator = KeyPairGenerator.getInstance(KeyProperties.KEY_ALGORITHM_EC, PROVIDER)
        generator.initialize(spec.build())
        generator.generateKeyPair()
        true
    } catch (_: Exception) {
        // StrongBox refused, or this platform attests nothing at all.
        false
    }

    /**
     * The keymaster security level in an attestation extension, and, on
     * the way past it, the verified-boot state.
     *
     * `extension` is the extension's DER as `X509Certificate` hands it
     * over: an OCTET STRING wrapping the `KeyDescription` sequence of
     * the Android key attestation schema. Only the two fields that are
     * wanted are read; everything else is walked over. The parsing is by
     * hand because the shell has no dependencies, and it is short because
     * the structure is: a sequence of six scalars, then two authorisation
     * lists in which the root of trust is the tag numbered 704.
     */
    private fun readAttestation(extension: ByteArray): Int? {
        val wrapper = Der.read(extension, 0, extension.size) ?: return null
        val description = Der.read(extension, wrapper.start, wrapper.end) ?: return null
        var at = description.start
        var level: Int? = null
        // attestationVersion, attestationSecurityLevel, keymasterVersion,
        // keymasterSecurityLevel: the fourth is the one that says which
        // element made the key, rather than which one signed the statement.
        for (index in 0 until 4) {
            val field = Der.read(extension, at, description.end) ?: return null
            if (index == 3) level = Der.integer(extension, field)
            at = field.end
        }
        // attestationChallenge and uniqueId, then the two authorisation
        // lists: what the OS asked for, and what the element enforces. The
        // root of trust is in the second on a device that has one.
        for (index in 0 until 4) {
            val field = Der.read(extension, at, description.end) ?: break
            if (index >= 2) readRootOfTrust(extension, field)
            at = field.end
        }
        return when (level) {
            KEYMASTER_STRONGBOX -> Core.SECURE_STRONGBOX
            KEYMASTER_TEE -> Core.SECURE_TEE
            KEYMASTER_SOFTWARE -> Core.SECURE_NONE
            else -> null
        }
    }

    /**
     * Sets [verifiedBoot] and [deviceLocked] from the root of trust in
     * one authorisation list.
     */
    private fun readRootOfTrust(bytes: ByteArray, list: Der) {
        var at = list.start
        while (true) {
            val entry = Der.read(bytes, at, list.end) ?: return
            at = entry.end
            if (entry.tag != ROOT_OF_TRUST_TAG || !entry.contextual) continue
            val root = Der.read(bytes, entry.start, entry.end) ?: return
            // verifiedBootKey, deviceLocked, verifiedBootState.
            val key = Der.read(bytes, root.start, root.end) ?: return
            val locked = Der.read(bytes, key.end, root.end) ?: return
            val state = Der.read(bytes, locked.end, root.end) ?: return
            deviceLocked = Der.boolean(bytes, locked) == true
            verifiedBoot = when (Der.integer(bytes, state)) {
                0 -> BOOT_VERIFIED
                1 -> BOOT_SELF_SIGNED
                2 -> BOOT_UNVERIFIED
                3 -> BOOT_FAILED
                else -> BOOT_UNKNOWN
            }
            return
        }
    }

    /** What [KeyInfo] says about the MAC key, where attestation said nothing. */
    private fun keyInfoLevel(): Int {
        val key = try {
            keystore?.getKey(MAC_ALIAS, null) as? SecretKey
        } catch (_: Exception) {
            null
        } ?: return Core.SECURE_NONE
        return try {
            val factory = SecretKeyFactory.getInstance(key.algorithm, PROVIDER)
            val info = factory.getKeySpec(key, KeyInfo::class.java) as KeyInfo
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                when (info.securityLevel) {
                    KeyProperties.SECURITY_LEVEL_STRONGBOX -> Core.SECURE_STRONGBOX
                    KeyProperties.SECURITY_LEVEL_TRUSTED_ENVIRONMENT -> Core.SECURE_TEE
                    else -> Core.SECURE_NONE
                }
            } else {
                @Suppress("DEPRECATION")
                if (info.isInsideSecureHardware) Core.SECURE_TEE else Core.SECURE_NONE
            }
        } catch (_: Exception) {
            Core.SECURE_NONE
        }
    }

    /**
     * One DER tag-length-value: where its content is, and enough of its
     * identifier to tell a context-specific tag from a universal one.
     */
    private class Der(val tag: Long, val contextual: Boolean, val start: Int, val end: Int) {
        companion object {
            /** The value beginning at `at`, or `null` if it is malformed. */
            fun read(bytes: ByteArray, at: Int, limit: Int): Der? {
                var p = at
                if (p >= limit) return null
                val identifier = bytes[p++].toInt() and 0xff
                var tag = (identifier and 0x1f).toLong()
                if (tag == HIGH_TAG) {
                    tag = 0
                    while (true) {
                        if (p >= limit) return null
                        val part = bytes[p++].toInt() and 0xff
                        tag = (tag shl 7) or (part and 0x7f).toLong()
                        if (part and 0x80 == 0) break
                        if (tag > MAX_TAG) return null
                    }
                }
                if (p >= limit) return null
                var length = bytes[p++].toInt() and 0xff
                if (length and 0x80 != 0) {
                    val count = length and 0x7f
                    if (count == 0 || count > 4) return null
                    length = 0
                    repeat(count) {
                        if (p >= limit) return null
                        length = (length shl 8) or (bytes[p++].toInt() and 0xff)
                    }
                }
                if (length < 0 || length > limit - p) return null
                val contextual = identifier and 0xc0 == 0x80
                return Der(tag, contextual, p, p + length)
            }

            /**
             * The value of an INTEGER or ENUMERATED, or `null` when it is
             * neither, is negative, or is wider than the small numbers
             * this schema uses.
             */
            fun integer(bytes: ByteArray, value: Der): Int? {
                if (value.contextual || (value.tag != INTEGER && value.tag != ENUMERATED)) return null
                val length = value.end - value.start
                if (length !in 1..4) return null
                var number = 0
                for (i in value.start until value.end) {
                    number = (number shl 8) or (bytes[i].toInt() and 0xff)
                }
                return if (number < 0) null else number
            }

            /**
             * The value of a BOOLEAN, or `null` when it is not one. DER
             * writes one content byte: zero is false and anything else,
             * `0xff` in practice, is true.
             */
            fun boolean(bytes: ByteArray, value: Der): Boolean? {
                if (value.contextual || value.tag != BOOLEAN) return null
                if (value.end - value.start != 1) return null
                return bytes[value.start].toInt() != 0
            }

            const val HIGH_TAG = 0x1fL
            const val MAX_TAG = 0xffffL
            const val BOOLEAN = 1L
            const val INTEGER = 2L
            const val ENUMERATED = 10L
        }
    }

    companion object {
        /** The blob, in `filesDir` beside the settings. */
        const val KEPT_NAME = "kept"

        /** Verified boot state: not read, or a device that reports none. */
        const val BOOT_UNKNOWN = 0

        /** The running system is the one the manufacturer signed. */
        const val BOOT_VERIFIED = 1

        /** It is signed by a key the owner installed. */
        const val BOOT_SELF_SIGNED = 2

        /** The bootloader is unlocked and nothing is verified. */
        const val BOOT_UNVERIFIED = 3

        /** Verification ran and failed. */
        const val BOOT_FAILED = 4

        private const val PROVIDER = "AndroidKeyStore"
        private const val MAC_ALIAS = "opensigner.mac"
        private const val WRAP_ALIAS = "opensigner.wrap"
        private const val ATTEST_ALIAS = "opensigner.attest"
        private const val MAC_ALGORITHM = "HmacSHA256"
        private const val WRAP_TRANSFORMATION = "AES/GCM/NoPadding"
        private const val WRAP_KEY_BITS = 256
        private const val ATTEST_CURVE = "secp256r1"
        private const val CHALLENGE_LEN = 32

        /** GCM's own IV length, which the platform generates and we store. */
        private const val IV_LEN = 12
        private const val GCM_TAG_BITS = 128

        /** Android key attestation, as an extension of the leaf certificate. */
        private const val ATTESTATION_OID = "1.3.6.1.4.1.11129.2.1.17"

        /** `RootOfTrust` in an authorisation list. */
        private const val ROOT_OF_TRUST_TAG = 704L

        private const val KEYMASTER_SOFTWARE = 0
        private const val KEYMASTER_TEE = 1
        private const val KEYMASTER_STRONGBOX = 2
    }
}
