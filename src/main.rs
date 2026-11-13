//! Native GTK4 UI. GTK4 selects Wayland directly when WAYLAND_DISPLAY is set.
use grub_studio::{core, inspect};
use grub_studio::core::MenuEntry;
use grub_studio::inspect::{OsRecord, Snapshot, Theme};
use gtk4 as gtk;
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use std::cell::RefCell;
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
    sidebar_visible: bool,
    selected_theme: String,
    selected_backup: Option<String>,
    selected_entry: Option<usize>,
    pending_entries: Vec<MenuEntry>,
    log: Vec<String>,
    chrome: Option<Chrome>,
}
// GTK widgets in Chrome live for the lifetime of the window. Updating a page
// never tears down the sidebar/header, preventing width jumps and focus loss.
#[derive(Clone)]
struct Chrome {
    root: gtk::Box,
    header: gtk::Box,
    sidebar: gtk::Box,
    scroller: gtk::ScrolledWindow,
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
    let mut result = ("dark".to_string(), default_lang.to_string());
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
fn apply_probe(s: &mut StudioState, raw: &str, deep: bool) -> Result<(), String> {
    let header = if deep { "GRUB_STUDIO_DEEP_V2" } else { "GRUB_STUDIO_PROBE_V2" };
    if raw.lines().next() != Some(header) { return Err("Invalid privileged scan output".into()); }
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
        s.pending_entries = s.snapshot.menu.clone();
        s.selected_entry = None;
        s.selected_theme = s.snapshot.config.get("GRUB_THEME").cloned().unwrap_or_default();
    }
    Ok(())
}
fn initial_state(demo: bool) -> StudioState {
    let prefs = load_prefs();
    let mut s = StudioState {
        snapshot: if demo { demo_snapshot() } else { inspect::inspect() },
        authorized: false, demo, busy: false, language: prefs.1, appearance: prefs.0,
        active:"home".into(), sidebar_visible:true, selected_theme:String::new(),
        selected_backup:None, selected_entry:None, pending_entries:vec![], log:vec![], chrome:None,
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
                        if let Err(e) = apply_probe(&mut s, &message, true) { s.log.push(e); }
                    } else if let Some(scan) = scan {
                        match apply_probe(&mut s, &scan, false) {
                            Ok(()) => { s.authorized = true; s.log.push("Polkit authentication and privileged scan succeeded".into()); }
                            Err(e) => { s.authorized = false; s.log.push(e); }
                        }
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
    v.set_margin_top(18); v.set_margin_bottom(20);
    v.set_margin_start(20); v.set_margin_end(20);
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
        if state.borrow().authorized && snapshot.profile.writable { tr(state, "status.ready") } else { tr(state, "status.readonly") }),
        "muted", true);
    if !snapshot.profile.writable {
        add_text(&banner, &snapshot.profile.reasons.join(" • "), "warning", true);
    }
    page.append(&banner);

    let stats=gtk::Grid::new();
    stats.add_css_class("stats-grid");
    stats.set_row_spacing(8);stats.set_column_spacing(8);
    let current_theme=snapshot.config.get("GRUB_THEME").and_then(|s|s.split('/').rev().nth(1))
        .unwrap_or("—");
    for (position,title,value) in [
        (0,tr(state,"home.theme"),current_theme.to_string()),
        (1,tr(state,"themes.installed"),snapshot.themes.len().to_string()),
        (2,tr(state,"home.others"),snapshot.systems.len().to_string()),
        (3,tr(state,"home.bootloader"),snapshot.profile.bootloader.clone()),
    ] {
        let tile=card();tile.add_css_class("metric-card");
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

fn page_systems(window: &gtk::ApplicationWindow, state: &Shared) -> gtk::Box {
    let page = page_root(&tr(state,"systems.title"), &tr(state,"systems.sub"));
    let actions = action_row();
    // Global refresh always performs a privileged UEFI/EFI scan. Only the
    // optional os-prober deep scan is unique to this page.
    let deep = button(&tr(state,"systems.deep"), true);
    let w=window.clone(); let s=state.clone();
    deep.connect_clicked(move |_| {
        let w2=w.clone(); let s2=s.clone();
        confirm(&w, &tr(&s,"systems.deep.title"), &tr(&s,"systems.deep.ask"),
            move || submit(&w2, &s2, vec!["--scan-deep".into()], true));
    });
    actions.append(&deep);
    page.append(&actions);
    let list = section(&page, &tr(state, "systems.results"));
    let items=state.borrow().snapshot.systems.clone();
    if items.is_empty() { add_text(&list, &tr(state,"systems.empty"), "muted", true); }
    for item in items {
        let row=row_with_heading(&item.name, &format!("{} · {}",item.source,item.detail));
        row.add_css_class("list-row");
        list.append(&row);
    }
    add_text(&page, &tr(state,"systems.footnote"), "muted", true);
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
    let grid=section(&page,&tr(state,"settings.general"));
    let timeout=entry_with_text(config.get("GRUB_TIMEOUT").map(String::as_str).unwrap_or("5"));
    form_row(&grid, &tr(state,"settings.timeout"), &timeout);
    let styles=["menu","hidden","countdown"];
    let style=gtk::DropDown::from_strings(&styles);
    style.set_selected(styles.iter().position(|x| Some(*x)==config.get("GRUB_TIMEOUT_STYLE").map(String::as_str)).unwrap_or(0) as u32);
    form_row(&grid,&tr(state,"settings.style"),&style);
    let gfx=entry_with_text(config.get("GRUB_GFXMODE").map(String::as_str).unwrap_or("auto"));
    form_row(&grid,&tr(state,"settings.resolution"),&gfx);
    // Theme selection belongs exclusively to the Theme Library. Saving
    // basic settings must preserve GRUB_THEME and never accidentally apply
    // an unsaved theme preview selected on another page.
    let current_theme=config.get("GRUB_THEME").cloned().unwrap_or_default();
    add_text(&grid,&tr(state,"settings.timeouthelp"),"muted",true);
    let row=action_row();
    let save=button(&tr(state,"settings.save"),true);
    save.set_sensitive(writable);
    let w=window.clone(); let s=state.clone();
    save.connect_clicked(move |_| {
        let timeout=timeout.text().to_string();
        let gfx=gfx.text().to_string();
        let style=styles.get(style.selected() as usize).unwrap_or(&"menu").to_string();
        if !core::valid_timeout(&timeout) || !core::valid_style(&style) || !core::valid_resolution(&gfx)
            || !core::valid_theme_path(&current_theme) {
            show_message(&w, &tr(&s,"dialog.invalid"), "Invalid timeout, resolution or theme path.", true); return;
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
    let kargs=entry_with_text(config.get("GRUB_CMDLINE_LINUX_DEFAULT").map(String::as_str).unwrap_or(""));
    kernel.append(&kargs);
    let save_kernel=button(&tr(state,"menu.kernel.save"),false);
    save_kernel.set_sensitive(writable && matches!(state.borrow().snapshot.profile.family,core::Family::Arch|core::Family::Debian));
    let w=window.clone(); let s=state.clone();
    save_kernel.connect_clicked(move |_| {
        let value=kargs.text().to_string();
        if !core::valid_kernel_args(&value) { show_message(&w,"GRUB","Invalid kernel parameters",true); return; }
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
    let info=section(&page,&tr(state,"menu.custom"));
    add_text(&info,&tr(state,"menu.custom.note"),"muted",true);
    if !profile.family.supports_menu_edit() { add_text(&info,&tr(state,"menu.readonly"),"warning",true); }
    let list=gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.add_css_class("studio-list");
    let items=state.borrow().pending_entries.clone();
    if items.is_empty() { add_text(&info,&tr(state,"menu.empty"),"muted",true); }
    for item in &items {
        let label=row_with_heading(&item.title,&format!("{} · {} · {}",item.uuid,item.path,
            if item.enabled {tr(state,"menu.enabled")} else {tr(state,"menu.hidden")}));
        label.set_margin_top(7); label.set_margin_bottom(7); label.set_margin_start(11); label.set_margin_end(11);
        list.append(&label);
    }
    info.append(&list);
    let fields=gtk::Box::new(gtk::Orientation::Vertical,7);
    let title=entry_with_text(""); title.set_placeholder_text(Some(&tr(state,"menu.field.title")));
    let uuid=entry_with_text(""); uuid.set_placeholder_text(Some(&tr(state,"menu.field.uuid")));
    let path=entry_with_text("/EFI/Microsoft/Boot/bootmgfw.efi");
    fields.append(&title);fields.append(&uuid);fields.append(&path);
    info.append(&fields);
    let s=state.clone();
    let title2=title.clone(); let uuid2=uuid.clone(); let path2=path.clone();
    list.connect_row_selected(move |_, row| {
        if let Some(index)=row.map(|r| r.index() as usize) {
            s.borrow_mut().selected_entry=Some(index);
            if let Some(item)=s.borrow().pending_entries.get(index) {
                title2.set_text(&item.title); uuid2.set_text(&item.uuid); path2.set_text(&item.path);
            }
        }
    });
    let selected_index=state.borrow().selected_entry;
    if let Some(index)=selected_index {
        if let Some(row)=list.row_at_index(index as i32) { list.select_row(Some(&row)); }
    }
    let controls=action_flow();
    let add=button(&tr(state,"menu.add"),false);
    let w=window.clone();let s=state.clone();
    let title1=title.clone();let uuid1=uuid.clone();let path1=path.clone();
    add.connect_clicked(move |_| {
        let item=MenuEntry {id:new_id(),title:title1.text().to_string(),uuid:uuid1.text().to_string(),
            path:path1.text().to_string(),enabled:true};
        if !core::valid_entry(&item) { show_message(&w,"GRUB Studio","Invalid EFI entry title, UUID or path",true);return; }
        s.borrow_mut().pending_entries.push(item);
        s.borrow_mut().selected_entry=None;
        rebuild(&w,&s);
    });
    controls.insert(&add,-1);
    let edit=button(&tr(state,"menu.edit"),false);
    let w=window.clone();let s=state.clone();
    edit.connect_clicked(move |_| {
        let idx=s.borrow().selected_entry;
        let Some(idx)=idx else { return };
        let Some(mut item)=s.borrow().pending_entries.get(idx).cloned() else {return};
        item.title=title.text().to_string();
        item.uuid=uuid.text().to_string();
        item.path=path.text().to_string();
        if !core::valid_entry(&item) { show_message(&w,"GRUB Studio","Invalid EFI entry",true);return; }
        s.borrow_mut().pending_entries[idx]=item;
        rebuild(&w,&s);
    });
    controls.insert(&edit,-1);
    // All local entry actions share one responsive toolbar. Each operation
    // appears once; this replaces two duplicate reorder/delete handlers.
    for (key,operation) in [("menu.up",-1i32),("menu.down",1),
                            ("menu.toggle",0),("menu.remove",2)] {
        let b=button(&tr(state,key),false);
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
            drop(data);
            rebuild(&w,&s);
        });
        controls.insert(&b,-1);
    }
    info.append(&controls);
    let commit=action_flow();
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
    let set_default=button(&tr(state,"menu.defaultcustom"),false);
    set_default.set_sensitive(editable);
    let w=window.clone();let s=state.clone();
    set_default.connect_clicked(move |_| {
        let chosen=s.borrow().selected_entry.and_then(|i| s.borrow().pending_entries.get(i).cloned());
        let Some(item)=chosen else {show_message(&w,"GRUB Studio",&tr(&s,"menu.selectfirst"),true);return};
        guarded_submit(&w,&s,vec!["--default".into(),item.id.clone()],
            tf(&s,"menu.default.confirm",&[("id",&item.id)]));
    });
    commit.insert(&set_default,-1);
    let preview=button(&tr(state,"menu.preview"),false);
    let w=window.clone();let s=state.clone();
    preview.connect_clicked(move |_| {
        match core::render_grub_entries(&s.borrow().pending_entries) {
            Ok(script)=>show_message(&w,&tr(&s,"menu.preview"),&script,false),
            Err(e)=>show_message(&w,"GRUB Studio",&e,true),
        }
    });
    commit.insert(&preview,-1);
    let discard=button(&tr(state,"menu.discard"),false);
    let w=window.clone();let s=state.clone();
    discard.connect_clicked(move |_| {
        let original=s.borrow().snapshot.menu.clone();
        {
            let mut data=s.borrow_mut();
            data.pending_entries=original;
            data.selected_entry=None;
        }
        rebuild(&w,&s);
    });
    commit.insert(&discard,-1);
    page.append(&commit);
    // Automatically generated entries are informative and cannot be edited.
    // Place them under one collapsed disclosure instead of duplicating the
    // custom-entry editor visual hierarchy.
    let generated=gtk::Expander::new(Some(&tr(state,"menu.detected")));
    generated.add_css_class("generated-expander");
    let generated_body=card();
    add_text(&generated_body,&tr(state,"menu.detected.note"),"muted",true);
    let titles=state.borrow().snapshot.generated.clone();
    if titles.is_empty() {add_text(&generated_body,&tr(state,"menu.empty"),"muted",true);}
    for (index,title) in titles.iter().enumerate() {
        add_text(&generated_body,&format!("{}  {}",index+1,title),"item-label",true);
    }
    generated.set_child(Some(&generated_body));
    page.append(&generated);
    add_text(&page,&tr(state,"menu.efiwarn"),"muted",true);
    page
}

fn page_themes(window: &gtk::ApplicationWindow, state: &Shared) -> gtk::Box {
    let page=page_root(&tr(state,"themes.title"),&tr(state,"themes.sub"));
    let selected=state.borrow().selected_theme.clone();
    let themes=state.borrow().snapshot.themes.clone();
    let active=state.borrow().snapshot.config.get("GRUB_THEME").cloned().unwrap_or_default();
    let writable=can_edit(&state.borrow());
    // FlowBox adapts between two columns on wide desktops and a single
    // vertical column on narrow windows. A horizontal Paned hid the theme
    // preview at 560–700 px window widths.
    let columns=gtk::FlowBox::new();
    columns.add_css_class("theme-split");
    columns.set_selection_mode(gtk::SelectionMode::None);
    columns.set_min_children_per_line(1);
    columns.set_max_children_per_line(2);
    columns.set_column_spacing(10);
    columns.set_row_spacing(10);
    columns.set_homogeneous(true);
    columns.set_vexpand(true);

    let list=card();
    list.add_css_class("theme-browser");
    list.set_size_request(275,-1);
    let list_header=gtk::Box::new(gtk::Orientation::Horizontal,8);
    let label=gtk::Label::new(Some(&format!("{}  ({})",tr(state,"themes.installed"),themes.len())));
    label.add_css_class("section-heading");label.set_xalign(0.0);label.set_hexpand(true);
    list_header.append(&label);
    list.append(&list_header);
    if themes.is_empty() {
        add_text(&list,&tr(state,"themes.placeholder"),"muted",true);
        add_text(&list,&tr(state,"status.readonly"),"warning",true);
    }
    let selector=gtk::ListBox::new();
    selector.add_css_class("studio-list");
    for theme in &themes {
        let display=if active==theme.path {format!("●  {}",theme.name)} else {format!("◇  {}",theme.name)};
        let row=row_with_heading(&display,&theme.path);
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
    scroll.set_min_content_height(220);
    scroll.set_vexpand(true);
    scroll.set_child(Some(&selector));
    list.append(&scroll);
    let install=button(&tr(state,"themes.import"),false);
    install.set_sensitive(writable);
    let w=window.clone();let s=state.clone();
    install.connect_clicked(move |_| {
        let dialog=gtk::FileChooserNative::new(Some(&tr(&s,"themes.import")),Some(&w),
            gtk::FileChooserAction::SelectFolder,Some("Select"),Some("Cancel"));
        // NativeDialog is not a toplevel widget; keep it alive until response.
        let dialog_hold=dialog.clone();
        let w2=w.clone();let s2=s.clone();
        dialog.connect_response(move |dialog_response,response| {
            if response==gtk::ResponseType::Accept {
                if let Some(path)=dialog_response.file().and_then(|file|file.path()) {
                    let folder=path.file_name().map(|x|x.to_string_lossy().to_string()).unwrap_or_default();
                    if !core::valid_simple_name(&folder) {show_message(&w2,"GRUB Studio","Theme folder must have an ASCII alphanumeric name",true);}
                    else {guarded_submit(&w2,&s2,vec!["--install-theme".into(),path.to_string_lossy().to_string(),folder],tr(&s2,"themes.import"));}
                }
            }
            dialog_hold.destroy();
        });
        dialog.show();
    });
    list.append(&install);
    columns.insert(&list,-1);

    let preview=card();
    preview.add_css_class("theme-details");
    preview.set_size_request(275,-1);
    add_text(&preview,&tr(state,"themes.preview"),"section-heading",true);
    let canvas=gtk::Box::new(gtk::Orientation::Vertical,5);
    canvas.add_css_class("boot-preview");
    add_text(&canvas,"GNU GRUB 2", "preview-title",true);
    // A representative view of this machine's actual detected GRUB entries.
    // Visual colors are only an approximation, not a rendering of theme.txt.
    let mut entries=state.borrow().snapshot.generated.iter().take(3).cloned().collect::<Vec<_>>();
    if entries.is_empty() {
        entries.push(state.borrow().snapshot.profile.name.clone());
        entries.extend(state.borrow().snapshot.systems.iter().take(2).map(|os|os.name.clone()));
    }
    for (i,entry) in entries.iter().enumerate() {
        let l=gtk::Label::new(Some(entry));
        l.set_xalign(0.0);l.add_css_class(if i==0{"preview-selected"}else{"preview-entry"});
        canvas.append(&l);
    }
    preview.append(&canvas);
    let selected_info=gtk::Label::new(Some(if selected.is_empty() {"—"}else{&selected}));
    selected_info.add_css_class("muted");
    selected_info.set_xalign(0.0);selected_info.set_wrap(true);
    selected_info.set_selectable(true);
    preview.append(&selected_info);
    add_text(&preview,&tr(state,"themes.approx"),"muted",true);
    let controls=action_row();
    let activate=button(&tr(state,"themes.use"),true);
    activate.set_sensitive(writable && !selected.is_empty());
    let w=window.clone();let s=state.clone();
    activate.connect_clicked(move |_| {
        let c=s.borrow().snapshot.config.clone();let selected=s.borrow().selected_theme.clone();
        if selected.is_empty() || !core::valid_theme_path(&selected){
            show_message(&w,"GRUB Studio","Select an installed theme",true);return;
        }
        guarded_submit(&w,&s,vec!["--apply".into(),
            c.get("GRUB_TIMEOUT").cloned().unwrap_or_else(||"5".into()),
            c.get("GRUB_TIMEOUT_STYLE").cloned().unwrap_or_else(||"menu".into()),
            c.get("GRUB_GFXMODE").cloned().unwrap_or_else(||"auto".into()),selected],tr(&s,"themes.use"));
    });
    controls.append(&activate);
    let disable=button(&tr(state,"themes.disable"),false);
    disable.set_sensitive(writable && !active.is_empty());
    let w=window.clone();let s=state.clone();
    disable.connect_clicked(move |_| {
        let c=s.borrow().snapshot.config.clone();
        guarded_submit(&w,&s,vec!["--apply".into(),
            c.get("GRUB_TIMEOUT").cloned().unwrap_or_else(||"5".into()),
            c.get("GRUB_TIMEOUT_STYLE").cloned().unwrap_or_else(||"menu".into()),
            c.get("GRUB_GFXMODE").cloned().unwrap_or_else(||"auto".into()),String::new()],tr(&s,"themes.disable"));
    });
    controls.append(&disable);
    preview.append(&controls);
    columns.insert(&preview,-1);
    page.append(&columns);
    let s=state.clone();
    let themes_for_selection=themes.clone();
    selector.connect_row_selected(move |_,row| {
        if let Some(row)=row {
            if let Some(theme)=themes_for_selection.get(row.index() as usize) {
                s.borrow_mut().selected_theme=theme.path.clone();
                selected_info.set_text(&theme.path);
                activate.set_sensitive(can_edit(&s.borrow()));
            }
        }
    });
    page
}

fn page_backups(window: &gtk::ApplicationWindow, state: &Shared) -> gtk::Box {
    let page=page_root(&tr(state,"backups.title"),&tr(state,"backups.sub"));
    let controls=action_row();
    let create=button(&tr(state,"backups.create"),true);
    create.set_sensitive(can_edit(&state.borrow()));
    let w=window.clone();let s=state.clone();
    create.connect_clicked(move |_| submit(&w,&s,vec!["--backup".into()],false));
    controls.append(&create);
    let restore=button(&tr(state,"backups.restore"),false);
    restore.set_sensitive(can_edit(&state.borrow()));
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
fn page_logs(_window: &gtk::ApplicationWindow, state: &Shared) -> gtk::Box {
    let page=page_root(&tr(state,"logs.title"),&tr(state,"logs.sub"));
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
    textview.set_vexpand(true);
    box_.append(&textview);
    page
}
fn page_about(_window: &gtk::ApplicationWindow, state: &Shared) -> gtk::Box {
    let page=page_root(&tr(state,"about.title"),"Rust Edition · GTK4 · Wayland · 2.1.0");
    for number in 1..=9 {
        let title=tr(state,&format!("about.{number:02}.title"));
        let text=tr(state,&format!("about.{number:02}.text"));
        let section=section(&page,&title);
        add_text(&section,&text,"muted",true);
    }
    add_text(&page,"GPL-3.0-or-later · Independent application · GTK4 runtime required", "muted",true);
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
fn icon_button(icon: &str, tooltip: &str) -> gtk::Button {
    let b=gtk::Button::from_icon_name(icon);
    b.add_css_class("icon-button");
    b.set_tooltip_text(Some(tooltip));
    b
}
fn ensure_chrome(window: &gtk::ApplicationWindow, state: &Shared) -> Chrome {
    if let Some(chrome)=state.borrow().chrome.clone() { return chrome; }
    let root=gtk::Box::new(gtk::Orientation::Vertical,0);
    root.add_css_class("app-root");
    root.set_size_request(620,440);
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

    let scroller=gtk::ScrolledWindow::new();
    scroller.set_policy(gtk::PolicyType::Never,gtk::PolicyType::Automatic);
    scroller.set_min_content_width(0);
    scroller.set_propagate_natural_width(false);
    scroller.set_hexpand(true);
    scroller.set_vexpand(true);
    scroller.add_css_class("page-scroller");
    body.append(&scroller);
    root.append(&body);

    let footer=gtk::Box::new(gtk::Orientation::Horizontal,8);
    footer.add_css_class("studio-footer");
    root.append(&footer);
    let chrome=Chrome {root,header,sidebar,scroller,footer};
    window.set_child(Some(&chrome.root));
    state.borrow_mut().chrome=Some(chrome.clone());
    chrome
}
fn rebuild(window: &gtk::ApplicationWindow, state: &Shared) {
    let s=state.borrow().clone();
    let chrome=ensure_chrome(window,state);
    chrome.root.remove_css_class("grub-light");
    chrome.root.remove_css_class("grub-dark");
    chrome.root.add_css_class(if s.appearance=="light" { "grub-light" } else { "grub-dark" });

    clear_box(&chrome.header);
    let header=&chrome.header;
    let hamburger=icon_button("open-menu-symbolic",&tr(state,"navigation"));
    hamburger.set_tooltip_text(Some(&tr(state,"navigation")));
    let w=window.clone();let state2=state.clone();
    hamburger.connect_clicked(move |_| {
        let old=state2.borrow().sidebar_visible;
        state2.borrow_mut().sidebar_visible=!old;
        rebuild(&w,&state2);
    });
    header.append(&hamburger);
    let emblem=gtk::Label::new(Some("G"));
    emblem.add_css_class("brand-icon");
    header.append(&emblem);
    let branding=gtk::Box::new(gtk::Orientation::Vertical,1);
    branding.set_hexpand(true);
    add_text(&branding,"GRUB Studio","brand",false);
    add_text(&branding,"UNIVERSAL  ·  2.1","brand-subtitle",false);
    header.append(&branding);
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
    header.append(&badge);
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
    let appearance=icon_button(if s.appearance=="light" {"weather-clear-night-symbolic"}else{"weather-clear-symbolic"},&tr(state,"theme.label"));
    appearance.set_tooltip_text(Some(&tr(state,"theme.label")));
    let w=window.clone();let state2=state.clone();
    appearance.connect_clicked(move |_| {
        let mut s=state2.borrow_mut();
        s.appearance=if s.appearance=="light" {"dark"}else{"light"}.into();
        save_prefs(&s);drop(s);rebuild(&w,&state2);
    });
    header.append(&appearance);
    let names:Vec<&str>=LANGUAGES.iter().map(|(_,name)|*name).collect();
    let language=gtk::DropDown::from_strings(&names);
    language.add_css_class("lang-select");
    language.set_selected(LANGUAGES.iter().position(|(id,_)| *id==s.language).unwrap_or(0) as u32);
    let w=window.clone();let state2=state.clone();
    language.connect_selected_notify(move |combo| {
        if let Some((locale,_))=LANGUAGES.get(combo.selected() as usize) {
            let mut st=state2.borrow_mut();
            if st.language==*locale {return;}
            st.language=(*locale).into();save_prefs(&st);
            drop(st);rebuild(&w,&state2);
        }
    });
    header.append(&language);

    let nav=&chrome.sidebar;
    clear_box(nav);
    add_text(nav,&tr(state,"navigation"),"nav-heading",false);
    for (page,icon,key) in [
        ("home","view-grid-symbolic","nav.home"),
        ("menu","view-list-symbolic","nav.menu"),
        ("systems","drive-harddisk-symbolic","nav.systems"),
        ("themes","applications-graphics-symbolic","nav.themes"),
        ("settings","emblem-system-symbolic","nav.settings")] {
        let b=gtk::Button::new();
        let line=gtk::Box::new(gtk::Orientation::Horizontal,11);
        let graphic=gtk::Image::from_icon_name(icon);
        graphic.set_pixel_size(17);
        graphic.add_css_class("nav-icon");
        line.append(&graphic);
        let label=gtk::Label::new(Some(&tr(state,key)));
        label.set_xalign(0.0);label.set_hexpand(true);
        label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        label.set_max_width_chars(17);
        line.append(&label);
        b.set_child(Some(&line));
        b.add_css_class("nav-button");
        if s.active==page {b.add_css_class("nav-active");}
        b.set_halign(gtk::Align::Fill);
        let w=window.clone();let st=state.clone();
        b.connect_clicked(move |_| {
            if st.borrow().active==page {return;}
            st.borrow_mut().active=page.into();rebuild(&w,&st);
        });
        nav.append(&b);
    }
    let tools_heading=add_text(nav,&tr(state,"nav.tools"),"nav-heading",false);
    tools_heading.add_css_class("nav-group-secondary");
    for (page,icon,key) in [
        ("backups","document-save-symbolic","nav.backups"),
        ("logs","text-x-generic-symbolic","nav.logs"),
        ("about","help-about-symbolic","nav.about")] {
        let b=gtk::Button::new();
        let line=gtk::Box::new(gtk::Orientation::Horizontal,11);
        let graphic=gtk::Image::from_icon_name(icon);
        graphic.set_pixel_size(17);graphic.add_css_class("nav-icon");
        line.append(&graphic);
        let label=gtk::Label::new(Some(&tr(state,key)));
        label.set_xalign(0.0);label.set_hexpand(true);
        label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        label.set_max_width_chars(17);
        line.append(&label);
        b.set_child(Some(&line));b.add_css_class("nav-button");
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
    nav.set_visible(s.sidebar_visible);
    // The outer window and its fixed-width navigation are persistent.
    // Only the active page is replaced, so tabs cannot resize the menu.
    let page=match s.active.as_str() {
        "menu"=>page_menu(window,state),
        "systems"=>page_systems(window,state),
        "themes"=>page_themes(window,state),
        "settings"=>page_settings(window,state),
        "backups"=>page_backups(window,state),
        "logs"=>page_logs(window,state),
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
    footer.append(&gtk::Label::new(Some(if s.demo {"DEMO"}else{"GTK4  ·  Wayland/X11"})));
}

fn main() {
    let args:Vec<String>=std::env::args().skip(1).collect();
    if args.iter().any(|a| a=="--version") { println!("GRUB Studio Universal 2.1.0 Rust Edition");return; }
    if args.iter().any(|a| a=="--self-test") {
        let locales=serde_json::from_str::<serde_json::Value>(include_str!("../assets/translations.json")).expect("valid locale catalog");
        for (code,_) in LANGUAGES {assert!(locales[*code]["nav.tools"].is_string(),"missing navigation translation: {code}");}
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
    let ui_smoke=args.iter().any(|a| a=="--ui-smoke");
    let demo=ui_smoke || args.iter().any(|a| a=="--demo");
    let app=gtk::Application::builder().application_id(APP_ID).build();
    app.connect_startup(|_| load_css());
    app.connect_activate(move |app| {
        let state=Rc::new(RefCell::new(initial_state(demo)));
        let window=gtk::ApplicationWindow::builder()
            .application(app).title("GRUB Studio Universal · Rust Edition")
            .default_width(if ui_smoke {720}else{1000})
            .default_height(if ui_smoke {540}else{650}).build();
        rebuild(&window,&state);
        window.present();
        if ui_smoke {
            // Exercise all destinations at a deliberately compact width.
            // Verify that navigating never changes the allocated sidebar
            // width, one of the regressions reported in v2.0.x.
            let w=window.clone();let st=state.clone();let application=app.clone();
            let mut next=0_usize;
            let mut baseline=0_i32;
            let routes=["home","menu","systems","themes","settings","backups","logs","about"];
            glib::timeout_add_local(Duration::from_millis(330),move || {
                let width=st.borrow().chrome.as_ref().map(|c|c.sidebar.width()).unwrap_or(0);
                if width>0 {
                    if baseline==0 {baseline=width;}
                    assert_eq!(width,baseline,"sidebar width changed when switching tabs");
                }
                if next==routes.len() {
                    assert!(baseline>0,"sidebar was never allocated");
                    println!("GTK4 navigation smoke: {} pages; sidebar {} px: OK",next,baseline);
                    application.quit();
                    return glib::ControlFlow::Break;
                }
                st.borrow_mut().active=routes[next].into();
                rebuild(&w,&st);
                next+=1;
                glib::ControlFlow::Continue
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
