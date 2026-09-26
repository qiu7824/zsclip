package com.zsclip.qqime;

import android.content.Context;
import android.content.SharedPreferences;
import android.os.Build;
import android.security.keystore.KeyGenParameterSpec;
import android.security.keystore.KeyProperties;
import android.util.Base64;
import org.json.JSONObject;
import java.nio.charset.StandardCharsets;
import java.security.KeyStore;
import java.util.UUID;
import javax.crypto.Cipher;
import javax.crypto.KeyGenerator;
import javax.crypto.SecretKey;
import javax.crypto.spec.GCMParameterSpec;

/** Stores only encrypted LAN credentials; clipboard contents are never persisted. */
final class PairingVault {
    private static final String ALIAS = "zsclip.qqime.lan.aes.v1";
    final SharedPreferences prefs;
    final String deviceId;
    private final byte[] aad;

    PairingVault(Context context) {
        prefs = context.getSharedPreferences("zsclip_qqime_bridge", Context.MODE_PRIVATE);
        String id = prefs.getString("device_id", "");
        if (id == null || id.isEmpty()) {
            id = UUID.randomUUID().toString();
            prefs.edit().putString("device_id", id).apply();
        }
        deviceId = id;
        aad = (context.getPackageName() + ":" + deviceId).getBytes(StandardCharsets.UTF_8);
    }

    JSONObject read() throws Exception {
        requireSupported();
        String stored = prefs.getString("pairing_v1", "");
        if (stored == null || stored.isEmpty()) return null;
        JSONObject sealed = new JSONObject(stored);
        byte[] iv = Base64.decode(sealed.getString("iv"), Base64.NO_WRAP);
        if (iv.length != 12) throw new IllegalStateException("配对凭据已失效，请重新配对");
        Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
        cipher.init(Cipher.DECRYPT_MODE, key(false), new GCMParameterSpec(128, iv));
        cipher.updateAAD(aad);
        byte[] data = cipher.doFinal(Base64.decode(sealed.getString("data"), Base64.NO_WRAP));
        return new JSONObject(new String(data, StandardCharsets.UTF_8));
    }

    String seal(JSONObject pairing) throws Exception {
        requireSupported();
        Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
        cipher.init(Cipher.ENCRYPT_MODE, key(true));
        cipher.updateAAD(aad);
        byte[] data = cipher.doFinal(pairing.toString().getBytes(StandardCharsets.UTF_8));
        return new JSONObject()
                .put("iv", Base64.encodeToString(cipher.getIV(), Base64.NO_WRAP))
                .put("data", Base64.encodeToString(data, Base64.NO_WRAP)).toString();
    }

    void clear() {
        prefs.edit().remove("pairing_v1").remove("last_remote_key").remove("blocked_remote_key")
                .remove("received_remote_keys").remove("server_sync_mode")
                .putString("sync_mode", SyncMode.MANUAL).putBoolean("auto", false).apply();
    }

    private SecretKey key(boolean create) throws Exception {
        KeyStore store = KeyStore.getInstance("AndroidKeyStore");
        store.load(null);
        if (!store.containsAlias(ALIAS)) {
            if (!create) throw new IllegalStateException("配对密钥已失效，请重新配对");
            KeyGenerator generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore");
            generator.init(new KeyGenParameterSpec.Builder(ALIAS,
                    KeyProperties.PURPOSE_ENCRYPT | KeyProperties.PURPOSE_DECRYPT)
                    .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                    .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                    .setRandomizedEncryptionRequired(true).build());
            return generator.generateKey();
        }
        return (SecretKey) store.getKey(ALIAS, null);
    }

    static void requireSupported() {
        if (Build.VERSION.SDK_INT < 23) {
            throw new IllegalStateException("ZSClip 同步需要 Android 6.0 或更高版本");
        }
    }
}
