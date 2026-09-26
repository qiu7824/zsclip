package com.zsclip.qqime;

import android.content.Context;
import android.content.res.AssetManager;
import android.util.DisplayMetrics;
import android.util.Log;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.lang.reflect.Method;

/** Completes the built-in skin's icon cache without replacing custom skin assets. */
public final class WhiteSkinResources {
    private static final String TAG = "ZSClipSkin";

    private WhiteSkinResources() {}

    public static synchronized void ensure(Context context) {
        if (context == null || !isDefaultSkin()) return;
        try {
            DisplayMetrics metrics = context.getResources().getDisplayMetrics();
            String density = Math.min(metrics.widthPixels, metrics.heightPixels) > 720
                    ? "xxdpi" : "xdpi";
            File skin = new File(context.getApplicationInfo().dataDir, "skin");
            // A missing directory means the host has not initialized its skin yet.
            if (!skin.isDirectory()) return;
            AssetManager assets = context.getAssets();
            copyMissing(assets, "skin_configer/qq_res/" + density, skin, true);
            File board = new File(skin, "board");
            copyMissing(assets, "skin_configer/board/" + density, board, false);
            copyMissing(assets, "skin_configer/board/common", board, false);
        } catch (IOException | RuntimeException error) {
            Log.w(TAG, "Unable to initialize keyboard icons: "
                    + error.getClass().getSimpleName());
        }
    }

    private static boolean isDefaultSkin() {
        try {
            Class<?> managerClass = Class.forName("com.tencent.qqpinyin.settings.p");
            Method instance = managerClass.getMethod("b");
            Object manager = instance.invoke(null);
            Object selected = managerClass.getMethod("O").invoke(manager);
            return selected instanceof Number && ((Number) selected).longValue() == 1L;
        } catch (ReflectiveOperationException | LinkageError | RuntimeException error) {
            return false;
        }
    }

    private static void copyMissing(AssetManager assets, String assetDirectory,
                                    File directory, boolean pressedVariants) throws IOException {
        if (!directory.isDirectory() && !directory.mkdirs()) {
            throw new IOException("Unable to create icon directory");
        }
        String[] names = assets.list(assetDirectory);
        if (names == null) return;
        for (String name : names) {
            if (!name.endsWith(".png") || name.indexOf('/') >= 0 || name.indexOf('\\') >= 0) {
                continue;
            }
            String source = assetDirectory + "/" + name;
            copyMissingFile(assets, source, new File(directory, name));
            if (pressedVariants) {
                String pressed = name.substring(0, name.length() - 4) + "_press.png";
                copyMissingFile(assets, source, new File(directory, pressed));
            }
        }
    }

    private static void copyMissingFile(AssetManager assets, String asset, File destination)
            throws IOException {
        if (destination.isFile() && destination.length() > 0) return;
        File staged = File.createTempFile("zsclip-icon-", ".tmp", destination.getParentFile());
        try {
            try (InputStream input = assets.open(asset);
                 FileOutputStream output = new FileOutputStream(staged)) {
                byte[] buffer = new byte[8192];
                int length;
                while ((length = input.read(buffer)) != -1) {
                    output.write(buffer, 0, length);
                }
            }
            // A concurrent host initialization may already have supplied this icon.
            if (destination.isFile() && destination.length() > 0) return;
            if (!staged.renameTo(destination)) throw new IOException("Unable to commit icon");
        } finally {
            if (staged.exists()) staged.delete();
        }
    }
}
