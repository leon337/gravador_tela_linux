use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Box as GtkBox, Button, ComboBoxText, Entry, Label, MessageDialog, Orientation};
use std::cell::RefCell;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

const APP_ID: &str = "br.com.leo.GravadorLinux";

#[derive(Clone, Copy)]
enum Mode { Screen, ScreenMic, ScreenSystem, ScreenBoth, Mic, System, AudioBoth }

impl Mode {
    fn from_index(i: u32) -> Self { match i { 0=>Self::Screen,1=>Self::ScreenMic,2=>Self::ScreenSystem,3=>Self::ScreenBoth,4=>Self::Mic,5=>Self::System,_=>Self::AudioBoth } }
    fn video(self)->bool { matches!(self,Self::Screen|Self::ScreenMic|Self::ScreenSystem|Self::ScreenBoth) }
    fn mic(self)->bool { matches!(self,Self::ScreenMic|Self::ScreenBoth|Self::Mic|Self::AudioBoth) }
    fn system(self)->bool { matches!(self,Self::ScreenSystem|Self::ScreenBoth|Self::System|Self::AudioBoth) }
}

struct Recorder { child: Option<Child>, output: Option<PathBuf> }
impl Recorder {
    fn new()->Self { Self{child:None,output:None} }
    fn start(&mut self, mode:Mode, dir:&Path, format:&str, quality:&str)->Result<PathBuf,String>{
        if self.child.is_some(){return Err("Já existe uma gravação em andamento.".into())}
        if mode.video() && std::env::var("XDG_SESSION_TYPE").unwrap_or_default() != "x11" {
            return Err("Esta versão inicial grava a tela em sessões X11. No Wayland, entre em uma sessão X11 do Linux Mint.".into())
        }
        if Command::new("sh").args(["-c","command -v ffmpeg >/dev/null"]).status().map_err(|e|e.to_string())?.success()==false { return Err("FFmpeg não encontrado. Execute o instalador novamente.".into()) }
        fs::create_dir_all(dir).map_err(|e|format!("Falha ao criar pasta: {e}"))?;
        let ext=if mode.video(){if format=="mp4"{"mp4"}else{"webm"}}else if format=="mp4"{"m4a"}else{"opus"};
        let output=dir.join(format!("gravacao-{}.{}",now(),ext));
        let log=File::create(output.with_extension(format!("{ext}.log"))).map_err(|e|e.to_string())?;
        let args=ffmpeg_args(mode,&output,format,quality)?;
        let child=Command::new("ffmpeg").args(args).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::from(log)).spawn().map_err(|e|format!("Falha ao iniciar FFmpeg: {e}"))?;
        self.output=Some(output.clone()); self.child=Some(child); Ok(output)
    }
    fn stop(&mut self)->Result<Option<PathBuf>,String>{
        let Some(mut child)=self.child.take() else{return Ok(None)};
        if let Some(stdin)=child.stdin.as_mut(){stdin.write_all(b"q\n").map_err(|e|e.to_string())?; let _=stdin.flush();}
        let status=child.wait().map_err(|e|e.to_string())?;
        let out=self.output.take();
        if !status.success(){return Err(format!("A gravação terminou com erro: {status}"))}
        Ok(out)
    }
}

fn main()->gtk::glib::ExitCode{
    let app=Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui); app.run()
}

fn build_ui(app:&Application){
    let recorder=Rc::new(RefCell::new(Recorder::new()));
    let window=ApplicationWindow::builder().application(app).title("Gravador Linux").default_width(520).default_height(430).build();
    let root=GtkBox::new(Orientation::Vertical,12); root.set_margin_top(22); root.set_margin_bottom(22); root.set_margin_start(22); root.set_margin_end(22);
    let title=Label::new(Some("Gravação de tela e áudio")); title.add_css_class("title-1"); title.set_halign(gtk::Align::Start);
    let info=Label::new(Some("MVP para Linux Mint. Captura de tela disponível inicialmente no X11.")); info.set_wrap(true); info.set_halign(gtk::Align::Start);
    let mode=ComboBoxText::new(); for t in ["Tela sem áudio","Tela + microfone","Tela + áudio do sistema","Tela + microfone + sistema","Somente microfone","Somente áudio do sistema","Somente microfone + sistema"]{mode.append_text(t)} mode.set_active(Some(3));
    let format=ComboBoxText::new(); format.append_text("WebM / Opus — leve"); format.append_text("MP4 / AAC — compatível"); format.set_active(Some(0));
    let quality=ComboBoxText::new(); quality.append_text("Econômica"); quality.append_text("Equilibrada"); quality.append_text("Alta"); quality.set_active(Some(1));
    let output=Entry::new(); output.set_text(&default_dir().to_string_lossy());
    let status=Label::new(Some("Pronto para gravar.")); status.set_wrap(true); status.set_halign(gtk::Align::Start);
    let buttons=GtkBox::new(Orientation::Horizontal,10); let start=Button::with_label("Iniciar gravação"); start.add_css_class("suggested-action"); let stop=Button::with_label("Parar"); stop.add_css_class("destructive-action"); stop.set_sensitive(false); buttons.append(&start); buttons.append(&stop);
    for w in [&title, &info]{root.append(w)}
    root.append(&field("Modo de gravação")); root.append(&mode); root.append(&field("Formato")); root.append(&format); root.append(&field("Qualidade")); root.append(&quality); root.append(&field("Pasta de saída")); root.append(&output); root.append(&buttons); root.append(&status); window.set_child(Some(&root));
    {
        let r=recorder.clone(); let w=window.clone(); let s=status.clone(); let st=stop.clone(); let sb=start.clone(); let m=mode.clone(); let f=format.clone(); let q=quality.clone(); let o=output.clone();
        start.connect_clicked(move |_|{
            let md=Mode::from_index(m.active().unwrap_or(3)); let fm=if f.active().unwrap_or(0)==0{"webm"}else{"mp4"}; let qu=match q.active().unwrap_or(1){0=>"economica",2=>"alta",_=>"equilibrada"};
            match r.borrow_mut().start(md,Path::new(o.text().as_str()),fm,qu){Ok(p)=>{s.set_text(&format!("● Gravando em: {}",p.display())); sb.set_sensitive(false); st.set_sensitive(true)},Err(e)=>error(&w,&e)}
        });
    }
    {
        let r=recorder.clone(); let w=window.clone(); let s=status.clone(); let sb=start.clone(); let st=stop.clone();
        stop.connect_clicked(move |_|{match r.borrow_mut().stop(){Ok(Some(p))=>{s.set_text(&format!("Gravação salva em: {}",p.display())); sb.set_sensitive(true); st.set_sensitive(false)},Ok(None)=>{},Err(e)=>error(&w,&e)}});
    }
    window.present();
}

fn field(t:&str)->Label{let l=Label::new(Some(t)); l.set_halign(gtk::Align::Start); l.add_css_class("heading"); l}
fn error(parent:&ApplicationWindow,msg:&str){let d=MessageDialog::builder().transient_for(parent).modal(true).text("Não foi possível iniciar").secondary_text(msg).build(); d.add_button("Fechar",gtk::ResponseType::Close); d.connect_response(|d,_|d.close()); d.present()}
fn now()->u64{SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()}
fn default_dir()->PathBuf{let h=PathBuf::from(std::env::var("HOME").unwrap_or_else(|_|".".into())); let v=h.join("Vídeos"); if v.exists(){v.join("Gravador Linux")}else{h.join("Videos/Gravador Linux")}}
fn geometry()->String{Command::new("sh").args(["-c","xrandr --current 2>/dev/null | awk '/\\*/ {print $1; exit}'"]).output().ok().map(|o|String::from_utf8_lossy(&o.stdout).trim().to_string()).filter(|s|s.contains('x')).unwrap_or_else(||"1920x1080".into())}
fn monitor()->Result<String,String>{let o=Command::new("pactl").arg("get-default-sink").output().map_err(|e|e.to_string())?; let s=String::from_utf8_lossy(&o.stdout).trim().to_string(); if s.is_empty(){Err("Saída de áudio padrão não encontrada.".into())}else{Ok(format!("{s}.monitor"))}}
fn ffmpeg_args(mode:Mode,out:&Path,format:&str,quality:&str)->Result<Vec<String>,String>{
    let mut a=vec!["-hide_banner".into(),"-y".into(),"-loglevel".into(),"warning".into()]; let mut aud=0usize;
    if mode.video(){a.extend(["-f".into(),"x11grab".into(),"-framerate".into(),if quality=="alta"{"60".into()}else{"30".into()},"-video_size".into(),geometry(),"-i".into(),std::env::var("DISPLAY").unwrap_or_else(|_|":0.0".into())]);}
    if mode.mic(){a.extend(["-thread_queue_size".into(),"1024".into(),"-f".into(),"pulse".into(),"-i".into(),"default".into()]); aud+=1;}
    if mode.system(){a.extend(["-thread_queue_size".into(),"1024".into(),"-f".into(),"pulse".into(),"-i".into(),monitor()?]); aud+=1;}
    if aud==2{let f=if mode.video(){1}else{0}; let s=f+1; a.extend(["-filter_complex".into(),format!("[{f}:a][{s}:a]amix=inputs=2:duration=longest[a]")]); if mode.video(){a.extend(["-map".into(),"0:v:0".into()])} a.extend(["-map".into(),"[a]".into()]);} else {if mode.video(){a.extend(["-map".into(),"0:v:0".into()])} if aud==1{a.extend(["-map".into(),format!("{}:a:0",if mode.video(){1}else{0})])}}
    if mode.video(){if format=="mp4"{a.extend(["-c:v".into(),"libx264".into(),"-preset".into(),"veryfast".into(),"-crf".into(),match quality{"economica"=>"30","alta"=>"20",_=>"24"}.into(),"-pix_fmt".into(),"yuv420p".into()]); if aud>0{a.extend(["-c:a".into(),"aac".into(),"-b:a".into(),"128k".into()])}}else{a.extend(["-c:v".into(),"libvpx-vp9".into(),"-deadline".into(),"realtime".into(),"-cpu-used".into(),"6".into(),"-crf".into(),match quality{"economica"=>"38","alta"=>"25",_=>"31"}.into(),"-b:v".into(),"0".into()]); if aud>0{a.extend(["-c:a".into(),"libopus".into(),"-b:a".into(),"96k".into()])}}}else if format=="mp4"{a.extend(["-c:a".into(),"aac".into(),"-b:a".into(),"128k".into()])}else{a.extend(["-c:a".into(),"libopus".into(),"-b:a".into(),match quality{"economica"=>"48k","alta"=>"128k",_=>"80k"}.into()])}
    a.push(out.to_string_lossy().to_string()); Ok(a)
}
