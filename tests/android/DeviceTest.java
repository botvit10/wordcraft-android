package ai.storyteller.wordcraft.test;

import android.app.Activity;
import android.app.Instrumentation;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.ContentValues;
import android.content.Intent;
import android.graphics.Bitmap;
import android.graphics.Rect;
import android.net.Uri;
import android.os.Bundle;
import android.os.SystemClock;
import android.provider.MediaStore;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.View;
import android.view.accessibility.AccessibilityNodeInfo;
import android.view.inputmethod.InputMethodManager;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.OutputStream;
import java.nio.file.Files;
import java.util.Arrays;
import java.util.concurrent.atomic.AtomicReference;

public final class DeviceTest extends Instrumentation {
    private String probe;

    @Override public void onCreate(Bundle args) {
        super.onCreate(args);
        probe = args.getString("probe", "all");
        start();
    }

    @Override public void onStart() {
        Bundle results = new Bundle();
        try {
            Activity app = startActivitySync(new Intent(Intent.ACTION_MAIN)
                .setClassName("ai.storyteller.wordcraft", "ai.storyteller.wordcraft.WordCraftActivity")
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK));
            long deadline = SystemClock.elapsedRealtime() + 15000;
            while (!app.hasWindowFocus() && SystemClock.elapsedRealtime() < deadline) SystemClock.sleep(100);
            check(app.hasWindowFocus(), "editor window has focus");
            if (!"keyboard".equals(probe)) {
                checkImage(app, Bitmap.CompressFormat.PNG, "image/png", ".png", false);
                checkImage(app, Bitmap.CompressFormat.JPEG, "image/jpeg", ".jpg", false);
                results.putString("picker", "PNG/JPEG found in Android Files, selected, imported byte-for-byte and canceled");
            }
            if (!"picker".equals(probe)) checkKeyboard(app, results);
            finish(Activity.RESULT_OK, results);
        } catch (Throwable e) {
            results.putString("error", android.util.Log.getStackTraceString(e));
            finish(Activity.RESULT_CANCELED, results);
        }
    }

    private void checkKeyboard(Activity app, Bundle results) throws Exception {
        InputMethodManager imm = app.getSystemService(InputMethodManager.class);
        // The editor opens the keyboard through NativeActivity. Check the actual UI inset.
        long deadline = SystemClock.elapsedRealtime() + 20000;
        while (height(app) < 1 && SystemClock.elapsedRealtime() < deadline) {
            View page = app.getWindow().getDecorView();
            tap(page.getWidth() * 0.5f, page.getHeight() * 0.55f);
            SystemClock.sleep(1000);
        }
        check(height(app) > 0 && height(app) < app.getWindow().getDecorView().getHeight(), "visible keyboard inset");
        results.putString("imeInsets", "observed keyboard overlap");

        // Commit the chosen historical entry while another value is the primary system clip.
        SystemClock.sleep(1500);
        String chosen = "ação — item anterior";
        ClipboardManager clipboard = app.getSystemService(ClipboardManager.class);
        runOnMainSync(() -> {
            clipboard.setPrimaryClip(ClipData.newPlainText("test", "outro item"));
            Bundle data = new Bundle();
            data.putString("text", chosen);
            imm.sendAppPrivateCommand(app.getCurrentFocus(), "ai.storyteller.wordcraft.test.TEXT", data);
        });
        awaitDocumentText(app, clipboard, chosen);
        results.putString("keyboard", "input connection commit → native editor → Android clipboard");

        checkImage(app, Bitmap.CompressFormat.PNG, "image/png", ".png", true);
        results.putString("keyboardImage", "image committed by the keyboard appeared in the document");
    }

    private int height(Activity app) throws Exception {
        return (Integer) app.getClass().getMethod("visibleEditorHeight").invoke(app);
    }

    private void awaitDocumentText(Activity app, ClipboardManager clipboard, String expected) {
        long deadline = SystemClock.elapsedRealtime() + 15000;
        boolean copied = false;
        while (!copied && SystemClock.elapsedRealtime() < deadline) {
            SystemClock.sleep(600);
            chord(KeyEvent.KEYCODE_A);
            chord(KeyEvent.KEYCODE_C);
            SystemClock.sleep(300);
            AtomicReference<ClipData> clip = new AtomicReference<>();
            runOnMainSync(() -> clip.set(clipboard.getPrimaryClip()));
            copied = clip.get() != null && expected.contentEquals(clip.get().getItemAt(0).coerceToText(app));
        }
        check(copied, "keyboard content reached the native document");
    }

    private void tap(float x, float y) {
        long t = SystemClock.uptimeMillis();
        for (int action : new int[] {MotionEvent.ACTION_DOWN, MotionEvent.ACTION_UP}) {
            MotionEvent event = MotionEvent.obtain(t, SystemClock.uptimeMillis(), action, x, y, 0);
            event.setSource(android.view.InputDevice.SOURCE_TOUCHSCREEN);
            check(getUiAutomation().injectInputEvent(event, true), "touch delivered");
            event.recycle();
        }
    }

    private void chord(int key) {
        long t = SystemClock.uptimeMillis();
        sendKeySync(new KeyEvent(t, t, KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_CTRL_LEFT, 0));
        sendKeySync(new KeyEvent(t, t, KeyEvent.ACTION_DOWN, key, 0, KeyEvent.META_CTRL_ON));
        sendKeySync(new KeyEvent(t, t, KeyEvent.ACTION_UP, key, 0, KeyEvent.META_CTRL_ON));
        sendKeySync(new KeyEvent(t, t, KeyEvent.ACTION_UP, KeyEvent.KEYCODE_CTRL_LEFT, 0));
    }

    private void checkImage(Activity app, Bitmap.CompressFormat format, String mime, String suffix, boolean fromKeyboard) throws Exception {
        ContentValues values = new ContentValues();
        values.put(MediaStore.Images.Media.DISPLAY_NAME, "wordcraft-test" + suffix);
        values.put(MediaStore.Images.Media.MIME_TYPE, mime);
        values.put(MediaStore.Images.Media.RELATIVE_PATH, android.os.Environment.DIRECTORY_DCIM + "/Camera");
        Uri uri = app.getContentResolver().insert(MediaStore.Images.Media.EXTERNAL_CONTENT_URI, values);
        check(uri != null, "test URI");
        try {
            ByteArrayOutputStream bytes = new ByteArrayOutputStream();
            Bitmap swatch = Bitmap.createBitmap(64, 64, Bitmap.Config.ARGB_8888);
            swatch.eraseColor(0xfffa18cd);
            swatch.compress(format, 90, bytes);
            swatch.recycle();
            try (OutputStream out = app.getContentResolver().openOutputStream(uri)) { out.write(bytes.toByteArray()); }
            if (fromKeyboard) {
                String imePackage = "ai.storyteller.wordcraft.test";
                app.grantUriPermission(imePackage, uri, Intent.FLAG_GRANT_READ_URI_PERMISSION);
                try {
                    Bundle data = new Bundle();
                    data.putParcelable("uri", uri);
                    runOnMainSync(() -> app.getSystemService(InputMethodManager.class).sendAppPrivateCommand(
                            app.getCurrentFocus(), "ai.storyteller.wordcraft.test.IMAGE", data));
                    long deadline = SystemClock.elapsedRealtime() + 15000;
                    boolean displayed = false;
                    while (!displayed && SystemClock.elapsedRealtime() < deadline) {
                        SystemClock.sleep(500);
                        Bitmap screen = getUiAutomation().takeScreenshot();
                        if (screen != null) {
                            Bitmap readable = screen.getConfig() == Bitmap.Config.HARDWARE
                                    ? screen.copy(Bitmap.Config.ARGB_8888, false) : screen;
                            displayed = hasSwatch(readable);
                            if (readable != screen) readable.recycle();
                            screen.recycle();
                        }
                    }
                    check(displayed, "keyboard image rendered on the page");
                } finally { app.revokeUriPermission(imePackage, uri, Intent.FLAG_GRANT_READ_URI_PERMISSION); }
                return;
            }
            String path = pickFromSystem(app, "wordcraft-test" + suffix);
            File image = new File(path);
            try { check(Arrays.equals(bytes.toByteArray(), Files.readAllBytes(image.toPath())), "URI import " + mime); }
            finally { image.delete(); }
            check(pickFromSystem(app, null).isEmpty(), "cancel");
        } finally {
            app.getContentResolver().delete(uri, null, null);
        }
    }

    private static boolean hasSwatch(Bitmap bitmap) {
        int hits = 0;
        for (int y = 1; y < bitmap.getHeight(); y += 3) {
            for (int x = 1; x < bitmap.getWidth(); x += 3) {
                int pixel = bitmap.getPixel(x, y);
                if (Math.abs(android.graphics.Color.red(pixel) - 250) < 8 &&
                        Math.abs(android.graphics.Color.green(pixel) - 24) < 8 &&
                        Math.abs(android.graphics.Color.blue(pixel) - 205) < 8 && ++hits >= 32) return true;
            }
        }
        return false;
    }

    private String pickFromSystem(Activity app, String filename) throws Exception {
        app.getClass().getMethod("pickPicture").invoke(app);
        long deadline = SystemClock.elapsedRealtime() + 30000;
        long nextTap = 0;
        boolean acted = false;
        String lastWindow = "";
        while (SystemClock.elapsedRealtime() < deadline) {
            AccessibilityNodeInfo root = getUiAutomation().getRootInActiveWindow();
            if (root != null && root.getPackageName() != null) {
                lastWindow = root.getPackageName().toString();
                if (lastWindow.contains("documentsui") && SystemClock.elapsedRealtime() >= nextTap) {
                    if (filename == null && !acted) {
                        acted = getUiAutomation().performGlobalAction(android.accessibilityservice.AccessibilityService.GLOBAL_ACTION_BACK);
                    } else if (filename != null) {
                        // Tablet layouts can select a tile first and expose a separate Open button.
                        boolean confirmed = acted && (clickLabel(root, "Open") || clickLabel(root, "Select"));
                        if (!confirmed && clickLabel(root, filename)) acted = true;
                    }
                    nextTap = SystemClock.elapsedRealtime() + 800;
                }
            }
            String path = (String) app.getClass().getMethod("takePictureResult").invoke(app);
            if (path != null) {
                check(acted && !path.startsWith("!"), "system image picker returned " + path);
                return path;
            }
            SystemClock.sleep(150);
        }
        throw new AssertionError("System image picker did not finish " + filename + " (tapped=" + acted + ", window=" + lastWindow + ")");
    }

    private boolean clickLabel(AccessibilityNodeInfo root, String label) {
        for (AccessibilityNodeInfo item : root.findAccessibilityNodeInfosByText(label)) {
            if (!item.isVisibleToUser() || !item.isEnabled()) continue;
            String text = String.valueOf(item.getText());
            String description = String.valueOf(item.getContentDescription());
            if (!label.equalsIgnoreCase(text) && !label.equalsIgnoreCase(description)) continue;
            for (AccessibilityNodeInfo row = item; row != null; row = row.getParent()) {
                if (row.isClickable() && row.performAction(AccessibilityNodeInfo.ACTION_CLICK)) return true;
            }
            Rect area = new Rect();
            item.getBoundsInScreen(area);
            if (!area.isEmpty()) {
                tap(area.exactCenterX(), area.exactCenterY());
                return true;
            }
        }
        return false;
    }

    private static void check(boolean value, String message) {
        if (!value) throw new AssertionError(message);
    }
}
