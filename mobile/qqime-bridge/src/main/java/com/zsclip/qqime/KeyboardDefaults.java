package com.zsclip.qqime;

import android.content.Context;
import android.content.SharedPreferences;
import android.util.Log;

/** Applies the bundled keyboard defaults once, including updates from earlier bridge packages. */
public final class KeyboardDefaults {
    private KeyboardDefaults() {}

    public static synchronized void apply(Object settings) {
        try {
            Class<?> application = Class.forName("com.tencent.qqpinyin.QQPYInputMethodApplication");
            Context context = (Context) application.getMethod("getApplictionContext").invoke(null);
            if (context == null || settings == null) return;
            SharedPreferences preferences = context.getSharedPreferences("zsclip_keyboard_defaults", Context.MODE_PRIVATE);
            if (preferences.getInt("revision", 0) >= 104) return;
            Class<?> type = settings.getClass();
            // Native toolbar nibble encoding: settings, keyboard switch, clipboard, input editor.
            type.getMethod("c", long.class).invoke(settings, 0x4f21L);
            type.getMethod("d", long.class).invoke(settings, 0x0100L);
            type.getMethod("J", int.class).invoke(settings, 0); // Native settings glyph, not account avatar.
            type.getMethod("S", int.class).invoke(settings, 3); // Native always-shift key, retains pinyin composition.
            if (!Boolean.TRUE.equals(type.getMethod("a", int.class).invoke(settings, 1))) return;
            preferences.edit().putInt("revision", 104).apply();
        } catch (ReflectiveOperationException | RuntimeException | LinkageError unavailable) {
            Log.w("ZSClipKeyboard", "Unable to initialize keyboard defaults: " + unavailable.getClass().getSimpleName());
        }
    }
}
