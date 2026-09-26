package com.zsclip.qqime;

import android.app.Activity;
import android.app.Dialog;
import android.content.res.ColorStateList;
import android.graphics.PorterDuff;
import android.graphics.PorterDuffColorFilter;
import android.graphics.drawable.ColorDrawable;
import android.graphics.drawable.Drawable;
import android.graphics.drawable.GradientDrawable;
import android.graphics.drawable.StateListDrawable;
import android.view.View;
import android.view.ViewGroup;
import android.view.ViewTreeObserver;
import android.widget.ImageView;
import android.widget.TextView;
import java.lang.ref.WeakReference;
import java.lang.reflect.Field;
import java.util.WeakHashMap;

/** Clipboard-only surfaces; observed after host binding and before native drawing. */
public final class ClipboardVisualStyle {
    private static final int WHITE = 0xffffffff, TEXT = 0xff30343b, MUTED = 0xff70757d, BORDER = 0xffe3e5e8;
    private static final int PAGE = 0, MENU = 1, DETAIL = 2;
    private static final WeakHashMap<View, Boolean> WATCHED = new WeakHashMap<View, Boolean>();
    private static final WeakHashMap<View, StyledBackground> BACKGROUNDS = new WeakHashMap<View, StyledBackground>();
    private static final WeakHashMap<View, String> NAMES = new WeakHashMap<View, String>();
    private static final ColorStateList TEXT_COLORS = new ColorStateList(
            new int[][]{new int[]{-android.R.attr.state_enabled}, new int[0]}, new int[]{0xff9ba0a6, TEXT});
    private static final ColorStateList MUTED_COLORS = ColorStateList.valueOf(MUTED);
    private static final PorterDuffColorFilter ICON_TINT = new PorterDuffColorFilter(TEXT, PorterDuff.Mode.SRC_IN);

    public static void attachPanel(Object controller) {
        try {
            if (!"com.tencent.qqpinyin.clipboard.a".equals(controller.getClass().getName())) return;
            Class<?> type = controller.getClass();
            while (type != null) {
                try {
                    Field field = type.getDeclaredField("b"); field.setAccessible(true);
                    watch((View) field.get(controller), PAGE); return;
                } catch (NoSuchFieldException inherited) { type = type.getSuperclass(); }
            }
        } catch (Exception ignored) { }
    }
    public static void attachActivity(Activity activity) {
        if (activity == null || !activity.getClass().getName().startsWith("com.tencent.qqpinyin.clipboard.")) return;
        View root = activity.findViewById(android.R.id.content);
        if (root instanceof ViewGroup && ((ViewGroup) root).getChildCount() == 1) root = ((ViewGroup) root).getChildAt(0);
        watch(root, PAGE);
    }
    public static void attachDialog(Dialog dialog) {
        View root = dialog.findViewById(android.R.id.content);
        if (root instanceof ViewGroup && ((ViewGroup) root).getChildCount() == 1) root = ((ViewGroup) root).getChildAt(0);
        watch(root, DETAIL);
    }
    static void watchMenu(View root) { watch(root, MENU); }
    static void watchDetail(View root) { watch(root, DETAIL); }

    private static void watch(final View root, final int mode) {
        if (root == null) return;
        apply(root, root, mode);
        if (WATCHED.containsKey(root)) return;
        WATCHED.put(root, Boolean.TRUE);
        final WeakReference<View> weak = new WeakReference<View>(root);
        root.getViewTreeObserver().addOnPreDrawListener(new ViewTreeObserver.OnPreDrawListener() {
            public boolean onPreDraw() {
                View current = weak.get();
                if (current != null) apply(current, current, mode);
                return true;
            }
        });
    }

    private static void apply(View view, View root, int mode) {
        String name = name(view);
        // Login controls retain their own native appearance and authorization behavior.
        if (name.equals("full_screen_loginout") || name.equals("half_screen_clip_login")
                || name.equals("half_screen_clip_loginbtn") || name.equals("tv_clip_login_desc")) return;
        if (view == root || surface(name)) background(view, false, view == root && mode != PAGE);
        if (name.equals("fl_yan_photo_bg") || name.equals("full_screen_local_clip_content_layout")) background(view, true, true);
        if (name.equals("full_screen_local_clip_content_layout") || name.equals("full_screen_cloud_clip_index")
                || name.equals("full_screen_listview") || backButton(name)) {
            if (view.getParent() instanceof View) background((View) view.getParent(), name.equals("full_screen_cloud_clip_index"), false);
        }
        if (name.equals("inner_layout") || name.equals("ll_container") || name.equals("clip_loading")) background(view, false, true);
        if (name.equals("v_clip_top_line") || name.equals("v_clip_bottom_line")) solid(view, BORDER);
        if (name.equals("local_clip_line") || name.equals("cloud_clip_line") || name.equals("localclip_bar") || name.equals("cloudclip_bar") || name.equals("v_stick")) solid(view, TEXT);
        if (isAction(name) || mode == MENU && view instanceof TextView) background(view, true, false);
        if (view instanceof TextView) {
            TextView text = (TextView) view;
            ColorStateList colors = name.contains("time") || name.equals("text_clip_nodata") ? MUTED_COLORS : TEXT_COLORS;
            // The host changes bar visibility when switching tabs; keep that indication.
            String bar = name.equals("local_clip_text") ? "local_clip_line" : name.equals("cloud_clip_text") ? "cloud_clip_line"
                    : name.equals("localclip_text") ? "localclip_bar" : name.equals("cloudclip_text") ? "cloudclip_bar" : "";
            if (!bar.isEmpty()) {
                int id = view.getResources().getIdentifier(bar, "id", view.getContext().getPackageName());
                View indicator = root.findViewById(id);
                if (indicator != null && indicator.getVisibility() != View.VISIBLE) colors = MUTED_COLORS;
            }
            if (text.getTextColors() != colors) text.setTextColor(colors);
        }
        if (view instanceof ImageView && (name.equals("iv_clip_close") || name.equals("iv_clip_settings") || name.equals("iv_clip_delete")
                || parentNamed(view, "full_screen_back") || parentNamed(view, "select_local_clip_back") || parentNamed(view, "select_cloud_clip_back"))) {
            ImageView icon = (ImageView) view;
            if (icon.getColorFilter() != ICON_TINT) icon.setColorFilter(ICON_TINT);
            if (name.equals("iv_clip_delete")) background(view, true, true);
        }
        if (view instanceof ViewGroup) {
            ViewGroup group = (ViewGroup) view;
            for (int i = 0; i < group.getChildCount(); i++) apply(group.getChildAt(i), root, mode);
        }
    }
    private static boolean surface(String name) {
        return name.equals("ll_clip_top_panel") || name.equals("v_clip_board_bg") || name.equals("clip_viewPager")
                || name.equals("half_screen_local_clip_layout") || name.equals("half_screen_cloud_clip_layout")
                || name.equals("full_screen_viewpager") || name.equals("full_screen_listview") || name.equals("mlv_clip_layout")
                || name.equals("cloud_clip_dialog_layout") || name.equals("cloud_clip_compile_layout")
                || name.equals("select_local_clip_listview") || name.equals("select_cloud_clip_listview");
    }
    private static boolean isAction(String name) {
        return name.startsWith("local_clip_") && (name.endsWith("upload") || name.endsWith("stick") || name.endsWith("delete"))
                || name.equals("cloud_clip_compile") || name.equals("cloud_clip_delete") || name.equals("clip_open_btn")
                || name.equals("dialog_delete") || name.equals("dialog_upload") || name.equals("dialog_stick")
                || name.equals("dialog_compile") || name.equals("dialog_close") || name.equals("compile_cancel") || name.equals("compile_complete")
                || name.equals("select_local_clip_delete") || name.equals("select_local_clip_upload") || name.equals("select_cloud_clip_delete");
    }
    private static boolean backButton(String name) { return name.equals("full_screen_back") || name.equals("select_local_clip_back") || name.equals("select_cloud_clip_back"); }
    private static boolean parentNamed(View view, String name) { return view.getParent() instanceof View && name((View) view.getParent()).equals(name); }
    private static String name(View view) {
        String cached = NAMES.get(view);
        if (cached != null) return cached;
        String result = "";
        try { if (view.getId() > 0) result = view.getResources().getResourceEntryName(view.getId()); } catch (Exception ignored) { }
        NAMES.put(view, result); return result;
    }
    private static void solid(View view, int color) {
        Drawable existing = view.getBackground();
        if (!(existing instanceof ColorDrawable) || ((ColorDrawable) existing).getColor() != color) view.setBackgroundColor(color);
    }
    private static void background(View view, boolean interactive, boolean outline) {
        StyledBackground cached = BACKGROUNDS.get(view);
        if (cached != null && cached.drawable == view.getBackground() && cached.interactive == interactive && cached.outline == outline) return;
        float density = view.getResources().getDisplayMetrics().density;
        GradientDrawable normal = shape(WHITE, outline ? BORDER : WHITE, outline ? Math.max(1, Math.round(density)) : 0, density * (outline ? 4 : 0));
        Drawable drawable = normal;
        if (interactive) {
            StateListDrawable states = new StateListDrawable();
            // Active rows remain white; a darker outline makes selection unambiguous.
            states.addState(new int[]{android.R.attr.state_selected}, shape(WHITE, MUTED, Math.max(1, Math.round(2 * density)), 4 * density));
            states.addState(new int[]{android.R.attr.state_pressed}, shape(0xfff1f2f3, BORDER, Math.max(1, Math.round(density)), 4 * density));
            states.addState(new int[0], normal); drawable = states;
        }
        BACKGROUNDS.put(view, new StyledBackground(drawable, interactive, outline)); view.setBackground(drawable);
    }
    private static final class StyledBackground {
        final Drawable drawable;
        final boolean interactive, outline;
        StyledBackground(Drawable drawable, boolean interactive, boolean outline) { this.drawable = drawable; this.interactive = interactive; this.outline = outline; }
    }
    private static GradientDrawable shape(int fill, int border, int width, float radius) {
        GradientDrawable result = new GradientDrawable(); result.setColor(fill); result.setStroke(width, border); result.setCornerRadius(radius); return result;
    }
    private ClipboardVisualStyle() { }
}
