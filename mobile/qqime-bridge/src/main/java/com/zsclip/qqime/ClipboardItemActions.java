package com.zsclip.qqime;

import android.app.Dialog;
import android.content.Context;
import android.util.TypedValue;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.widget.LinearLayout;
import android.widget.FrameLayout;
import android.widget.TextView;
import android.widget.Toast;
import java.lang.ref.WeakReference;
import java.lang.reflect.Field;

/** Adds one action to the host's existing text-item menus without copying its content. */
public final class ClipboardItemActions {
    private static final String TAG = "zsclip_send_selected_item";

    public static void attach(final Object controller) {
        try {
            boolean local = "com.tencent.qqpinyin.clipboard.l".equals(controller.getClass().getName());
            boolean cloud = "com.tencent.qqpinyin.clipboard.j".equals(controller.getClass().getName());
            if (!local && !cloud) return;
            Object menu = field(controller, local ? "o" : "r");
            View root = (View) field(menu, "a");
            int id = root.getResources().getIdentifier(local ? "inner_layout" : "ll_container", "id", root.getContext().getPackageName());
            View container = root.findViewById(id);
            if (!(container instanceof LinearLayout) || root.findViewWithTag(TAG) != null) return;
            LinearLayout row = (LinearLayout) container;
            TextView reference = firstText(row);
            if (reference == null) return;
            final WeakReference<Object> owner = new WeakReference<Object>(controller);
            final String selectedField = local ? "r" : "u";
            TextView button = action(row.getContext(), reference);
            button.setOnClickListener(new View.OnClickListener() { public void onClick(View view) {
                Object current = owner.get();
                if (current == null) return;
                try { send(view.getContext(), field(current, selectedField)); }
                catch (Exception unavailable) { unavailable(view.getContext()); }
            }});
            int existing = local ? 3 : 2;
            ViewGroup.LayoutParams bounds = row.getLayoutParams();
            float weight = 1.25f;
            if (bounds.width > 0) {
                float unit = (float) bounds.width / existing;
                weight = Math.max(1f, (reference.getPaint().measureText("发送到电脑") + reference.getTextSize()) / unit);
                bounds.width = Math.round(unit * (existing + weight)); row.setLayoutParams(bounds);
            }
            // QQLinearLayout rescales direct TextView children from cached raw pixels.
            // A plain native slot keeps that pass from scaling the final font a second time.
            FrameLayout slot = new FrameLayout(row.getContext());
            slot.addView(button, new FrameLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT));
            row.addView(slot, new LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.MATCH_PARENT, weight));
            ClipboardVisualStyle.watchMenu(row);
        } catch (Exception ignored) { /* Host menus keep working if their layout has changed. */ }
    }

    public static void attachDetail(final Dialog dialog) {
        try {
            ViewGroup root = (ViewGroup) dialog.findViewById(android.R.id.content);
            if (root == null || root.findViewWithTag(TAG) != null) return;
            View child = root.getChildCount() == 1 ? root.getChildAt(0) : root;
            if (!(child instanceof LinearLayout)) return;
            LinearLayout content = (LinearLayout) child;
            if (content.getOrientation() != LinearLayout.VERTICAL) return;
            int timeId = root.getResources().getIdentifier("dialog_time", "id", dialog.getContext().getPackageName());
            View nativeLabel = root.findViewById(timeId);
            TextView button = action(dialog.getContext(), nativeLabel instanceof TextView ? (TextView) nativeLabel : null);
            if (nativeLabel == null) button.setTextSize(13);
            button.setTextColor(0xff30343b);
            int padding = Math.round(12 * dialog.getContext().getResources().getDisplayMetrics().density);
            button.setPadding(padding, padding, padding, padding); button.setFocusable(true);
            final WeakReference<Dialog> owner = new WeakReference<Dialog>(dialog);
            button.setOnClickListener(new View.OnClickListener() { public void onClick(View view) {
                Dialog current = owner.get();
                if (current == null) return;
                try { send(view.getContext(), field(current, "j")); }
                catch (Exception unavailable) { unavailable(view.getContext()); }
            }});
            content.addView(button, new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));
            ClipboardVisualStyle.watchDetail(content);
        } catch (Exception ignored) { }
    }

    private static TextView action(Context context, TextView reference) {
        TextView button = new MatchingAction(context, reference);
        button.setTag(TAG); button.setText("发送到电脑"); button.setGravity(Gravity.CENTER);
        button.setSingleLine(true);
        if (reference != null) {
            button.setTextSize(TypedValue.COMPLEX_UNIT_PX, reference.getTextSize());
            button.setTextColor(reference.getTextColors()); button.setTypeface(reference.getTypeface());
        }
        button.setFocusable(true); button.setContentDescription("将此条目发送到已配对电脑");
        return button;
    }
    private static final class MatchingAction extends TextView {
        private final WeakReference<TextView> reference;
        MatchingAction(Context context, TextView reference) { super(context); this.reference = new WeakReference<TextView>(reference); }
        @Override protected void onMeasure(int width, int height) {
            TextView nativeText = reference.get();
            if (nativeText != null) {
                if (getTextSize() != nativeText.getTextSize()) setTextSize(TypedValue.COMPLEX_UNIT_PX, nativeText.getTextSize());
                if (getTypeface() != nativeText.getTypeface()) setTypeface(nativeText.getTypeface());
                if (getTextScaleX() != nativeText.getTextScaleX()) setTextScaleX(nativeText.getTextScaleX());
                if (getLetterSpacing() != nativeText.getLetterSpacing()) setLetterSpacing(nativeText.getLetterSpacing());
                if (getIncludeFontPadding() != nativeText.getIncludeFontPadding()) setIncludeFontPadding(nativeText.getIncludeFontPadding());
            }
            super.onMeasure(width, height);
        }
    }
    private static TextView firstText(ViewGroup parent) {
        for (int i = 0; i < parent.getChildCount(); i++) if (parent.getChildAt(i) instanceof TextView) return (TextView) parent.getChildAt(i);
        return null;
    }
    private static void send(Context context, Object clip) throws Exception {
        if (clip == null || !"com.tencent.qqpinyin.clipboard.e".equals(clip.getClass().getName())) throw new IllegalArgumentException();
        Object value = field(clip, "b");
        if (!(value instanceof String)) throw new IllegalArgumentException();
        LanBridge.sendItem(context, BridgeProtocol.checkedText((String) value));
    }
    private static Object field(Object object, String name) throws Exception {
        Field field = object.getClass().getDeclaredField(name); field.setAccessible(true); return field.get(object);
    }
    private static void unavailable(Context context) { Toast.makeText(context, "此条目暂时无法发送，请重新长按后重试", Toast.LENGTH_LONG).show(); }
    private ClipboardItemActions() { }
}
