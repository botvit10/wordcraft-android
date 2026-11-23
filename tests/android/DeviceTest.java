package ai.storyteller.wordcraft.test;

import android.app.Activity;
import android.app.Instrumentation;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.ContentValues;
import android.content.Intent;
import android.content.IntentFilter;
import android.graphics.Bitmap;
import android.net.Uri;
import android.os.Bundle;
import android.os.SystemClock;
import android.provider.MediaStore;
import android.view.KeyEvent;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputConnection;
import android.view.inputmethod.InputContentInfo;
import android.content.ClipDescription;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.OutputStream;
import java.nio.file.Files;
import java.util.Arrays;
import java.util.concurrent.atomic.AtomicReference;

public final class DeviceTest extends Instrumentation {
    @Override public void onCreate(Bundle args) { super.onCreate(args); start(); }

    @Override public void onStart() {
        Bundle results = new Bundle();
        try {
            Activity app = startActivitySync(new Intent(Intent.ACTION_MAIN)
                .setClassName("ai.storyteller.wordcraft", "ai.storyteller.wordcraft.WordCraftActivity")
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK));
            long deadline = SystemClock.elapsedRealtime() + 15000;
            while (!app.hasWindowFocus() && SystemClock.elapsedRealtime() < deadline) SystemClock.sleep(100);
            check(app.hasWindowFocus(), "editor window has focus");
            EditorInfo info = new EditorInfo();
            AtomicReference<InputConnection> connection = new AtomicReference<>();
            runOnMainSync(() -> connection.set(app.getCurrentFocus().onCreateInputConnection(info)));
            check(connection.get() != null && Arrays.asList(info.contentMimeTypes).contains("image/*"), "IME accepts pictures");

            // Commit the chosen historical entry while another value is the primary system clip.
            SystemClock.sleep(1500);
            String chosen = "ação — item anterior";
            ClipboardManager clipboard = app.getSystemService(ClipboardManager.class);
            runOnMainSync(() -> {
                clipboard.setPrimaryClip(ClipData.newPlainText("test", "outro item"));
                connection.get().commitText(chosen, 1);
            });
            deadline = SystemClock.elapsedRealtime() + 15000;
            boolean copied = false;
            while (!copied && SystemClock.elapsedRealtime() < deadline) {
                SystemClock.sleep(600);
                chord(KeyEvent.KEYCODE_A);
                chord(KeyEvent.KEYCODE_C);
                SystemClock.sleep(300);
                AtomicReference<ClipData> clip = new AtomicReference<>();
                runOnMainSync(() -> clip.set(clipboard.getPrimaryClip()));
                copied = clip.get() != null && chosen.contentEquals(clip.get().getItemAt(0).coerceToText(app));
            }
            check(copied, "selected keyboard history text reached the native document");
            results.putString("keyboard", "input connection commit → native editor → Android clipboard");

            checkImage(app, Bitmap.CompressFormat.PNG, "image/png", ".png");
            checkImage(app, Bitmap.CompressFormat.JPEG, "image/jpeg", ".jpg");
            results.putString("picker", "PNG/JPEG content URI results and cancellation");
            finish(Activity.RESULT_OK, results);
        } catch (Throwable e) {
            results.putString("error", android.util.Log.getStackTraceString(e));
            finish(Activity.RESULT_CANCELED, results);
        }
    }

    private void chord(int key) {
        long t = SystemClock.uptimeMillis();
        sendKeySync(new KeyEvent(t, t, KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_CTRL_LEFT, 0));
        sendKeySync(new KeyEvent(t, t, KeyEvent.ACTION_DOWN, key, 0, KeyEvent.META_CTRL_ON));
        sendKeySync(new KeyEvent(t, t, KeyEvent.ACTION_UP, key, 0, KeyEvent.META_CTRL_ON));
        sendKeySync(new KeyEvent(t, t, KeyEvent.ACTION_UP, KeyEvent.KEYCODE_CTRL_LEFT, 0));
    }

    private void checkImage(Activity app, Bitmap.CompressFormat format, String mime, String suffix) throws Exception {
        ContentValues values = new ContentValues();
        values.put(MediaStore.Images.Media.DISPLAY_NAME, "wordcraft-test" + suffix);
        values.put(MediaStore.Images.Media.MIME_TYPE, mime);
        Uri uri = app.getContentResolver().insert(MediaStore.Images.Media.EXTERNAL_CONTENT_URI, values);
        check(uri != null, "test URI");
        try {
            ByteArrayOutputStream bytes = new ByteArrayOutputStream();
            Bitmap.createBitmap(4, 3, Bitmap.Config.ARGB_8888).compress(format, 90, bytes);
            try (OutputStream out = app.getContentResolver().openOutputStream(uri)) { out.write(bytes.toByteArray()); }
            String path = pickerResult(app, Activity.RESULT_OK, new Intent().setData(uri));
            File image = new File(path);
            try { check(Arrays.equals(bytes.toByteArray(), Files.readAllBytes(image.toPath())), "URI import " + mime); }
            finally { image.delete(); }
            check(pickerResult(app, Activity.RESULT_CANCELED, null).isEmpty(), "cancel");
        } finally {
            app.getContentResolver().delete(uri, null, null);
        }
    }

    private String pickerResult(Activity app, int result, Intent data) throws Exception {
        IntentFilter filter = new IntentFilter(Intent.ACTION_OPEN_DOCUMENT);
        filter.addCategory(Intent.CATEGORY_OPENABLE);
        filter.addDataType("image/*");
        ActivityMonitor monitor = addMonitor(filter, new ActivityResult(result, data), true);
        try {
            app.getClass().getMethod("pickPicture").invoke(app);
            long deadline = SystemClock.elapsedRealtime() + 10000;
            while (SystemClock.elapsedRealtime() < deadline) {
                String path = (String) app.getClass().getMethod("takePictureResult").invoke(app);
                if (path != null) {
                    check(monitor.getHits() == 1 && !path.startsWith("!"), "image picker request/result");
                    return path;
                }
                SystemClock.sleep(50);
            }
            throw new AssertionError("No result from picker");
        } finally { removeMonitor(monitor); }
    }

    private static void check(boolean value, String message) {
        if (!value) throw new AssertionError(message);
    }
}
