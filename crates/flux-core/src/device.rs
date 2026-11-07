//! Audio I/O. The audio callback owns the `Renderer`; the control thread talks to it only
//! through a lock-free queue of plain messages and boxed plans. Old plans are sent back to be
//! dropped off the audio thread. Meters are published through atomics. When there is no output
//! device (CI, headless MCP) a "null" device renders on a timer so transport and meters still work.

use crate::analysis::LiveLoudness;
use crate::render::{Msg, Plan, Renderer};
use serde::Serialize;
use std::sync::atomic::{
    AtomicBool, AtomicI64, AtomicU32, AtomicU64, AtomicUsize, Ordering::Relaxed,
};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const SCOPE: usize = 4096;
pub const MAX_TRACK_METERS: usize = 64;

pub enum Cmd {
    Msg(Msg),
    Plan(Box<Plan>),
    Monitor(bool),
}

fn f2u(v: f32) -> u32 {
    v.to_bits()
}
fn u2f(v: u32) -> f32 {
    f32::from_bits(v)
}

/// State published by the audio thread.
pub struct Shared {
    pub pos: AtomicI64,
    pub playing: AtomicBool,
    pub peak: [AtomicU32; 2],
    pub lufs_m: AtomicU32,
    pub lufs_s: AtomicU32,
    pub lufs_i: AtomicU32,
    pub glitches: AtomicU64,
    pub callbacks: AtomicU64,
    pub load: AtomicU32,
    pub track_peak: Vec<AtomicU32>,
    pub scope_l: Vec<AtomicU32>,
    pub scope_r: Vec<AtomicU32>,
    pub scope_w: AtomicUsize,
    pub in_peak: AtomicU32,
    pub alive: AtomicBool,
}
impl Shared {
    fn new() -> Arc<Self> {
        let z = || AtomicU32::new(0);
        Arc::new(Shared {
            pos: AtomicI64::new(0),
            playing: AtomicBool::new(false),
            peak: [z(), z()],
            lufs_m: AtomicU32::new(f2u(-70.0)),
            lufs_s: AtomicU32::new(f2u(-70.0)),
            lufs_i: AtomicU32::new(f2u(-70.0)),
            glitches: AtomicU64::new(0),
            callbacks: AtomicU64::new(0),
            load: z(),
            track_peak: (0..MAX_TRACK_METERS).map(|_| z()).collect(),
            scope_l: (0..SCOPE).map(|_| z()).collect(),
            scope_r: (0..SCOPE).map(|_| z()).collect(),
            scope_w: AtomicUsize::new(0),
            in_peak: z(),
            alive: AtomicBool::new(true),
        })
    }
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct Meters {
    pub pos: i64,
    pub playing: bool,
    pub peak: [f32; 2],
    pub lufs_m: f32,
    pub lufs_s: f32,
    pub lufs_i: f32,
    pub glitches: u64,
    pub load: f32,
    pub tracks: Vec<f32>,
    pub input_peak: f32,
}

/// Everything the audio callback needs. Preallocated; `process` never allocates.
struct Cb {
    r: Renderer,
    rx: rtrb::Consumer<Cmd>,
    garbage: rtrb::Producer<Box<Plan>>,
    sh: Arc<Shared>,
    live: LiveLoudness,
    ratio: f64,
    // resampler FIFO at project rate
    fl: Vec<f32>,
    fr: Vec<f32>,
    fcount: usize,
    fpos: f64,
    tl: Vec<f32>,
    tr: Vec<f32>,
    mon: Option<rtrb::Consumer<f32>>,
    monitor: bool,
    out_sr: f32,
    latency: i64,
}

const FIFO: usize = 16384;
const CHUNK: usize = 1024;

impl Cb {
    fn new(
        r: Renderer,
        rx: rtrb::Consumer<Cmd>,
        garbage: rtrb::Producer<Box<Plan>>,
        sh: Arc<Shared>,
        out_sr: u32,
        mon: Option<rtrb::Consumer<f32>>,
    ) -> Self {
        let sr = r.plan.project.sample_rate;
        let latency = r.latency() as i64;
        Cb {
            live: LiveLoudness::new(out_sr as f32),
            ratio: sr as f64 / out_sr as f64,
            r,
            rx,
            garbage,
            sh,
            fl: vec![0.0; FIFO],
            fr: vec![0.0; FIFO],
            fcount: 0,
            fpos: 1.0,
            tl: vec![0.0; CHUNK],
            tr: vec![0.0; CHUNK],
            mon,
            monitor: false,
            out_sr: out_sr as f32,
            latency,
        }
    }

    fn drain(&mut self) {
        while let Ok(c) = self.rx.pop() {
            match c {
                Cmd::Msg(m) => {
                    if let Msg::Seek(_) = m {
                        self.fcount = 0;
                        self.fpos = 1.0;
                        self.live.reset();
                    }
                    if let Msg::Play(true) = m {
                        self.live.reset();
                    }
                    self.r.apply(m);
                }
                Cmd::Plan(p) => {
                    let new_sr = p.project.sample_rate as f64;
                    let old = self.r.swap_plan(*p);
                    self.latency = self.r.latency() as i64;
                    self.ratio = new_sr / self.out_sr as f64;
                    // hand the old plan back; if the queue is full, keep it alive until next time
                    if let Err(rtrb::PushError::Full(b)) = self.garbage.push(Box::new(old)) {
                        std::mem::forget(b);
                    }
                }
                Cmd::Monitor(m) => self.monitor = m,
            }
        }
    }

    /// Fill `n` output frames into tl/tr (n <= CHUNK).
    fn render_chunk(&mut self, n: usize) {
        if (self.ratio - 1.0).abs() < 1e-9 {
            let (l, r) = (&mut self.tl[..n], &mut self.tr[..n]);
            self.r.render(l, r);
            return;
        }
        for k in 0..n {
            while self.fpos + 3.0 >= self.fcount as f64 {
                if self.fcount + 256 > FIFO {
                    let drop = (self.fpos.floor() as usize)
                        .saturating_sub(2)
                        .min(self.fcount);
                    self.fl.copy_within(drop..self.fcount, 0);
                    self.fr.copy_within(drop..self.fcount, 0);
                    self.fcount -= drop;
                    self.fpos -= drop as f64;
                }
                let c = self.fcount;
                let (a, b) = (&mut self.fl[c..c + 256], &mut self.fr[c..c + 256]);
                self.r.render(a, b);
                self.fcount += 256;
            }
            let i = self.fpos.floor() as usize;
            let t = (self.fpos - i as f64) as f32;
            let i = i.max(1);
            self.tl[k] = crate::dsp::hermite(
                self.fl[i - 1],
                self.fl[i],
                self.fl[i + 1],
                self.fl[i + 2],
                t,
            );
            self.tr[k] = crate::dsp::hermite(
                self.fr[i - 1],
                self.fr[i],
                self.fr[i + 1],
                self.fr[i + 2],
                t,
            );
            self.fpos += self.ratio;
        }
    }

    fn publish(&mut self, n: usize) {
        let sh = &self.sh;
        let (mut pl, mut pr) = (0f32, 0f32);
        let mut w = sh.scope_w.load(Relaxed);
        for k in 0..n {
            pl = pl.max(self.tl[k].abs());
            pr = pr.max(self.tr[k].abs());
            sh.scope_l[w].store(f2u(self.tl[k]), Relaxed);
            sh.scope_r[w].store(f2u(self.tr[k]), Relaxed);
            w = (w + 1) % SCOPE;
        }
        sh.scope_w.store(w, Relaxed);
        let old = [u2f(sh.peak[0].load(Relaxed)), u2f(sh.peak[1].load(Relaxed))];
        sh.peak[0].store(f2u(pl.max(old[0] * 0.93)), Relaxed);
        sh.peak[1].store(f2u(pr.max(old[1] * 0.93)), Relaxed);
        if self.r.playing {
            self.live.push(&self.tl[..n], &self.tr[..n]);
            sh.lufs_m.store(f2u(self.live.momentary), Relaxed);
            sh.lufs_s.store(f2u(self.live.short_term), Relaxed);
            sh.lufs_i.store(f2u(self.live.integrated()), Relaxed);
        }
        for (i, t) in self.r.plan.tracks.iter().enumerate().take(MAX_TRACK_METERS) {
            sh.track_peak[i].store(f2u(t.peak), Relaxed);
        }
        sh.pos
            .store((self.r.pos as i64 - self.latency).max(0), Relaxed);
        sh.playing.store(self.r.playing, Relaxed);
    }

    /// Interleaved output of any channel count.
    fn process<T: Copy>(&mut self, out: &mut [T], ch: usize, conv: impl Fn(f32) -> T) {
        let t0 = Instant::now();
        self.drain();
        let frames = out.len() / ch.max(1);
        let mut done = 0;
        while done < frames {
            let n = (frames - done).min(CHUNK);
            self.render_chunk(n);
            if self.monitor {
                if let Some(m) = self.mon.as_mut() {
                    for k in 0..n {
                        let a = m.pop().unwrap_or(0.0);
                        let b = m.pop().unwrap_or(a);
                        self.tl[k] += a;
                        self.tr[k] += b;
                    }
                }
            } else if let Some(m) = self.mon.as_mut() {
                // keep the monitor queue from backing up
                while m.pop().is_ok() {}
            }
            self.publish(n);
            for k in 0..n {
                let f = (done + k) * ch;
                match ch {
                    1 => out[f] = conv((self.tl[k] + self.tr[k]) * 0.5),
                    _ => {
                        out[f] = conv(self.tl[k]);
                        out[f + 1] = conv(self.tr[k]);
                        for c in 2..ch {
                            out[f + c] = conv(0.0);
                        }
                    }
                }
            }
            done += n;
        }
        let budget = frames as f64 / self.out_sr as f64;
        let used = t0.elapsed().as_secs_f64();
        self.sh
            .load
            .store(f2u((used / budget.max(1e-6)) as f32), Relaxed);
        if used > budget {
            self.sh.glitches.fetch_add(1, Relaxed);
        }
        self.sh.callbacks.fetch_add(1, Relaxed);
    }
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub default: bool,
    pub channels: u16,
    pub sample_rate: u32,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct DeviceList {
    pub host: String,
    pub hosts: Vec<String>,
    pub outputs: Vec<DeviceInfo>,
    pub inputs: Vec<DeviceInfo>,
}

#[derive(Clone, Debug, Default, serde::Deserialize, Serialize)]
#[serde(default)]
pub struct DevicePrefs {
    pub output: Option<String>,
    pub input: Option<String>,
    pub buffer: Option<u32>,
    pub null: bool,
}

pub struct AudioEngine {
    pub shared: Arc<Shared>,
    tx: rtrb::Producer<Cmd>,
    garbage: rtrb::Consumer<Box<Plan>>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    pub backend: String,
    pub device: String,
    pub sample_rate: u32,
    pub buffer: u32,
    pub error: Option<String>,
    mon_tx: Option<rtrb::Producer<f32>>,
    rec: Option<Recorder>,
}

impl AudioEngine {
    /// Open the output (or the null device). `mk` builds the plan; it is called again if the
    /// device fails and the engine falls back to the null device.
    pub fn start(mk: &dyn Fn() -> Plan, prefs: &DevicePrefs) -> AudioEngine {
        let e = Self::start_once(mk(), prefs);
        if e.backend == "dead" {
            let mut p = prefs.clone();
            p.null = true;
            let mut n = Self::start_once(mk(), &p);
            n.error = e.error.clone();
            return n;
        }
        e
    }

    fn start_once(plan: Plan, prefs: &DevicePrefs) -> AudioEngine {
        let shared = Shared::new();
        let (tx, rx) = rtrb::RingBuffer::<Cmd>::new(1024);
        let (gtx, grx) = rtrb::RingBuffer::<Box<Plan>>::new(64);
        let (mtx, mrx) = rtrb::RingBuffer::<f32>::new(48000 * 2);
        let stop = Arc::new(AtomicBool::new(false));
        let r = Renderer::new(plan);
        let proj_sr = r.plan.project.sample_rate;
        let null = prefs.null || std::env::var_os("FLUX_NULL_AUDIO").is_some();
        let (ready_tx, ready_rx) =
            std::sync::mpsc::channel::<Result<(String, String, u32, u32), String>>();
        let sh = shared.clone();
        let st = stop.clone();
        let prefs2 = prefs.clone();
        let thread = std::thread::Builder::new()
            .name("flux-audio".into())
            .spawn(move || {
                #[cfg(feature = "device")]
                if !null {
                    match open_output(&prefs2, proj_sr) {
                        Ok((dev, name, backend, cfg, fmt)) => {
                            let sr = cfg.sample_rate;
                            let ch = cfg.channels as usize;
                            let buf = match cfg.buffer_size {
                                cpal::BufferSize::Fixed(b) => b,
                                _ => 0,
                            };
                            let mut cb = Cb::new(r, rx, gtx, sh.clone(), sr, Some(mrx));
                            let err_sh = sh.clone();
                            let err = move |_e: cpal::Error| {
                                err_sh.glitches.fetch_add(1, Relaxed);
                            };
                            use cpal::traits::{DeviceTrait, StreamTrait};
                            let stream = match fmt {
                                cpal::SampleFormat::I16 => dev.build_output_stream::<i16, _, _>(
                                    cfg,
                                    move |d: &mut [i16], _| {
                                        cb.process(d, ch, |v| (v.clamp(-1.0, 1.0) * 32767.0) as i16)
                                    },
                                    err,
                                    None,
                                ),
                                cpal::SampleFormat::U16 => dev.build_output_stream::<u16, _, _>(
                                    cfg,
                                    move |d: &mut [u16], _| {
                                        cb.process(d, ch, |v| {
                                            ((v.clamp(-1.0, 1.0) + 1.0) * 32767.5) as u16
                                        })
                                    },
                                    err,
                                    None,
                                ),
                                cpal::SampleFormat::I32 => dev.build_output_stream::<i32, _, _>(
                                    cfg,
                                    move |d: &mut [i32], _| {
                                        cb.process(d, ch, |v| {
                                            (v.clamp(-1.0, 1.0) as f64 * 2147483647.0) as i32
                                        })
                                    },
                                    err,
                                    None,
                                ),
                                _ => dev.build_output_stream::<f32, _, _>(
                                    cfg,
                                    move |d: &mut [f32], _| cb.process(d, ch, |v| v),
                                    err,
                                    None,
                                ),
                            };
                            match stream {
                                Ok(s) => {
                                    if let Err(e) = s.play() {
                                        let _ = ready_tx.send(Err(format!(
                                            "dead:could not start the output: {e}"
                                        )));
                                        return;
                                    }
                                    let _ = ready_tx.send(Ok((backend, name, sr, buf)));
                                    while !st.load(Relaxed) {
                                        std::thread::sleep(Duration::from_millis(50));
                                    }
                                    drop(s);
                                    return;
                                }
                                Err(e) => {
                                    let _ = ready_tx
                                        .send(Err(format!("dead:could not open the output: {e}")));
                                    return;
                                }
                            }
                        }
                        Err(e) => {
                            // fall through to the null device below, reporting why
                            let _ = ready_tx.send(Err(e));
                            run_null(Cb::new(r, rx, gtx, sh, proj_sr, Some(mrx)), proj_sr, &st);
                            return;
                        }
                    }
                }
                let _ = ready_tx.send(Ok((
                    "none".into(),
                    "No output (silent)".into(),
                    proj_sr,
                    480,
                )));
                let _ = &prefs2;
                run_null(Cb::new(r, rx, gtx, sh, proj_sr, Some(mrx)), proj_sr, &st);
            })
            .expect("audio thread");
        let (backend, device, sample_rate, buffer, error) =
            match ready_rx.recv_timeout(Duration::from_secs(8)) {
                Ok(Ok((b, d, s, bf))) => (b, d, s, bf, None),
                Ok(Err(e)) => match e.strip_prefix("dead:") {
                    Some(e) => (
                        "dead".into(),
                        String::new(),
                        proj_sr,
                        0,
                        Some(e.to_string()),
                    ),
                    None => (
                        "none".into(),
                        "No output (silent)".into(),
                        proj_sr,
                        480,
                        Some(e),
                    ),
                },
                Err(_) => (
                    "none".into(),
                    "No output (silent)".into(),
                    proj_sr,
                    480,
                    Some("the audio device did not respond".into()),
                ),
            };
        AudioEngine {
            shared,
            tx,
            garbage: grx,
            stop,
            thread: Some(thread),
            backend,
            device,
            sample_rate,
            buffer,
            error,
            mon_tx: Some(mtx),
            rec: None,
        }
    }

    fn push(&mut self, c: Cmd) {
        // the queue is large; if it is ever full, wait briefly rather than lose a message
        let mut c = c;
        for _ in 0..200 {
            match self.tx.push(c) {
                Ok(()) => break,
                Err(rtrb::PushError::Full(back)) => {
                    c = back;
                    std::thread::sleep(Duration::from_millis(2));
                }
            }
        }
        while self.garbage.pop().is_ok() {}
    }
    pub fn send(&mut self, m: Msg) {
        self.push(Cmd::Msg(m));
    }
    pub fn set_plan(&mut self, p: Plan) {
        self.push(Cmd::Plan(Box::new(p)));
    }
    pub fn set_monitor(&mut self, on: bool) {
        self.push(Cmd::Monitor(on));
    }

    pub fn meters(&self, ntracks: usize) -> Meters {
        let s = &self.shared;
        Meters {
            pos: s.pos.load(Relaxed),
            playing: s.playing.load(Relaxed),
            peak: [u2f(s.peak[0].load(Relaxed)), u2f(s.peak[1].load(Relaxed))],
            lufs_m: u2f(s.lufs_m.load(Relaxed)),
            lufs_s: u2f(s.lufs_s.load(Relaxed)),
            lufs_i: u2f(s.lufs_i.load(Relaxed)),
            glitches: s.glitches.load(Relaxed),
            load: u2f(s.load.load(Relaxed)),
            tracks: s
                .track_peak
                .iter()
                .take(ntracks.min(MAX_TRACK_METERS))
                .map(|a| u2f(a.load(Relaxed)))
                .collect(),
            input_peak: u2f(s.in_peak.load(Relaxed)),
        }
    }

    /// The last `n` output samples (stereo), oldest first.
    pub fn scope(&self, n: usize) -> (Vec<f32>, Vec<f32>) {
        let n = n.min(SCOPE);
        let w = self.shared.scope_w.load(Relaxed);
        let mut l = Vec::with_capacity(n);
        let mut r = Vec::with_capacity(n);
        for k in 0..n {
            let i = (w + SCOPE - n + k) % SCOPE;
            l.push(u2f(self.shared.scope_l[i].load(Relaxed)));
            r.push(u2f(self.shared.scope_r[i].load(Relaxed)));
        }
        (l, r)
    }

    pub fn start_recording(
        &mut self,
        path: &std::path::Path,
        prefs: &DevicePrefs,
    ) -> anyhow::Result<()> {
        if self.rec.is_some() {
            anyhow::bail!("already recording");
        }
        let mon = self.mon_tx.take();
        match Recorder::start(path, prefs, self.shared.clone(), mon) {
            Ok(r) => {
                self.rec = Some(r);
                Ok(())
            }
            Err((e, mon)) => {
                self.mon_tx = mon;
                Err(e)
            }
        }
    }
    pub fn recording(&self) -> Option<RecStatus> {
        self.rec.as_ref().map(|r| r.status())
    }
    pub fn pause_recording(&mut self, paused: bool) {
        if let Some(r) = &self.rec {
            r.paused.store(paused, Relaxed);
        }
    }
    pub fn stop_recording(&mut self) -> Option<RecResult> {
        let r = self.rec.take()?;
        let (res, mon) = r.stop();
        self.mon_tx = mon;
        Some(res)
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        if let Some(r) = self.rec.take() {
            let _ = r.stop();
        }
        self.stop.store(true, Relaxed);
        self.shared.alive.store(false, Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn run_null(mut cb: Cb, sr: u32, stop: &AtomicBool) {
    let n = (sr / 100) as usize;
    let mut buf = vec![0f32; n * 2];
    let mut next = Instant::now();
    while !stop.load(Relaxed) {
        cb.process(&mut buf, 2, |v| v);
        next += Duration::from_millis(10);
        let now = Instant::now();
        if next > now {
            std::thread::sleep(next - now);
        } else {
            next = now;
        }
    }
}

#[cfg(feature = "device")]
type Opened = (
    cpal::Device,
    String,
    String,
    cpal::StreamConfig,
    cpal::SampleFormat,
);

#[cfg(feature = "device")]
fn open_output(prefs: &DevicePrefs, want_sr: u32) -> Result<Opened, String> {
    use cpal::traits::{DeviceTrait, HostTrait};
    let host = cpal::default_host();
    let backend = format!("{:?}", host.id());
    let mut dev = None;
    if let Some(want) = &prefs.output {
        if let Ok(mut it) = host.output_devices() {
            dev = it.find(|d| d.id().map(|i| i.to_string()).ok().as_deref() == Some(want.as_str()));
        }
    }
    let dev = dev
        .or_else(|| host.default_output_device())
        .ok_or_else(|| "no audio output was found".to_string())?;
    let name = dev
        .description()
        .map(|d| d.name().to_string())
        .unwrap_or_else(|_| "Output".into());
    let def = dev
        .default_output_config()
        .map_err(|e| format!("the output has no usable format: {e}"))?;
    // prefer the project rate when the device supports it, so playback needs no resampling
    let mut cfg_sr = def.sample_rate();
    let mut fmt = def.sample_format();
    if let Ok(confs) = dev.supported_output_configs() {
        for c in confs {
            if c.min_sample_rate() <= want_sr
                && c.max_sample_rate() >= want_sr
                && c.channels() >= 2
                && c.sample_format() == cpal::SampleFormat::F32
            {
                cfg_sr = want_sr;
                fmt = cpal::SampleFormat::F32;
                break;
            }
        }
    }
    let channels = def.channels().max(1);
    let buffer_size = match prefs.buffer {
        Some(b) if b >= 32 => cpal::BufferSize::Fixed(b),
        _ => cpal::BufferSize::Default,
    };
    let cfg = cpal::StreamConfig {
        channels,
        sample_rate: cfg_sr,
        buffer_size,
    };
    Ok((dev, name, backend, cfg, fmt))
}

pub fn list_devices() -> DeviceList {
    #[cfg(feature = "device")]
    {
        use cpal::traits::{DeviceTrait, HostTrait};
        let host = cpal::default_host();
        let did = host
            .default_output_device()
            .and_then(|d| d.id().ok())
            .map(|i| i.to_string());
        let iid = host
            .default_input_device()
            .and_then(|d| d.id().ok())
            .map(|i| i.to_string());
        let info = |d: &cpal::Device, def: &Option<String>, out: bool| -> Option<DeviceInfo> {
            let id = d.id().ok()?.to_string();
            let name = d
                .description()
                .map(|x| x.name().to_string())
                .unwrap_or_else(|_| id.clone());
            let (ch, sr) = if out {
                d.default_output_config()
                    .map(|c| (c.channels(), c.sample_rate()))
                    .unwrap_or((2, 48000))
            } else {
                d.default_input_config()
                    .map(|c| (c.channels(), c.sample_rate()))
                    .unwrap_or((1, 48000))
            };
            Some(DeviceInfo {
                default: def.as_deref() == Some(id.as_str()),
                id,
                name,
                channels: ch,
                sample_rate: sr,
            })
        };
        DeviceList {
            host: format!("{:?}", host.id()),
            hosts: cpal::available_hosts()
                .iter()
                .map(|h| format!("{h:?}"))
                .collect(),
            outputs: host
                .output_devices()
                .map(|it| it.filter_map(|d| info(&d, &did, true)).collect())
                .unwrap_or_default(),
            inputs: host
                .input_devices()
                .map(|it| it.filter_map(|d| info(&d, &iid, false)).collect())
                .unwrap_or_default(),
        }
    }
    #[cfg(not(feature = "device"))]
    DeviceList::default()
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct RecStatus {
    pub seconds: f64,
    pub peak: f32,
    pub paused: bool,
    pub path: String,
    pub device: String,
    pub channels: u16,
    pub sample_rate: u32,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct RecResult {
    pub path: String,
    pub seconds: f64,
    pub frames: u64,
    pub sample_rate: u32,
    pub channels: u16,
    pub error: Option<String>,
}

/// Records from an input device straight to a 32-bit float WAV on disk, flushed every
/// half second, so a crash loses at most that much.
pub struct Recorder {
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    frames: Arc<AtomicU64>,
    writer: Option<std::thread::JoinHandle<Result<(), String>>>,
    stream: Option<std::thread::JoinHandle<Option<rtrb::Producer<f32>>>>,
    path: std::path::PathBuf,
    sr: u32,
    ch: u16,
    device: String,
    sh: Arc<Shared>,
}

type RecStart = Result<Recorder, (anyhow::Error, Option<rtrb::Producer<f32>>)>;

impl Recorder {
    #[allow(unused_variables, unused_mut)]
    fn start(
        path: &std::path::Path,
        prefs: &DevicePrefs,
        sh: Arc<Shared>,
        mut mon: Option<rtrb::Producer<f32>>,
    ) -> RecStart {
        #[cfg(feature = "device")]
        {
            use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
            let host = cpal::default_host();
            let mut dev = None;
            if let Some(want) = &prefs.input {
                if let Ok(mut it) = host.input_devices() {
                    dev = it.find(|d| {
                        d.id().map(|i| i.to_string()).ok().as_deref() == Some(want.as_str())
                    });
                }
            }
            let Some(dev) = dev.or_else(|| host.default_input_device()) else {
                return Err((anyhow::anyhow!("no microphone or input was found"), mon));
            };
            let cfg = match dev.default_input_config() {
                Ok(c) => c,
                Err(e) => {
                    return Err((anyhow::anyhow!("the input has no usable format: {e}"), mon))
                }
            };
            let sr = cfg.sample_rate();
            let ch = cfg.channels().max(1);
            let device = dev
                .description()
                .map(|d| d.name().to_string())
                .unwrap_or_else(|_| "Input".into());
            let (mut ptx, mut prx) = rtrb::RingBuffer::<f32>::new((sr as usize) * ch as usize * 4);
            let stop = Arc::new(AtomicBool::new(false));
            let paused = Arc::new(AtomicBool::new(false));
            let frames = Arc::new(AtomicU64::new(0));
            let spec = hound::WavSpec {
                channels: ch,
                sample_rate: sr,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            };
            let mut w = match hound::WavWriter::create(path, spec) {
                Ok(w) => w,
                Err(e) => {
                    return Err((
                        anyhow::anyhow!("could not create the recording file: {e}"),
                        mon,
                    ))
                }
            };
            let (st, fr) = (stop.clone(), frames.clone());
            let writer = std::thread::spawn(move || -> Result<(), String> {
                let mut last = Instant::now();
                loop {
                    let mut got = false;
                    while let Ok(v) = prx.pop() {
                        w.write_sample(v).map_err(|e| e.to_string())?;
                        got = true;
                    }
                    if last.elapsed() > Duration::from_millis(500) {
                        w.flush().map_err(|e| e.to_string())?;
                        last = Instant::now();
                    }
                    if st.load(Relaxed) && !got {
                        break;
                    }
                    if !got {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                }
                let _ = &fr;
                w.finalize().map_err(|e| e.to_string())
            });
            let (st2, pz, fr2, sh2) = (stop.clone(), paused.clone(), frames.clone(), sh.clone());
            let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<(), String>>();
            let stream = std::thread::spawn(move || {
                let chn = ch as usize;
                let mut monp = mon;
                let res = dev.build_input_stream::<f32, _, _>(
                    cpal::StreamConfig {
                        channels: ch,
                        sample_rate: sr,
                        buffer_size: cpal::BufferSize::Default,
                    },
                    move |d: &[f32], _| {
                        let mut pk = 0f32;
                        for v in d {
                            pk = pk.max(v.abs());
                        }
                        let old = u2f(sh2.in_peak.load(Relaxed));
                        sh2.in_peak.store(f2u(pk.max(old * 0.9)), Relaxed);
                        if let Some(m) = monp.as_mut() {
                            for f in d.chunks(chn) {
                                let a = f[0];
                                let b = *f.get(1).unwrap_or(&a);
                                let _ = m.push(a);
                                let _ = m.push(b);
                            }
                        }
                        if pz.load(Relaxed) {
                            return;
                        }
                        for v in d {
                            let _ = ptx.push(*v);
                        }
                        fr2.fetch_add((d.len() / chn) as u64, Relaxed);
                    },
                    |_e| {},
                    None,
                );
                match res {
                    Ok(s) => {
                        if let Err(e) = s.play() {
                            let _ = ready_tx.send(Err(e.to_string()));
                            return None;
                        }
                        let _ = ready_tx.send(Ok(()));
                        while !st2.load(Relaxed) {
                            std::thread::sleep(Duration::from_millis(20));
                        }
                        drop(s);
                        None
                    }
                    Err(e) => {
                        let _ = ready_tx.send(Err(e.to_string()));
                        None
                    }
                }
            });
            match ready_rx.recv_timeout(Duration::from_secs(8)) {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    stop.store(true, Relaxed);
                    return Err((anyhow::anyhow!("could not open the input: {e}"), None));
                }
                Err(_) => {
                    stop.store(true, Relaxed);
                    return Err((anyhow::anyhow!("the input did not respond"), None));
                }
            }
            Ok(Recorder {
                stop,
                paused,
                frames,
                writer: Some(writer),
                stream: Some(stream),
                path: path.to_path_buf(),
                sr,
                ch,
                device,
                sh,
            })
        }
        #[cfg(not(feature = "device"))]
        Err((anyhow::anyhow!("this build has no audio input"), mon))
    }

    fn status(&self) -> RecStatus {
        RecStatus {
            seconds: self.frames.load(Relaxed) as f64 / self.sr as f64,
            peak: u2f(self.sh.in_peak.load(Relaxed)),
            paused: self.paused.load(Relaxed),
            path: self.path.display().to_string(),
            device: self.device.clone(),
            channels: self.ch,
            sample_rate: self.sr,
        }
    }

    fn stop(mut self) -> (RecResult, Option<rtrb::Producer<f32>>) {
        self.stop.store(true, Relaxed);
        let mon = self.stream.take().and_then(|t| t.join().ok()).flatten();
        let err = self
            .writer
            .take()
            .and_then(|t| t.join().ok())
            .and_then(|r| r.err());
        let frames = self.frames.load(Relaxed);
        (
            RecResult {
                path: self.path.display().to_string(),
                seconds: frames as f64 / self.sr as f64,
                frames,
                sample_rate: self.sr,
                channels: self.ch,
                error: err,
            },
            mon,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::{Media, Pool};
    use crate::project::{MediaRef, Project};

    #[test]
    fn null_device_plays_and_meters() {
        let sr = 48000u32;
        let x: Vec<f32> = (0..sr as usize * 2)
            .map(|i| (crate::dsp::TAU * 440.0 * i as f32 / sr as f32).sin() * 0.5)
            .collect();
        let mut pool = Pool::new();
        pool.insert(
            "m".into(),
            Arc::new(Media::new("m", vec![x.clone(), x], sr, sr)),
        );
        let mut p = Project::new("t", sr);
        p.media.push(MediaRef {
            id: "m".into(),
            frames: sr as i64 * 2,
            ..Default::default()
        });
        let t = p.add_track("A");
        p.add_clip(t, "m", 0, 0, None).unwrap();
        let pool = Arc::new(pool);
        let mut e = AudioEngine::start(
            &|| Plan::build(&p, pool.clone()),
            &DevicePrefs {
                null: true,
                ..Default::default()
            },
        );
        e.send(Msg::Play(true));
        std::thread::sleep(Duration::from_millis(400));
        let m = e.meters(1);
        assert!(m.playing);
        assert!(m.pos > 9000, "pos {}", m.pos);
        assert!(m.peak[0] > 0.3 && m.peak[0] < 0.55, "peak {}", m.peak[0]);
        assert!(m.lufs_m > -20.0);
        // swapping the plan does not stop playback
        let mut p2 = p.clone();
        p2.tracks[0].gain_db = -12.0;
        let pool2 = Arc::new(Pool::new());
        e.set_plan(Plan::build(&p2, pool2));
        e.send(Msg::Play(false));
        std::thread::sleep(Duration::from_millis(100));
        assert!(!e.meters(1).playing);
        let (l, _) = e.scope(512);
        assert_eq!(l.len(), 512);
    }

    #[test]
    fn device_list_does_not_panic() {
        let d = list_devices();
        let _ = serde_json::to_string(&d).unwrap();
    }
}
