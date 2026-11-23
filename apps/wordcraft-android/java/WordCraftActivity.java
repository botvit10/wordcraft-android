package ai.storyteller.wordcraft;

import android.app.NativeActivity;
import android.content.ContentResolver;
import android.content.Intent;
import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.graphics.Rect;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.text.InputType;
import android.view.ViewGroup;
import android.view.inputmethod.BaseInputConnection;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputConnection;
import android.view.inputmethod.InputContentInfo;
import android.view.KeyEvent;
import android.view.View;
import android.view.WindowInsets;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.util.concurrent.ConcurrentLinkedQueue;
import java.util.concurrent.atomic.AtomicReference;

/** Android services for the native editor. All state belongs to this document window. */
public final class WordCraftActivity extends NativeActivity {
    private static final int PICK_IMAGE = 82;
    private final AtomicReference<String> pictureResult = new AtomicReference<>();
    // T = committed text, I = private cache image, P = paste command, E = feedback.
    private final ConcurrentLinkedQueue<String> keyboardEvents = new ConcurrentLinkedQueue<>();
    private volatile int visibleHeight = -1;

    @Override protected void onCreate(Bundle state) {
        super.onCreate(state);
        ViewGroup content = findViewById(android.R.id.content);
        View nativeView = content.getChildAt(0);
        // NativeActivity asks the IMM to show input for its native content view. The focused
        // proxy serves that view's connection while the native surface keeps all touch input.
        View editor = new View(this) {
            @Override public boolean checkInputConnectionProxy(View view) { return view == nativeView; }
            @Override public boolean onCheckIsTextEditor() { return true; }
            @Override public InputConnection onCreateInputConnection(EditorInfo info) {
                info.inputType = InputType.TYPE_NULL; // preserve NativeActivity's key-event typing
                info.imeOptions = EditorInfo.IME_FLAG_NO_EXTRACT_UI;
                info.contentMimeTypes = new String[] {"image/*"};
                return new BaseInputConnection(this, false) {
                    @Override public boolean sendKeyEvent(KeyEvent event) {
                        if (takeBulkText(event)) return true;
                        return super.sendKeyEvent(event);
                    }
                    @Override public boolean performContextMenuAction(int id) {
                        if (id == android.R.id.paste) {
                            keyboardEvents.add("P");
                            return true;
                        }
                        return super.performContextMenuAction(id);
                    }
                    @Override public boolean commitContent(InputContentInfo content, int flags, Bundle opts) {
                        return acceptKeyboardImage(content, flags);
                    }
                };
            }
        };
        editor.setFocusableInTouchMode(true);
        editor.setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_NO);
        addContentView(editor, new ViewGroup.LayoutParams(1, 1));
        editor.requestFocus();
        View decor = getWindow().getDecorView();
        decor.setOnApplyWindowInsetsListener((v, insets) -> {
            updateHeight(v, insets);
            return v.onApplyWindowInsets(insets);
        });
        decor.getViewTreeObserver().addOnGlobalLayoutListener(
            () -> updateHeight(decor, decor.getRootWindowInsets()));
        decor.requestApplyInsets();
    }

    private void updateHeight(View decor, WindowInsets insets) {
        View content = findViewById(android.R.id.content);
        if (content == null) return;
        int[] origin = new int[2];
        content.getLocationOnScreen(origin);
        if (Build.VERSION.SDK_INT >= 30 && insets != null) {
            if (!insets.isVisible(WindowInsets.Type.ime())) {
                visibleHeight = -1;
                return;
            }
            int[] top = new int[2];
            decor.getLocationOnScreen(top);
            visibleHeight = top[1] + decor.getHeight() - insets.getInsets(WindowInsets.Type.ime()).bottom - origin[1];
        } else {
            Rect frame = new Rect();
            decor.getWindowVisibleDisplayFrame(frame);
            int height = frame.bottom - origin[1];
            // Pre-R includes navigation bars in this measurement even without an IME.
            visibleHeight = content.getHeight() - height > 80 * getResources().getDisplayMetrics().density ? height : -1;
        }
    }

    public int visibleEditorHeight() { return visibleHeight; }

    private boolean takeBulkText(KeyEvent event) {
        // BaseInputConnection can deliver a whole keyboard-clipboard entry as ACTION_MULTIPLE.
        String text = event.getCharacters();
        if (event.getAction() == KeyEvent.ACTION_MULTIPLE && event.getKeyCode() == KeyEvent.KEYCODE_UNKNOWN
                && text != null && !text.isEmpty()) {
            keyboardEvents.add("T" + text);
            return true;
        }
        return false;
    }

    @Override public boolean dispatchKeyEvent(KeyEvent event) {
        return takeBulkText(event) || super.dispatchKeyEvent(event);
    }

    private boolean acceptKeyboardImage(InputContentInfo content, int flags) {
        if (content == null || !content.getDescription().hasMimeType("image/*")) return false;
        boolean grant = (flags & InputConnection.INPUT_CONTENT_GRANT_READ_URI_PERMISSION) != 0;
        try {
            if (grant) content.requestPermission();
        } catch (SecurityException e) {
            keyboardEvents.add("EThe keyboard did not grant access to the image");
            return false;
        }
        new Thread(() -> {
            try {
                File image = importPicture(content.getContentUri(), 16 << 20);
                runOnUiThread(() -> {
                    if (isDestroyed()) image.delete();
                    else keyboardEvents.add("I" + image.getAbsolutePath());
                });
            } catch (IOException | RuntimeException e) {
                keyboardEvents.add("ECould not read the keyboard image (up to 16 MiB)");
            } finally {
                if (grant) content.releasePermission();
            }
        }, "WordCraft keyboard image").start();
        return true;
    }

    public String takeKeyboardEvent() { return keyboardEvents.poll(); }

    public void pickPicture() {
        runOnUiThread(() -> {
            Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT)
                .addCategory(Intent.CATEGORY_OPENABLE).setType("image/*");
            try {
                startActivityForResult(intent, PICK_IMAGE);
            } catch (RuntimeException e) {
                pictureResult.set("!Android could not open the image picker");
            }
        });
    }

    @Override protected void onActivityResult(int request, int result, Intent data) {
        super.onActivityResult(request, result, data);
        if (request != PICK_IMAGE) return;
        Uri uri = data == null ? null : data.getData();
        if (result != RESULT_OK || uri == null) {
            pictureResult.set("");
            return;
        }
        // Providers can stream from the cloud. Keep disk/network I/O off the Activity thread.
        new Thread(() -> {
            try {
                File image = importPicture(uri, 64 << 20);
                runOnUiThread(() -> {
                    if (isDestroyed()) image.delete();
                    else pictureResult.set(image.getAbsolutePath());
                });
            } catch (IOException | RuntimeException e) {
                pictureResult.set("!Could not read the selected image (up to 64 MiB and 50 megapixels)");
            }
        }, "WordCraft image import").start();
    }

    public String takePictureResult() { return pictureResult.getAndSet(null); }

    @Override protected void onDestroy() {
        String result = pictureResult.getAndSet(null);
        if (result != null && result.startsWith("/")) new File(result).delete();
        String event;
        while ((event = keyboardEvents.poll()) != null)
            if (event.startsWith("I")) new File(event.substring(1)).delete();
        super.onDestroy();
    }

    private File importPicture(Uri uri, int limit) throws IOException {
        if (!ContentResolver.SCHEME_CONTENT.equals(uri.getScheme())) throw new IOException("Expected a content URI");
        File file = File.createTempFile("wordcraft-picture-", ".img", getCacheDir());
        boolean complete = false;
        try {
            try (InputStream in = getContentResolver().openInputStream(uri);
                 FileOutputStream out = new FileOutputStream(file)) {
                if (in == null) throw new IOException("Provider unavailable");
                byte[] buf = new byte[32768];
                int total = 0, n;
                while ((n = in.read(buf)) != -1) {
                    total += n;
                    if (total > limit) throw new IOException("Image too large");
                    out.write(buf, 0, n);
                }
            }
            BitmapFactory.Options info = new BitmapFactory.Options();
            info.inJustDecodeBounds = true;
            BitmapFactory.decodeFile(file.getPath(), info);
            if (info.outWidth <= 0 || info.outHeight <= 0 || info.outWidth > 16000 || info.outHeight > 16000
                    || (long) info.outWidth * info.outHeight > 50000000L) throw new IOException("Invalid image dimensions");
            // Keep the original engine-supported formats. Cameras can also offer HEIF/AVIF.
            if (!("image/png".equals(info.outMimeType) || "image/jpeg".equals(info.outMimeType)
                    || "image/gif".equals(info.outMimeType) || "image/webp".equals(info.outMimeType)
                    || "image/bmp".equals(info.outMimeType) || "image/x-ms-bmp".equals(info.outMimeType))) {
                BitmapFactory.Options decode = new BitmapFactory.Options();
                decode.inSampleSize = 1;
                while ((long)(info.outWidth / decode.inSampleSize) * (info.outHeight / decode.inSampleSize) > 8000000L)
                    decode.inSampleSize *= 2;
                Bitmap pixels = BitmapFactory.decodeFile(file.getPath(), decode);
                if (pixels == null) throw new IOException("Unsupported image");
                try (FileOutputStream out = new FileOutputStream(file)) {
                    if (!pixels.compress(Bitmap.CompressFormat.PNG, 100, out))
                        throw new IOException("Conversion failed");
                } finally {
                    pixels.recycle();
                }
                if (file.length() > limit) throw new IOException("Converted image too large");
            }
            complete = true;
            return file;
        } finally {
            if (!complete) file.delete();
        }
    }

}
