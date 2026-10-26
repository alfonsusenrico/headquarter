package com.alfonsusenrico.headquarter

import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import java.io.File
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * The phone's iroh key on disk, encrypted with an AES-GCM key the Android Keystore holds
 * (add-android-app D2). The file alone is useless off this phone.
 */
class PhoneKey(
    private val file: File,
) {
    /** The phone's key, or null when the file is missing or can no longer be read. */
    fun read(): ByteArray? {
        if (!file.exists()) return null
        return try {
            val stored = file.readBytes()
            val iv = stored.copyOfRange(0, IV_BYTES)
            val cipher = Cipher.getInstance(TRANSFORMATION)
            cipher.init(Cipher.DECRYPT_MODE, wrappingKey(), GCMParameterSpec(TAG_BITS, iv))
            cipher.doFinal(stored, IV_BYTES, stored.size - IV_BYTES)
        } catch (error: java.security.GeneralSecurityException) {
            // The Keystore key was lost, such as after the phone's lock settings were
            // reset: the phone has to pair again with a new key.
            android.util.Log.w(TAG, "the phone's key can no longer be read: ${error.javaClass.simpleName}")
            null
        }
    }

    fun write(key: ByteArray) {
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, wrappingKey())
        val sealed = cipher.iv + cipher.doFinal(key)
        val partial = File(file.parentFile, "${file.name}.partial")
        partial.writeBytes(sealed)
        if (!partial.renameTo(file)) error("cannot save the phone's key")
    }

    private fun wrappingKey(): SecretKey {
        val store = KeyStore.getInstance(KEYSTORE).apply { load(null) }
        (store.getKey(ALIAS, null) as? SecretKey)?.let { return it }
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, KEYSTORE)
        generator.init(
            KeyGenParameterSpec
                .Builder(ALIAS, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .build(),
        )
        return generator.generateKey()
    }

    private companion object {
        const val TAG = "Headquarter"
        const val KEYSTORE = "AndroidKeyStore"
        const val ALIAS = "phone-key-wrap"
        const val TRANSFORMATION = "AES/GCM/NoPadding"
        const val IV_BYTES = 12
        const val TAG_BITS = 128
    }
}
