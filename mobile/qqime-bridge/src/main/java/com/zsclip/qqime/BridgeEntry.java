package com.zsclip.qqime;

import android.app.Activity;
import android.content.Intent;
import android.inputmethodservice.InputMethodService;
import android.util.Log;
import android.util.TypedValue;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.widget.LinearLayout;
import android.widget.TextView;
import java.lang.ref.WeakReference;

/** Entry points for the QQ Pinyin 8.7.15 host. */
public final class BridgeEntry {
    private static final String TAG = "ZSClipBridge";
    private static final String BUTTON_TAG = "zsclip_connection_button";
    private static WeakReference<InputMethodService> activeService = new WeakReference<InputMethodService>(null);

    private BridgeEntry() {}

    public static void startIme(InputMethodService service) {
        activeService = new WeakReference<InputMethodService>(service);
        ensureSkin(service);
        try {
            LanBridge.start(service);
        } catch (RuntimeException | LinkageError error) {
            Log.w(TAG, "Unable to start clipboard connection: " + error.getClass().getSimpleName());
        }
    }

    public static void stopIme() {
        activeService.clear();
        try {
            LanBridge.stop();
        } catch (RuntimeException | LinkageError error) {
            Log.w(TAG, "Unable to stop clipboard connection: " + error.getClass().getSimpleName());
        }
    }

    public static void refreshIme() {
        InputMethodService service = activeService.get();
        if (service != null) ensureSkin(service);
        try {
            LanBridge.refresh();
        } catch (RuntimeException | LinkageError error) {
            Log.w(TAG, "Unable to refresh clipboard connection: " + error.getClass().getSimpleName());
        }
    }

    private static void ensureSkin(InputMethodService service) {
        try {
            WhiteSkinResources.ensure(service);
        } catch (RuntimeException | LinkageError error) {
            Log.w(TAG, "Unable to initialize keyboard icons: " + error.getClass().getSimpleName());
        }
    }

    public static void attachClipboard(final Activity activity) {
        try {
            int id = activity.getResources().getIdentifier(
                    "full_screen_checkbox", "id", activity.getPackageName());
            View selection = activity.findViewById(id);
            if (selection == null || !(selection.getParent() instanceof LinearLayout)) return;
            LinearLayout header = (LinearLayout) selection.getParent();
            if (header.findViewWithTag(BUTTON_TAG) != null) return;
            TextView button = new TextView(activity);
            button.setTag(BUTTON_TAG);
            button.setText("ZSClip");
            button.setTextSize(TypedValue.COMPLEX_UNIT_SP, 14);
            button.setTextColor(0xffffffff);
            button.setGravity(Gravity.CENTER);
            button.setContentDescription("连接 ZSClip 电脑剪贴板");
            button.setFocusable(true);
            TypedValue background = new TypedValue();
            if (activity.getTheme().resolveAttribute(
                    android.R.attr.selectableItemBackground, background, true)) {
                button.setBackgroundResource(background.resourceId);
            }
            button.setOnClickListener(new View.OnClickListener() {
                @Override public void onClick(View view) {
                    activity.startActivity(new Intent(activity, BridgeActivity.class));
                }
            });
            header.addView(button, header.indexOfChild(selection),
                    new LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.MATCH_PARENT, 1.1f));
        } catch (RuntimeException | LinkageError error) {
            Log.w(TAG, "Unable to show clipboard connection: " + error.getClass().getSimpleName());
        }
    }
}
