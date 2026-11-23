package ai.storyteller.wordcraft.test;

import android.content.ClipDescription;
import android.inputmethodservice.InputMethodService;
import android.net.Uri;
import android.os.Bundle;
import android.view.View;
import android.view.inputmethod.InputConnection;
import android.view.inputmethod.InputContentInfo;

/** Exercises the system's served InputConnection from a separate keyboard process. */
public final class TestKeyboard extends InputMethodService {
    @Override public void onAppPrivateCommand(String action, Bundle data) {
        InputConnection ic = getCurrentInputConnection();
        if (ic == null || data == null || getCurrentInputEditorInfo() == null ||
                !"ai.storyteller.wordcraft".equals(getCurrentInputEditorInfo().packageName)) return;
        if ("ai.storyteller.wordcraft.test.TEXT".equals(action)) {
            ic.commitText(data.getString("text"), 1);
        } else if ("ai.storyteller.wordcraft.test.IMAGE".equals(action)) {
            Uri uri = data.getParcelable("uri", Uri.class);
            ClipDescription desc = new ClipDescription("test image", new String[] {"image/png"});
            ic.commitContent(new InputContentInfo(uri, desc, null), InputConnection.INPUT_CONTENT_GRANT_READ_URI_PERMISSION, null);
        }
    }

    @Override public View onCreateInputView() {
        android.widget.TextView panel = new android.widget.TextView(this);
        panel.setText("WordCraft test keyboard");
        panel.setMinHeight(getResources().getDisplayMetrics().heightPixels / 3);
        return panel;
    }
}
