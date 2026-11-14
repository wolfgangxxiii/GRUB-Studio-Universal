//! Native GTK4 UI with libadwaita styling. GTK4 selects Wayland directly
//! when WAYLAND_DISPLAY is set; the system window decorations remain intact.
use grub_studio::{core, inspect};
use grub_studio::core::MenuEntry;
use grub_studio::inspect::{OsRecord, Snapshot, Theme};
use gtk4 as gtk;
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const APP_ID: &str = "io.github.grubstudio.Universal";
const LANGUAGES: &[(&str, &str)] = &[
    ("pl", "Polski"), ("en", "English"), ("de", "Deutsch"), ("es", "Español"),
    ("fr", "Français"), ("uk", "Українська"), ("pt_BR", "Português (Brasil)"),
];

#[derive(Clone)]
struct StudioState {
    snapshot: Snapshot,
    authorized: bool,
    demo: bool,
    busy: bool,
    language: String,
    appearance: String,
    active: String,
    selected_theme: String,
    selected_backup: Option<String>,
    selected_entry: Option<usize>,
    pending_entries: Vec<MenuEntry>,
    menu_form_draft: Option<MenuFormDraft>,
    settings_draft: Option<SettingsDraft>,
    log: Vec<String>,
    chrome: Option<Chrome>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct MenuFormDraft {
    selected: Option<usize>,
    title: String,
    uuid: String,
    path: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct SettingsDraft {
    timeout: String,
    style: String,
    gfx: String,
    kernel_args: String,
}
impl SettingsDraft {
    fn from_config(config: &BTreeMap<String, String>) -> Self {
        Self {
            timeout: config.get("GRUB_TIMEOUT").cloned().unwrap_or_else(|| "5".into()),
            style: config.get("GRUB_TIMEOUT_STYLE").cloned().unwrap_or_else(|| "menu".into()),
            gfx: config.get("GRUB_GFXMODE").cloned().unwrap_or_else(|| "auto".into()),
            kernel_args: config.get("GRUB_CMDLINE_LINUX_DEFAULT").cloned().unwrap_or_default(),
        }
    }
}
// GTK widgets in Chrome live for the lifetime of the window. Updating a page
// never tears down the sidebar/header, preventing width jumps and focus loss.
#[derive(Clone)]
struct Chrome {
    root: gtk::Box,
    header: gtk::Box,
    sidebar: gtk::Box,
    scroller: gtk::ScrolledWindow,
    theme_actions: gtk::Box,
    footer: gtk::Box,
}
type Shared = Rc<RefCell<StudioState>>;

fn translate(lang: &str, key: &str) -> String {
    static TABLE: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    let dictionary = TABLE.get_or_init(|| serde_json::from_str(include_str!("../assets/translations.json"))
        .expect("Bundled translations must be valid JSON"));
    dictionary.get(lang).and_then(|v| v.get(key)).and_then(|v| v.as_str())
        .or_else(|| dictionary.get("en").and_then(|v| v.get(key)).and_then(|v| v.as_str()))
        .unwrap_or(key).into()
}
fn tr(state: &Shared, key: &str) -> String { translate(&state.borrow().language, key) }
fn tf(state: &Shared, key: &str, args: &[(&str, &str)]) -> String {
    let mut value = tr(state, key);
    for (key, replacement) in args { value = value.replace(&format!("{{{key}}}"), replacement); }
    value
}
fn preferences_path() -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/dev/null"));
    std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".config"))
        .join("bazzite-grub-studio/preferences.json")
}
fn load_prefs() -> (String, String) {
    let locale = std::env::var("LC_ALL").or_else(|_| std::env::var("LANG")).unwrap_or_else(|_| "en".into());
    let prefix = locale.split(|c| c == '_' || c == '-' || c == '.').next().unwrap_or("en");
    let default_lang = if locale.starts_with("pt") { "pt_BR" } else if LANGUAGES.iter().any(|(id, _)| *id == prefix) { prefix } else { "en" };
    let mut result = ("system".to_string(), default_lang.to_string());
    let path = preferences_path();
    if let Ok(meta) = fs::symlink_metadata(&path) {
        if !meta.file_type().is_symlink() && meta.len() < 8192 {
            if let Ok(bytes) = fs::read(&path) {
                if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                    let mode = json.get("appearance").and_then(|v| v.as_str()).unwrap_or("");
                    let lang = json.get("language").and_then(|v| v.as_str()).unwrap_or("");
                    if matches!(mode, "dark" | "light" | "system") { result.0 = mode.to_string(); }
                    if LANGUAGES.iter().any(|(id, _)| *id == lang) { result.1 = lang.to_string(); }
                }
            }
        }
    }
    result
}
fn save_prefs(s: &StudioState) {
    let path = preferences_path();
    if let Some(parent) = path.parent() {
        if fs::create_dir_all(parent).is_err() { return; }
        if fs::symlink_metadata(parent).map(|m| m.file_type().is_symlink()).unwrap_or(true) { return; }
        if fs::symlink_metadata(&path).map(|m| m.file_type().is_symlink()).unwrap_or(false) { return; }
        let temp = parent.join(format!(".preferences-{}-{}.tmp", std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos()));
        let content = serde_json::json!({"appearance":s.appearance, "language":s.language}).to_string();
        if fs::write(&temp, content).is_ok() { let _ = fs::rename(temp, path); }
    }
}
fn demo_snapshot() -> Snapshot {
    let mut snapshot = inspect::inspect();
    snapshot.profile.name = "Bazzite KDE (demo)".into();
    snapshot.profile.writable = true;
    snapshot.profile.reasons.clear();
    snapshot.profile.family = core::Family::Bazzite;
    snapshot.config.insert("GRUB_TIMEOUT".into(), "5".into());
    snapshot.config.insert("GRUB_TIMEOUT_STYLE".into(), "menu".into());
    snapshot.config.insert("GRUB_GFXMODE".into(), "1920x1080".into());
    snapshot.config.insert("GRUB_THEME".into(), "/boot/grub2/themes/legion/theme.txt".into());
    snapshot.themes = vec![
        Theme {name:"legion".into(), path:"/boot/grub2/themes/legion/theme.txt".into()},
        Theme {name:"lenovo".into(), path:"/boot/grub2/themes/lenovo/theme.txt".into()},
        Theme {name:"asus-rog".into(), path:"/boot/grub2/themes/asus-rog/theme.txt".into()},
    ];
    snapshot.systems = vec![
        OsRecord {name:"Windows Boot Manager".into(), source:"UEFI".into(),
            detail:"Boot0001* Windows Boot Manager".into()},
        OsRecord {name:"Fedora (Bazzite)".into(), source:"UEFI".into(),
            detail:"Boot0002* Fedora (GRUB)".into()},
    ];
    snapshot.deployments = vec!["0 · bazzite · 44.20261113 (booted)".into(), "1 · bazzite · 44.20261106".into()];
    snapshot.backups = vec!["20261113-130311-1024".into(), "20261112-214500-1124".into()];
    snapshot.menu = vec![
        MenuEntry {id:"gs-0123456789ab".into(), title:"Windows 11".into(), uuid:"9A4C-10EF".into(),
            path:"/EFI/Microsoft/Boot/bootmgfw.efi".into(), enabled:true},
    ];
    snapshot.generated = vec!["Bazzite Linux".into(), "Bazzite Linux (rollback)".into(), "Windows Boot Manager".into()];
    snapshot
}
fn run_admin(args: &[String]) -> Result<String, String> {
    let pkexec = inspect::program_path("pkexec").ok_or("Polkit / pkexec is missing")?;
    let exe = std::env::current_exe().map_err(|e| format!("Cannot locate application executable: {e}"))?;
    let helper = exe.parent().ok_or("Invalid application path")?.join("grub-studio-helper");
    if !helper.is_file() { return Err(format!("Missing separate helper executable: {}", helper.display())); }
    // The AppImage runtime normally mounts its SquashFS through FUSE for the
    // desktop user. A root process launched by pkexec may not be allowed to
    // traverse that mount. Copy just the compiled helper to a private,
    // short-lived real-filesystem directory before invoking Polkit.
    let staged = StagedHelper::new(&helper)?;
    let result = Command::new(pkexec).arg(&staged.path).args(args).output()
        .map_err(|e| format!("Cannot launch Polkit: {e}"))?;
    if result.status.success() { Ok(String::from_utf8_lossy(&result.stdout).into_owned()) }
    else if result.status.code() == Some(126) { Err("Administrator authentication cancelled".into()) }
    else {
        let stderr = String::from_utf8_lossy(&result.stderr);
        Err(format!("Operation denied or failed: {}", stderr.chars().take(1800).collect::<String>()))
    }
}
struct StagedHelper { path: PathBuf, directory: PathBuf }
impl StagedHelper {
    fn new(source: &std::path::Path) -> Result<Self, String> {
        let file_type = fs::symlink_metadata(source)
            .map_err(|e| format!("Cannot inspect administrator helper: {e}"))?.file_type();
        if !file_type.is_file() || file_type.is_symlink() {
            return Err("Administrator helper must be a regular file".into());
        }
        let mut entropy = [0_u8; 16];
        fs::File::open("/dev/urandom").and_then(|mut file| file.read_exact(&mut entropy))
            .map_err(|e| format!("Cannot create a secure temporary helper directory: {e}"))?;
        let nonce = entropy.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let directory = std::env::temp_dir().join(format!("grub-studio-helper-{}-{nonce}", std::process::id()));
        fs::DirBuilder::new().mode(0o700).create(&directory)
            .map_err(|e| format!("Cannot create temporary helper directory: {e}"))?;
        let path = directory.join("grub-studio-helper");
        if let Err(e) = fs::copy(source, &path)
            .and_then(|_| fs::set_permissions(&path, fs::Permissions::from_mode(0o700))) {
            let _ = fs::remove_file(&path);
            let _ = fs::remove_dir(&directory);
            return Err(format!("Cannot prepare administrator helper: {e}"));
        }
        Ok(Self { path, directory })
    }
}
impl Drop for StagedHelper {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        let _ = fs::remove_dir(&self.directory);
    }
}
fn apply_probe(s: &mut StudioState, raw: &str, deep: bool, preserve_drafts: bool) -> Result<(), String> {
    let header = if deep { "GRUB_STUDIO_DEEP_V2" } else { "GRUB_STUDIO_PROBE_V2" };
    if raw.lines().next() != Some(header) { return Err("Invalid privileged scan output".into()); }
    if !deep && !raw.lines().any(|line| line.starts_with("PROFILE\t")) {
        return Err("Incomplete privileged scan (missing system profile)".into());
    }
    let active_theme = s.snapshot.config.get("GRUB_THEME").cloned().unwrap_or_default();
    let form_dirty = s.menu_form_draft.as_ref().is_some_and(|draft| {
        if let Some(entry) = draft.selected.and_then(|i| s.pending_entries.get(i)) {
            draft.title != entry.title || draft.uuid != entry.uuid || draft.path != entry.path
        } else {
            !draft.title.is_empty() || !draft.uuid.is_empty()
                || (draft.path != "/EFI/Microsoft/Boot/bootmgfw.efi" && !draft.path.is_empty())
        }
    });
    let keep_drafts = preserve_drafts && (s.pending_entries != s.snapshot.menu
        || form_dirty || s.selected_theme != active_theme);
    let saved_entries = if keep_drafts { Some((s.pending_entries.clone(),s.selected_entry,
        s.menu_form_draft.clone(),s.selected_theme.clone())) } else { None };
    let previously_found = if deep { Vec::new() } else { s.snapshot.themes.clone() };
    let mut profile_received = false;
    if !deep {
        s.snapshot.themes.clear();
        s.snapshot.systems.clear();
        s.snapshot.backups.clear();
        s.snapshot.menu.clear();
        s.snapshot.generated.clear();
        s.snapshot.deployments.clear();
        s.snapshot.config.clear();
    }
    for line in raw.lines().skip(1) {
        let p: Vec<&str> = line.split('\t').collect();
        match p.as_slice() {
            ["PROFILE", family, writable, name, reason] if !deep => {
                profile_received = true;
                s.snapshot.profile.writable = *writable == "1";
                s.snapshot.profile.name = (*name).to_string();
                s.snapshot.profile.reasons = if reason.is_empty() { vec![] } else { vec![(*reason).to_string()] };
                s.snapshot.profile.family = match *family {
                    "bazzite" => core::Family::Bazzite, "fedora" => core::Family::Fedora,
                    "debian" => core::Family::Debian, "suse" => core::Family::Suse,
                    "arch" => core::Family::Arch, _ => core::Family::ReadOnly,
                };
            }
            ["CONFIG", key, val] if !deep => { s.snapshot.config.insert((*key).into(), (*val).into()); }
            ["THEME", name, path] if !deep => { s.snapshot.themes.push(Theme {name:(*name).into(),path:(*path).into()}); }
            ["OS", name, source, detail] => {
                let record = OsRecord {name:(*name).into(),source:(*source).into(),detail:(*detail).into()};
                if !s.snapshot.systems.contains(&record) { s.snapshot.systems.push(record); }
            }
            ["BACKUP", name] if !deep => { s.snapshot.backups.push((*name).into()); }
            ["DEPLOY", info] if !deep => { s.snapshot.deployments.push((*info).into()); }
            ["ENTRY", id, title, uuid, path, enabled] if !deep => {
                s.snapshot.menu.push(MenuEntry {id:(*id).into(),title:(*title).into(),uuid:(*uuid).into(),
                    path:(*path).into(),enabled:*enabled=="1"});
            }
            ["GENERATED", title] if !deep => { s.snapshot.generated.push((*title).into()); }
            _ => {}
        }
    }
    if !deep {
        if !profile_received { return Err("Incomplete privileged scan (missing system profile)".into()); }
        for theme in previously_found {
            if !s.snapshot.themes.iter().any(|other| other.path==theme.path) {
                s.snapshot.themes.push(theme);
            }
        }
        s.snapshot.themes.sort_by_key(|t| (t.name.to_lowercase(),t.path.clone()));
        if let Some((entries,selected,form,theme)) = saved_entries {
            s.pending_entries = entries;
            s.selected_entry = selected;
            s.menu_form_draft = form;
            s.selected_theme = theme;
            s.log.push("Unsaved EFI/theme drafts kept during refresh".into());
        } else {
            s.pending_entries = s.snapshot.menu.clone();
            s.selected_entry = None;
            s.menu_form_draft = None;
            s.selected_theme = s.snapshot.config.get("GRUB_THEME").cloned().unwrap_or_default();
        }
    }
    Ok(())
}
fn initial_state(demo: bool) -> StudioState {
    let prefs = load_prefs();
    let mut s = StudioState {
        snapshot: if demo { demo_snapshot() } else { inspect::inspect() },
        authorized: false, demo, busy: false, language: prefs.1, appearance: prefs.0,
        active:"home".into(), selected_theme:String::new(),
        selected_backup:None, selected_entry:None, pending_entries:vec![],
        menu_form_draft:None, settings_draft:None, log:vec![], chrome:None,
    };
    if demo {
        s.authorized = true;
        s.pending_entries = s.snapshot.menu.clone();
        s.selected_theme = s.snapshot.config.get("GRUB_THEME").cloned().unwrap_or_default();
        s.log.push("DEMO: all administrative writes are disabled".into());
    } else { s.log.push("Waiting for Polkit authentication after the window opens".into()); }
    s
}
fn can_edit(s: &StudioState) -> bool {
    s.authorized && !s.demo && s.snapshot.profile.writable && !s.busy
}
fn editing_hint(state: &Shared) -> String {
    let s=state.borrow();
    if s.demo {return translate(&s.language,"hint.demo");}
    if s.busy {return translate(&s.language,"hint.busy");}
    if !s.authorized {return translate(&s.language,"hint.unlock");}
    if !s.snapshot.profile.writable {
        return s.snapshot.profile.reasons.first().cloned()
            .unwrap_or_else(||translate(&s.language,"hint.readonly"));
    }
    translate(&s.language,"hint.select")
}
fn kconfig_value(content: &str, section: &str, key: &str) -> Option<String> {
    let mut current="";
    for raw in content.lines() {
        let line=raw.trim();
        if line.starts_with('#') || line.starts_with(';') || line.is_empty() {continue;}
        if line.starts_with('[') && line.ends_with(']') {
            current=&line[1..line.len()-1];continue;
        }
        if current!=section {continue;}
        if let Some((name,value))=line.split_once('=') {
            if name.trim()==key {return Some(value.trim().to_owned());}
        }
    }
    None
}
fn desktop_config_path(name: &str) -> PathBuf {
    let fallback=std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(||PathBuf::from("/dev/null"));
    std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from)
        .unwrap_or_else(||fallback.join(".config")).join(name)
}
fn cursor_choice(kde_config: &str, env_theme: Option<&str>, env_size: Option<&str>) -> (Option<String>, Option<i32>) {
    let theme=env_theme.filter(|name| !name.trim().is_empty()).map(str::to_owned)
        .or_else(||kconfig_value(kde_config,"Mouse","cursorTheme"))
        .filter(|name| !name.is_empty() && name.len()<=128 && name.bytes().all(|b|
            b.is_ascii_alphanumeric() || matches!(b,b'-'|b'_'|b'.')));
    let size=env_size.map(str::to_owned)
        .or_else(||kconfig_value(kde_config,"Mouse","cursorSize"))
        .and_then(|value|value.parse::<i32>().ok()).filter(|size|(8..=256).contains(size));
    (theme,size)
}
fn apply_desktop_cursor() {
    // linuxdeploy brings GTK and libXcursor, but the theme itself belongs to
    // the host KDE session. Honor the session environment first and Plasma's
    // kcminputrc second. Never ship or force a cursor theme in the AppImage.
    let kde=fs::read_to_string(desktop_config_path("kcminputrc")).unwrap_or_default();
    let theme=std::env::var("XCURSOR_THEME").ok();
    let size=std::env::var("XCURSOR_SIZE").ok();
    let (theme,size)=cursor_choice(&kde,theme.as_deref(),size.as_deref());
    if let Some(settings)=gtk::Settings::default() {
        if let Some(theme)=theme {settings.set_property("gtk-cursor-theme-name",theme);}
        if let Some(size)=size {settings.set_property("gtk-cursor-theme-size",size);}
    }
}
fn system_prefers_dark() -> bool {
    if let Some(settings)=gtk::Settings::default() {
        let theme: String=settings.property("gtk-theme-name");
        if theme.to_ascii_lowercase().contains("dark") {return true;}
        if settings.property::<bool>("gtk-application-prefer-dark-theme") {return true;}
    }
    // KDE doesn't always bridge its color scheme into GTK settings.
    let kde=fs::read_to_string(desktop_config_path("kdeglobals")).unwrap_or_default();
    if let Some(name)=kconfig_value(&kde,"General","ColorScheme") {
        let lower=name.to_ascii_lowercase();
        if lower.contains("dark") || lower.contains("breeze-dark") {return true;}
        if lower.contains("light") || lower=="breeze" {return false;}
    }
    if let Some(rgb)=kconfig_value(&kde,"Colors:Window","BackgroundNormal") {
        let items=rgb.split(',').map(|n|n.trim().parse::<u8>()).collect::<Result<Vec<_>,_>>();
        if let Ok(items)=items {if items.len()==3 {
            let brightness=items[0] as u32*299+items[1] as u32*587+items[2] as u32*114;
            return brightness<128_000;
        }}
    }
    false
}
fn apply_appearance(appearance: &str) -> bool {
    let manager=adw::StyleManager::default();
    match appearance {
        "light"=>manager.set_color_scheme(adw::ColorScheme::ForceLight),
        "dark"=>manager.set_color_scheme(adw::ColorScheme::ForceDark),
        _=>{
            // Some KDE sessions do not advertise the color scheme to the
            // portal. Retain the KDE fallback from pre-Adwaita releases.
            let scheme=if manager.system_supports_color_schemes() {
                adw::ColorScheme::Default
            } else if system_prefers_dark() {
                adw::ColorScheme::ForceDark
            } else {
                adw::ColorScheme::ForceLight
            };
            manager.set_color_scheme(scheme);
        }
    }
    manager.is_dark()
}
fn show_message(window: &gtk::ApplicationWindow, title: &str, body: &str, error: bool) {
    let dialog = gtk::MessageDialog::builder()
        .transient_for(window).modal(true)
        .message_type(if error { gtk::MessageType::Error } else { gtk::MessageType::Info })
        .buttons(gtk::ButtonsType::Ok).text(title).secondary_text(body).build();
    dialog.connect_response(|dialog, _| dialog.close());
    dialog.present();
}
fn confirm(window: &gtk::ApplicationWindow, title: &str, message: &str, action: impl FnOnce() + 'static) {
    let dialog = gtk::MessageDialog::builder()
        .transient_for(window).modal(true).message_type(gtk::MessageType::Warning)
        .buttons(gtk::ButtonsType::YesNo).text(title).secondary_text(message).build();
    let f = Rc::new(RefCell::new(Some(action)));
    dialog.connect_response(move |dialog, response| {
        dialog.close();
        if response == gtk::ResponseType::Yes {
            if let Some(f) = f.borrow_mut().take() { f(); }
        }
    });
    dialog.present();
}
fn submit(window: &gtk::ApplicationWindow, shared: &Shared, args: Vec<String>, deep: bool) {
    if shared.borrow().busy { return; }
    if shared.borrow().demo {
        show_message(window, "Demo", "Preview mode never writes to the system.", false);
        return
    }
    let probing = args.first().map(String::as_str) == Some("--probe");
    let initial_probe = probing && !shared.borrow().authorized;
    let operation = args.first().cloned().unwrap_or_default();
    if !shared.borrow().authorized && !probing {
        show_message(window, "Polkit", "Select Unlock in the top bar to request administrator authentication.", true);
        return
    }
    {
        let mut s = shared.borrow_mut();
        s.busy = true;
        s.log.push(format!("START: {}", args.first().cloned().unwrap_or_default()));
    }
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = run_admin(&args).and_then(|output| {
            if deep { Ok((output, None)) }
            else if args.first().map(String::as_str) == Some("--probe") {
                Ok(("System scan complete".into(), Some(output)))
            }
            else {
                let refreshed = run_admin(&["--probe".into()]).ok();
                Ok((output, refreshed))
            }
        });
        let _ = tx.send(result);
    });
    let window = window.clone();
    let state = shared.clone();
    glib::timeout_add_local(Duration::from_millis(100), move || match rx.try_recv() {
        Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
        result => {
            let response = match result {
                Ok(Ok((message, scan))) => {
                    let mut s = state.borrow_mut();
                    if deep {
                        if let Err(e) = apply_probe(&mut s, &message, true, false) { s.log.push(e); }
                    } else if let Some(scan) = scan {
                        match apply_probe(&mut s, &scan, false, probing && !initial_probe) {
                            Ok(()) => { s.authorized = true; s.log.push("Polkit authentication and privileged scan succeeded".into()); }
                            Err(e) => { s.authorized = false; s.log.push(e); }
                        }
                    }
                    if matches!(operation.as_str(), "--apply" | "--kernel-args" | "--restore") {
                        s.settings_draft = None;
                    }
                    s.log.push(message.chars().take(1400).collect());
                    Ok(message)
                }
                Ok(Err(e)) => {
                    let mut s=state.borrow_mut();
                    if probing {s.authorized=false;}
                    s.log.push(e.clone());
                    Err(e)
                }
                Err(_) => Err("Administrative worker stopped unexpectedly".into()),
            };
            state.borrow_mut().busy = false;
            rebuild(&window, &state);
            match response {
                Ok(info) => if !deep && !initial_probe { show_message(&window, "GRUB Studio", &info, false); },
                Err(err) => show_message(&window, "GRUB Studio", &err, true),
            }
            glib::ControlFlow::Break
        }
    });
}
fn guarded_submit(window: &gtk::ApplicationWindow, state: &Shared, args: Vec<String>, note: String) {
    let w = window.clone();
    let s = state.clone();
    confirm(window, &tr(state, "dialog.confirm"),
        &format!("{}\n\n{}", note, tr(state, "dialog.risk")), move || submit(&w, &s, args, false));
}

fn add_text(parent: &gtk::Box, text: impl AsRef<str>, class: &str, wrap: bool) -> gtk::Label {
    let label = gtk::Label::new(Some(text.as_ref()));
    label.set_xalign(0.0);
    label.set_wrap(wrap);
    label.add_css_class(class);
    parent.append(&label);
    label
}
fn button(label: &str, primary: bool) -> gtk::Button {
    let b = gtk::Button::with_label(label);
    b.add_css_class(if primary { "suggested-action" } else { "flat" });
    b.add_css_class("action-button");
    b
}
fn preference_choice(label: &str, group: &str, id: &str, selected: bool) -> gtk::Button {
    let choice=gtk::Button::with_label(label);
    choice.add_css_class("prefs-choice");
    choice.add_css_class(group);
    choice.set_widget_name(id);
    choice.set_hexpand(true);
    if selected {choice.add_css_class("prefs-selected");}
    choice
}

// Find a button in the preferences popover without relying on the visual
// language or the order of GTK's internal widget children.
fn find_preference_choice(widget: &gtk::Widget, group: &str, id: &str) -> Option<gtk::Button> {
    if widget.has_css_class(group) && widget.widget_name()==id {
        return widget.clone().downcast::<gtk::Button>().ok();
    }
    let mut child=widget.first_child();
    while let Some(widget)=child {
        if let Some(found)=find_preference_choice(&widget,group,id) {return Some(found);}
        child=widget.next_sibling();
    }
    None
}
fn popup_button(label: &str, contents: &gtk::Box) -> gtk::MenuButton {
    let toggle = gtk::MenuButton::new();
    toggle.set_label(label);
    toggle.add_css_class("action-button");
    toggle.add_css_class("popup-action");
    let popover = gtk::Popover::new();
    popover.set_child(Some(contents));
    toggle.set_popover(Some(&popover));
    toggle
}
fn popup_contents() -> gtk::Box {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 5);
    content.add_css_class("popup-contents");
    content
}
fn action_row() -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    row.add_css_class("actions");
    row
}
fn action_flow() -> gtk::FlowBox {
    let flow=gtk::FlowBox::new();
    flow.add_css_class("action-flow");
    flow.set_selection_mode(gtk::SelectionMode::None);
    flow.set_min_children_per_line(1);
    flow.set_max_children_per_line(6);
    flow.set_column_spacing(6);
    flow.set_row_spacing(6);
    flow
}
fn card() -> gtk::Box {
    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 10);
    box_.add_css_class("studio-card");
    box_
}
fn section(parent: &gtk::Box, heading: &str) -> gtk::Box {
    let content = card();
    add_text(&content, heading, "section-heading", true);
    parent.append(&content);
    content
}
fn page_root(title: &str, subtitle: &str) -> gtk::Box {
    let v = gtk::Box::new(gtk::Orientation::Vertical, 10);
    v.set_hexpand(true);
    v.set_margin_top(15); v.set_margin_bottom(18);
    v.set_margin_start(16); v.set_margin_end(16);
    add_text(&v, title, "page-title", true);
    add_text(&v, subtitle, "muted", true);
    v
}
fn row_with_heading(title: &str, subtitle: &str) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Vertical, 3);
    add_text(&row, title, "row-title", true);
    add_text(&row, subtitle, "muted", true);
    row
}
fn form_row(parent: &gtk::Box, name: &str, widget: &impl IsA<gtk::Widget>) {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let label = gtk::Label::new(Some(name));
    label.set_xalign(0.0); label.set_hexpand(true); label.set_wrap(true);
    row.append(&label);
    widget.set_hexpand(true);
    widget.set_size_request(135, -1);
    row.append(widget);
    parent.append(&row);
}

fn page_home(_window: &gtk::ApplicationWindow, state: &Shared) -> gtk::Box {
    let snapshot = state.borrow().snapshot.clone();
    let page = page_root(&tr(state, "home.title"), &tr(state, "home.sub"));
    let banner = card();
    banner.add_css_class("hero");
    add_text(&banner,&tr(state,"home.distribution"),"section-heading",true);
    add_text(&banner, &snapshot.profile.name, "hero-title", true);
    add_text(&banner, &format!("{} · {} · {}",
        snapshot.profile.family.label(), snapshot.profile.bootloader,
        if !state.borrow().authorized {tr(state,"status.locked")}
        else if snapshot.profile.writable {tr(state,"status.ready")}
        else {tr(state,"status.readonly")}),
        "muted", true);
    if !snapshot.profile.writable {
        add_text(&banner, &snapshot.profile.reasons.join(" • "), "warning", true);
    }
    page.append(&banner);

    // The dashboard is a true, full-width 2x2 grid. Without homogeneous
    // columns GTK measures each cell only by its label and leaves the rest of
    // the content area empty (especially noticeable on widescreen displays).
    let stats=gtk::Grid::new();
    stats.add_css_class("stats-grid");
    stats.set_hexpand(true);
    stats.set_halign(gtk::Align::Fill);
    stats.set_column_homogeneous(true);
    stats.set_row_homogeneous(true);
    stats.set_row_spacing(10);stats.set_column_spacing(10);
    let current_theme=snapshot.config.get("GRUB_THEME").and_then(|s|s.split('/').rev().nth(1))
        .unwrap_or("—");
    for (position,title,value) in [
        (0,tr(state,"home.theme"),current_theme.to_string()),
        (1,tr(state,"themes.installed"),snapshot.themes.len().to_string()),
        (2,tr(state,"home.others"),snapshot.systems.len().to_string()),
        (3,tr(state,"home.bootloader"),snapshot.profile.bootloader.clone()),
    ] {
        let tile=card();tile.add_css_class("metric-card");
        tile.set_hexpand(true);
        tile.set_halign(gtk::Align::Fill);
        tile.set_vexpand(true);
        add_text(&tile,title,"metric-label",true);
        add_text(&tile,value,"metric-value",true);
        stats.attach(&tile,position%2,position/2,1,1);
    }
    page.append(&stats);
    if !snapshot.deployments.is_empty(){
        let overview = section(&page, &tr(state, "home.deployments"));
        for dep in &snapshot.deployments { add_text(&overview, dep, "item-label", true); }
    }
    let warning = section(&page, &tr(state, "home.warning.title"));
    warning.add_css_class("warning-card");
    add_text(&warning, &tr(state,"home.warning.text"), "warning", true);
    page
}

fn combined_boot_records(generated: &[String], systems: &[OsRecord]) -> Vec<(String,Vec<String>)> {
    let mut rows:Vec<(String,Vec<String>)>=Vec::new();
    for name in generated {
        if let Some(row)=rows.iter_mut().find(|(title,_)|title.eq_ignore_ascii_case(name)) {
            if !row.1.iter().any(|source|source=="GRUB") {row.1.push("GRUB".into());}
        } else {rows.push((name.clone(),vec!["GRUB".into()]));}
    }
    for record in systems {
        let evidence=if record.detail.is_empty() {record.source.clone()}
            else {format!("{} · {}",record.source,record.detail)};
        if let Some(row)=rows.iter_mut().find(|(title,_)|title.eq_ignore_ascii_case(&record.name)) {
            if !row.1.contains(&evidence) {row.1.push(evidence);}
        } else {rows.push((record.name.clone(),vec![evidence]));}
    }
    rows
}

fn systems_panel(window: &gtk::ApplicationWindow, state: &Shared) -> gtk::Box {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 10);
    add_text(&page,&tr(state,"menu.discovery.note"),"muted",true);
    // GRUB menu entries and discovered EFI records describe the same system
    // in many installations. Merge identical names and keep both evidence
    // sources on one row so Windows is not listed twice.
    let snapshot=state.borrow();
    let rows=combined_boot_records(&snapshot.snapshot.generated,&snapshot.snapshot.systems);
    drop(snapshot);
    if rows.is_empty() {add_text(&page,&tr(state,"systems.empty"),"empty-state",true);}
    else {
        let list=gtk::ListBox::new();
        list.add_css_class("studio-list");
        list.set_selection_mode(gtk::SelectionMode::None);
        for (name,sources) in &rows {
            let row=gtk::Box::new(gtk::Orientation::Vertical,3);
            row.set_margin_top(8);row.set_margin_bottom(8);
            row.set_margin_start(12);row.set_margin_end(12);
            add_text(&row,name,"row-title",true);
            let detail=sources.join("  ·  ");
            let secondary=add_text(&row,&detail,"muted",false);
            secondary.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
            secondary.set_tooltip_text(Some(&detail));
            list.append(&row);
        }
        page.append(&list);
    }
    // Global refresh is available in the top bar. A deep os-prober scan is
    // a secondary optional action; it must not dominate this read-only list.
    let deep=button(&tr(state,"systems.deep"),false);
    deep.set_halign(gtk::Align::Start);
    deep.set_sensitive(!state.borrow().demo && !state.borrow().busy);
    if state.borrow().demo {deep.set_tooltip_text(Some(&editing_hint(state)));}
    let w=window.clone(); let s=state.clone();
    deep.connect_clicked(move |_| {
        let w2=w.clone(); let s2=s.clone();
        confirm(&w, &tr(&s,"systems.deep.title"), &tr(&s,"systems.deep.ask"),
            move || submit(&w2, &s2, vec!["--scan-deep".into()], true));
    });
    page.append(&deep);
    add_text(&page,&tr(state,"systems.footnote"),"muted",true);
    page
}

fn entry_with_text(value: &str) -> gtk::Entry {
    let entry = gtk::Entry::new();
    entry.set_text(value);
    entry
}
fn page_settings(window: &gtk::ApplicationWindow, state: &Shared) -> gtk::Box {
    let page=page_root(&tr(state,"settings.title"), &tr(state,"settings.sub"));
    let config=state.borrow().snapshot.config.clone();
    let writable=can_edit(&state.borrow());
    let draft=state.borrow().settings_draft.clone()
        .unwrap_or_else(|| SettingsDraft::from_config(&config));
    if !writable {
        let current=SettingsDraft::from_config(&config);
        add_text(&page,editing_hint(state),"warning",true);
        let summary=section(&page,&tr(state,"settings.general"));
        for (key,value) in [
            ("settings.timeout",current.timeout.as_str()),
            ("settings.style",current.style.as_str()),
            ("settings.resolution",current.gfx.as_str()),
        ] {
            summary.append(&row_with_heading(&tr(state,key),value));
        }
        if !current.kernel_args.is_empty() {
            summary.append(&row_with_heading(&tr(state,"menu.kernel"),&current.kernel_args));
        }
        return page;
    }
    let needs_draft=state.borrow().settings_draft.is_none();
    if needs_draft {
        state.borrow_mut().settings_draft=Some(draft.clone());
    }
    let grid=section(&page,&tr(state,"settings.general"));
    let timeout=entry_with_text(&draft.timeout);
    timeout.set_sensitive(writable);
    let s=state.clone();
    timeout.connect_changed(move |entry| {
        if let Some(draft)=s.borrow_mut().settings_draft.as_mut() {
            draft.timeout=entry.text().to_string();
        }
    });
    form_row(&grid, &tr(state,"settings.timeout"), &timeout);
    let styles=["menu","hidden","countdown"];
    let style_labels=[tr(state,"settings.style.menu"),tr(state,"settings.style.hidden"),
        tr(state,"settings.style.countdown")];
    let style_refs=style_labels.iter().map(String::as_str).collect::<Vec<_>>();
    let style=gtk::DropDown::from_strings(&style_refs);
    style.set_selected(styles.iter().position(|x| *x==draft.style.as_str()).unwrap_or(0) as u32);
    style.set_sensitive(writable);
    let s=state.clone();
    style.connect_selected_notify(move |combo| {
        if let Some(draft)=s.borrow_mut().settings_draft.as_mut() {
            draft.style=styles.get(combo.selected() as usize).unwrap_or(&"menu").to_string();
        }
    });
    form_row(&grid,&tr(state,"settings.style"),&style);
    // Offer known-safe presets instead of requiring users to type the GRUB
    // resolution syntax. Retain an existing distro-specific value as a choice.
    let mut gfx_choices=["auto","640x480","800x600","1024x768","1280x720",
        "1280x800","1366x768","1600x900","1920x1080","2560x1440","3840x2160"]
        .iter().map(|value|value.to_string()).collect::<Vec<_>>();
    if !gfx_choices.contains(&draft.gfx) {gfx_choices.push(draft.gfx.clone());}
    let gfx_refs=gfx_choices.iter().map(String::as_str).collect::<Vec<_>>();
    let gfx=gtk::DropDown::from_strings(&gfx_refs);
    gfx.set_selected(gfx_choices.iter().position(|value|value==&draft.gfx).unwrap_or(0) as u32);
    gfx.set_sensitive(writable);
    let s=state.clone();
    let gfx_for_state=gfx_choices.clone();
    gfx.connect_selected_notify(move |combo| {
        if let Some(draft)=s.borrow_mut().settings_draft.as_mut() {
            if let Some(value)=gfx_for_state.get(combo.selected() as usize) {draft.gfx=value.clone();}
        }
    });
    form_row(&grid,&tr(state,"settings.resolution"),&gfx);
    // Theme selection belongs exclusively to the Theme Library. Saving
    // basic settings must preserve GRUB_THEME and never accidentally apply
    // an unsaved theme preview selected on another page.
    let current_theme=config.get("GRUB_THEME").cloned().unwrap_or_default();
    add_text(&grid,&tr(state,"settings.timeouthelp"),"muted",true);
    let row=action_row();
    let save=button(&tr(state,"settings.save"),true);
    save.set_sensitive(writable);
    if !writable {save.set_tooltip_text(Some(&editing_hint(state)));}
    let w=window.clone(); let s=state.clone();
    let gfx_for_save=gfx_choices.clone();
    save.connect_clicked(move |_| {
        let timeout=timeout.text().to_string();
        let gfx=gfx_for_save.get(gfx.selected() as usize).cloned().unwrap_or_else(||"auto".into());
        let style=styles.get(style.selected() as usize).unwrap_or(&"menu").to_string();
        if !core::valid_timeout(&timeout) || !core::valid_style(&style) || !core::valid_resolution(&gfx)
            || !core::valid_theme_path(&current_theme) {
            show_message(&w, &tr(&s,"dialog.invalid"), &tr(&s,"dialog.invalidsettings"), true); return;
        }
        guarded_submit(&w,&s,vec!["--apply".into(),timeout,style,gfx,current_theme.clone()],tr(&s,"dialog.applysettings"));
    });
    row.append(&save);
    page.append(&row);
    let visibility=section(&page,&tr(state,"settings.visibility"));
    add_text(&visibility,&tr(state,"settings.visibility.note"),"muted",true);
    let actions=action_row();
    for (id,key) in [("show","settings.visibility.show"),("auto","settings.visibility.auto"),("hide","settings.visibility.hide")] {
        let b=button(&tr(state,key),false);
        b.set_sensitive(writable && matches!(state.borrow().snapshot.profile.family,core::Family::Bazzite|core::Family::Fedora));
        let w=window.clone(); let s=state.clone();
        b.connect_clicked(move |_| guarded_submit(&w,&s,vec!["--visibility".into(),id.into()],tr(&s,"dialog.applyvisibility")));
        actions.append(&b);
    }
    visibility.append(&actions);
    let kernel=section(&page,&tr(state,"menu.kernel"));
    add_text(&kernel,&tr(state,"menu.kernel.note"),"muted",true);
    let kargs=entry_with_text(&draft.kernel_args);
    kargs.set_sensitive(writable && matches!(state.borrow().snapshot.profile.family,core::Family::Arch|core::Family::Debian));
    let s=state.clone();
    kargs.connect_changed(move |entry| {
        if let Some(draft)=s.borrow_mut().settings_draft.as_mut() {
            draft.kernel_args=entry.text().to_string();
        }
    });
    kernel.append(&kargs);
    let save_kernel=button(&tr(state,"menu.kernel.save"),false);
    save_kernel.set_sensitive(writable && matches!(state.borrow().snapshot.profile.family,core::Family::Arch|core::Family::Debian));
    let w=window.clone(); let s=state.clone();
    save_kernel.connect_clicked(move |_| {
        let value=kargs.text().to_string();
        if !core::valid_kernel_args(&value) { show_message(&w,"GRUB",&tr(&s,"dialog.invalidkernel"),true); return; }
        guarded_submit(&w,&s,vec!["--kernel-args".into(),value.clone()],
            tf(&s,"menu.kernel.confirm",&[("args",&value)]));
    });
    kernel.append(&save_kernel);
    page
}

fn new_id() -> String {
    let nanos=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    format!("gs-{:012x}", (nanos ^ (std::process::id() as u128)) & 0xffffffffffff)
}
fn page_menu(window: &gtk::ApplicationWindow, state: &Shared) -> gtk::Box {
    let page=page_root(&tr(state,"menu.title"),&tr(state,"menu.sub"));
    let profile=state.borrow().snapshot.profile.clone();
    let editable=can_edit(&state.borrow()) && profile.family.supports_menu_edit();
    let local_editable=editable;
    let info=section(&page,&tr(state,"menu.custom"));
    if editable {add_text(&info,&tr(state,"menu.custom.note"),"muted",true);}
    if !editable {
        let why=if !profile.family.supports_menu_edit() {tr(state,"menu.readonly")} else {editing_hint(state)};
        add_text(&info,why,"warning",true);
    }
    let editor=gtk::FlowBox::new();
    editor.add_css_class("menu-editor-flow");
    editor.set_selection_mode(gtk::SelectionMode::None);
    editor.set_min_children_per_line(1);
    editor.set_max_children_per_line(2);
    editor.set_column_spacing(12);
    editor.set_row_spacing(10);
    let list_side=gtk::Box::new(gtk::Orientation::Vertical,7);
    list_side.set_size_request(265,-1);
    list_side.set_hexpand(!local_editable);
    list_side.add_css_class("menu-entry-list");
    let form_side=gtk::Box::new(gtk::Orientation::Vertical,8);
    form_side.set_size_request(335,-1);
    form_side.set_visible(local_editable);
    form_side.add_css_class("menu-entry-fields");
    add_text(&form_side,&tr(state,"menu.entry"),"section-heading",true);
    editor.insert(&list_side,-1);
    editor.insert(&form_side,-1);
    info.append(&editor);
    let list=gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.add_css_class("studio-list");
    let items=state.borrow().pending_entries.clone();
    if items.is_empty() {add_text(&list_side,&tr(state,"menu.empty"),"empty-state",true);}
    for item in &items {
        let label=row_with_heading(&item.title,&format!("{} · {} · {}",item.uuid,item.path,
            if item.enabled {tr(state,"menu.enabled")} else {tr(state,"menu.hidden")}));
        label.set_margin_top(7); label.set_margin_bottom(7); label.set_margin_start(11); label.set_margin_end(11);
        list.append(&label);
    }
    list_side.append(&list);
    let fields=gtk::Box::new(gtk::Orientation::Vertical,7);
    let title=entry_with_text(""); title.set_placeholder_text(Some(&tr(state,"menu.field.title")));
    let uuid=entry_with_text(""); uuid.set_placeholder_text(Some(&tr(state,"menu.field.uuid")));
    let path=entry_with_text("/EFI/Microsoft/Boot/bootmgfw.efi");
    title.set_sensitive(local_editable);
    uuid.set_sensitive(local_editable);
    path.set_sensitive(local_editable);
    fields.append(&title);fields.append(&uuid);fields.append(&path);
    form_side.append(&fields);
    let selected_index=state.borrow().selected_entry;
    if let Some(index)=selected_index {
        if let Some(row)=list.row_at_index(index as i32) { list.select_row(Some(&row)); }
    }
    let saved=state.borrow().menu_form_draft.clone()
        .filter(|draft| draft.selected==selected_index);
    if let Some(draft)=saved {
        title.set_text(&draft.title);
        uuid.set_text(&draft.uuid);
        path.set_text(&draft.path);
    } else if let Some(item)=selected_index.and_then(|i| state.borrow().pending_entries.get(i).cloned()) {
        title.set_text(&item.title);
        uuid.set_text(&item.uuid);
        path.set_text(&item.path);
    }
    for field in [&title,&uuid,&path] {
        let s=state.clone();
        let title=title.clone();let uuid=uuid.clone();let path=path.clone();
        field.connect_changed(move |_| {
            let selected=s.borrow().selected_entry;
            s.borrow_mut().menu_form_draft=Some(MenuFormDraft {
                selected,title:title.text().to_string(),uuid:uuid.text().to_string(),path:path.text().to_string(),
            });
        });
    }
    let s=state.clone();
    let title2=title.clone(); let uuid2=uuid.clone(); let path2=path.clone();
    list.connect_row_selected(move |_, row| {
        let index=row.map(|r| r.index() as usize);
        let selected=index.and_then(|i| s.borrow().pending_entries.get(i).cloned());
        s.borrow_mut().selected_entry=index;
        if let Some(item)=selected {
            title2.set_text(&item.title); uuid2.set_text(&item.uuid); path2.set_text(&item.path);
        } else {
            title2.set_text("");uuid2.set_text("");
            path2.set_text("/EFI/Microsoft/Boot/bootmgfw.efi");
        }
        s.borrow_mut().menu_form_draft=Some(MenuFormDraft {
            selected:index,title:title2.text().to_string(),uuid:uuid2.text().to_string(),path:path2.text().to_string(),
        });
    });
    let controls=action_flow();
    let add=button(&tr(state,"menu.add"),false);
    add.set_sensitive(local_editable);
    let w=window.clone();let s=state.clone();
    let title1=title.clone();let uuid1=uuid.clone();let path1=path.clone();
    add.connect_clicked(move |_| {
        let item=MenuEntry {id:new_id(),title:title1.text().to_string(),uuid:uuid1.text().to_string(),
            path:path1.text().to_string(),enabled:true};
        if !core::valid_entry(&item) { show_message(&w,"GRUB Studio",&tr(&s,"dialog.invalidentry"),true);return; }
        s.borrow_mut().pending_entries.push(item);
        s.borrow_mut().selected_entry=None;
        s.borrow_mut().menu_form_draft=None;
        rebuild(&w,&s);
    });
    controls.insert(&add,-1);
    let edit=button(&tr(state,"menu.edit"),false);
    edit.set_sensitive(local_editable && selected_index.is_some());
    let w=window.clone();let s=state.clone();
    edit.connect_clicked(move |_| {
        let idx=s.borrow().selected_entry;
        let Some(idx)=idx else { return };
        let Some(mut item)=s.borrow().pending_entries.get(idx).cloned() else {return};
        item.title=title.text().to_string();
        item.uuid=uuid.text().to_string();
        item.path=path.text().to_string();
        if !core::valid_entry(&item) { show_message(&w,"GRUB Studio",&tr(&s,"dialog.invalidentry"),true);return; }
        s.borrow_mut().pending_entries[idx]=item;
        s.borrow_mut().menu_form_draft=None;
        rebuild(&w,&s);
    });
    controls.insert(&edit,-1);
    // Reorder, hide and delete are contextual operations: they do not need
    // permanent toolbar space, especially at 620–760 px.
    let extra_entry=popup_contents();
    for (key,operation) in [("menu.up",-1i32),("menu.down",1),
                            ("menu.toggle",0),("menu.remove",2)] {
        let b=button(&tr(state,key),false);
        b.set_sensitive(local_editable && selected_index.is_some());
        b.set_halign(gtk::Align::Fill);
        let w=window.clone();let s=state.clone();
        b.connect_clicked(move |_| {
            let idx=s.borrow().selected_entry;
            let Some(idx)=idx else{return};
            let mut data=s.borrow_mut();
            if idx>=data.pending_entries.len(){return}
            match operation {
                -1 if idx>0 => { data.pending_entries.swap(idx,idx-1);data.selected_entry=Some(idx-1); }
                1 if idx+1<data.pending_entries.len() => {data.pending_entries.swap(idx,idx+1);data.selected_entry=Some(idx+1);}
                0 => data.pending_entries[idx].enabled=!data.pending_entries[idx].enabled,
                2 => {data.pending_entries.remove(idx);data.selected_entry=None;},
                _ => {}
            }
            data.menu_form_draft=None;
            drop(data);
            rebuild(&w,&s);
        });
        extra_entry.append(&b);
    }
    let more_entry=popup_button(&tr(state,"menu.entry.actions"),&extra_entry);
    more_entry.set_sensitive(local_editable && selected_index.is_some());
    controls.insert(&more_entry,-1);
    form_side.append(&controls);
    let commit=action_flow();
    commit.set_visible(local_editable);
    let save=button(&tr(state,"menu.save"),true);
    save.set_sensitive(editable);
    let w=window.clone();let s=state.clone();
    save.connect_clicked(move |_| {
        let entries=s.borrow().pending_entries.clone();
        match core::entries_to_tsv(&entries) {
            Ok(rows) => guarded_submit(&w,&s,vec!["--menu-update".into(),core::hex_encode(&rows)],tr(&s,"menu.save.confirm")),
            Err(e) => show_message(&w,"GRUB Studio",&e,true),
        }
    });
    commit.insert(&save,-1);
    let preview=button(&tr(state,"menu.preview"),false);
    let w=window.clone();let s=state.clone();
    preview.connect_clicked(move |_| {
        match core::render_grub_entries(&s.borrow().pending_entries) {
            Ok(script)=>show_message(&w,&tr(&s,"menu.preview"),&script,false),
            Err(e)=>show_message(&w,"GRUB Studio",&e,true),
        }
    });
    commit.insert(&preview,-1);
    let extra_save=popup_contents();
    let set_default=button(&tr(state,"menu.defaultcustom"),false);
    set_default.set_sensitive(editable && selected_index.is_some());
    set_default.set_halign(gtk::Align::Fill);
    let w=window.clone();let s=state.clone();
    set_default.connect_clicked(move |_| {
        let chosen=s.borrow().selected_entry.and_then(|i| s.borrow().pending_entries.get(i).cloned());
        let Some(item)=chosen else {show_message(&w,"GRUB Studio",&tr(&s,"menu.selectfirst"),true);return};
        guarded_submit(&w,&s,vec!["--default".into(),item.id.clone()],
            tf(&s,"menu.default.confirm",&[("id",&item.id)]));
    });
    extra_save.append(&set_default);
    let discard=button(&tr(state,"menu.discard"),false);
    discard.set_sensitive(local_editable);
    discard.set_halign(gtk::Align::Fill);
    let w=window.clone();let s=state.clone();
    discard.connect_clicked(move |_| {
        let original=s.borrow().snapshot.menu.clone();
        {
            let mut data=s.borrow_mut();
            data.pending_entries=original;
            data.selected_entry=None;
            data.menu_form_draft=None;
        }
        rebuild(&w,&s);
    });
    extra_save.append(&discard);
    let more_save=popup_button(&tr(state,"menu.more"),&extra_save);
    more_save.set_sensitive(local_editable);
    commit.insert(&more_save,-1);
    page.append(&commit);
    let snapshot=state.borrow();
    let found=combined_boot_records(&snapshot.snapshot.generated,&snapshot.snapshot.systems).len();
    drop(snapshot);
    let discovery=gtk::Expander::new(Some(&format!("{}  ({found})",tr(state,"menu.discovery"))));
    discovery.add_css_class("generated-expander");
    discovery.set_expanded(true);
    discovery.set_child(Some(&systems_panel(window,state)));
    page.append(&discovery);
    add_text(&page,&tr(state,"menu.efiwarn"),"muted",true);
    page
}

fn page_themes(window: &gtk::ApplicationWindow, state: &Shared) -> gtk::Box {
    let page=page_root(&tr(state,"themes.title"),&tr(state,"themes.sub"));
    let selected=state.borrow().selected_theme.clone();
    let themes=state.borrow().snapshot.themes.clone();
    let active=state.borrow().snapshot.config.get("GRUB_THEME").cloned().unwrap_or_default();
    let writable=can_edit(&state.borrow());
    let list=card();
    list.add_css_class("theme-browser");
    let list_header=gtk::Box::new(gtk::Orientation::Horizontal,8);
    let label=gtk::Label::new(Some(&format!("{}  ({})",tr(state,"themes.installed"),themes.len())));
    label.add_css_class("section-heading");label.set_xalign(0.0);label.set_hexpand(true);
    list_header.append(&label);
    list.append(&list_header);
    if themes.is_empty() {
        add_text(&list,&tr(state,"themes.placeholder"),"muted",true);
    }
    let selector=gtk::ListBox::new();
    selector.add_css_class("studio-list");
    for theme in &themes {
        let display=if active==theme.path {format!("●  {}",theme.name)} else {format!("◇  {}",theme.name)};
        let row=gtk::Box::new(gtk::Orientation::Horizontal,10);
        let icon=gtk::Image::from_icon_name("applications-graphics-symbolic");
        icon.set_pixel_size(20);icon.add_css_class("theme-icon-fallback");
        icon.set_size_request(42,38);
        row.append(&icon);
        let labels=gtk::Box::new(gtk::Orientation::Vertical,3);
        add_text(&labels,&display,"row-title",false);
        let location=add_text(&labels,&theme.path,"muted",false);
        location.set_max_width_chars(70);
        location.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        location.set_tooltip_text(Some(&theme.path));
        row.append(&labels);
        row.add_css_class("theme-option");
        if active==theme.path {row.add_css_class("theme-current");}
        row.set_margin_top(5);row.set_margin_bottom(5);
        row.set_margin_start(10);row.set_margin_end(10);
        selector.append(&row);
    }
    if let Some(index)=themes.iter().position(|t|t.path==selected) {
        if let Some(row)=selector.row_at_index(index as i32) {selector.select_row(Some(&row));}
    }
    let scroll=gtk::ScrolledWindow::new();
    scroll.set_policy(gtk::PolicyType::Never,gtk::PolicyType::Automatic);
    // Three installed themes fit without blank space. Longer collections
    // scroll, keeping the selected-theme actions above the window footer.
    scroll.set_min_content_height(180);
    scroll.set_max_content_height(240);
    scroll.set_propagate_natural_height(true);
    scroll.set_vexpand(false);
    scroll.set_visible(!themes.is_empty());
    scroll.set_child(Some(&selector));
    list.append(&scroll);
    let install=button(&tr(state,"themes.import"),false);
    install.set_sensitive(writable);
    let w=window.clone();let s=state.clone();
    install.connect_clicked(move |_| {
        let dialog=gtk::FileChooserNative::new(Some(&tr(&s,"themes.import")),Some(&w),
            gtk::FileChooserAction::SelectFolder,Some(&tr(&s,"themes.import")),Some(&tr(&s,"menu.cancel")));
        // NativeDialog is not a toplevel widget; keep it alive until response.
        let dialog_hold=dialog.clone();
        let w2=w.clone();let s2=s.clone();
        dialog.connect_response(move |dialog_response,response| {
            if response==gtk::ResponseType::Accept {
                if let Some(path)=dialog_response.file().and_then(|file|file.path()) {
                    let folder=path.file_name().map(|x|x.to_string_lossy().to_string()).unwrap_or_default();
                    if !core::valid_simple_name(&folder) {show_message(&w2,"GRUB Studio",&tr(&s2,"dialog.invalidthemefolder"),true);}
                    else {guarded_submit(&w2,&s2,vec!["--install-theme".into(),path.to_string_lossy().to_string(),folder],tr(&s2,"themes.import"));}
                }
            }
            dialog_hold.destroy();
        });
        dialog.show();
    });
    // Import belongs to this list and stays in its header; a separate
    // full-width button below the list needlessly pushed actions offscreen.
    list_header.append(&install);
    page.append(&list);

    // Keep primary actions fixed to the bottom of the window. At 700x540
    // the theme list may scroll, but Apply and Disable never scroll away.
    let theme_bar=state.borrow().chrome.as_ref().expect("persistent chrome").theme_actions.clone();
    let selected_name=themes.iter().find(|t|t.path==selected).map(|t|t.name.as_str()).unwrap_or("—");
    let current_name=gtk::Label::new(Some(selected_name));
    current_name.add_css_class("theme-toolbar-name");
    current_name.set_xalign(0.0);
    current_name.set_hexpand(true);
    current_name.set_ellipsize(gtk::pango::EllipsizeMode::End);
    let selected_tip=if selected.is_empty(){tr(state,"themes.selectfirst")}else{selected.clone()};
    current_name.set_tooltip_text(Some(&selected_tip));
    theme_bar.append(&current_name);
    let current_state_word=if !selected.is_empty() && selected==active {tr(state,"themes.active")}else{tr(state,"themes.detected")};
    let current_status=gtk::Label::new(Some(&current_state_word));
    current_status.set_visible(!selected.is_empty());
    current_status.add_css_class("theme-selected-status");
    current_status.add_css_class(if selected==active && !selected.is_empty() {"theme-state-active"}else{"theme-state-idle"});
    theme_bar.append(&current_status);
    let controls=action_row();
    controls.add_css_class("theme-toolbar-actions");
    let activate=button(&tr(state,"themes.use"),true);
    let can_apply_theme=writable && themes.iter().any(|t|t.path==selected) && selected!=active;
    activate.set_sensitive(can_apply_theme);
    if !can_apply_theme {
        activate.remove_css_class("suggested-action");
        activate.add_css_class("flat");
    }
    let activate_tip=if writable {tr(state,"themes.selectfirst")} else {editing_hint(state)};
    activate.set_tooltip_text(Some(&activate_tip));
    let w=window.clone();let s=state.clone();
    activate.connect_clicked(move |_| {
        let c=s.borrow().snapshot.config.clone();let selected=s.borrow().selected_theme.clone();
        if selected.is_empty() || !core::valid_theme_path(&selected){
            show_message(&w,"GRUB Studio",&tr(&s,"themes.selectfirst"),true);return;
        }
        guarded_submit(&w,&s,vec!["--apply".into(),
            c.get("GRUB_TIMEOUT").cloned().unwrap_or_else(||"5".into()),
            c.get("GRUB_TIMEOUT_STYLE").cloned().unwrap_or_else(||"menu".into()),
            c.get("GRUB_GFXMODE").cloned().unwrap_or_else(||"auto".into()),selected],tr(&s,"themes.use"));
    });
    controls.append(&activate);
    let disable=button(&tr(state,"themes.disable"),false);
    disable.set_sensitive(writable && !active.is_empty());
    if !writable {disable.set_tooltip_text(Some(&editing_hint(state)));}
    let w=window.clone();let s=state.clone();
    disable.connect_clicked(move |_| {
        let c=s.borrow().snapshot.config.clone();
        guarded_submit(&w,&s,vec!["--apply".into(),
            c.get("GRUB_TIMEOUT").cloned().unwrap_or_else(||"5".into()),
            c.get("GRUB_TIMEOUT_STYLE").cloned().unwrap_or_else(||"menu".into()),
            c.get("GRUB_GFXMODE").cloned().unwrap_or_else(||"auto".into()),String::new()],tr(&s,"themes.disable"));
    });
    controls.append(&disable);
    theme_bar.append(&controls);
    theme_bar.set_visible(true);
    let s=state.clone();
    let themes_for_selection=themes.clone();
    let active_theme=active.clone();
    let active_word=tr(state,"themes.active");
    let detected_word=tr(state,"themes.detected");
    selector.connect_row_selected(move |_,row| {
        if let Some(row)=row {
            if let Some(theme)=themes_for_selection.get(row.index() as usize) {
                s.borrow_mut().selected_theme=theme.path.clone();
                current_name.set_text(&theme.name);
                current_name.set_tooltip_text(Some(&theme.path));
                current_status.set_visible(true);
                current_status.set_text(if theme.path==active_theme {&active_word}else{&detected_word});
                current_status.remove_css_class("theme-state-active");
                current_status.remove_css_class("theme-state-idle");
                current_status.add_css_class(if theme.path==active_theme {"theme-state-active"}else{"theme-state-idle"});
                let can_apply=can_edit(&s.borrow()) && theme.path!=active_theme;
                activate.set_sensitive(can_apply);
                let tip=if can_apply {tr(&s,"themes.use")}
                    else if !can_edit(&s.borrow()) {editing_hint(&s)}
                    else {tr(&s,"themes.active")};
                activate.set_tooltip_text(Some(&tip));
                activate.remove_css_class(if can_apply {"flat"}else{"suggested-action"});
                activate.add_css_class(if can_apply {"suggested-action"}else{"flat"});
            }
        }
    });
    page
}

fn backups_panel(window: &gtk::ApplicationWindow, state: &Shared) -> gtk::Box {
    let page=gtk::Box::new(gtk::Orientation::Vertical,9);
    add_text(&page,&tr(state,"backups.sub"),"muted",true);
    let controls=action_row();
    let create=button(&tr(state,"backups.create"),true);
    create.set_sensitive(can_edit(&state.borrow()));
    if !can_edit(&state.borrow()) {create.set_tooltip_text(Some(&editing_hint(state)));}
    let w=window.clone();let s=state.clone();
    create.connect_clicked(move |_| submit(&w,&s,vec!["--backup".into()],false));
    controls.append(&create);
    let restore=button(&tr(state,"backups.restore"),false);
    restore.set_sensitive(can_edit(&state.borrow()));
    if !can_edit(&state.borrow()) {restore.set_tooltip_text(Some(&editing_hint(state)));}
    let w=window.clone();let s=state.clone();
    restore.connect_clicked(move |_| {
        let Some(name)=s.borrow().selected_backup.clone() else {
            show_message(&w,"GRUB Studio",&tr(&s,"dialog.selectbackup"),true);return;
        };
        guarded_submit(&w,&s,vec!["--restore".into(),name.clone()],tf(&s,"dialog.restore",&[("name",&name)]));
    });
    controls.append(&restore);
    page.append(&controls);
    let box_=section(&page,&tr(state,"backups.saved"));
    let backups=state.borrow().snapshot.backups.clone();
    if backups.is_empty() {add_text(&box_,&tr(state,"backups.empty"),"muted",true);}
    let list=gtk::ListBox::new();
    list.add_css_class("studio-list");
    for backup in &backups {
        let label=row_with_heading(backup,"/var/lib/…/backups/");
        label.set_margin_top(8);label.set_margin_bottom(8);label.set_margin_start(11);label.set_margin_end(11);
        list.append(&label);
    }
    let s=state.clone();
    list.connect_row_selected(move |_,r| {
        if let Some(r)=r {s.borrow_mut().selected_backup=backups.get(r.index() as usize).cloned();}
    });
    box_.append(&list);
    add_text(&page,&tr(state,"backups.note"),"muted",true);
    page
}
fn logs_panel(state: &Shared) -> gtk::Box {
    let page=gtk::Box::new(gtk::Orientation::Vertical,9);
    add_text(&page,&tr(state,"logs.sub"),"muted",true);
    let row=action_row();
    let copy=button(&tr(state,"logs.copy"),false);
    let log=state.borrow().log.join("\n\n");
    let text=log.clone();
    copy.connect_clicked(move |_| {if let Some(display)=gdk::Display::default(){ display.clipboard().set_text(&text);}});
    row.append(&copy);page.append(&row);
    let box_=section(&page,"");
    let textview=gtk::TextView::new();
    textview.set_monospace(true);textview.set_editable(false);textview.set_cursor_visible(false);
    textview.set_wrap_mode(gtk::WrapMode::WordChar);
    textview.buffer().set_text(&log);
    let scroll=gtk::ScrolledWindow::new();
    scroll.set_min_content_height(160);
    scroll.set_max_content_height(240);
    scroll.set_propagate_natural_height(false);
    scroll.set_child(Some(&textview));
    box_.append(&scroll);
    page
}
fn page_tools(window: &gtk::ApplicationWindow, state: &Shared) -> gtk::Box {
    let page=page_root(&tr(state,"tools.title"),&tr(state,"tools.sub"));
    let backups=gtk::Expander::new(Some(&tr(state,"backups.title")));
    backups.add_css_class("generated-expander");
    backups.set_expanded(true);
    backups.set_child(Some(&backups_panel(window,state)));
    page.append(&backups);
    let logs=gtk::Expander::new(Some(&tr(state,"logs.title")));
    logs.add_css_class("generated-expander");
    logs.set_child(Some(&logs_panel(state)));
    page.append(&logs);
    page
}
fn page_about(_window: &gtk::ApplicationWindow, state: &Shared) -> gtk::Box {
    let page=page_root(&tr(state,"about.title"),"Rust Edition · GTK4 · libadwaita · 3.0.1");
    for number in 1..=9 {
        let title=tr(state,&format!("about.{number:02}.title"));
        let text=tr(state,&format!("about.{number:02}.text"));
        let section=section(&page,&title);
        add_text(&section,&text,"muted",true);
    }
    add_text(&page,"GPL-3.0-or-later · Independent application · GTK4 and libadwaita runtime", "muted",true);
    page
}

fn load_css() {
    let provider=gtk::CssProvider::new();
    // load_from_data is available on GTK4 before 4.12; keep older desktop support.
    provider.load_from_data(include_str!("../assets/style.css"));
    if let Some(display)=gdk::Display::default(){
        gtk::style_context_add_provider_for_display(&display,&provider,gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }
}
fn clear_box(container: &gtk::Box) {
    while let Some(child)=container.first_child() { container.remove(&child); }
}
// Used by the headless GTK layout check. Inspect the rendered widget tree,
// rather than relying only on the Grid construction flags.
fn descendant_with_class(widget: &gtk::Widget, css_class: &str) -> Option<gtk::Widget> {
    if widget.has_css_class(css_class) {return Some(widget.clone());}
    let mut child=widget.first_child();
    while let Some(node)=child {
        if let Some(found)=descendant_with_class(&node,css_class) {return Some(found);}
        child=node.next_sibling();
    }
    None
}
fn icon_button(icon: &str, tooltip: &str) -> gtk::Button {
    let b=gtk::Button::from_icon_name(icon);
    b.add_css_class("icon-button");
    b.set_tooltip_text(Some(tooltip));
    b
}
/// Icons are drawn as consistent monochrome vectors; the host's icon theme
/// cannot silently replace half of our navigation with colourful app icons.
fn nav_icon(kind: &'static str) -> gtk::DrawingArea {
    let area=gtk::DrawingArea::new();
    area.set_content_width(19);
    area.set_content_height(19);
    area.add_css_class("nav-icon");
    area.set_draw_func(move |widget, cr, w, h| {
        let color=widget.style_context().color();
        cr.set_source_rgba(color.red().into(),color.green().into(),color.blue().into(),color.alpha().into());
        cr.set_line_width(1.8);
        cr.set_line_cap(gtk::cairo::LineCap::Round);
        cr.set_line_join(gtk::cairo::LineJoin::Round);
        let _=cr.save();
        cr.translate((w as f64-19.0)/2.0,(h as f64-19.0)/2.0);
        match kind {
            "home" => {
                for (x,y) in [(2.0,2.0),(10.5,2.0),(2.0,10.5),(10.5,10.5)] {
                    cr.rectangle(x,y,6.0,6.0);
                }
                let _=cr.stroke();
            }
            "menu" => {
                for y in [4.0,9.5,15.0] {
                    cr.arc(3.0,y,1.0,0.0,std::f64::consts::TAU);
                    let _=cr.fill();
                    cr.move_to(7.0,y);cr.line_to(17.0,y);
                }
                let _=cr.stroke();
            }
            "themes" => {
                cr.arc(9.5,9.5,7.3,0.0,std::f64::consts::TAU);
                let _=cr.stroke();
                for (x,y) in [(6.0,6.0),(12.8,6.7),(5.6,12.3),(11.5,12.9)] {
                    cr.arc(x,y,1.1,0.0,std::f64::consts::TAU);
                    let _=cr.fill();
                }
            }
            "settings" => {
                for (y,x) in [(4.0,6.0),(9.5,13.0),(15.0,8.0)] {
                    cr.move_to(2.0,y);cr.line_to(17.0,y);
                    let _=cr.stroke();
                    cr.arc(x,y,2.1,0.0,std::f64::consts::TAU);
                    let _=cr.fill();
                }
            }
            "tools" => {
                cr.move_to(5.4,13.6);cr.line_to(12.7,6.3);
                cr.move_to(3.2,14.1);cr.arc(4.1,14.9,2.1,0.0,std::f64::consts::TAU);
                cr.move_to(12.4,3.2);cr.line_to(16.0,2.5);
                cr.line_to(16.5,6.0);cr.line_to(13.4,8.2);
                let _=cr.stroke();
            }
            _ => {
                cr.arc(9.5,9.5,7.3,0.0,std::f64::consts::TAU);
                let _=cr.stroke();
                cr.arc(9.5,5.7,1.0,0.0,std::f64::consts::TAU);
                let _=cr.fill();
                cr.move_to(9.5,9.0);cr.line_to(9.5,13.5);
                let _=cr.stroke();
            }
        }
        let _=cr.restore();
    });
    area
}
fn apply_sidebar_density(sidebar: &gtk::Box, compact: bool) {
    sidebar.set_size_request(if compact {62} else {196}, -1);
    if compact {sidebar.add_css_class("nav-compact");}
    else {sidebar.remove_css_class("nav-compact");}
    let mut node=sidebar.first_child();
    while let Some(widget)=node {
        let next=widget.next_sibling();
        if widget.has_css_class("nav-heading") || widget.has_css_class("nav-footer") {
            widget.set_visible(!compact);
        }
        if let Ok(button)=widget.clone().downcast::<gtk::Button>() {
            if let Some(line)=button.child().and_then(|child|child.downcast::<gtk::Box>().ok()) {
                let mut leaf=line.first_child();
                while let Some(part)=leaf {
                    let sibling=part.next_sibling();
                    if part.has_css_class("nav-label") {part.set_visible(!compact);}
                    leaf=sibling;
                }
            }
        }
        node=next;
    }
}
fn ensure_chrome(window: &gtk::ApplicationWindow, state: &Shared) -> Chrome {
    if let Some(chrome)=state.borrow().chrome.clone() { return chrome; }
    let root=gtk::Box::new(gtk::Orientation::Vertical,0);
    root.add_css_class("app-root");
    root.set_size_request(620,440);
    // Keep all custom controls in the application content. GTK and the window
    // manager retain responsibility for the actual window decorations. This
    // avoids custom HeaderBar layout regressions on KDE/Wayland.
    let header=gtk::Box::new(gtk::Orientation::Horizontal,9);
    header.add_css_class("studio-header");
    root.append(&header);

    let body=gtk::Box::new(gtk::Orientation::Horizontal,0);
    body.add_css_class("studio-body");
    body.set_hexpand(true);
    body.set_vexpand(true);
    let sidebar=gtk::Box::new(gtk::Orientation::Vertical,2);
    sidebar.add_css_class("studio-sidebar");
    sidebar.set_size_request(196,-1);
    sidebar.set_hexpand(false);
    sidebar.set_halign(gtk::Align::Start);
    body.append(&sidebar);
    // GTK4's allocated width is observed, so icon-only navigation is applied
    // on resize without rebuilding page content or losing the user's edits.
    let responsive_sidebar=sidebar.clone();
    window.connect_notify_local(Some("width"),move |window,_| {
        if window.width()>0 {apply_sidebar_density(&responsive_sidebar,window.width()<860);}
    });
    // On some Wayland compositors and X11 window managers GTK4 does not
    // notify the read-only widget width property after an external resize.
    // A low-frequency geometry fallback keeps real-time transitions correct.
    let watched_window=window.downgrade();
    let watched_sidebar=sidebar.downgrade();
    glib::timeout_add_local(Duration::from_millis(220),move || {
        let (Some(window),Some(sidebar))=(watched_window.upgrade(),watched_sidebar.upgrade())
            else {return glib::ControlFlow::Break};
        if !window.is_visible() {return glib::ControlFlow::Break;}
        let compact=window.width()<860;
        if window.width()>0 && sidebar.has_css_class("nav-compact")!=compact {
            apply_sidebar_density(&sidebar,compact);
        }
        glib::ControlFlow::Continue
    });

    let scroller=gtk::ScrolledWindow::new();
    scroller.set_policy(gtk::PolicyType::Never,gtk::PolicyType::Automatic);
    scroller.set_min_content_width(0);
    scroller.set_propagate_natural_width(false);
    scroller.set_hexpand(true);
    scroller.set_vexpand(true);
    scroller.add_css_class("page-scroller");
    body.append(&scroller);
    root.append(&body);

    let theme_actions=gtk::Box::new(gtk::Orientation::Horizontal,10);
    theme_actions.add_css_class("theme-action-bar");
    theme_actions.set_visible(false);
    root.append(&theme_actions);
    let footer=gtk::Box::new(gtk::Orientation::Horizontal,8);
    footer.add_css_class("studio-footer");
    root.append(&footer);
    let chrome=Chrome {root,header,sidebar,scroller,theme_actions,footer};
    window.set_child(Some(&chrome.root));
    state.borrow_mut().chrome=Some(chrome.clone());
    chrome
}
fn rebuild(window: &gtk::ApplicationWindow, state: &Shared) {
    let s=state.borrow().clone();
    let chrome=ensure_chrome(window,state);
    chrome.root.remove_css_class("grub-light");
    chrome.root.remove_css_class("grub-dark");
    let use_dark=apply_appearance(&s.appearance);
    chrome.root.add_css_class(if use_dark { "grub-dark" } else { "grub-light" });

    clear_box(&chrome.header);
    let header=&chrome.header;
    let logo_bytes=glib::Bytes::from_static(include_bytes!("../assets/grub-studio-256.png"));
    let logo_texture=gdk::Texture::from_bytes(&logo_bytes).expect("bundled application icon is PNG");
    let brand_icon=gtk::Picture::for_paintable(&logo_texture);
    brand_icon.set_size_request(36,36);
    brand_icon.add_css_class("brand-app-icon");
    header.append(&brand_icon);
    let brand=gtk::Box::new(gtk::Orientation::Vertical,1);
    brand.set_hexpand(true);
    add_text(&brand,"GRUB Studio","brand",false);
    add_text(&brand,"UNIVERSAL  ·  3.0.1","brand-subtitle",false);
    header.append(&brand);
    // Custom controls never become part of the window manager's title bar.
    let status=if s.demo {"DEMO".into()}
        else if s.busy {tr(state,"status.scanning")}
        else if s.authorized&&s.snapshot.profile.writable {tr(state,"status.ready")}
        else if s.authorized {tr(state,"status.readonly")}
        else {format!("🔒 {}",tr(state,"auth.unlock"))};
    let badge=gtk::Label::new(Some(&status));
    badge.add_css_class("status-badge");
    if !s.authorized&&!s.demo {badge.add_css_class("status-locked");}
    else if s.authorized&&!s.snapshot.profile.writable {badge.add_css_class("status-readonly");}
    badge.set_max_width_chars(22);
    badge.set_ellipsize(gtk::pango::EllipsizeMode::End);
    badge.set_tooltip_text(Some(&status));
    // A locked badge next to an Unlock button duplicates the same state.
    // Keep one call to action until the first Polkit probe succeeds.
    if s.demo || s.authorized || s.busy { header.append(&badge); }
    if !s.authorized && !s.demo {
        let unlock=button(&tr(state,"auth.unlock"),true);
        unlock.set_sensitive(!s.busy);
        let w=window.clone();let st=state.clone();
        unlock.connect_clicked(move |_|submit(&w,&st,vec!["--probe".into()],false));
        header.append(&unlock);
    }
    let refresh=icon_button("view-refresh-symbolic",&tr(state,"home.refresh"));
    refresh.set_sensitive(!s.busy && !s.demo);
    let w=window.clone();let st=state.clone();
    refresh.connect_clicked(move |_|submit(&w,&st,vec!["--probe".into()],false));
    header.append(&refresh);
    // Appearance and language are preferences, not global editing actions.
    // Put them in one compact menu so the header fits 620px / HiDPI windows.
    let settings_content=gtk::Box::new(gtk::Orientation::Vertical,9);
    settings_content.add_css_class("prefs-panel");
    settings_content.add_css_class(if use_dark {"prefs-dark"}else{"prefs-light"});
    add_text(&settings_content,&tr(state,"prefs.title"),"prefs-heading",false);
    settings_content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    // A GTK DropDown opens a *second* popover. Nested grabs can stall on
    // Wayland, and rebuilding synchronously from selected-notify destroys
    // the active menu during GTK's own selection event. Flat buttons avoid
    // both failure modes while retaining keyboard-accessible choices.
    let preferences_popover=gtk::Popover::new();
    preferences_popover.add_css_class("studio-prefs-popover");
    preferences_popover.set_position(gtk::PositionType::Bottom);
    add_text(&settings_content,&tr(state,"theme.label"),"prefs-label",false);
    let appearance_buttons=gtk::Box::new(gtk::Orientation::Horizontal,5);
    for (mode,key) in [("system","theme.system"),("light","theme.light"),("dark","theme.dark")] {
        let choice=preference_choice(&tr(state,key),"prefs-appearance",mode,s.appearance==mode);
        let w=window.downgrade();let st=state.clone();let popover=preferences_popover.downgrade();
        choice.connect_clicked(move |_| {
            let mut current=st.borrow_mut();
            if current.appearance==mode {
                if let Some(popover)=popover.upgrade() {popover.popdown();}
                return;
            }
            current.appearance=mode.into();
            if !current.demo {save_prefs(&current);}
            drop(current);
            // Closing the menu before scheduling an idle refresh prevents
            // GTK from handling events on a widget that's being destroyed.
            if let Some(popover)=popover.upgrade() {popover.popdown();}
            if let Some(w)=w.upgrade() {
                let st=st.clone();
                glib::idle_add_local_once(move || rebuild(&w,&st));
            }
        });
        appearance_buttons.append(&choice);
    }
    settings_content.append(&appearance_buttons);
    add_text(&settings_content,&tr(state,"language.label"),"prefs-label",false);
    let language_buttons=gtk::Grid::new();
    language_buttons.set_column_spacing(5);
    language_buttons.set_row_spacing(5);
    for (index,&(locale,name)) in LANGUAGES.iter().enumerate() {
        let choice=preference_choice(name,"prefs-language",locale,s.language==locale);
        let w=window.downgrade();let st=state.clone();let popover=preferences_popover.downgrade();
        choice.connect_clicked(move |_| {
            let mut current=st.borrow_mut();
            if current.language==locale {
                if let Some(popover)=popover.upgrade() {popover.popdown();}
                return;
            }
            current.language=locale.into();
            if !current.demo {save_prefs(&current);}
            drop(current);
            if let Some(popover)=popover.upgrade() {popover.popdown();}
            if let Some(w)=w.upgrade() {
                let st=st.clone();
                glib::idle_add_local_once(move || rebuild(&w,&st));
            }
        });
        language_buttons.attach(&choice,(index%2) as i32,(index/2) as i32,1,1);
    }
    settings_content.append(&language_buttons);
    let preferences=gtk::MenuButton::new();
    preferences.set_child(Some(&nav_icon("settings")));
    preferences.set_tooltip_text(Some(&tr(state,"prefs.title")));
    preferences.add_css_class("icon-button");
    preferences.add_css_class("prefs-trigger");
    preferences_popover.set_child(Some(&settings_content));
    preferences.set_popover(Some(&preferences_popover));
    header.append(&preferences);

    let nav=&chrome.sidebar;
    clear_box(nav);
    add_text(nav,&tr(state,"navigation"),"nav-heading",false);
    for (page,key) in [
        ("home","nav.home"),
        ("menu","nav.menu"),
        ("themes","nav.themes"),
        ("settings","nav.settings")] {
        let b=gtk::Button::new();
        let line=gtk::Box::new(gtk::Orientation::Horizontal,11);
        let graphic=nav_icon(page);
        line.append(&graphic);
        let label=gtk::Label::new(Some(&tr(state,key)));
        label.set_xalign(0.0);label.set_hexpand(true);
        label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        label.set_max_width_chars(17);
        label.add_css_class("nav-label");
        line.append(&label);
        b.set_child(Some(&line));
        b.add_css_class("nav-button");
        b.set_tooltip_text(Some(&tr(state,key)));
        if s.active==page {b.add_css_class("nav-active");}
        b.set_halign(gtk::Align::Fill);
        let w=window.clone();let st=state.clone();
        b.connect_clicked(move |_| {
            if st.borrow().active==page {return;}
            st.borrow_mut().active=page.into();rebuild(&w,&st);
        });
        nav.append(&b);
    }
    for (page,key) in [
        ("tools","tools.title"),
        ("about","nav.about")] {
        let b=gtk::Button::new();
        let line=gtk::Box::new(gtk::Orientation::Horizontal,11);
        let graphic=nav_icon(page);
        line.append(&graphic);
        let label=gtk::Label::new(Some(&tr(state,key)));
        label.set_xalign(0.0);label.set_hexpand(true);
        label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        label.set_max_width_chars(17);
        label.add_css_class("nav-label");
        line.append(&label);
        b.set_child(Some(&line));b.add_css_class("nav-button");
        if page=="tools" {b.add_css_class("nav-group-secondary");}
        b.set_tooltip_text(Some(&tr(state,key)));
        if s.active==page {b.add_css_class("nav-active");}
        let w=window.clone();let st=state.clone();
        b.connect_clicked(move |_| {
            if st.borrow().active==page {return;}
            st.borrow_mut().active=page.into();rebuild(&w,&st);
        });
        nav.append(&b);
    }
    let spacer=gtk::Box::new(gtk::Orientation::Vertical,0);spacer.set_vexpand(true);
    nav.append(&spacer);
    add_text(nav,&format!("{} · GRUB",s.snapshot.profile.family.label()),"nav-footer",true);
    nav.set_visible(true);
    let window_width=if window.width()>0 {window.width()} else {window.default_width()};
    apply_sidebar_density(nav,window_width<860);
    // The outer window and its fixed-width navigation are persistent.
    // Only the active page is replaced, so tabs cannot resize the menu.
    clear_box(&chrome.theme_actions);
    chrome.theme_actions.set_visible(false);
    let page=match s.active.as_str() {
        "menu"=>page_menu(window,state),
        "themes"=>page_themes(window,state),
        "settings"=>page_settings(window,state),
        "tools"=>page_tools(window,state),
        "about"=>page_about(window,state),
        _=>page_home(window,state),
    };
    chrome.scroller.set_child(Some(&page));
    let footer=&chrome.footer;
    clear_box(footer);
    let label=gtk::Label::new(Some(&tr(state,"footer")));
    label.set_hexpand(true);label.set_xalign(0.0);
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    footer.append(&label);
    footer.append(&gtk::Label::new(Some("GTK4 + Adwaita  ·  Wayland/X11")));
}

#[cfg(test)]
mod ui_tests {
    use super::*;

    #[test]
    fn kde_cursor_is_read_from_mouse_section_only() {
        let config="[General]\ncursorTheme=wrong\n[Mouse]\ncursorTheme=Breeze_Snow\ncursorSize=32\n[Other]\ncursorTheme=other\n";
        assert_eq!(cursor_choice(config,None,None),(Some("Breeze_Snow".into()),Some(32)));
    }
    #[test]
    fn env_cursor_settings_override_kde_settings() {
        let config="[Mouse]\ncursorTheme=Breeze\ncursorSize=24\n";
        assert_eq!(cursor_choice(config,Some("Bibata-Modern-Ice"),Some("48")),
            (Some("Bibata-Modern-Ice".into()),Some(48)));
    }
    #[test]
    fn unsafe_cursor_themes_and_sizes_are_ignored() {
        let config="[Mouse]\ncursorTheme=../../elsewhere\ncursorSize=100000\n";
        assert_eq!(cursor_choice(config,None,None),(None,None));
    }
    #[test]
    fn appearance_modes_and_ui_hints_are_translated() {
        for (locale,_) in LANGUAGES {
            for key in ["theme.system","status.locked","hint.demo","hint.busy","hint.unlock","hint.readonly","hint.select","menu.discovery","menu.discovery.note",
                "settings.style.menu","settings.style.hidden","settings.style.countdown"] {
                assert_ne!(translate(locale,key),key,"Missing {locale}: {key}");
            }
        }
    }
    #[test]
    fn discovered_records_merge_grub_and_efi_evidence_without_duplicating_systems() {
        let generated=vec!["Windows Boot Manager".into(),"Linux".into()];
        let systems=vec![OsRecord {name:"windows boot manager".into(),source:"UEFI".into(),
            detail:"Boot0001".into()}];
        let records=combined_boot_records(&generated,&systems);
        assert_eq!(records.len(),2);
        assert_eq!(records[0].0,"Windows Boot Manager");
        assert_eq!(records[0].1,vec!["GRUB","UEFI · Boot0001"]);
    }
    #[test]
    fn refreshing_preserves_unsaved_efi_changes() {
        let mut state = initial_state(true);
        let original = state.snapshot.menu[0].clone();
        state.pending_entries[0].title = "Windows edited locally".into();
        let response = format!("GRUB_STUDIO_PROBE_V2\nPROFILE\tbazzite\t1\tBazzite\t\nENTRY\t{}\t{}\t{}\t{}\t1\n",
            original.id,original.title,original.uuid,original.path);
        apply_probe(&mut state,&response,false,true).unwrap();
        assert_eq!(state.pending_entries[0].title,"Windows edited locally");
        assert_eq!(state.snapshot.menu[0].title,"Windows 11");
        apply_probe(&mut state,&response,false,false).unwrap();
        assert_eq!(state.pending_entries[0].title,"Windows 11");
    }
    #[test]
    fn header_uses_bundled_png_artwork() {
        let image=include_bytes!("../assets/grub-studio-256.png");
        assert!(image.starts_with(b"\x89PNG\r\n\x1a\n"),"application artwork must be PNG");
    }
}

fn main() {
    let args:Vec<String>=std::env::args().skip(1).collect();
    if args.iter().any(|a| a=="--version") { println!("GRUB Studio Universal 3.0.1 Rust / Adwaita Edition");return; }
    if args.iter().any(|a| a=="--self-test") {
        let locales=serde_json::from_str::<serde_json::Value>(include_str!("../assets/translations.json")).expect("valid locale catalog");
        for (code,_) in LANGUAGES {assert!(locales[*code]["tools.title"].is_string(),"missing tools translation: {code}");}
        let exe=std::env::current_exe().expect("executable location");
        let helper=exe.parent().expect("binary directory").join("grub-studio-helper");
        let staged=StagedHelper::new(&helper).expect("portable helper staging");
        assert!(staged.path.is_file(),"staged helper must exist on a normal filesystem");
        println!("Translations, portable administrator helper staging: OK");
        return;
    }
    unsafe extern "C" { fn geteuid() -> u32; }
    if unsafe { geteuid() } == 0 {
        eprintln!("Start the GTK window as a normal user, not with sudo. The helper uses Polkit.");
        std::process::exit(2);
    }
    let ui_smoke=args.iter().any(|a| a=="--ui-smoke" || a=="--ui-smoke-wide");
    let prefs_smoke=args.iter().any(|a| a=="--ui-prefs-smoke");
    let home_layout_smoke=args.iter().any(|a| a=="--ui-home-layout-smoke");
    let ui_resize_report=args.iter().any(|a| a=="--ui-resize-report");
    let ui_wide=args.iter().any(|a| a=="--ui-smoke-wide");
    let demo_themes=args.iter().any(|a| a=="--demo-themes");
    let demo_page=args.iter().find_map(|a|a.strip_prefix("--demo-page="))
        .filter(|value|["home","menu","themes","settings","tools","about"].contains(value))
        .map(str::to_owned);
    let preview_appearance=args.iter().find_map(|a|a.strip_prefix("--appearance="))
        .filter(|v| matches!(*v,"light"|"dark"|"system")).map(str::to_owned);
    let preview_language=args.iter().find_map(|a|a.strip_prefix("--language="))
        .filter(|v| LANGUAGES.iter().any(|(id,_)| id==v)).map(str::to_owned);
    let demo=ui_smoke || prefs_smoke || home_layout_smoke || ui_resize_report || demo_themes || demo_page.is_some() || args.iter().any(|a| a=="--demo");
    // AdwApplication installs the official Adwaita stylesheet and connects
    // the style manager to desktop appearance preferences. The application
    // window is still GtkApplicationWindow so that KDE/Wayland controls its
    // system decorations (no custom HeaderBar regression).
    let app=adw::Application::builder().application_id(APP_ID).build();
    app.connect_startup(|_| {load_css();apply_desktop_cursor();});
    app.connect_activate(move |app| {
        if ui_smoke {
            if let Some(settings)=gtk::Settings::default() {
                println!("GTK4 cursor settings: {} / {}",
                    settings.property::<String>("gtk-cursor-theme-name"),
                    settings.property::<i32>("gtk-cursor-theme-size"));
            }
        }
        let state=Rc::new(RefCell::new(initial_state(demo)));
        if prefs_smoke {
            let mut current=state.borrow_mut();
            current.appearance="system".into();
            current.language="pl".into();
        }
        if demo_themes {state.borrow_mut().active="themes".into();}
        if let Some(ref page)=demo_page {state.borrow_mut().active=page.clone();}
        if let Some(ref mode)=preview_appearance {state.borrow_mut().appearance=mode.clone();}
        if let Some(ref lang)=preview_language {state.borrow_mut().language=lang.clone();}
        let window=gtk::ApplicationWindow::builder()
            .application(app).title("GRUB Studio Universal · Rust Edition")
            .default_width(if ui_smoke {if ui_wide {1100}else{720}}else{1000})
            .default_height(if ui_smoke {540}else{650}).build();
        rebuild(&window,&state);
        window.present();
        if home_layout_smoke {
            let window=window.clone();let application=app.clone();
            glib::timeout_add_local_once(Duration::from_millis(750),move || {
                let root=window.clone().upcast::<gtk::Widget>();
                let grid=descendant_with_class(&root,"stats-grid")
                    .expect("dashboard metric grid missing")
                    .downcast::<gtk::Grid>().expect("dashboard must be a GtkGrid");
                assert!(grid.is_column_homogeneous() && grid.is_row_homogeneous(),
                    "dashboard columns and rows must be homogeneous");
                let first=grid.child_at(0,0).expect("top-left metric missing");
                let second=grid.child_at(1,0).expect("top-right metric missing");
                let third=grid.child_at(0,1).expect("bottom-left metric missing");
                let fourth=grid.child_at(1,1).expect("bottom-right metric missing");
                assert!(grid.width()>500,"dashboard grid must occupy the full content width");
                // GTK distributes an odd number of pixels with a one-pixel
                // difference between homogeneous columns; this is expected.
                assert!((first.width()-second.width()).abs()<=1,"metric columns differ");
                assert!((third.width()-fourth.width()).abs()<=1,"second metric row columns differ");
                assert_eq!(first.height(),third.height(),"metric rows differ in height");
                let rendered_columns=first.width()+second.width()+grid.column_spacing() as i32;
                println!("GTK4 home geometry: grid={} cards={}+{} gap={}",
                    grid.width(),first.width(),second.width(),grid.column_spacing());
                // GTK widget allocation can include padding/unused expansion
                // outside child allocations. Check substantial real coverage,
                // alongside homogeneous columns, instead of exact pixel sum.
                assert!(rendered_columns*5>=grid.width()*4,
                    "dashboard columns cover less than 80% of allocated width");
                println!("GTK4 home layout smoke: 4 equally sized cards filling {} px: OK",rendered_columns);
                application.quit();
            });
        }
        if ui_smoke {
            // Exercise all destinations at a deliberately compact width.
            // Verify that navigating never changes the allocated sidebar
            // width, one of the regressions reported in v2.0.x.
            let w=window.clone();let st=state.clone();let application=app.clone();
            let mut next=0_usize;
            let mut baseline=0_i32;
            let routes=["home","menu","themes","settings","tools","about"];
            glib::timeout_add_local(Duration::from_millis(330),move || {
                let width=st.borrow().chrome.as_ref().map(|c|c.sidebar.width()).unwrap_or(0);
                if width>0 {
                    if baseline==0 {baseline=width;}
                    assert_eq!(width,baseline,"sidebar width changed when switching tabs");
                }
                if next==routes.len() {
                    assert!(baseline>0,"sidebar was never allocated");
                    assert_eq!(baseline,if ui_wide {196}else{62},"responsive sidebar has wrong width");
                    println!("GTK4 navigation smoke: {} pages; sidebar {} px: OK",next,baseline);
                    application.quit();
                    return glib::ControlFlow::Break;
                }
                st.borrow_mut().active=routes[next].into();
                rebuild(&w,&st);
                assert!(w.titlebar().is_none(),"window decorations must remain system-managed");
                if let Some(chrome)=st.borrow().chrome.as_ref() {
                    assert!(chrome.sidebar.is_visible(),"navigation must never be hidden");
                    assert!(chrome.header.first_child()
                        .and_then(|child|child.downcast::<gtk::Picture>().ok()).is_some(),
                        "application artwork must be in the stable content header");
                    assert_eq!(chrome.theme_actions.is_visible(),routes[next]=="themes",
                        "theme actions must only appear on the Themes page");
                }
                next+=1;
                glib::ControlFlow::Continue
            });
        }
        if prefs_smoke {
            // Reproduce the actual freeze path: open the popover, click its
            // theme/language controls, let the main loop drain its idle
            // rebuild, and repeat on the newly rendered popover.
            let w=window.clone();let st=state.clone();let application=app.clone();
            let steps=[
                ("prefs-appearance","dark","dark","pl"),
                ("prefs-language","en","dark","en"),
                ("prefs-appearance","light","light","en"),
                ("prefs-language","de","light","de"),
                ("prefs-appearance","system","system","de"),
                ("prefs-language","pl","system","pl"),
            ];
            let mut step=0;
            glib::timeout_add_local(Duration::from_millis(300),move || {
                if step>0 {
                    // Verify that the official libadwaita palette actually
                    // switches, in addition to storing the chosen preference.
                    match steps[step-1].2 {
                        "dark"=>assert!(adw::StyleManager::default().is_dark(),"Adwaita did not switch to dark"),
                        "light"=>assert!(!adw::StyleManager::default().is_dark(),"Adwaita did not switch to light"),
                        _=>(),
                    }
                }
                if step==steps.len() {
                    assert_eq!(st.borrow().appearance,"system");
                    assert_eq!(st.borrow().language,"pl");
                    println!("GTK4 preferences smoke: {} appearance/language changes: OK",step);
                    application.quit();
                    return glib::ControlFlow::Break;
                }
                let (group,id,appearance,language)=steps[step];
                let chrome=st.borrow().chrome.clone().expect("persistent window chrome");
                let menu=chrome.header.last_child().expect("preferences menu button")
                    .downcast::<gtk::MenuButton>().expect("GTK MenuButton");
                let popover=menu.popover().expect("preferences popover");
                popover.popup();
                let content=popover.child().expect("preferences content");
                let choice=find_preference_choice(&content,group,id)
                    .unwrap_or_else(||panic!("missing preferences option {group}/{id}"));
                choice.emit_clicked();
                assert_eq!(st.borrow().appearance,appearance);
                assert_eq!(st.borrow().language,language);
                assert!(w.titlebar().is_none(),"window decorations must stay system-managed");
                step+=1;
                glib::ControlFlow::Continue
            });
        }
        if ui_resize_report {
            let w=window.clone();let st=state.clone();let application=app.clone();
            glib::timeout_add_local_once(Duration::from_millis(4200),move || {
                let sidebar=st.borrow().chrome.as_ref().map(|c|c.sidebar.width()).unwrap_or(0);
                assert!(w.width()>=650 && w.width()<860,"window was not resized to the compact range: {}",w.width());
                assert_eq!(sidebar,62,"sidebar must collapse after live window resize");
                println!("GTK4 live resize: sidebar {sidebar} px: OK");
                application.quit();
            });
        }
        if !demo {
            let w=window.clone();let s=state.clone();
            glib::timeout_add_local_once(Duration::from_millis(300),move || {
                submit(&w,&s,vec!["--probe".into()],false);
            });
        }
    });
    // Flags such as --demo are consumed by GRUB Studio above, not by GTK's
    // GApplication parser (which otherwise rejects them as unknown options).
    let _=app.run_with_args(&["grub-studio-universal"]);
}

#[cfg(test)]
mod interface_tests {
    use super::*;

    #[test]
    fn important_actions_and_errors_are_localized_in_all_seven_languages() {
        let keys=["prefs.title","themes.use","themes.details","themes.placeholder","menu.entry.actions",
            "menu.more","dialog.invalidsettings","dialog.invalidkernel",
            "dialog.invalidentry","dialog.invalidthemefolder"];
        for (locale,_) in LANGUAGES {
            for key in keys {
                assert_ne!(translate(locale,key),key,"missing {key} in {locale}");
            }
        }
    }
    #[test]
    fn settings_draft_starts_from_actual_grub_config() {
        let config=BTreeMap::from([
            ("GRUB_TIMEOUT".into(),"9".into()),
            ("GRUB_TIMEOUT_STYLE".into(),"countdown".into()),
            ("GRUB_GFXMODE".into(),"2560x1440".into()),
            ("GRUB_CMDLINE_LINUX_DEFAULT".into(),"quiet splash".into()),
        ]);
        let draft=SettingsDraft::from_config(&config);
        assert_eq!(draft.timeout,"9");
        assert_eq!(draft.style,"countdown");
        assert_eq!(draft.gfx,"2560x1440");
        assert_eq!(draft.kernel_args,"quiet splash");
        assert_eq!(SettingsDraft::from_config(&BTreeMap::new()).timeout,"5");
    }
}
