//! Calls on this editor window's WordCraftActivity.
use std::cell::RefCell;
use std::sync::mpsc::{Receiver, Sender, channel};

use jni::objects::{JObject, JString};
use jni::{Env, jni_sig, jni_str};
use wordcraft_ui_egui::WordApp;

fn with_activity<T>(action: impl FnOnce(&mut Env, &JObject) -> jni::errors::Result<T>) -> Result<T, String> {
    let app = super::APP.with_borrow(Clone::clone).ok_or("No active Android window")?;
    // SAFETY: AndroidApp owns the live Activity and VM for this call.
    let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) };
    vm.attach_current_thread(|env| {
        let activity = unsafe { JObject::from_raw(env, app.activity_as_ptr().cast()) };
        action(env, &activity)
    })
    .map_err(|_| "Android could not complete the requested operation".into())
}

thread_local! {
    static PICTURE: RefCell<Option<Sender<Option<String>>>> = const { RefCell::new(None) };
}

pub fn pick_picture() -> Receiver<Option<String>> {
    let (send, recv) = channel();
    PICTURE.with_borrow_mut(|slot| *slot = Some(send));
    if let Err(e) = with_activity(|env, activity| {
        env.call_method(activity, jni_str!("pickPicture"), jni_sig!("()V"), &[])?;
        Ok(())
    }) {
        log::error!("Image picker: {e}");
        PICTURE.take();
    }
    recv
}

// Send the result just before WordApp polls its channel. The caller then removes the cache
// file once insert/change has read it. An empty string is Android's cancellation result.
pub fn poll_picture(ctx: &egui::Context, app: &mut WordApp) -> Option<String> {
    if !PICTURE.with_borrow(Option::is_some) {
        return None;
    }
    let result = with_activity(|env, activity| {
        let obj = env.call_method(activity, jni_str!("takePictureResult"), jni_sig!("()Ljava/lang/String;"), &[])?.l()?;
        if obj.is_null() { Ok(None) } else { Ok(Some(env.cast_local::<JString>(obj)?.try_to_string(env)?)) }
    });
    let path = match result {
        Ok(None) => {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            return None;
        }
        Ok(Some(value)) if value.starts_with('!') => {
            app.status(&value[1..]);
            None
        }
        Ok(Some(value)) if !value.is_empty() => Some(value),
        Ok(_) => None,
        Err(error) => {
            app.status(error);
            None
        }
    };
    if let Some(tx) = PICTURE.take() {
        let _ = tx.send(path.clone());
    }
    path
}

pub fn keyboard_input(ctx: &egui::Context, raw: &mut egui::RawInput, app: &mut WordApp) {
    if let Ok((height, text)) = with_activity(|env, activity| {
        let height = env.call_method(activity, jni_str!("visibleEditorHeight"), jni_sig!("()I"), &[])?.i()?;
        let obj = env.call_method(activity, jni_str!("takeKeyboardEvent"), jni_sig!("()Ljava/lang/String;"), &[])?.l()?;
        let text = if obj.is_null() { None } else { Some(env.cast_local::<JString>(obj)?.try_to_string(env)?) };
        Ok((height, text))
    }) {
        super::apply_keyboard_inset(ctx, raw, height);
        if let Some(event) = text {
            if let Some(text) = event.strip_prefix('T') {
                raw.events.push(egui::Event::Paste(text.into()));
            } else if let Some(path) = event.strip_prefix('I') {
                if app.canvas.focused {
                    let _ = app.run("insert.picture", serde_json::json!({"path": path}));
                } else {
                    app.status("Place the cursor in the document to paste an image");
                }
                let _ = std::fs::remove_file(path);
            } else if event == "P" {
                if app.canvas.focused {
                    let _ = app.run("edit.paste", serde_json::json!({}));
                } else if let Ok(Some(wordcraft_ui_egui::ClipboardContent::Text(text))) = super::clipboard::read() {
                    raw.events.push(egui::Event::Paste(text));
                }
            } else if let Some(msg) = event.strip_prefix('E') {
                app.status(msg);
            }
            ctx.request_repaint();
        }
    }
}
