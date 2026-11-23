//! System clipboard access on an explicit editor copy, cut, or paste.
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;

use jni::objects::{JObject, JString, JValue};
use jni::{Env, jni_sig, jni_str};
use wordcraft_ui_egui::ClipboardContent;

fn with_manager<T>(action: impl FnOnce(&mut Env, &JObject, &JObject) -> jni::errors::Result<T>) -> Result<T, String> {
    let app = super::APP.with(|slot| slot.borrow().clone()).ok_or("No active Android window")?;
    // SAFETY: AndroidApp owns the live Activity and provides its JavaVM.
    let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) };
    vm.attach_current_thread(|env| {
        // SAFETY: borrowed from app for this call. JObject does not delete the reference.
        let activity = unsafe { JObject::from_raw(env, app.activity_as_ptr().cast()) };
        let service = env.new_string("clipboard")?;
        let manager = env
            .call_method(&activity, jni_str!("getSystemService"), jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"), &[JValue::Object(&service)])?
            .l()?;
        action(env, &activity, &manager)
    })
    .map_err(|_| "Android could not access the clipboard".into())
}

pub fn write_text(text: &str) -> Result<(), String> {
    with_manager(|env, _, manager| {
        let label = env.new_string("WordCraft")?;
        let value = env.new_string(text)?;
        let clip = env
            .call_static_method(
                jni_str!("android/content/ClipData"),
                jni_str!("newPlainText"),
                jni_sig!("(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;"),
                &[JValue::Object(&label), JValue::Object(&value)],
            )?
            .l()?;
        env.call_method(manager, jni_str!("setPrimaryClip"), jni_sig!("(Landroid/content/ClipData;)V"), &[JValue::Object(&clip)])?;
        Ok(())
    })
}

enum Item {
    Text(String),
    Image(File),
}

fn first_item(env: &mut Env, activity: &JObject, manager: &JObject) -> jni::errors::Result<Option<Item>> {
    let clip = env.call_method(manager, jni_str!("getPrimaryClip"), jni_sig!("()Landroid/content/ClipData;"), &[])?.l()?;
    if clip.is_null() || env.call_method(&clip, jni_str!("getItemCount"), jni_sig!("()I"), &[])?.i()? < 1 {
        return Ok(None);
    }
    let item = env.call_method(&clip, jni_str!("getItemAt"), jni_sig!("(I)Landroid/content/ClipData$Item;"), &[JValue::Int(0)])?.l()?;
    let uri = env.call_method(&item, jni_str!("getUri"), jni_sig!("()Landroid/net/Uri;"), &[])?.l()?;
    if !uri.is_null() {
        let description = env.call_method(&clip, jni_str!("getDescription"), jni_sig!("()Landroid/content/ClipDescription;"), &[])?.l()?;
        let mime = env.new_string("image/*")?;
        let image = env.call_method(&description, jni_str!("hasMimeType"), jni_sig!("(Ljava/lang/String;)Z"), &[JValue::Object(&mime)])?.z()?;
        let scheme = env.call_method(&uri, jni_str!("getScheme"), jni_sig!("()Ljava/lang/String;"), &[])?.l()?;
        if image && !scheme.is_null() && env.cast_local::<JString>(scheme)?.try_to_string(env)? == "content" {
            let resolver = env.call_method(activity, jni_str!("getContentResolver"), jni_sig!("()Landroid/content/ContentResolver;"), &[])?.l()?;
            let mode = env.new_string("r")?;
            let parcel = env
                .call_method(
                    &resolver,
                    jni_str!("openFileDescriptor"),
                    jni_sig!("(Landroid/net/Uri;Ljava/lang/String;)Landroid/os/ParcelFileDescriptor;"),
                    &[JValue::Object(&uri), JValue::Object(&mode)],
                )?
                .l()?;
            if !parcel.is_null() {
                let fd = env.call_method(&parcel, jni_str!("detachFd"), jni_sig!("()I"), &[])?.i()?;
                if fd >= 0 {
                    // SAFETY: detachFd transfers sole ownership to the caller.
                    return Ok(Some(Item::Image(unsafe { File::from_raw_fd(fd) })));
                }
            }
        }
    }
    let text = env.call_method(&item, jni_str!("getText"), jni_sig!("()Ljava/lang/CharSequence;"), &[])?.l()?;
    if text.is_null() {
        return Ok(None);
    }
    let string = env.call_method(&text, jni_str!("toString"), jni_sig!("()Ljava/lang/String;"), &[])?.l()?;
    Ok(Some(Item::Text(env.cast_local::<JString>(string)?.try_to_string(env)?)))
}

pub fn read() -> Result<Option<ClipboardContent>, String> {
    match with_manager(first_item)? {
        Some(Item::Text(text)) => Ok(Some(ClipboardContent::Text(text))),
        Some(Item::Image(file)) => {
            const LIMIT: u64 = 16 << 20;
            let mut bytes = Vec::new();
            file.take(LIMIT + 1).read_to_end(&mut bytes).map_err(|_| "Could not read the clipboard image")?;
            if bytes.len() as u64 > LIMIT {
                return Err("Clipboard image exceeds 16 MiB".into());
            }
            Ok(Some(ClipboardContent::Image(bytes)))
        }
        None => Ok(None),
    }
}
