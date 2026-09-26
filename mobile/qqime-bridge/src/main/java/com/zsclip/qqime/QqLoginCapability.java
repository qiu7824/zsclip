package com.zsclip.qqime;

import android.content.Context;
import android.content.pm.PackageInfo;
import android.content.pm.PackageManager;
import android.content.pm.Signature;
import android.os.Build;
import java.security.MessageDigest;

/** Reports the installed certificate and local login state without reading account credentials. */
final class QqLoginCapability {
    private static final String ORIGINAL_CERTIFICATE_SHA256 =
            "40949385044d5e1624bb225782d209b3824c1e04b9d877f280c8b129f5c87c6f";
    static final String BLOCKED = "QQ 快捷授权可能因安装签名变化被拒绝；可打开输入法账号页尝试官方短信登录。局域网同步无需账号登录。";

    static boolean hasOriginalCertificate(Context context) {
        try {
            int flags = Build.VERSION.SDK_INT >= 28 ? PackageManager.GET_SIGNING_CERTIFICATES : PackageManager.GET_SIGNATURES;
            PackageInfo info = context.getPackageManager().getPackageInfo(context.getPackageName(), flags);
            Signature[] signatures = Build.VERSION.SDK_INT >= 28
                    ? (info.signingInfo == null ? null : info.signingInfo.getApkContentsSigners()) : info.signatures;
            if (signatures == null || signatures.length != 1) return false;
            return originalDigest(MessageDigest.getInstance("SHA-256").digest(signatures[0].toByteArray()));
        } catch (Exception unavailable) { return false; }
    }
    static boolean originalDigest(byte[] digest) {
        StringBuilder value = new StringBuilder();
        for (byte part : digest) value.append(String.format(java.util.Locale.ROOT, "%02x", part & 255));
        return ORIGINAL_CERTIFICATE_SHA256.equals(value.toString());
    }
    static boolean hasExistingLogin() {
        try {
            Object manager = Class.forName("com.tencent.qqpinyin.data.y").getMethod("a").invoke(null);
            // The native d() method returns null unless AccountMgr.isLogin() is true.
            // Do not call getSgid() or read any token while presenting capabilities.
            return manager.getClass().getMethod("d").invoke(manager) != null;
        } catch (Exception unavailable) { return false; }
    }
    static boolean canAuthorize(Context context) { return hasExistingLogin(); }
    static String notice(Context context) {
        if (hasOriginalCertificate(context)) return "QQ 云授权需要先登录输入法账号；局域网同步无需账号登录。";
        return hasExistingLogin()
                ? "已检测到输入法登录状态，可授权电脑；是否有效由官方服务确认。QQ 快捷登录可能受安装签名限制，官方短信入口仍可使用。"
                : BLOCKED;
    }
    static String loginRequiredMessage(Context context) {
        return "请先在输入法账号页完成登录；可尝试官方短信登录，QQ 快捷授权可能因安装签名变化被拒绝。";
    }
    private QqLoginCapability() { }
}
