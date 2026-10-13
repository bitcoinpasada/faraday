package app.faradayoffline.companion

import android.content.ContentProvider
import android.content.ContentValues
import android.content.Context
import android.content.Intent
import android.database.Cursor
import android.database.MatrixCursor
import android.net.Uri
import android.os.ParcelFileDescriptor
import android.provider.OpenableColumns
import java.io.FileNotFoundException
import java.security.SecureRandom

/**
 * The one way a file leaves the Companion through another app
 * (docs/COMPANION.md §6.3, Share): a read-only content provider of the
 * app's own, the platform's `ContentProvider` and not AndroidX's
 * `FileProvider`.
 *
 * It serves at most one file, the one the person pressed Share for, from
 * memory: nothing is written to storage. Its address carries a random
 * token, so an address from an earlier share names nothing. The provider
 * is not exported; another app reads the file only through the read grant
 * the share's intent carries for that one address, and [withdraw] revokes
 * that grant and forgets the file. The activity withdraws when the person
 * comes back to the app after the share and when it is destroyed.
 *
 * Nothing here is logged.
 */
class ShareProvider : ContentProvider() {

    override fun onCreate(): Boolean = true

    override fun getType(uri: Uri): String? = offered(uri)?.mime

    /** The two columns a receiving app asks for: the name and the size. */
    override fun query(
        uri: Uri,
        projection: Array<out String>?,
        selection: String?,
        selectionArgs: Array<out String>?,
        sortOrder: String?,
    ): Cursor? {
        val offer = offered(uri) ?: return null
        val asked = projection ?: arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE)
        val columns = asked.filter { it == OpenableColumns.DISPLAY_NAME || it == OpenableColumns.SIZE }
        val row = columns.map { if (it == OpenableColumns.DISPLAY_NAME) offer.name else offer.bytes.size.toLong() }
        return MatrixCursor(columns.toTypedArray(), 1).apply { addRow(row) }
    }

    /**
     * The file, read only, through a pipe: the bytes go from memory to the
     * reader and nowhere else.
     */
    override fun openFile(uri: Uri, mode: String): ParcelFileDescriptor? {
        if (mode != "r") throw SecurityException("read only")
        val offer = offered(uri) ?: throw FileNotFoundException()
        val copy = offer.bytes.copyOf()
        val (read, write) = ParcelFileDescriptor.createPipe()
        Thread({
            try {
                ParcelFileDescriptor.AutoCloseOutputStream(write).use { it.write(copy) }
            } catch (_: Exception) {
                // The reader went away; nothing to say.
            } finally {
                copy.fill(0)
            }
        }, "share").start()
        return read
    }

    override fun insert(uri: Uri, values: ContentValues?): Uri? = throw UnsupportedOperationException()

    override fun update(uri: Uri, values: ContentValues?, selection: String?, selectionArgs: Array<out String>?): Int =
        throw UnsupportedOperationException()

    override fun delete(uri: Uri, selection: String?, selectionArgs: Array<out String>?): Int =
        throw UnsupportedOperationException()

    /** The file on offer, held in memory. */
    private class Offer(val uri: Uri, val name: String, val mime: String, val bytes: ByteArray)

    companion object {
        /** The provider's authority, as the manifest declares it. */
        const val AUTHORITY = "app.faradayoffline.companion.share"

        @Volatile
        private var offer: Offer? = null

        private fun offered(uri: Uri): Offer? = offer?.takeIf { it.uri == uri }

        /**
         * Puts one file on offer in place of any other and answers its
         * address. The caller grants the read on that address alone.
         */
        fun offer(context: Context, name: String, mime: String, bytes: ByteArray): Uri {
            withdraw(context)
            val token = ByteArray(TOKEN_LEN).also { SecureRandom().nextBytes(it) }
                .joinToString("") { "%02x".format(it) }
            val uri = Uri.Builder()
                .scheme("content")
                .authority(AUTHORITY)
                .appendPath(token)
                .appendPath(name)
                .build()
            offer = Offer(uri, name, mime, bytes.copyOf())
            return uri
        }

        /** The share has ended: its grant is revoked and its bytes wiped. */
        fun withdraw(context: Context) {
            val gone = offer ?: return
            offer = null
            try {
                context.revokeUriPermission(gone.uri, Intent.FLAG_GRANT_READ_URI_PERMISSION)
            } catch (_: Exception) {
            }
            gone.bytes.fill(0)
        }

        private const val TOKEN_LEN = 16
    }
}
