package com.zsclip.qqime;

import android.content.ContentProviderOperation;
import android.content.ContentResolver;
import android.content.ContentValues;
import android.content.Context;
import android.content.OperationApplicationException;
import android.content.pm.ProviderInfo;
import android.database.Cursor;
import android.net.Uri;
import android.util.Log;

import java.text.SimpleDateFormat;
import java.util.ArrayList;
import java.util.Date;
import java.util.Locale;

/** Imports explicitly received text into the host's local clipboard history. */
public final class QqClipboardHistory {
    private static final String AUTHORITY = "com.tencent.qqinput.clipboardprovider";
    private static final Uri CLIPS = Uri.parse("content://" + AUTHORITY + "/clipboard");
    private static final String MATCH = "content = ? AND type = ?";

    private QqClipboardHistory() {}

    /** Call before publishing the text to the system clipboard. */
    public static synchronized boolean ingest(Context context, String text) {
        if (context == null || text == null || text.trim().isEmpty()) return false;
        try {
            ProviderInfo provider = context.getPackageManager().resolveContentProvider(AUTHORITY, 0);
            if (provider == null || !context.getPackageName().equals(provider.packageName)) {
                return false;
            }
            ContentResolver resolver = context.getContentResolver();
            String[] match = {text, "local"};
            if (contains(resolver, match)) return true;

            ContentValues values = new ContentValues();
            values.put("content", text);
            values.put("serverId", "");
            values.put("upload", 0);
            values.put("type", "local");
            values.put("time", Long.parseLong(
                    new SimpleDateFormat("yyyyMMddHHmmss", Locale.US).format(new Date())));
            values.put("stickTime", 0L);
            values.put("back_up", "");

            ArrayList<ContentProviderOperation> operations = new ArrayList<ContentProviderOperation>();
            operations.add(ContentProviderOperation.newAssertQuery(CLIPS)
                    .withSelection(MATCH, match).withExpectedCount(0).build());
            operations.add(ContentProviderOperation.newInsert(CLIPS).withValues(values).build());
            try {
                resolver.applyBatch(AUTHORITY, operations);
            } catch (OperationApplicationException concurrentInsert) {
                // The host may have collected the same text while this operation was queued.
                if (!contains(resolver, match)) throw concurrentInsert;
            }
            resolver.notifyChange(CLIPS, null);
            return true;
        } catch (Exception | LinkageError failure) {
            Log.w("ZSClipHistory", "Unable to save received clipboard text: "
                    + failure.getClass().getSimpleName());
            return false;
        }
    }

    private static boolean contains(ContentResolver resolver, String[] match) {
        try (Cursor cursor = resolver.query(CLIPS, new String[]{"id"}, MATCH, match, null)) {
            if (cursor == null) throw new IllegalStateException("Clipboard provider unavailable");
            return cursor.moveToFirst();
        }
    }
}
