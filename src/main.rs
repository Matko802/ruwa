use ksni::menu::{Disposition, StandardItem};
use ksni::{Category, Icon, MenuItem, Status, ToolTip, Tray};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[allow(dead_code)]
mod settings;

use settings::profile_dir;

const WHATSAPP_HOME: &str = "https://web.whatsapp.com/";
const ICON_SIZE: i32 = 64;

#[derive(Debug, Clone)]
enum TrayAction {
    Show,
    Reload,
    Settings,
    Quit,
}

fn verbose() -> bool {
    matches!(
        std::env::var("RUWA_VERBOSE")
            .or_else(|_| std::env::var("RUWA_LOG"))
            .as_deref(),
        Ok("1") | Ok("debug") | Ok("verbose") | Ok("trace")
    )
}

fn vlog(msg: &str) {
    if verbose() {
        eprintln!("ruwa: {msg}");
    }
}

fn blend(a: u8, b: u8, t: f64) -> u8 {
    (f64::from(a) * (1.0 - t) + f64::from(b) * t).round() as u8
}

fn paint_disc(out: &mut [u8], size: usize, cx: f64, cy: f64, radius: f64, rgb: (u8, u8, u8)) {
    let dist = |x: f64, y: f64| ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
    for y in 0..size {
        for x in 0..size {
            let d = dist(x as f64, y as f64);
            if d < radius {
                let edge = ((d - (radius - 1.5)) / 1.5).clamp(0.0, 1.0);
                let i = (y * size + x) * 4;
                out[i + 1] = blend(out[i + 1], rgb.0, 1.0 - edge);
                out[i + 2] = blend(out[i + 2], rgb.1, 1.0 - edge);
                out[i + 3] = blend(out[i + 3], rgb.2, 1.0 - edge);
            }
        }
    }
}

fn paint_icon_with_attention(attention: bool) -> Icon {
    let size = ICON_SIZE as usize;
    let mut out = vec![0u8; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let i = (y * size + x) * 4;
            out[i] = 255;
            out[i + 1] = 0x12;
            out[i + 2] = 0x17;
            out[i + 3] = 0x1a;
        }
    }
    paint_disc(&mut out, size, 32.0, 32.0, 22.0, (0x25, 0xD3, 0x66));
    paint_disc(&mut out, size, 32.0, 32.0, 11.5, (0xFF, 0xFF, 0xFF));
    paint_disc(&mut out, size, 27.5, 36.5, 5.5, (0x25, 0xD3, 0x66));
    if attention {
        paint_disc(&mut out, size, 51.0, 13.0, 7.5, (0xFF, 0x53, 0x53));
    }
    Icon {
        width: ICON_SIZE,
        height: ICON_SIZE,
        data: out,
    }
}

struct RuwaTray {
    tx: Sender<TrayAction>,
    engine_alive: Arc<AtomicBool>,
}

fn item(label: &str, action: TrayAction, tx: &Sender<TrayAction>) -> MenuItem<RuwaTray> {
    let tx = tx.clone();
    MenuItem::Standard(StandardItem {
        label: String::from(label),
        enabled: true,
        visible: true,
        disposition: Disposition::Normal,
        activate: Box::new(move |_| {
            let _ = tx.send(action.clone());
        }),
        ..Default::default()
    })
}

impl Tray for RuwaTray {
    fn id(&self) -> String {
        String::from("ruwa")
    }

    fn title(&self) -> String {
        String::from("Ruwa")
    }

    fn category(&self) -> Category {
        Category::Communications
    }

    fn status(&self) -> Status {
        if self.engine_alive.load(Ordering::Relaxed) {
            Status::Active
        } else {
            Status::NeedsAttention
        }
    }

    fn icon_name(&self) -> String {
        String::from("ruwa")
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        vec![paint_icon_with_attention(false)]
    }

    fn attention_icon_name(&self) -> String {
        String::from("ruwa")
    }

    fn attention_icon_pixmap(&self) -> Vec<Icon> {
        vec![paint_icon_with_attention(true)]
    }

    fn tool_tip(&self) -> ToolTip {
        let description = if self.engine_alive.load(Ordering::Relaxed) {
            String::from("WhatsApp desktop client")
        } else {
            String::from("WhatsApp desktop client (window closed — Show to reopen)")
        };
        ToolTip {
            icon_name: String::new(),
            icon_pixmap: Vec::new(),
            title: String::from("Ruwa"),
            description,
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.tx.send(TrayAction::Show);
    }

    fn menu(&self) -> Vec<MenuItem<RuwaTray>> {
        vec![
            item("Show", TrayAction::Show, &self.tx),
            item("Reload", TrayAction::Reload, &self.tx),
            item("Settings", TrayAction::Settings, &self.tx),
            MenuItem::Separator,
            item("Quit", TrayAction::Quit, &self.tx),
        ]
    }
}

fn engine_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("RUWA_ENGINE") {
        let pb = PathBuf::from(p);
        if pb.is_file() {
            return Some(pb);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for name in ["ruwa-engine", ".ruwa-engine-wrapped"] {
                let cand = dir.join(name);
                if cand.is_file() {
                    return Some(cand);
                }
            }
        }
    }
    None
}

fn engine_env(cmd: &mut Command, profile: &Path) {
    let prefs = settings::read_prefs(profile);
    let backend = std::env::var("RUWA_BACKEND")
        .ok()
        .map(|s| settings::DisplayBackend::from_str(&s))
        .unwrap_or(prefs.display);
    match backend {
        settings::DisplayBackend::X11 => {
            cmd.env_remove("WAYLAND_DISPLAY");
            cmd.env("RUWA_BACKEND", "x11");
        }
        settings::DisplayBackend::Wayland => {
            cmd.env("RUWA_BACKEND", "wayland");
        }
        settings::DisplayBackend::Auto => {
            cmd.env("RUWA_BACKEND", "auto");
        }
    }
}

fn spawn_engine(engine: &Path, profile: &Path, extra_args: &[String]) -> std::io::Result<Child> {
    let mut cmd = Command::new(engine);
    engine_env(&mut cmd, profile);
    if extra_args.is_empty() {
        cmd.arg(WHATSAPP_HOME);
    } else {
        cmd.args(extra_args);
    }
    cmd.stdin(std::process::Stdio::null()).spawn()
}

fn terminate_child(child: &mut Child) {
    let pid = child.id();
    let _ = Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(_) => break,
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn child_alive(child: &mut Option<Child>) -> bool {
    match child {
        Some(c) => match c.try_wait() {
            Ok(None) => true,
            Ok(Some(_)) => false,
            Err(_) => false,
        },
        None => false,
    }
}

fn reap_if_exited(child: &mut Option<Child>) -> bool {
    if let Some(c) = child {
        match c.try_wait() {
            Ok(Some(status)) => {
                println!("ruwa: engine exited ({status})");
                *child = None;
                return true;
            }
            Ok(None) => return false,
            Err(e) => {
                eprintln!("ruwa: wait on engine failed: {e:?}");
                *child = None;
                return true;
            }
        }
    }
    false
}

fn settings_binary() -> Option<PathBuf> {
    settings::sibling_binary("ruwa-settings")
}

fn open_settings() {
    if let Some(bin) = settings_binary() {
        let _ = Command::new(bin)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    } else {
        eprintln!("ruwa: ruwa-settings not found next to ruwa binary");
    }
}

fn read_lock_pid(lock_path: &Path) -> Option<u32> {
    std::fs::read_to_string(lock_path)
        .ok()?
        .trim()
        .parse::<u32>()
        .ok()
}

fn usage() -> &'static str {
    "ruwa — lightweight WhatsApp desktop client\n\nUsage: ruwa [options] [url...]\n\nOptions:\n  --show        Focus the running window (default for second instance)\n  --quit        Quit the running instance and exit\n  --minimized   Start minimized to tray (no window)\n  --reload      Ask the running instance to reload (via new engine launch)\n  -h, --help    Show this help\n\nEnv:\n  RUWA_PROFILE   Override profile dir\n  RUWA_ENGINE    Override engine binary\n  RUWA_BACKEND   auto|wayland|x11 (overrides Settings display backend)\n  RUWA_VERBOSE=1 Extra logging\n"
}

fn main() {
    let raw_args: Vec<String> = std::env::args().skip(1).collect();
    let mut extra_args: Vec<String> = Vec::new();
    let mut want_quit = false;
    let mut want_minimized = false;
    for a in &raw_args {
        match a.as_str() {
            "--quit" => want_quit = true,
            "--minimized" => want_minimized = true,
            "--show" | "--reload" => {}
            "-h" | "--help" => {
                print!("{}", usage());
                return;
            }
            _ if a.starts_with('-') => {
                eprintln!("ruwa: unknown option '{a}'");
                eprintln!("{}", usage());
                std::process::exit(2);
            }
            _ => extra_args.push(a.clone()),
        }
    }

    let profile = profile_dir();
    let _ = std::fs::create_dir_all(&profile);

    let lock_path = profile.join(".ruwa-lock");
    let mut lock_file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)
        .expect("ruwa: cannot open lock file");
    {
        use fs2::FileExt;
        if lock_file.try_lock_exclusive().is_err() {
            if want_quit {
                if let Some(pid) = read_lock_pid(&lock_path) {
                    vlog(&format!("quitting running instance pid {pid}"));
                    let _ = Command::new("kill")
                        .arg(pid.to_string())
                        .stdin(std::process::Stdio::null())
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .status();
                } else {
                    eprintln!("ruwa: already running but supervisor pid unknown");
                }
                std::process::exit(0);
            }
            let _ = writeln!(
                std::io::stderr(),
                "ruwa is already running, focusing existing window"
            );
            if let Some(engine) = engine_path() {
                let _ = spawn_engine(&engine, &profile, &extra_args);
            }
            std::process::exit(0);
        }
    }
    if want_quit {
        eprintln!("ruwa: not running");
        std::process::exit(0);
    }
    {
        let pid = std::process::id().to_string();
        lock_file.set_len(0).ok();
        let _ = lock_file.write_all(pid.as_bytes());
        let _ = lock_file.flush();
    }

    let engine = engine_path().unwrap_or_else(|| {
        eprintln!("ruwa: engine not found (RUWA_ENGINE unset, no ruwa-engine sibling)");
        std::process::exit(1);
    });

    let prefs = settings::read_prefs(&profile);
    let start_minimized = want_minimized || prefs.start_minimized;
    let close_to_tray = prefs.close_to_tray;
    vlog(&format!(
        "prefs: close_to_tray={close_to_tray} start_minimized={start_minimized} display={:?}",
        prefs.display
    ));

    let engine_alive = Arc::new(AtomicBool::new(false));
    let (tx, rx): (Sender<TrayAction>, Receiver<TrayAction>) = mpsc::channel();
    let tray_handle = {
        use ksni::blocking::TrayMethods;
        match (RuwaTray {
            tx: tx.clone(),
            engine_alive: engine_alive.clone(),
        })
        .spawn()
        {
            Ok(h) => Some(h),
            Err(err) => {
                eprintln!("ruwa: tray unavailable ({err:?}), running engine directly");
                None
            }
        }
    };

    let Some(handle) = tray_handle else {
        use std::os::unix::process::CommandExt;
        let mut cmd = Command::new(&engine);
        engine_env(&mut cmd, &profile);
        if extra_args.is_empty() {
            cmd.arg(WHATSAPP_HOME);
        } else {
            cmd.args(&extra_args);
        }
        let err = cmd.exec();
        eprintln!("ruwa: failed to exec engine: {err:?}");
        std::process::exit(1);
    };

    let mut child: Option<Child> = None;
    if !start_minimized {
        match spawn_engine(&engine, &profile, &extra_args) {
            Ok(c) => {
                println!("ruwa: engine pid {:?}", c.id());
                engine_alive.store(true, Ordering::Relaxed);
                child = Some(c);
            }
            Err(e) => {
                eprintln!("ruwa: failed to spawn engine: {e:?}");
                std::process::exit(1);
            }
        }
    } else {
        println!("ruwa: starting minimized to tray");
    }

    loop {
        match rx.recv_timeout(Duration::from_millis(300)) {
            Ok(TrayAction::Show) => {
                if child_alive(&mut child) {
                    let _ = spawn_engine(&engine, &profile, &[String::from(WHATSAPP_HOME)]);
                } else {
                    reap_if_exited(&mut child);
                    match spawn_engine(&engine, &profile, &extra_args) {
                        Ok(c) => {
                            println!("ruwa: engine pid {:?}", c.id());
                            engine_alive.store(true, Ordering::Relaxed);
                            child = Some(c);
                        }
                        Err(e) => eprintln!("ruwa: failed to spawn engine: {e:?}"),
                    }
                }
            }
            Ok(TrayAction::Reload) => {
                if let Some(c) = child.as_mut() {
                    vlog("reloading: restarting engine");
                    terminate_child(c);
                    let _ = c.wait();
                    child = None;
                }
                match spawn_engine(&engine, &profile, &extra_args) {
                    Ok(c) => {
                        println!("ruwa: engine pid {:?}", c.id());
                        engine_alive.store(true, Ordering::Relaxed);
                        child = Some(c);
                    }
                    Err(e) => eprintln!("ruwa: failed to spawn engine: {e:?}"),
                }
            }
            Ok(TrayAction::Settings) => open_settings(),
            Ok(TrayAction::Quit) => {
                if let Some(c) = child.as_mut() {
                    terminate_child(c);
                }
                break;
            }
            Err(RecvTimeoutError::Timeout) => {
                if reap_if_exited(&mut child) {
                    engine_alive.store(false, Ordering::Relaxed);
                    if !close_to_tray {
                        println!("ruwa: engine exited, quitting tray");
                        break;
                    }
                    vlog("engine exited, staying in tray (close-to-tray enabled)");
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    if let Some(c) = child.as_mut() {
        terminate_child(c);
    }
    drop(handle);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_has_expected_pixels() {
        let icon = paint_icon_with_attention(false);
        assert_eq!(icon.width, 64);
        assert_eq!(icon.height, 64);
        assert_eq!(icon.data.len(), 64 * 64 * 4);
        assert_eq!(&icon.data[0..4], &[255, 0x12, 0x17, 0x1a]);
    }

    #[test]
    fn icon_center_is_whatsapp_green() {
        let icon = paint_icon_with_attention(false);
        let size = 64usize;
        let i = (32 * size + 20) * 4;
        assert_eq!(icon.data[i], 255);
        assert_eq!(&icon.data[i + 1..i + 4], &[0x25, 0xD3, 0x66]);
    }

    #[test]
    fn attention_icon_has_red_dot() {
        let plain = paint_icon_with_attention(false);
        let alert = paint_icon_with_attention(true);
        assert_eq!(plain.data.len(), alert.data.len());
        assert_ne!(plain.data, alert.data);
        let size = 64usize;
        let i = (13 * size + 51) * 4;
        assert_eq!(&alert.data[i + 1..i + 4], &[0xFF, 0x53, 0x53]);
    }
}
