package com.zsclip.qqime;

import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageInfo;
import android.os.SystemClock;
import android.util.Base64;
import org.json.JSONObject;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.security.KeyFactory;
import java.security.MessageDigest;
import java.security.SecureRandom;
import java.security.interfaces.RSAPublicKey;
import java.security.spec.MGF1ParameterSpec;
import java.security.spec.X509EncodedKeySpec;
import java.util.Arrays;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import javax.crypto.Cipher;
import javax.crypto.spec.GCMParameterSpec;
import javax.crypto.spec.OAEPParameterSpec;
import javax.crypto.spec.PSource;
import javax.crypto.spec.SecretKeySpec;

/** One-shot, explicitly confirmed account authorization for a paired desktop. */
final class QqCloudAuthorization {
    static final int MAX_BYTES = 32768;
    static final String FORMAT = "zsclip-qq-cloud-auth-v1";
    private static final long WINDOW_MS = 300000L;
    private static final String LOGIN_ACTIVITY = "com.tencent.qqpinyin.account.activity.UserCenterLoginActivity";

    static final class Request {
        final String requestId;
        final String fingerprint;
        final RSAPublicKey publicKey;
        final long expiresAtMs;
        private final long elapsedDeadline;

        private Request(String requestId, String fingerprint, RSAPublicKey publicKey, long expiresAtMs, long remaining) {
            this.requestId = requestId;
            this.fingerprint = fingerprint;
            this.publicKey = publicKey;
            this.expiresAtMs = expiresAtMs;
            this.elapsedDeadline = SystemClock.elapsedRealtime() + remaining;
        }

        String displayFingerprint() {
            return displayConfirmationCode(fingerprint);
        }

        void checkFresh() throws IOException {
            if (System.currentTimeMillis() >= expiresAtMs || SystemClock.elapsedRealtime() >= elapsedDeadline) {
                throw new IOException("授权窗口已过期，请在电脑重新点击“连接 QQ 云剪贴板”");
            }
        }
    }

    static Request parseRequest(JSONObject data) throws Exception {
        try {
            if (!"zsclip-qq-cloud-request-v1".equals(data.getString("format"))) throw new IllegalArgumentException();
            String id = data.getString("request_id");
            String keyText = data.getString("public_key");
            String fingerprint = data.getString("fingerprint");
            Object expiry = data.get("expires_at_ms");
            if (!id.matches("[0-9a-f]{32}") || !fingerprint.matches("[0-9a-f]{12}")
                    || !(expiry instanceof Long || expiry instanceof Integer)
                    || keyText.length() > 1024 || !keyText.matches("[A-Za-z0-9+/]+={0,2}")) {
                throw new IllegalArgumentException();
            }
            byte[] der = Base64.decode(keyText, Base64.NO_WRAP);
            if (!Base64.encodeToString(der, Base64.NO_WRAP).equals(keyText)) throw new IllegalArgumentException();
            Object key = KeyFactory.getInstance("RSA").generatePublic(new X509EncodedKeySpec(der));
            if (!(key instanceof RSAPublicKey)) throw new IllegalArgumentException();
            RSAPublicKey rsa = (RSAPublicKey) key;
            if (rsa.getModulus().bitLength() != 2048 || !Arrays.equals(der, rsa.getEncoded())) {
                throw new IllegalArgumentException();
            }
            byte[] digest = MessageDigest.getInstance("SHA-256").digest(der);
            StringBuilder calculated = new StringBuilder(12);
            for (int i = 0; i < 6; i++) {
                calculated.append(Character.forDigit((digest[i] >>> 4) & 15, 16));
                calculated.append(Character.forDigit(digest[i] & 15, 16));
            }
            if (!calculated.toString().equals(fingerprint)) throw new IllegalArgumentException();
            long expiresAtMs = ((Number) expiry).longValue();
            long now = System.currentTimeMillis();
            if (expiresAtMs <= now) throw new IOException("授权窗口已过期，请在电脑重新点击“连接 QQ 云剪贴板”");
            long remaining = expiresAtMs - now;
            if (remaining <= 0 || remaining > WINDOW_MS) {
                throw new IOException("电脑授权有效期异常，请校准手机和电脑时间后重试");
            }
            return new Request(id, fingerprint, rsa, expiresAtMs, remaining);
        } catch (IOException known) {
            throw known;
        } catch (Exception invalid) {
            throw new IOException("电脑授权信息无效，请在电脑重新发起连接");
        }
    }

    /** Called only after the user has accepted the matching fingerprint dialog. */
    static JSONObject sealAccount(Context context, Request request, String lanDeviceId) throws Exception {
        request.checkFresh();
        byte[] plaintext = null;
        byte[] aesKey = new byte[32];
        try {
            Object manager = Class.forName("com.tencent.qqpinyin.data.y").getMethod("a").invoke(null);
            Object user = manager.getClass().getMethod("d").invoke(manager);
            if (user == null) throw new LoginRequiredException();
            Object value = user.getClass().getMethod("getSgid").invoke(user);
            if (!(value instanceof String) || ((String) value).isEmpty()) throw new LoginRequiredException();
            String sgid = (String) value;
            if (sgid.length() > 4096 || !isAsciiGraphic(sgid)) throw new IOException("QQ 账号授权信息不可用，请重新登录 QQ 账号");
            Class<?> factory = Class.forName("com.tencent.qqpinyin.util.af$a");
            Object utility = factory.getMethod("a", Context.class).invoke(null, context);
            Object originalDeviceId = utility.getClass().getMethod("a").invoke(utility);
            if (!(originalDeviceId instanceof String) || ((String) originalDeviceId).isEmpty()
                    || ((String) originalDeviceId).length() > 256 || !isAsciiGraphic((String) originalDeviceId)) {
                throw new IOException("输入法设备标识不可用，请重启 QQ 输入法后重试");
            }
            JSONObject account = new JSONObject().put("sgid", sgid).put("device_id", originalDeviceId)
                    .put("version", originalVersion(context));
            plaintext = account.toString().getBytes(StandardCharsets.UTF_8);
            account.remove("sgid");
            if (plaintext.length > 16368) throw new IOException("QQ 账号授权信息过长");
            SecureRandom random = new SecureRandom();
            random.nextBytes(aesKey);
            byte[] nonce = new byte[12];
            random.nextBytes(nonce);
            Cipher aes = Cipher.getInstance("AES/GCM/NoPadding");
            aes.init(Cipher.ENCRYPT_MODE, new SecretKeySpec(aesKey, "AES"), new GCMParameterSpec(128, nonce));
            aes.updateAAD((FORMAT + ":" + request.requestId + ":" + lanDeviceId).getBytes(StandardCharsets.UTF_8));
            byte[] ciphertext = aes.doFinal(plaintext);
            Cipher rsa = Cipher.getInstance("RSA/ECB/OAEPWithSHA-256AndMGF1Padding");
            rsa.init(Cipher.ENCRYPT_MODE, request.publicKey,
                    new OAEPParameterSpec("SHA-256", "MGF1", MGF1ParameterSpec.SHA256, PSource.PSpecified.DEFAULT));
            byte[] wrappedKey = rsa.doFinal(aesKey);
            request.checkFresh();
            return new JSONObject().put("format", FORMAT).put("request_id", request.requestId)
                    .put("wrapped_key", Base64.encodeToString(wrappedKey, Base64.NO_WRAP))
                    .put("nonce", Base64.encodeToString(nonce, Base64.NO_WRAP))
                    .put("ciphertext", Base64.encodeToString(ciphertext, Base64.NO_WRAP));
        } catch (IOException known) {
            throw known;
        } catch (Exception failure) {
            // Reflection/provider errors must never expose account values in UI or logs.
            throw new IOException("无法安全授权 QQ 云剪贴板，请重新登录 QQ 输入法账号后重试");
        } finally {
            Arrays.fill(aesKey, (byte) 0);
            if (plaintext != null) Arrays.fill(plaintext, (byte) 0);
        }
    }

    /** Binds desktop confirmation to the encrypted account sent by this phone. */
    static String confirmationCode(JSONObject envelope) throws IOException {
        try {
            MessageDigest hash = MessageDigest.getInstance("SHA-256");
            String[] fields = {"wrapped_key", "nonce", "ciphertext"};
            for (int i = 0; i < fields.length; i++) {
                String encoded = envelope.getString(fields[i]);
                byte[] value = Base64.decode(encoded, Base64.NO_WRAP);
                if (!Base64.encodeToString(value, Base64.NO_WRAP).equals(encoded)
                        || (i == 0 && value.length != 256) || (i == 1 && value.length != 12)
                        || (i == 2 && (value.length < 16 || value.length > 16384))) {
                    throw new IllegalArgumentException();
                }
                hash.update(value);
            }
            byte[] digest = hash.digest();
            StringBuilder result = new StringBuilder(12);
            for (int i = 0; i < 6; i++) {
                result.append(Character.forDigit((digest[i] >>> 4) & 15, 16));
                result.append(Character.forDigit(digest[i] & 15, 16));
            }
            return result.toString();
        } catch (Exception invalid) {
            throw new IOException("无法校验本机授权内容，请重新发起连接");
        }
    }

    static void verifyReceipt(JSONObject response, String localCode) throws IOException {
        if (response == null || !Boolean.TRUE.equals(response.opt("ok"))) {
            throw new IOException("电脑未接受授权内容，请重新发起连接");
        }
        if (!localCode.matches("[0-9a-f]{12}") || !localCode.equals(response.opt("confirmation_code"))) {
            throw new IOException("两端授权确认码不一致，授权未确认");
        }
    }

    static String displayConfirmationCode(String value) {
        return (value.substring(0, 4) + " " + value.substring(4, 8) + " " + value.substring(8, 12))
                .toUpperCase(java.util.Locale.ROOT);
    }

    static Intent loginIntent(Context context) {
        try {
            Intent intent = new Intent().setClassName(context.getPackageName(), LOGIN_ACTIVITY);
            return context.getPackageManager().resolveActivity(intent, 0) == null ? null : intent;
        } catch (RuntimeException unavailable) { return null; }
    }

    private static String originalVersion(Context context) {
        try {
            PackageInfo info = context.getPackageManager().getPackageInfo(context.getPackageName(), 0);
            String name = info.versionName == null ? "" : info.versionName;
            Matcher version = Pattern.compile("^([0-9]+\\.[0-9]+\\.[0-9]+)(?:[-+.]zsclip.*)?$").matcher(name);
            if (version.matches() && info.versionCode > 0) {
                String result = version.group(1) + "." + info.versionCode;
                if (result.length() <= 64) return result;
            }
        } catch (Exception ignored) { }
        return "8.7.15.6291";
    }

    private static boolean isAsciiGraphic(String value) {
        for (int i = 0; i < value.length(); i++) if (value.charAt(i) < 0x21 || value.charAt(i) > 0x7e) return false;
        return true;
    }

    static final class LoginRequiredException extends IOException {
        LoginRequiredException() { super("请先在 QQ 输入法账号页完成登录，登录后重新授权电脑"); }
    }

    private QqCloudAuthorization() { }
}
