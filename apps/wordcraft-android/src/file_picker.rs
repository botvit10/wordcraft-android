//! An in-app file picker for Android, drawn with egui — no Java, no native dialog.
//!
//! # Why this exists
//!
//! `rfd` has no Android backend (its `backend.rs` compiles no module at all for a
//! target that is neither Linux/BSD, macOS, Windows nor wasm32), and Android's
//! own picker — the Storage Access Framework — needs `startActivityForResult`,
//! which `android-activity` 0.6 does not expose. So a ported desktop app has no
//! file dialogs at all: Open and Save As are silently treated as cancelled.
//! This module draws the picker instead.
//!
//! # How it plugs in
//!
//! Every app in this suite has a host-services seam that is *asynchronous* on
//! the platforms that can't block the frame — the browser build uses it, and so
//! does this. [`request`] queues a dialog and returns a [`Receiver`] the app
//! polls once a frame; [`draw`] answers on it from a later frame. The app's own
//! follow-up (open the file, save there and carry on) then runs, unchanged.
//!
//! Shape per app:
//!
//! - `Services::file_dialog(req) -> Receiver<Option<String>>` (WordCraft, this
//!   crate's `FileDialogRequest`): answer `Some(path)`, or `None` for cancelled.
//! - `Services::pick_save_async(name) -> Receiver<Option<String>>` (GridCraft,
//!   DeckCraft): the same, for Save As and the exports. Without it the app is
//!   handed only finished bytes and never learns where they went.
//! - `Services::open_async` + `Services::inbox` and `pick_picture_async(inbox)`
//!   (GridCraft, DeckCraft — the web seam): the shell polls the receiver, reads
//!   the file and pushes `(name, bytes)` into the inbox.
//! - `download(name, bytes)` (GridCraft, DeckCraft — the last resort): ask for a
//!   path, then write the bytes there.
//! - `rfd::AsyncFileDialog` (PdfCraft): the crate's own `crate::rfd` shim, which
//!   registers here and hands back a future the picker's answer completes.
//!
//! An empty answer means cancelled, which is what each of those seams reads as
//! "the user backed out". Never drop the sender instead.
//!
//! # One dialog per window, not per process
//!
//! The dialog in flight is a **thread-local**, not a global. A second window of
//! the same app is a second `android_main` on a thread of its own (a second
//! Activity, which android-activity supports), and a global would give the two
//! windows one shared dialog: the second window's request would cancel the
//! first's, both would draw it, and the file the user tapped could open in the
//! other window.
//!
//! # Permissions
//!
//! Android 10+ forbids reading `/storage/emulated/0/Download` by path. Two
//! routes exist and this picker works with either, because it probes each root
//! and only offers the ones it can actually read:
//!
//! - **App folder** — `external_data_path()`, the app's own directory. Always
//!   readable and writable, no permission at all. Files can be pushed there over
//!   USB.
//! - **Shared storage** — Download / Pictures / DCIM / Documents. Needs
//!   "All files access" (`MANAGE_EXTERNAL_STORAGE`, granted by hand in
//!   Settings ▸ Apps ▸ <App> ▸ Special access) or `READ_MEDIA_IMAGES` for
//!   reading media. All four are declared in the crate's `Cargo.toml`.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::mpsc::{Receiver, Sender, channel};

/// The app's external files directory, captured in `android_main`.
///
/// `<shared storage>/Android/data/<package>/files` — always readable and
/// writable with no permission, so it is both the picker's default root and the
/// place Save falls back to. Shared by every window of the app, which is right:
/// there is one such folder.
pub static EXT_DIR: OnceLock<PathBuf> = OnceLock::new();

const SHARED_ROOT: &str = "/storage/emulated/0";

/// Which dialog is open.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Document,
    Recipients,
    /// An image to insert or replace.
    Picture,
    /// Several files.
    OpenMany,
    /// A path to save to.
    Save,
    /// A folder.
    Folder,
}

impl Kind {
    pub fn extensions(self) -> Option<&'static [&'static str]> {
        match self {
            Self::Picture => Some(&["png", "jpg", "jpeg", "gif", "webp", "bmp"]),
            Self::Document => Some(&["docx", "docm", "dotx", "dotm", "doc", "dot", "odt", "rtf", "txt", "md", "html", "htm", "tex", "json"]),
            Self::Recipients => Some(&["csv", "tsv", "txt", "xlsx", "xlsm", "ods"]),
            Self::OpenMany | Self::Save | Self::Folder => None,
        }
    }
}

/// The dialog being shown: what was asked, and how to answer it.
struct Pending {
    kind: Kind,
    title: String,
    /// Directory being browsed.
    dir: PathBuf,
    /// File name for a Save dialog.
    name: String,
    /// Chosen files, for a multi-select dialog.
    picked: Vec<PathBuf>,
    /// A message to show inside the picker (usually a permission problem).
    note: Option<String>,
    /// Whether the starting directory was already applied.
    started: bool,
    /// Where the answer goes; the app polls it once a frame.
    answer: Option<Sender<Vec<PathBuf>>>,
}

thread_local! {
    /// The dialog *this* window is showing. One per thread; see the module docs.
    static PENDING: RefCell<Option<Pending>> = const { RefCell::new(None) };
}

/// Queue a dialog and return the receiver its answer arrives on.
///
/// A dialog already open in *this* window is answered "cancelled" first; the app
/// can only use one answer at a time, and leaving the old request waiting would
/// strand it.
pub fn request(kind: Kind, title: &str, suggested: Option<String>) -> Receiver<Vec<PathBuf>> {
    let (tx, rx) = channel();
    PENDING.with(|slot| {
        let mut slot = slot.borrow_mut();
        if let Some(prev) = slot.take()
            && let Some(tx) = prev.answer
        {
            let _ = tx.send(Vec::new());
        }
        *slot = Some(Pending {
            kind,
            title: title.to_string(),
            dir: PathBuf::new(),
            name: suggested.unwrap_or_default(),
            picked: Vec::new(),
            note: None,
            started: false,
            answer: Some(tx),
        });
    });
    rx
}

/// Whether this window has a dialog waiting to be drawn.
#[allow(dead_code)]
pub fn is_open() -> bool {
    PENDING.with(|slot| slot.borrow().is_some())
}

/// Close this window's dialog, if any, answering "cancelled" — for a shell that
/// is quitting while one is open.
#[allow(dead_code)]
pub fn cancel() {
    PENDING.with(|slot| {
        if let Some(pending) = slot.borrow_mut().take()
            && let Some(tx) = pending.answer
        {
            let _ = tx.send(Vec::new());
        }
    });
}

/// Roots to offer, as (label, path).
fn roots() -> Vec<(String, PathBuf)> {
    let mut v = Vec::new();
    if let Some(d) = EXT_DIR.get() {
        v.push(("App folder".to_string(), d.clone()));
        // Sibling of the app's own files, still owned by the app.
        if let Some(parent) = d.parent() {
            v.push(("App storage".to_string(), parent.to_path_buf()));
        }
    }
    for (label, sub) in [("Download", "Download"), ("Pictures", "Pictures"), ("DCIM", "DCIM"), ("Documents", "Documents")] {
        v.push((label.to_string(), Path::new(SHARED_ROOT).join(sub)));
    }
    v.push(("Internal storage".to_string(), PathBuf::from(SHARED_ROOT)));
    v
}

/// The app's folder in `Android/media`, which *is* reachable over a USB cable (`Android/data` is
/// not, from Android 11 on): `…/Android/data/<package>/files` → `…/Android/media/<package>`.
pub fn media_dir() -> Option<PathBuf> {
    media_dir_from(EXT_DIR.get()?)
}

/// The derivation, on its own so it can be pinned by a test: `…/Android/data/<package>/files`
/// → `…/Android/media/<package>`.
fn media_dir_from(external_files: &Path) -> Option<PathBuf> {
    let package_dir = external_files.parent()?;
    let android = package_dir.parent()?.parent()?;
    Some(android.join("media").join(package_dir.file_name()?))
}

/// The Fonts folder inside it: the visible folder a font file is dropped into.
pub fn fonts_dir() -> Option<PathBuf> {
    Some(media_dir()?.join("Fonts"))
}

/// Point the font scan at a folder the user can reach over a USB cable.
///
/// # Why this is needed on Android and on no other platform
///
/// `system_font_dirs()` in this app's font code takes its Linux branch here and scans
/// `/usr/share/fonts`, `$HOME/.fonts` and `$HOME/.local/share/fonts` — and none of those exist on
/// Android: the platform's own faces are in `/system/fonts`, and `HOME` is unset. So the app knows
/// only its bundled families and there is no way to add one — where VectorCraft and PhotoCraft have
/// `set_platform_font_files`/`set_app_font_dir` hooks, this app's font code has neither. It *does*
/// read `HOME` at every scan, though, so pointing `HOME` at the app's media folder and putting the
/// fonts where that branch looks gives the same result with no change to the app: drop a `.ttf`
/// into `Fonts/` over the cable, reopen the app, and it is in the font list.
///
/// The two names the scan looks for are hidden ones (`.fonts`, `.local/share/fonts`), so both are
/// symlinks to the visible `Fonts` folder — the user drops a file in one place and the app reads it
/// from the other.
///
/// The scan runs once per process (a `OnceLock` in the app), which is why a new font needs the app
/// reopened rather than a reload.
#[cfg(target_os = "android")]
pub fn register_fonts() {
    let Some(fonts) = fonts_dir() else {
        log::warn!("fonts: no Fonts folder to register; adding a font will not work");
        return;
    };
    let Some(media) = media_dir() else {
        return;
    };
    if let Err(e) = std::fs::create_dir_all(&fonts) {
        log::warn!("fonts: could not make {}: {e}", fonts.display());
        return;
    }
    log::info!("fonts: the Fonts folder is {}", fonts.display());
    // SAFETY: called from `android_main` before eframe starts, so nothing else is reading the
    // environment yet; the app's font code only ever reads `HOME`.
    unsafe { std::env::set_var("HOME", &media) };
    link_fonts_dir(&fonts, &media.join(".fonts"));
    link_fonts_dir(&fonts, &media.join(".local").join("share").join("fonts"));
}

/// Make `link` a directory symlink to `target`, replacing a stale one. Failure is logged and
/// survivable: the folder the user drops fonts into still exists, the app just cannot see it yet.
#[cfg(target_os = "android")]
fn link_fonts_dir(target: &Path, link: &Path) {
    if let Some(parent) = link.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if std::fs::read_link(link).is_ok() {
        let _ = std::fs::remove_file(link);
    }
    if !link.exists()
        && let Err(e) = std::os::unix::fs::symlink(target, link)
    {
        log::warn!("fonts: could not link {} → {}: {e}", link.display(), target.display());
    }
}


/// The best directory to open first: the app folder is always writable, so it
/// is the only safe default — a desktop path arriving as a suggestion never
/// exists on the device.
fn default_dir() -> PathBuf {
    EXT_DIR.get().cloned().unwrap_or_else(|| PathBuf::from(SHARED_ROOT))
}

fn readable(dir: &Path) -> bool {
    std::fs::read_dir(dir).is_ok()
}

/// Directory entries, folders first, filtered by `extensions` when given.
fn entries(dir: &Path, extensions: Option<&[&str]>) -> Result<(Vec<PathBuf>, Vec<PathBuf>), String> {
    let rd = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let (mut dirs, mut files) = (Vec::new(), Vec::new());
    for entry in rd.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            dirs.push(path);
        } else if let Some(exts) = extensions {
            let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase());
            if ext.as_ref().is_some_and(|e| exts.iter().any(|w| w.eq_ignore_ascii_case(e))) {
                files.push(path);
            }
        } else {
            files.push(path);
        }
    }
    dirs.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()));
    files.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()));
    Ok((dirs, files))
}

fn file_label(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
}

/// Draw the picker if this window has a dialog queued. Called from the shell's
/// `ui`, *after* forwarding to the app's own `ui`, so the window floats above
/// the editor.
pub fn draw(ctx: &egui::Context) {
    // Taken out for the duration rather than borrowed, so nothing in here can trip over a
    // re-entrant borrow; it is put back below while the user has not answered.
    let Some(mut pending) = PENDING.with(|slot| slot.borrow_mut().take()) else {
        return;
    };
    let kind = pending.kind;
    // First frame: pick a starting directory.
    if !pending.started {
        pending.started = true;
        let downloads = Path::new(SHARED_ROOT).join("Download");
        pending.dir = if kind == Kind::Picture && readable(&downloads) { downloads } else { default_dir() };
        if kind == Kind::Save && pending.name.is_empty() {
            pending.name = "untitled".to_string();
        }
    }

    let mut answer: Option<Vec<PathBuf>> = None;
    let mut cancel = false;
    let mut navigate: Option<PathBuf> = None;

    let dir = pending.dir.clone();
    let listing = entries(&dir, kind.extensions());
    let multi = kind == Kind::OpenMany;
    // A tablet screen is smaller than the desktop default, and in portrait
    // narrower than a hard-coded desktop width. `content_rect` is the area egui
    // considers safe (it excludes an OS status bar where the backend reports one).
    let screen = ctx.content_rect();
    let (w, h) = ((screen.width() * 0.92).min(720.0), (screen.height() * 0.82).min(620.0));
    let title = pending.title.clone();

    egui::Window::new(title)
        .collapsible(false)
        .resizable(true)
        .default_width(w)
        .default_height(h)
        .default_pos(screen.center() - egui::vec2(w, h) / 2.0)
        .show(ctx, |ui| {
            let layer = ui.layer_id();
            ui.memory_mut(|memory| memory.set_modal_layer(layer));
            // Finger-sized rows: the default spacing is sized for a mouse.
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
            ui.spacing_mut().button_padding = egui::vec2(10.0, 8.0);

            // Roots
            ui.horizontal_wrapped(|ui| {
                for (label, path) in roots() {
                    let ok = readable(&path);
                    let here = path == dir;
                    let text = if ok { label } else { format!("{label} \u{1F512}") };
                    if ui.add_enabled(ok && !here, egui::Button::new(text)).clicked() {
                        navigate = Some(path);
                    }
                }
            });
            ui.separator();

            // Current path + up
            ui.horizontal_wrapped(|ui| {
                if ui.button("\u{2B06} Up").clicked()
                    && let Some(parent) = dir.parent()
                {
                    navigate = Some(parent.to_path_buf());
                }
                if ui.button("\u{1F4C1} App folder").clicked() {
                    navigate = Some(default_dir());
                }
                ui.label(egui::RichText::new(dir.display().to_string()).monospace().small());
            });

            if kind == Kind::Picture {
                ui.label("PNG, JPEG, GIF, WebP, BMP");
            }
            if let Some(note) = &pending.note {
                ui.colored_label(egui::Color32::from_rgb(220, 160, 60), note);
            }

            ui.separator();
            let list_h = (h - 190.0).max(120.0);
            egui::ScrollArea::vertical().max_height(list_h).show(ui, |ui| match &listing {
                Err(e) => {
                    ui.label("Cannot read this folder.");
                    ui.label(egui::RichText::new(e.clone()).monospace().weak().small());
                    ui.label("Grant \"All files access\" in Settings \u{25B8} Apps \u{25B8} Special access to browse shared storage.");
                }
                Ok((dirs, files)) => {
                    if dirs.is_empty() && files.is_empty() {
                        ui.label("(empty \u{2014} no matching files)");
                    }
                    for d in dirs {
                        if ui.button(format!("\u{1F4C1} {}", file_label(d))).clicked() {
                            navigate = Some(d.clone());
                        }
                    }
                    if kind != Kind::Folder {
                        for f in files {
                            let chosen = pending.picked.iter().any(|p| p == f);
                            let mark = if chosen { "\u{2611}" } else { "\u{2610}" };
                            let label = if multi { format!("{mark} {}", file_label(f)) } else { format!("   {}", file_label(f)) };
                            if ui.button(label).clicked() {
                                if kind == Kind::Save {
                                    pending.name = file_label(f);
                                } else if multi {
                                    if chosen {
                                        pending.picked.retain(|p| p != f);
                                    } else {
                                        pending.picked.push(f.clone());
                                    }
                                } else {
                                    answer = Some(vec![f.clone()]);
                                }
                            }
                        }
                    }
                }
            });

            ui.separator();
            ui.horizontal_wrapped(|ui| match kind {
                Kind::Save => {
                    ui.label("File name:");
                    ui.add(egui::TextEdit::singleline(&mut pending.name).desired_width((w - 260.0).max(160.0)));
                    if ui.button("Save").clicked() {
                        let name = pending.name.trim().to_string();
                        if name.is_empty() {
                            pending.note = Some("Enter a file name.".to_string());
                        } else {
                            answer = Some(vec![dir.join(name)]);
                        }
                    }
                }
                Kind::Folder => {
                    if ui.button("Choose this folder").clicked() {
                        answer = Some(vec![dir.clone()]);
                    }
                }
                Kind::OpenMany => {
                    let n = pending.picked.len();
                    let label = if n == 0 { "Place".to_string() } else { format!("Place {n} file(s)") };
                    if ui.add_enabled(n > 0, egui::Button::new(label)).clicked() {
                        answer = Some(pending.picked.clone());
                    }
                }
                Kind::Document | Kind::Recipients | Kind::Picture => {
                    if ui.button("Open").clicked() {
                        // A file has to be tapped in the list above for an Open dialog.
                        pending.note = Some("Tap a file to open it.".to_string());
                    }
                }
            });
            if ui.button("Cancel").clicked() {
                cancel = true;
            }

            if dir == default_dir() {
                ui.label(egui::RichText::new("Tip: push files here over USB, or grant \"All files access\" to browse the whole tablet.").weak().small());
            }
        });

    if let Some(dir) = navigate {
        pending.dir = dir;
        pending.note = None;
    }
    if answer.is_some() || cancel {
        // Cancelled: an empty answer is what every seam here reads as one.
        let paths = answer.unwrap_or_default();
        if let Some(tx) = pending.answer.take() {
            let _ = tx.send(paths);
        }
    } else {
        // Still open: it stays this window's dialog. The app only wakes the window about once a
        // second while it waits, so the picker keeps asking for frames itself.
        PENDING.with(|slot| *slot.borrow_mut() = Some(pending));
        ctx.request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Session, WordApp, android_services};
    use egui::{Context, Event, Pos2, RawInput, epaint::Shape};
    use serde_json::json;

    fn frame(ctx: &Context, events: Vec<Event>) -> egui::FullOutput {
        let input = RawInput {
            screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(1000.0, 800.0))),
            events,
            ..Default::default()
        };
        ctx.run_ui(input, |ui| draw(ui.ctx()))
    }

    fn label_pos(output: &egui::FullOutput, label: &str) -> Option<Pos2> {
        output.shapes.iter().find_map(|clipped| match &clipped.shape {
            Shape::Text(text) if text.galley.text().trim() == label => Some(text.pos + text.galley.size() * 0.5),
            _ => None,
        })
    }

    fn choose(app: &mut WordApp, dir: &Path, cmd: &str, name: &str, visible: &[&str], hidden: &[&str]) {
        app.run(cmd, json!({})).unwrap();
        // Substitute a temporary Download directory for the device's shared storage.
        PENDING.with_borrow_mut(|slot| {
            let picker = slot.as_mut().expect("command opened the Android picker");
            picker.dir = dir.to_owned();
            picker.started = true;
        });
        let ctx = Context::default();
        frame(&ctx, vec![]).drop_without_applying_deltas();
        let output = frame(&ctx, vec![]);
        for name in visible {
            assert!(label_pos(&output, name).is_some(), "{cmd} must list {name}");
        }
        for name in hidden {
            assert!(label_pos(&output, name).is_none(), "{cmd} must hide {name}");
        }
        let pos = label_pos(&output, name).unwrap();
        output.drop_without_applying_deltas();
        for pressed in [true, false] {
            frame(&ctx, vec![Event::PointerMoved(pos), Event::PointerButton {
                pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default(),
            }]).drop_without_applying_deltas();
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while app.file_dialog_open() {
            assert!(std::time::Instant::now() < deadline, "picker did not deliver {name}");
            ctx.run_ui(Default::default(), |ui| app.logic(ui.ctx())).drop_without_applying_deltas();
            std::thread::yield_now();
        }
    }

    #[test]
    fn pictures_and_documents_use_separate_listings_and_actions() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path();
        let image = image::RgbImage::from_pixel(4, 3, image::Rgb([10, 20, 30]));
        let png = dir.join("diagram.PNG");
        let jpeg = dir.join("photo.JpEg");
        image.save(&png).unwrap();
        image.save(&jpeg).unwrap();

        let doc = dir.join("letter.docx");
        let mut app = WordApp::new(Session::new(Default::default()), android_services());
        app.run("file.save", json!({"path": doc})).unwrap();
        let images = ["diagram.PNG", "photo.JpEg"];
        for name in images {
            choose(&mut app, dir, "insert.picture", name, &images, &["letter.docx"]);
            // Inserting selects the new picture. Place the caret after it for the next insert.
            app.run("caret.right", json!({})).unwrap();
        }
        assert_eq!(app.session.doc.media.len(), 2);
        for file in [&png, &jpeg] {
            let bytes = std::fs::read(file).unwrap();
            assert!(app.session.doc.media.values().any(|media| media.as_slice() == bytes));
        }
        app.run("file.save", json!({"path": doc})).unwrap();
        let mut reopen = WordApp::new(Session::new(Default::default()), android_services());
        choose(&mut reopen, dir, "ui.openFileDialog", "letter.docx", &["letter.docx"], &images);
        assert_eq!(reopen.session.path.as_deref(), Some(doc.as_path()));
        assert_eq!(reopen.session.doc.media.len(), 2);
    }
}
