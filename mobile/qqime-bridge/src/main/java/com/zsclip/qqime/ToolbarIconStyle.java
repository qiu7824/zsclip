package com.zsclip.qqime;

import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.graphics.ColorFilter;
import android.graphics.PixelFormat;
import android.graphics.Rect;
import android.graphics.RectF;
import android.graphics.drawable.BitmapDrawable;
import android.graphics.drawable.Drawable;
import android.view.View;
import android.view.ViewGroup;
import android.view.ViewTreeObserver;
import android.widget.ImageView;
import java.lang.ref.WeakReference;
import java.lang.reflect.Field;
import java.util.WeakHashMap;

/** Matches the actual neighbouring toolbar icon, including transparent asset margins. */
public final class ToolbarIconStyle {
    private static final WeakHashMap<View, Boolean> WATCHED = new WeakHashMap<View, Boolean>();
    private static final WeakHashMap<Bitmap, Rect> INK = new WeakHashMap<Bitmap, Rect>();
    public static void normalize(View item) {
        try {
            if (!isGear(item)) return;
            if (!WATCHED.containsKey(item)) {
                WATCHED.put(item, Boolean.TRUE);
                final WeakReference<View> target = new WeakReference<View>(item);
                item.getViewTreeObserver().addOnPreDrawListener(new ViewTreeObserver.OnPreDrawListener() {
                    public boolean onPreDraw() {
                        View current = target.get();
                        if (current != null) align(current);
                        return true;
                    }
                });
            }
            align(item);
        } catch (Exception ignored) { }
    }

    private static boolean isGear(View item) throws Exception {
        if (((Number) item.getClass().getMethod("getItemId").invoke(item)).intValue() != 1) return false;
        Object data = item.getClass().getMethod("getViewData").invoke(item);
        String[] assets = (String[]) field(data, "i");
        return assets != null && assets.length == 1 && "more_setting_board_icon.png".equals(assets[0]);
    }

    private static void align(View item) {
        try {
            if (!isGear(item)) return;
            View scope = item;
            // The settings key and scrolling keys have different immediate parents.
            for (int i = 0; i < 5 && scope.getParent() instanceof View; i++) {
                scope = (View) scope.getParent();
                if ("com.tencent.qqpinyin.client.ToolbarViewNew".equals(scope.getClass().getName())) break;
            }
            ImageView reference = peer(scope, item);
            ImageView image = (ImageView) field(item, "a");
            if (reference == null || image == null) return;
            ViewGroup.LayoutParams from = reference.getLayoutParams(), bounds = image.getLayoutParams();
            if (from == null || bounds == null) return;
            if (bounds.width != from.width || bounds.height != from.height) {
                bounds.width = from.width; bounds.height = from.height; image.setLayoutParams(bounds);
            }
            if (image.getPaddingLeft() != reference.getPaddingLeft() || image.getPaddingTop() != reference.getPaddingTop()
                    || image.getPaddingRight() != reference.getPaddingRight() || image.getPaddingBottom() != reference.getPaddingBottom()) {
                image.setPadding(reference.getPaddingLeft(), reference.getPaddingTop(), reference.getPaddingRight(), reference.getPaddingBottom());
            }
            if (image.getScaleType() != reference.getScaleType()) image.setScaleType(reference.getScaleType());
            if (reference.getScaleType() == ImageView.ScaleType.MATRIX) image.setImageMatrix(reference.getImageMatrix());
            Drawable original = image.getDrawable(), neighbour = reference.getDrawable();
            if (original == null || neighbour == null) return;
            RectF ink = ink(neighbour);
            if (ink == null) return;
            if (original instanceof IconCanvas) {
                IconCanvas canvas = (IconCanvas) original;
                if (canvas.matches(neighbour, ink)) return;
                original = canvas.source;
            }
            RectF ownInk = ink(original);
            if (ownInk == null) return;
            image.setImageDrawable(new IconCanvas(original, neighbour, ownInk, ink));
        } catch (Exception ignored) { }
    }

    private static ImageView peer(View root, View target) throws Exception {
        if (root.getVisibility() != View.VISIBLE) return null;
        if (root != target && root.getClass() == target.getClass()) {
            int id = ((Number) root.getClass().getMethod("getItemId").invoke(root)).intValue();
            if (id > 1) {
                ImageView image = (ImageView) field(root, "a");
                if (image != null && image.getVisibility() == View.VISIBLE && ink(image.getDrawable()) != null) return image;
            }
        }
        if (root instanceof ViewGroup) {
            ViewGroup group = (ViewGroup) root;
            for (int i = 0; i < group.getChildCount(); i++) {
                ImageView result = peer(group.getChildAt(i), target);
                if (result != null) return result;
            }
        }
        return null;
    }

    private static RectF ink(Drawable drawable) {
        if (drawable == null) return null;
        Drawable current = drawable.getCurrent();
        if (!(current instanceof BitmapDrawable)) return null;
        Bitmap bitmap = ((BitmapDrawable) current).getBitmap();
        if (bitmap == null || bitmap.isRecycled() || bitmap.getWidth() * bitmap.getHeight() > 1048576) return null;
        Rect content = INK.get(bitmap);
        if (content == null) {
            int w = bitmap.getWidth(), h = bitmap.getHeight();
            int left = w, top = h, right = 0, bottom = 0;
            int[] row = new int[w];
            for (int y = 0; y < h; y++) {
                bitmap.getPixels(row, 0, w, 0, y, w, 1);
                for (int x = 0; x < w; x++) if ((row[x] >>> 24) >= 16) {
                    left = Math.min(left, x); top = Math.min(top, y);
                    right = Math.max(right, x + 1); bottom = Math.max(bottom, y + 1);
                }
            }
            content = new Rect(left, top, right, bottom); INK.put(bitmap, content);
        }
        if (content.isEmpty() || drawable.getIntrinsicWidth() <= 0 || drawable.getIntrinsicHeight() <= 0) return null;
        float sx = (float) drawable.getIntrinsicWidth() / bitmap.getWidth();
        float sy = (float) drawable.getIntrinsicHeight() / bitmap.getHeight();
        return new RectF(content.left * sx, content.top * sy, content.right * sx, content.bottom * sy);
    }

    /** A native drawable canvas preserves state/tint and normalizes only the gear's visible ink. */
    private static final class IconCanvas extends Drawable {
        final Drawable source;
        final int width, height;
        final RectF sourceInk, targetInk;
        IconCanvas(Drawable source, Drawable peer, RectF sourceInk, RectF targetInk) {
            this.source = source; width = peer.getIntrinsicWidth(); height = peer.getIntrinsicHeight();
            this.sourceInk = sourceInk; this.targetInk = targetInk;
            setState(source.getState());
        }
        boolean matches(Drawable peer, RectF ink) {
            return width == peer.getIntrinsicWidth() && height == peer.getIntrinsicHeight() && targetInk.equals(ink);
        }
        public void draw(Canvas canvas) {
            Rect b = getBounds();
            float scale = Math.min(targetInk.width() / sourceInk.width(), targetInk.height() / sourceInk.height());
            int checkpoint = canvas.save();
            canvas.translate(b.left, b.top); canvas.scale((float) b.width() / width, (float) b.height() / height);
            canvas.translate(targetInk.centerX() - sourceInk.centerX() * scale, targetInk.centerY() - sourceInk.centerY() * scale);
            canvas.scale(scale, scale);
            source.setBounds(0, 0, source.getIntrinsicWidth(), source.getIntrinsicHeight()); source.draw(canvas);
            canvas.restoreToCount(checkpoint);
        }
        public int getIntrinsicWidth() { return width; }
        public int getIntrinsicHeight() { return height; }
        public boolean isStateful() { return source.isStateful(); }
        protected boolean onStateChange(int[] state) { boolean changed = source.setState(state); if (changed) invalidateSelf(); return changed; }
        public void setAlpha(int alpha) { source.setAlpha(alpha); }
        public void setColorFilter(ColorFilter filter) { source.setColorFilter(filter); }
        public int getOpacity() { return PixelFormat.TRANSLUCENT; }
    }
    private static Object field(Object owner, String name) throws Exception {
        Field field = owner.getClass().getDeclaredField(name); field.setAccessible(true); return field.get(owner);
    }
    private ToolbarIconStyle() { }
}
