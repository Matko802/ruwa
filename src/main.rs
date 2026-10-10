use ksni::menu::{Disposition, StandardItem};
use ksni::{Category, Icon, MenuItem, Status, ToolTip, Tray};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

#[allow(dead_code)]
mod settings;

use settings::profile_dir;

const WHATSAPP_HOME: &str = "https://web.whatsapp.com/";
const ICON_SIZE: i32 = 64;

#[derive(Debug, Clone)]
enum TrayAction {
    Show,
    Settings,
    Quit,
}

fn blend(a: u8, b: u8, t: f64) -> u8 {
    (f64::from(a) * (1.0 - t) + f64::from(b) * t).round() as u8
}

fn paint_icon() -> Icon {
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
    let dist = |x: f64, y: f64, cx: f64, cy: f64| ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
    for y in 0..size {
        for x in 0..size {
            let d = dist(x as f64, y as f64, 30.0, 34.0);
            if d < 21.0 {
                let edge = ((d - 19.5) / 1.5).clamp(0.0, 1.0);
                let i = (y * size + x) * 4;
                out[i + 1] = blend(out[i + 1], 0x25, 1.0 - edge);
                out[i + 2] = blend(out[i + 2], 0xD3, 1.0 - edge);
                out[i + 3] = blend(out[i + 3], 0x66, 1.0 - edge);
            }
        }
    }
    Icon {
        width: ICON_SIZE,
        height: ICON_SIZE,
        data: out,
    }
}

struct RuwaTray {
    tx: Sender<TrayAction>,
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
        Status::Active
    }

    fn icon_name(&self) -> String {
        String::from("ruwa")
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        vec![paint_icon()]
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            icon_name: String::new(),
            icon_pixmap: Vec::new(),
            title: String::from("Ruwa"),
            description: String::from("WhatsApp desktop client"),
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.tx.send(TrayAction::Show);
    }

    fn menu(&self) -> Vec<MenuItem<RuwaTray>> {
        vec![
            item("Show", TrayAction::Show, &self.tx),
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

fn spawn_engine(engine: &Path, extra_args: &[String]) -> std::io::Result<Child> {
    let mut cmd = Command::new(engine);
    if extra_args.is_empty() {
        cmd.arg(WHATSAPP_HOME);
    } else {
        cmd.args(extra_args);
    }
    cmd.stdin(std::process::Stdio::null()).spawn()
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

fn main() {
    let extra_args: Vec<String> = std::env::args().skip(1).collect();
    let profile = profile_dir();
    let _ = std::fs::create_dir_all(&profile);

    let lock_path = profile.join(".ruwa-lock");
    let lock_file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)
        .expect("ruwa: cannot open lock file");
    {
        use fs2::FileExt;
        if lock_file.try_lock_exclusive().is_err() {
            let _ = writeln!(
                std::io::stderr(),
                "ruwa is already running, focusing existing window"
            );
            if let Some(engine) = engine_path() {
                let _ = spawn_engine(&engine, &extra_args);
            }
            std::process::exit(0);
        }
    }

    let engine = engine_path().unwrap_or_else(|| {
        eprintln!("ruwa: engine not found (RUWA_ENGINE unset, no ruwa-engine sibling)");
        std::process::exit(1);
    });

    let (tx, rx): (Sender<TrayAction>, Receiver<TrayAction>) = mpsc::channel();
    let tray_handle = {
        use ksni::blocking::TrayMethods;
        match (RuwaTray { tx: tx.clone() }).spawn() {
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
        if extra_args.is_empty() {
            cmd.arg(WHATSAPP_HOME);
        } else {
            cmd.args(&extra_args);
        }
        let err = cmd.exec();
        eprintln!("ruwa: failed to exec engine: {err:?}");
        std::process::exit(1);
    };

    let mut child = spawn_engine(&engine, &extra_args).unwrap_or_else(|e| {
        eprintln!("ruwa: failed to spawn engine: {e:?}");
        std::process::exit(1);
    });
    println!("ruwa: engine pid {:?}", child.id());

    loop {
        match rx.recv_timeout(Duration::from_millis(300)) {
            Ok(TrayAction::Show) => {
                let _ = spawn_engine(&engine, &[String::from(WHATSAPP_HOME)]);
            }
            Ok(TrayAction::Settings) => open_settings(),
            Ok(TrayAction::Quit) => {
                let _ = child.kill();
                break;
            }
            Err(RecvTimeoutError::Timeout) => match child.try_wait() {
                Ok(Some(status)) => {
                    println!("ruwa: engine exited ({status}), quitting tray");
                    break;
                }
                Ok(None) => {}
                Err(e) => {
                    eprintln!("ruwa: wait on engine failed: {e:?}");
                    break;
                }
            },
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    drop(handle);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_has_expected_pixels() {
        let icon = paint_icon();
        assert_eq!(icon.width, 64);
        assert_eq!(icon.height, 64);
        assert_eq!(icon.data.len(), 64 * 64 * 4);
        assert_eq!(&icon.data[0..4], &[255, 0x12, 0x17, 0x1a]);
    }
}
