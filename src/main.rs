use gtk::glib::{self, ControlFlow};
use gtk::prelude::*;
use gtk::{
    Align, Application, ApplicationWindow, Box as GtkBox, Button, CheckButton, ComboBoxText,
    CssProvider, FileChooserAction, FileChooserNative, Label, MessageDialog, Orientation,
    ResponseType, Separator,
};
use std::cell::RefCell;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::rc::Rc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const APP_ID: &str = "br.com.leo.GravadorLinux";

const CSS: &str = r#"
window {
    background: #0b1020;
    color: #eef4ff;
}

.app-shell {
    padding: 28px;
}

.brand-title {
    font-size: 28px;
    font-weight: 800;
    color: #f7faff;
}

.brand-subtitle {
    font-size: 14px;
    color: #93a4bd;
}

.status-pill {
    background: rgba(34, 211, 238, 0.12);
    border: 1px solid rgba(34, 211, 238, 0.35);
    border-radius: 999px;
    padding: 7px 12px;
    color: #67e8f9;
    font-weight: 700;
}

.warning-banner {
    background: rgba(245, 158, 11, 0.12);
    border: 1px solid rgba(245, 158, 11, 0.38);
    border-radius: 12px;
    padding: 12px 14px;
    color: #fbbf24;
}

.card {
    background: rgba(18, 27, 48, 0.92);
    border: 1px solid rgba(148, 163, 184, 0.16);
    border-radius: 18px;
    padding: 20px;
}

.card-title {
    font-size: 16px;
    font-weight: 750;
    color: #f8fafc;
}

.card-description,
.muted {
    color: #93a4bd;
}

.option-row {
    background: rgba(9, 15, 30, 0.58);
    border: 1px solid rgba(148, 163, 184, 0.12);
    border-radius: 12px;
    padding: 12px 14px;
}

checkbutton {
    color: #e5edf8;
    font-weight: 650;
}

combobox button,
.folder-button {
    min-height: 42px;
    border-radius: 11px;
    background: #17233b;
    border: 1px solid rgba(148, 163, 184, 0.22);
    color: #eef4ff;
}

.record-button {
    min-height: 54px;
    border-radius: 14px;
    background: #06b6d4;
    color: #041017;
    font-size: 16px;
    font-weight: 800;
    border: none;
}

.record-button:hover {
    background: #22d3ee;
}

.stop-button {
    min-height: 54px;
    border-radius: 14px;
    background: #ef4444;
    color: #ffffff;
    font-size: 16px;
    font-weight: 800;
    border: none;
}

.stop-button:hover {
    background: #f87171;
}

.timer {
    font-size: 34px;
    font-weight: 800;
    font-family: monospace;
    color: #f8fafc;
}

.recording-dot {
    color: #f87171;
    font-size: 18px;
}

.path-label {
    color: #cbd5e1;
    font-family: monospace;
    font-size: 12px;
}

separator {
    background: rgba(148, 163, 184, 0.14);
}
"#;

#[derive(Clone, Copy)]
enum Mode {
    Screen,
    ScreenMic,
    ScreenSystem,
    ScreenBoth,
    Mic,
    System,
    AudioBoth,
}

impl Mode {
    fn from_flags(screen: bool, mic: bool, system: bool) -> Result<Self, String> {
        match (screen, mic, system) {
            (true, false, false) => Ok(Self::Screen),
            (true, true, false) => Ok(Self::ScreenMic),
            (true, false, true) => Ok(Self::ScreenSystem),
            (true, true, true) => Ok(Self::ScreenBoth),
            (false, true, false) => Ok(Self::Mic),
            (false, false, true) => Ok(Self::System),
            (false, true, true) => Ok(Self::AudioBoth),
            (false, false, false) => Err("Selecione pelo menos uma fonte de gravação.".into()),
        }
    }

    fn video(self) -> bool {
        matches!(
            self,
            Self::Screen | Self::ScreenMic | Self::ScreenSystem | Self::ScreenBoth
        )
    }

    fn mic(self) -> bool {
        matches!(
            self,
            Self::ScreenMic | Self::ScreenBoth | Self::Mic | Self::AudioBoth
        )
    }

    fn system(self) -> bool {
        matches!(
            self,
            Self::ScreenSystem | Self::ScreenBoth | Self::System | Self::AudioBoth
        )
    }
}

struct Recorder {
    child: Option<Child>,
    output: Option<PathBuf>,
}

impl Recorder {
    fn new() -> Self {
        Self {
            child: None,
            output: None,
        }
    }

    fn start(
        &mut self,
        mode: Mode,
        dir: &Path,
        format: &str,
        quality: &str,
    ) -> Result<PathBuf, String> {
        if self.child.is_some() {
            return Err("Já existe uma gravação em andamento.".into());
        }

        if mode.video() && std::env::var("XDG_SESSION_TYPE").unwrap_or_default() != "x11" {
            return Err(
                "A captura de tela desta versão funciona em sessões X11. Entre em uma sessão X11 do Linux Mint ou grave somente áudio."
                    .into(),
            );
        }

        if !Command::new("sh")
            .args(["-c", "command -v ffmpeg >/dev/null"])
            .status()
            .map_err(|e| e.to_string())?
            .success()
        {
            return Err("FFmpeg não encontrado. Execute o instalador novamente.".into());
        }

        fs::create_dir_all(dir).map_err(|e| format!("Falha ao criar pasta: {e}"))?;
        let ext = if mode.video() {
            if format == "mp4" { "mp4" } else { "webm" }
        } else if format == "mp4" {
            "m4a"
        } else {
            "opus"
        };

        let output = dir.join(format!("gravacao-{}.{}", now(), ext));
        let log = File::create(output.with_extension(format!("{ext}.log")))
            .map_err(|e| e.to_string())?;
        let args = ffmpeg_args(mode, &output, format, quality)?;
        let child = Command::new("ffmpeg")
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::from(log))
            .spawn()
            .map_err(|e| format!("Falha ao iniciar FFmpeg: {e}"))?;

        self.output = Some(output.clone());
        self.child = Some(child);
        Ok(output)
    }

    fn stop(&mut self) -> Result<Option<PathBuf>, String> {
        let Some(mut child) = self.child.take() else {
            return Ok(None);
        };

        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(b"q\n").map_err(|e| e.to_string())?;
            let _ = stdin.flush();
        }

        let status = child.wait().map_err(|e| e.to_string())?;
        let output = self.output.take();
        if !status.success() {
            return Err(format!("A gravação terminou com erro: {status}"));
        }
        Ok(output)
    }
}

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_startup(|_| install_css());
    app.connect_activate(build_ui);
    app.run()
}

fn install_css() {
    let provider = CssProvider::new();
    provider.load_from_data(CSS);
    gtk::style_context_add_provider_for_display(
        &gtk::gdk::Display::default().expect("Não foi possível acessar o display"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn build_ui(app: &Application) {
    let recorder = Rc::new(RefCell::new(Recorder::new()));
    let selected_dir = Rc::new(RefCell::new(default_dir()));
    let timer_source: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Gravador Linux")
        .default_width(760)
        .default_height(720)
        .resizable(true)
        .build();

    let root = GtkBox::new(Orientation::Vertical, 18);
    root.add_css_class("app-shell");

    let header = GtkBox::new(Orientation::Horizontal, 16);
    let brand = GtkBox::new(Orientation::Vertical, 4);
    brand.set_hexpand(true);

    let title = Label::new(Some("Gravador Linux"));
    title.add_css_class("brand-title");
    title.set_halign(Align::Start);

    let subtitle = Label::new(Some("Capture sua tela e áudio com poucos cliques"));
    subtitle.add_css_class("brand-subtitle");
    subtitle.set_halign(Align::Start);

    let ready_badge = Label::new(Some("● PRONTO"));
    ready_badge.add_css_class("status-pill");
    ready_badge.set_valign(Align::Center);

    brand.append(&title);
    brand.append(&subtitle);
    header.append(&brand);
    header.append(&ready_badge);
    root.append(&header);

    if std::env::var("XDG_SESSION_TYPE").unwrap_or_default() != "x11" {
        let warning = Label::new(Some(
            "⚠ Sessão Wayland detectada: nesta versão, a tela exige X11. A gravação de áudio continua disponível.",
        ));
        warning.add_css_class("warning-banner");
        warning.set_wrap(true);
        warning.set_halign(Align::Fill);
        warning.set_xalign(0.0);
        root.append(&warning);
    }

    let sources_card = card();
    let sources_title = Label::new(Some("O que você quer gravar?"));
    sources_title.add_css_class("card-title");
    sources_title.set_halign(Align::Start);

    let sources_description = Label::new(Some("Ative uma ou mais fontes. A combinação é configurada automaticamente."));
    sources_description.add_css_class("card-description");
    sources_description.set_halign(Align::Start);
    sources_description.set_wrap(true);

    let screen = CheckButton::with_label("Tela");
    screen.set_active(true);
    let mic = CheckButton::with_label("Microfone");
    mic.set_active(true);
    let system_audio = CheckButton::with_label("Áudio do sistema");
    system_audio.set_active(true);

    for option in [&screen, &mic, &system_audio] {
        option.add_css_class("option-row");
        option.set_halign(Align::Fill);
        sources_card.append(option);
    }

    sources_card.prepend(&sources_description);
    sources_card.prepend(&sources_title);
    root.append(&sources_card);

    let settings_card = card();
    let settings_title = Label::new(Some("Configurações"));
    settings_title.add_css_class("card-title");
    settings_title.set_halign(Align::Start);
    settings_card.append(&settings_title);

    let settings_grid = GtkBox::new(Orientation::Horizontal, 14);
    let format_box = field_box("Formato");
    let format = ComboBoxText::new();
    format.append_text("WebM / Opus — leve");
    format.append_text("MP4 / AAC — compatível");
    format.set_active(Some(0));
    format.set_hexpand(true);
    format_box.append(&format);

    let quality_box = field_box("Qualidade");
    let quality = ComboBoxText::new();
    quality.append_text("Econômica");
    quality.append_text("Equilibrada");
    quality.append_text("Alta");
    quality.set_active(Some(1));
    quality.set_hexpand(true);
    quality_box.append(&quality);

    settings_grid.append(&format_box);
    settings_grid.append(&quality_box);
    settings_card.append(&settings_grid);

    let separator = Separator::new(Orientation::Horizontal);
    settings_card.append(&separator);

    let folder_title = Label::new(Some("Pasta de saída"));
    folder_title.add_css_class("muted");
    folder_title.set_halign(Align::Start);

    let folder_row = GtkBox::new(Orientation::Horizontal, 12);
    let folder_path = Label::new(Some(&selected_dir.borrow().to_string_lossy()));
    folder_path.add_css_class("path-label");
    folder_path.set_halign(Align::Start);
    folder_path.set_hexpand(true);
    folder_path.set_ellipsize(gtk::pango::EllipsizeMode::Middle);

    let choose_folder = Button::with_label("Escolher pasta");
    choose_folder.add_css_class("folder-button");
    folder_row.append(&folder_path);
    folder_row.append(&choose_folder);
    settings_card.append(&folder_title);
    settings_card.append(&folder_row);
    root.append(&settings_card);

    {
        let selected_dir = selected_dir.clone();
        let folder_path = folder_path.clone();
        let window = window.clone();
        choose_folder.connect_clicked(move |_| {
            let chooser = FileChooserNative::builder()
                .title("Escolher pasta de saída")
                .transient_for(&window)
                .action(FileChooserAction::SelectFolder)
                .accept_label("Selecionar")
                .cancel_label("Cancelar")
                .build();

            let selected_dir = selected_dir.clone();
            let folder_path = folder_path.clone();
            chooser.connect_response(move |dialog, response| {
                if response == ResponseType::Accept {
                    if let Some(path) = dialog.file().and_then(|file| file.path()) {
                        folder_path.set_text(&path.to_string_lossy());
                        *selected_dir.borrow_mut() = path;
                    }
                }
                dialog.destroy();
            });
            chooser.show();
        });
    }

    let recording_card = card();
    let recording_header = GtkBox::new(Orientation::Horizontal, 10);
    let recording_dot = Label::new(Some("●"));
    recording_dot.add_css_class("recording-dot");
    recording_dot.set_visible(false);

    let status = Label::new(Some("Pronto para gravar"));
    status.set_halign(Align::Start);
    status.set_hexpand(true);
    status.add_css_class("muted");

    let timer = Label::new(Some("00:00:00"));
    timer.add_css_class("timer");
    timer.set_halign(Align::Center);

    recording_header.append(&recording_dot);
    recording_header.append(&status);
    recording_card.append(&recording_header);
    recording_card.append(&timer);

    let controls = GtkBox::new(Orientation::Horizontal, 12);
    let start = Button::with_label("●  Iniciar gravação");
    start.add_css_class("record-button");
    start.set_hexpand(true);

    let stop = Button::with_label("■  Parar e salvar");
    stop.add_css_class("stop-button");
    stop.set_hexpand(true);
    stop.set_sensitive(false);

    controls.append(&start);
    controls.append(&stop);
    recording_card.append(&controls);
    root.append(&recording_card);

    {
        let recorder = recorder.clone();
        let window = window.clone();
        let status = status.clone();
        let timer = timer.clone();
        let timer_source = timer_source.clone();
        let recording_dot = recording_dot.clone();
        let ready_badge = ready_badge.clone();
        let stop = stop.clone();
        let start_button = start.clone();
        let screen = screen.clone();
        let mic = mic.clone();
        let system_audio = system_audio.clone();
        let format = format.clone();
        let quality = quality.clone();
        let selected_dir = selected_dir.clone();

        start.connect_clicked(move |_| {
            let mode = match Mode::from_flags(
                screen.is_active(),
                mic.is_active(),
                system_audio.is_active(),
            ) {
                Ok(mode) => mode,
                Err(message) => {
                    error(&window, "Configuração incompleta", &message);
                    return;
                }
            };

            let selected_format = if format.active().unwrap_or(0) == 0 {
                "webm"
            } else {
                "mp4"
            };
            let selected_quality = match quality.active().unwrap_or(1) {
                0 => "economica",
                2 => "alta",
                _ => "equilibrada",
            };
            let output_dir = selected_dir.borrow().clone();

            match recorder.borrow_mut().start(
                mode,
                &output_dir,
                selected_format,
                selected_quality,
            ) {
                Ok(path) => {
                    status.set_text(&format!("Gravando em {}", path.display()));
                    status.remove_css_class("muted");
                    recording_dot.set_visible(true);
                    ready_badge.set_text("● GRAVANDO");
                    start_button.set_sensitive(false);
                    stop.set_sensitive(true);
                    screen.set_sensitive(false);
                    mic.set_sensitive(false);
                    system_audio.set_sensitive(false);
                    format.set_sensitive(false);
                    quality.set_sensitive(false);

                    let started_at = Instant::now();
                    timer.set_text("00:00:00");
                    let timer_label = timer.clone();
                    let source = glib::timeout_add_seconds_local(1, move || {
                        timer_label.set_text(&format_duration(started_at.elapsed().as_secs()));
                        ControlFlow::Continue
                    });
                    *timer_source.borrow_mut() = Some(source);
                }
                Err(message) => error(&window, "Não foi possível iniciar", &message),
            }
        });
    }

    {
        let recorder = recorder.clone();
        let window = window.clone();
        let status = status.clone();
        let timer_source = timer_source.clone();
        let recording_dot = recording_dot.clone();
        let ready_badge = ready_badge.clone();
        let start = start.clone();
        let stop_button = stop.clone();
        let screen = screen.clone();
        let mic = mic.clone();
        let system_audio = system_audio.clone();
        let format = format.clone();
        let quality = quality.clone();

        stop.connect_clicked(move |_| {
            match recorder.borrow_mut().stop() {
                Ok(Some(path)) => {
                    if let Some(source) = timer_source.borrow_mut().take() {
                        source.remove();
                    }
                    status.set_text(&format!("Gravação salva em {}", path.display()));
                    status.add_css_class("muted");
                    recording_dot.set_visible(false);
                    ready_badge.set_text("● CONCLUÍDO");
                    start.set_sensitive(true);
                    stop_button.set_sensitive(false);
                    screen.set_sensitive(true);
                    mic.set_sensitive(true);
                    system_audio.set_sensitive(true);
                    format.set_sensitive(true);
                    quality.set_sensitive(true);
                }
                Ok(None) => {}
                Err(message) => error(&window, "Falha ao finalizar", &message),
            }
        });
    }

    window.set_child(Some(&root));
    window.present();
}

fn card() -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 12);
    container.add_css_class("card");
    container
}

fn field_box(title: &str) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 7);
    container.set_hexpand(true);
    let label = Label::new(Some(title));
    label.add_css_class("muted");
    label.set_halign(Align::Start);
    container.append(&label);
    container
}

fn error(parent: &ApplicationWindow, title: &str, message: &str) {
    let dialog = MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .text(title)
        .secondary_text(message)
        .build();
    dialog.add_button("Fechar", ResponseType::Close);
    dialog.connect_response(|dialog, _| dialog.close());
    dialog.present();
}

fn format_duration(total_seconds: u64) -> String {
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn default_dir() -> PathBuf {
    let home = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()));
    let videos = home.join("Vídeos");
    if videos.exists() {
        videos.join("Gravador Linux")
    } else {
        home.join("Videos/Gravador Linux")
    }
}

fn geometry() -> String {
    Command::new("sh")
        .args([
            "-c",
            "xrandr --current 2>/dev/null | awk '/\\*/ {print $1; exit}'",
        ])
        .output()
        .ok()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|size| size.contains('x'))
        .unwrap_or_else(|| "1920x1080".into())
}

fn monitor() -> Result<String, String> {
    let output = Command::new("pactl")
        .arg("get-default-sink")
        .output()
        .map_err(|e| e.to_string())?;
    let sink = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if sink.is_empty() {
        Err("Saída de áudio padrão não encontrada.".into())
    } else {
        Ok(format!("{sink}.monitor"))
    }
}

fn ffmpeg_args(
    mode: Mode,
    output: &Path,
    format: &str,
    quality: &str,
) -> Result<Vec<String>, String> {
    let mut args = vec![
        "-hide_banner".into(),
        "-y".into(),
        "-loglevel".into(),
        "warning".into(),
    ];
    let mut audio_inputs = 0usize;

    if mode.video() {
        args.extend([
            "-f".into(),
            "x11grab".into(),
            "-framerate".into(),
            if quality == "alta" { "60".into() } else { "30".into() },
            "-video_size".into(),
            geometry(),
            "-i".into(),
            std::env::var("DISPLAY").unwrap_or_else(|_| ":0.0".into()),
        ]);
    }

    if mode.mic() {
        args.extend([
            "-thread_queue_size".into(),
            "1024".into(),
            "-f".into(),
            "pulse".into(),
            "-i".into(),
            "default".into(),
        ]);
        audio_inputs += 1;
    }

    if mode.system() {
        args.extend([
            "-thread_queue_size".into(),
            "1024".into(),
            "-f".into(),
            "pulse".into(),
            "-i".into(),
            monitor()?,
        ]);
        audio_inputs += 1;
    }

    if audio_inputs == 2 {
        let first = if mode.video() { 1 } else { 0 };
        let second = first + 1;
        args.extend([
            "-filter_complex".into(),
            format!("[{first}:a][{second}:a]amix=inputs=2:duration=longest[a]"),
        ]);
        if mode.video() {
            args.extend(["-map".into(), "0:v:0".into()]);
        }
        args.extend(["-map".into(), "[a]".into()]);
    } else {
        if mode.video() {
            args.extend(["-map".into(), "0:v:0".into()]);
        }
        if audio_inputs == 1 {
            args.extend([
                "-map".into(),
                format!("{}:a:0", if mode.video() { 1 } else { 0 }),
            ]);
        }
    }

    if mode.video() {
        if format == "mp4" {
            args.extend([
                "-c:v".into(),
                "libx264".into(),
                "-preset".into(),
                "veryfast".into(),
                "-crf".into(),
                match quality {
                    "economica" => "30",
                    "alta" => "20",
                    _ => "24",
                }
                .into(),
                "-pix_fmt".into(),
                "yuv420p".into(),
            ]);
            if audio_inputs > 0 {
                args.extend([
                    "-c:a".into(),
                    "aac".into(),
                    "-b:a".into(),
                    "128k".into(),
                ]);
            }
        } else {
            args.extend([
                "-c:v".into(),
                "libvpx-vp9".into(),
                "-deadline".into(),
                "realtime".into(),
                "-cpu-used".into(),
                "6".into(),
                "-crf".into(),
                match quality {
                    "economica" => "38",
                    "alta" => "25",
                    _ => "31",
                }
                .into(),
                "-b:v".into(),
                "0".into(),
            ]);
            if audio_inputs > 0 {
                args.extend([
                    "-c:a".into(),
                    "libopus".into(),
                    "-b:a".into(),
                    "96k".into(),
                ]);
            }
        }
    } else if format == "mp4" {
        args.extend([
            "-c:a".into(),
            "aac".into(),
            "-b:a".into(),
            "128k".into(),
        ]);
    } else {
        args.extend([
            "-c:a".into(),
            "libopus".into(),
            "-b:a".into(),
            match quality {
                "economica" => "48k",
                "alta" => "128k",
                _ => "80k",
            }
            .into(),
        ]);
    }

    args.push(output.to_string_lossy().to_string());
    Ok(args)
}
