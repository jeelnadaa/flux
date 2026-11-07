/* Flux landing: scroll scenes, kinetic type, binaural demo. */
(function () {
  const $ = (s, r) => (r || document).querySelector(s),
    $$ = (s, r) => [...(r || document).querySelectorAll(s)];
  const calm = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const hasG = !!(window.gsap && window.ScrollTrigger);
  if (hasG) gsap.registerPlugin(ScrollTrigger);
  const clamp = (v, a = 0, b = 1) => Math.min(b, Math.max(a, v));
  const seg = (p, a, b) => clamp((p - a) / (b - a));
  const ease = (t) => t * t * (3 - 2 * t);
  const lerp = (a, b, t) => a + (b - a) * t;
  function fit(c) {
    const d = Math.min(devicePixelRatio || 1, 2),
      r = c.getBoundingClientRect();
    const w = Math.max(1, Math.round(r.width * d)),
      h = Math.max(1, Math.round(r.height * d));
    if (c.width !== w || c.height !== h) {
      c.width = w;
      c.height = h;
    }
    c.W = r.width;
    c.H = r.height;
    const x = c.getContext("2d");
    x.setTransform(d, 0, 0, d, 0, 0);
    return x;
  }

  /* -------- Split type -------- */
  $$("[data-split]").forEach((el) => {
    const t = el.textContent;
    el.textContent = "";
    [...t].forEach((ch) => {
      const s = document.createElement("span");
      s.className = "c";
      s.textContent = ch === " " ? " " : ch;
      el.appendChild(s);
    });
  });
  $$("[data-letters]").forEach((el) => {
    const t = el.textContent;
    el.textContent = "";
    [...t].forEach((ch, i) => {
      const s = document.createElement("span");
      s.className = "l";
      s.style.setProperty("--i", i);
      s.textContent = ch === " " ? " " : ch;
      el.appendChild(s);
    });
  });
  const heroLetters = $$(".hero .c"),
    markLetters = $$("#bigmark .c");
  let heroCx = [];
  function lockWidths() {
    [...heroLetters, ...markLetters].forEach((c) => {
      c.style.width = "";
    });
    [...heroLetters, ...markLetters].forEach((c) => {
      c.style.width = c.getBoundingClientRect().width + "px";
    });
    heroCx = heroLetters.map((c) => {
      const b = c.getBoundingClientRect();
      return b.left + b.width / 2;
    });
  }
  const intro = heroLetters.map(() => ({ v: calm || !hasG ? 1 : 0 }));

  /* -------- Sound: a small synth through an HRTF panner -------- */
  const A = {
    ctx: null,
    an: null,
    data: null,
    pan: null,
    master: null,
    on: true,
    live: false,
    skip: false,
  };
  try {
    if (localStorage.getItem("flux-sound") === "off") A.on = false;
  } catch (e) {}
  function startAudio() {
    const C = window.AudioContext || window.webkitAudioContext;
    if (!C) return;
    const ctx = (A.ctx = new C());
    A.master = ctx.createGain();
    A.master.gain.value = 0;
    A.an = ctx.createAnalyser();
    A.an.fftSize = 256;
    A.an.smoothingTimeConstant = 0.72;
    A.data = new Uint8Array(A.an.frequencyBinCount);
    A.pan = ctx.createPanner();
    A.pan.panningModel = "HRTF";
    A.pan.distanceModel = "inverse";
    A.pan.refDistance = 1;
    const ir = ctx.createBuffer(2, ctx.sampleRate * 2.4, ctx.sampleRate);
    for (let c = 0; c < 2; c++) {
      const d = ir.getChannelData(c);
      for (let i = 0; i < d.length; i++)
        d[i] = (Math.random() * 2 - 1) * Math.pow(1 - i / d.length, 3.2);
    }
    const rev = ctx.createConvolver();
    rev.buffer = ir;
    const wet = ctx.createGain();
    wet.gain.value = 0.22;
    const bus = ctx.createGain();
    bus.connect(A.pan);
    A.pan.connect(A.master);
    bus.connect(wet);
    wet.connect(rev);
    rev.connect(A.master);
    A.master.connect(A.an);
    A.an.connect(ctx.destination);
    const noise = ctx.createBuffer(1, ctx.sampleRate * 0.2, ctx.sampleRate);
    const nd = noise.getChannelData(0);
    for (let i = 0; i < nd.length; i++) nd[i] = Math.random() * 2 - 1;
    const chords = [
      [146.83, 220, 277.18, 349.23],
      [123.47, 185, 246.94, 293.66],
      [110, 164.81, 220, 277.18],
      [130.81, 196, 246.94, 329.63],
    ];
    const bpm = 84,
      beat = 60 / bpm;
    let next = ctx.currentTime + 0.08,
      step = 0;
    function note(f, t, d, g, type) {
      const o = ctx.createOscillator(),
        a = ctx.createGain(),
        lp = ctx.createBiquadFilter();
      o.type = type;
      o.frequency.value = f;
      lp.type = "lowpass";
      lp.frequency.setValueAtTime(2200, t);
      lp.frequency.exponentialRampToValueAtTime(500, t + d);
      a.gain.setValueAtTime(0, t);
      a.gain.linearRampToValueAtTime(g, t + 0.012);
      a.gain.exponentialRampToValueAtTime(0.0005, t + d);
      o.connect(lp);
      lp.connect(a);
      a.connect(bus);
      o.start(t);
      o.stop(t + d + 0.05);
    }
    function hat(t, g) {
      const s = ctx.createBufferSource(),
        a = ctx.createGain(),
        hp = ctx.createBiquadFilter();
      s.buffer = noise;
      hp.type = "highpass";
      hp.frequency.value = 6500;
      a.gain.setValueAtTime(g, t);
      a.gain.exponentialRampToValueAtTime(0.0005, t + 0.07);
      s.connect(hp);
      hp.connect(a);
      a.connect(bus);
      s.start(t);
      s.stop(t + 0.1);
    }
    function kick(t) {
      const o = ctx.createOscillator(),
        a = ctx.createGain();
      o.frequency.setValueAtTime(130, t);
      o.frequency.exponentialRampToValueAtTime(42, t + 0.12);
      a.gain.setValueAtTime(0.42, t);
      a.gain.exponentialRampToValueAtTime(0.0005, t + 0.3);
      o.connect(a);
      a.connect(A.master);
      o.start(t);
      o.stop(t + 0.32);
    }
    A.timer = setInterval(() => {
      while (next < ctx.currentTime + 0.25) {
        const bar = Math.floor(step / 16) % 4,
          s16 = step % 16,
          ch = chords[bar];
        if (s16 % 8 === 0) ch.forEach((f) => note(f, next, 2.6, 0.05, "sawtooth"));
        if ([0, 3, 6, 10, 12].includes(s16))
          note(ch[(s16 * 3) % 4] * 2, next, 0.55, 0.085, "triangle");
        if (s16 % 2 === 1) hat(next, 0.05);
        if (s16 === 0 || s16 === 7 || s16 === 10) kick(next);
        next += beat / 4;
        step++;
      }
    }, 60);
    ctx.onstatechange = paintSound;
  }
  /* Sound is on by default. Browsers keep it silent until the first click, tap or key press, so the hint stays up until then. */
  const sb = $("#snd"),
    sndt = $("#sndt"),
    sndh = $("#sndh");
  const gestures = ["pointerdown", "keydown", "touchend"];
  function fadeTo(v, d) {
    const g = A.master.gain,
      T = A.ctx.currentTime;
    g.cancelScheduledValues(T);
    g.setValueAtTime(g.value, T);
    g.linearRampToValueAtTime(v, T + d);
  }
  function paintSound() {
    A.live = A.on && !!A.ctx && A.ctx.state === "running";
    sb.classList.toggle("on", A.live);
    sb.setAttribute("aria-checked", A.on);
    sndt.textContent = A.on ? "Sound on" : "Sound off";
    sndh.hidden = !A.on || A.live;
    if (A.live) gestures.forEach((g) => removeEventListener(g, firstGesture, true));
  }
  function wake() {
    if (!A.on) return;
    if (!A.ctx) startAudio();
    if (!A.ctx) return;
    A.ctx.resume().then(() => {
      if (A.on) fadeTo(0.55, 1.2);
      paintSound();
    });
  }
  function firstGesture(e) {
    if (!A.on) return;
    if (sb.contains(e.target)) {
      A.skip = true;
      setTimeout(() => {
        A.skip = false;
      }, 1000);
    }
    wake();
  }
  gestures.forEach((g) => addEventListener(g, firstGesture, true));
  sb.addEventListener("click", () => {
    if (A.skip) {
      A.skip = false;
      return;
    }
    A.on = !A.on;
    try {
      localStorage.setItem("flux-sound", A.on ? "on" : "off");
    } catch (e) {}
    if (A.on) wake();
    else if (A.ctx) {
      fadeTo(0, 0.3);
      setTimeout(() => {
        if (!A.on && A.ctx.state === "running") A.ctx.suspend();
      }, 350);
    }
    paintSound();
  });
  document.addEventListener("visibilitychange", () => {
    if (!A.ctx) return;
    if (document.hidden) A.ctx.suspend();
    else if (A.on) A.ctx.resume();
  });
  wake();
  paintSound();
  function spectrum(i, n, t) {
    if (A.live) {
      const b = Math.floor(2 + (i / n) * 42);
      return Math.pow(A.data[b] / 255, 1.5);
    }
    return 0.5 + 0.5 * Math.sin(t * 1.5 + i * 0.85) * Math.sin(t * 0.62 + i * 0.33);
  }

  /* -------- Hero field: WebGL2 engraved flow lines -------- */
  const heroState = { s: 0, mx: 0.5, my: 0.5, tx: 0.5, ty: 0.5 };
  (function field() {
    const c = $("#field");
    if (!c) return;
    const gl = c.getContext("webgl2", { antialias: false, alpha: false });
    if (!gl) return;
    const vs = `#version 300 es\nin vec2 p;void main(){gl_Position=vec4(p,0.,1.);}`;
    const fs = `#version 300 es
precision highp float;uniform vec2 r;uniform float t,s;uniform vec2 m;out vec4 o;
float h(vec2 p){return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453);}
float n(vec2 p){vec2 i=floor(p),f=fract(p);f=f*f*(3.-2.*f);return mix(mix(h(i),h(i+vec2(1,0)),f.x),mix(h(i+vec2(0,1)),h(i+vec2(1,1)),f.x),f.y);}
float fbm(vec2 p){float a=.5,v=0.;for(int i=0;i<4;i++){v+=a*n(p);p=p*2.03+vec2(11.,7.);a*=.5;}return v;}
void main(){
 vec2 uv=gl_FragCoord.xy/r;float as=r.x/r.y;vec2 p=vec2(uv.x*as,uv.y);vec2 mp=vec2(m.x*as,m.y);
 float d=distance(p,mp);
 float w=fbm(vec2(p.x*1.25+t*.045,p.y*1.5-t*.03));
 float w2=fbm(vec2(p.x*.7-t*.02,p.y*.8+t*.025)+w);
 float y=p.y+(.34+.5*s)*(w2-.45)+.10*exp(-d*d*7.)*sin(d*22.-t*2.4);
 float N=mix(44.,16.,s);
 float q=y*N;float fw=fwidth(q);
 float l=abs(fract(q)-.5);
 float th=mix(.03,.46,pow(smoothstep(.28,.78,w2),2.2));
 float line=1.-smoothstep(th-fw,th+fw,l);
 vec3 ground=vec3(.086,.129,.878),ink=vec3(.965,.957,.925),hot=vec3(1.,.353,.11);
 float glow=exp(-d*d*5.);
 vec3 lc=mix(ink,hot,smoothstep(.15,.9,glow));
 float a=mix(.2,.62,w2)+glow*.42;
 a*=1.-.55*smoothstep(.34,.0,uv.y);
 vec3 col=mix(ground,lc,line*a);
 o=vec4(col,1.);}`;
    const sh = (ty, src) => {
      const x = gl.createShader(ty);
      gl.shaderSource(x, src);
      gl.compileShader(x);
      if (!gl.getShaderParameter(x, gl.COMPILE_STATUS)) console.warn(gl.getShaderInfoLog(x));
      return x;
    };
    const pr = gl.createProgram();
    gl.attachShader(pr, sh(gl.VERTEX_SHADER, vs));
    gl.attachShader(pr, sh(gl.FRAGMENT_SHADER, fs));
    gl.linkProgram(pr);
    gl.useProgram(pr);
    const b = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, b);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
    const loc = gl.getAttribLocation(pr, "p");
    gl.enableVertexAttribArray(loc);
    gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);
    const U = {
      r: gl.getUniformLocation(pr, "r"),
      t: gl.getUniformLocation(pr, "t"),
      s: gl.getUniformLocation(pr, "s"),
      m: gl.getUniformLocation(pr, "m"),
    };
    function size() {
      const d = Math.min(devicePixelRatio || 1, 1.5),
        R = c.getBoundingClientRect();
      c.width = R.width * d;
      c.height = R.height * d;
      gl.viewport(0, 0, c.width, c.height);
    }
    size();
    addEventListener("resize", size);
    heroState.draw = (t) => {
      gl.uniform2f(U.r, c.width, c.height);
      gl.uniform1f(U.t, t);
      gl.uniform1f(U.s, heroState.s);
      gl.uniform2f(U.m, heroState.mx, heroState.my);
      gl.drawArrays(gl.TRIANGLES, 0, 3);
    };
    heroState.draw(calm ? 12 : 0);
  })();
  const hero = $(".hero");
  addEventListener(
    "pointermove",
    (e) => {
      heroState.tx = e.clientX / innerWidth;
      heroState.ty = 1 - e.clientY / innerHeight;
    },
    { passive: true },
  );

  /* -------- Synthetic programme material -------- */
  function makeStems(N) {
    const v = [],
      d = [],
      b = [],
      o = [];
    for (let i = 0; i < N; i++) {
      const x = i / N,
        beat = (x * 48) % 1,
        bar = (x * 12) % 1;
      const phrase =
        Math.pow(Math.max(0, Math.sin(x * 34 + 1.2)), 0.6) *
        (Math.sin(x * 340) * 0.25 + 0.7) *
        (x > 0.12 ? 1 : 0.12);
      v.push(clamp(phrase * (0.65 + 0.35 * Math.sin(x * 900))));
      d.push(clamp(Math.exp(-beat * 6) * 0.95 + Math.exp(-((beat * 2) % 1) * 12) * 0.32));
      b.push(clamp((0.42 + 0.2 * Math.sin(x * 60)) * (1 - Math.exp(-beat * 4) * 0.6)));
      o.push(clamp(0.26 + 0.18 * Math.sin(x * 22) + 0.1 * Math.sin(x * 210) + 0.1 * bar));
    }
    const mix = v.map((_, i) => clamp(v[i] * 0.55 + d[i] * 0.5 + b[i] * 0.3 + o[i] * 0.25));
    return { v, d, b, o, mix };
  }

  /* -------- Scene plumbing -------- */
  const scenes = [],
    pins = [];
  function scene(el, len, draw, resting) {
    const s = { p: 0, t: 0, near: false, draw, el };
    scenes.push(s);
    if (calm || !hasG) {
      s.draw(resting == null ? 0.5 : resting, 0);
      addEventListener("resize", () => s.draw(resting == null ? 0.5 : resting, 0));
      return s;
    }
    pins.push({
      el,
      make() {
        ScrollTrigger.create({
          trigger: el,
          start: "top top",
          end: () => "+=" + innerHeight * len,
          pin: true,
          anticipatePin: 1,
          onUpdate: (x) => {
            s.t = x.progress;
          },
        });
        ScrollTrigger.create({
          trigger: el,
          start: "top bottom",
          end: () => "+=" + innerHeight * (len + 2),
          onToggle: (x) => {
            s.near = x.isActive;
          },
        });
      },
    });
    return s;
  }

  /* -------- Scene: waveform to master -------- */
  (function waveScene() {
    const c = $("#wavec");
    if (!c) return;
    const N = 340,
      S = makeStems(N);
    const off = document.createElement("canvas");
    off.width = N;
    off.height = 72;
    const ox = off.getContext("2d");
    const heat = (v) => {
      const r = lerp(22, 255, clamp(v * 1.55)),
        g = lerp(33, 235, Math.pow(v, 2.1)),
        bb = v < 0.34 ? lerp(224, 120, v / 0.34) : lerp(120, 60, clamp((v - 0.34) * 1.6));
      return `rgb(${r | 0},${g | 0},${bb | 0})`;
    };
    for (let x = 0; x < N; x++)
      for (let y = 0; y < 72; y++) {
        const f = 1 - y / 72,
          t = x / N;
        const low = S.b[x] * Math.exp(-f * 11),
          kick = S.d[x] * Math.exp(-f * 3.2) * 0.9,
          voc =
            S.v[x] *
            (Math.pow(Math.max(0, Math.sin(f * 44 + Math.sin(t * 70) * 2.4)), 6) *
              Math.exp(-f * 1.5)),
          air = S.o[x] * 0.22 * (1 - f * 0.4);
        ox.fillStyle = heat(
          clamp(low + kick + voc * 0.9 + air * (0.5 + 0.5 * Math.sin(f * 90 + x * 0.7))),
        );
        ox.fillRect(x, y, 1, 1);
      }
    const steps = $$("#swave .step"),
      rail = $$("#wrail span"),
      lufs = $("#lufs");
    const paper = "#F6F4EC",
      ember = "#FF5A1C",
      cob = "#6B76FF";
    const cuts = [0.2, 0.38, 0.55, 0.74];
    scene(
      $("#swave"),
      3.6,
      (p) => {
        const x = fit(c),
          W = c.W,
          H = c.H;
        x.clearRect(0, 0, W, H);
        const ml = W * 0.04,
          mw = W * 0.92,
          cy = H * 0.71,
          amp = Math.min(H * 0.21, 240),
          bw = mw / N;
        const a = seg(p, 0, 0.18),
          b = ease(seg(p, 0.2, 0.34)),
          bOut = ease(seg(p, 0.44, 0.52)),
          sp = ease(seg(p, 0.46, 0.6)),
          spOut = ease(seg(p, 0.7, 0.78)),
          m = ease(seg(p, 0.76, 0.9));
        const reveal = ease(a);
        x.globalAlpha = 1;
        /* spectrogram columns, grown out of the waveform */
        if (sp > 0 && spOut < 1) {
          for (let i = 0; i < N; i++) {
            const hgt = amp * S.mix[i];
            const t2 = lerp(cy - hgt, cy - amp * 1.25, sp),
              b2 = lerp(cy + hgt, cy + amp * 1.25, sp);
            x.globalAlpha = sp * (1 - spOut);
            x.drawImage(off, i, 0, 1, 72, ml + i * bw, t2, bw + 0.6, b2 - t2);
          }
          x.globalAlpha = 1;
          const sel = ease(seg(p, 0.58, 0.64)) * (1 - spOut);
          if (sel > 0) {
            const sx = ml + mw * 0.58,
              sw = mw * 0.13 * sel,
              sy = cy - amp * 0.42,
              sh = amp * 0.62;
            x.strokeStyle = paper;
            x.lineWidth = 1.5;
            x.setLineDash([5, 4]);
            x.strokeRect(sx, sy, sw, sh);
            x.setLineDash([]);
            x.fillStyle = paper;
            x.font = '500 12px "Geist Mono",monospace';
            x.globalAlpha = sel;
            x.fillText("Chair creak 2.1 to 4.8 kHz", sx, sy - 8);
            x.globalAlpha = 1;
          }
        }
        /* wave bars */
        const waveA = 1 - sp + m;
        if (waveA > 0.001) {
          for (let i = 0; i < N; i++) {
            const u = i / N,
              grow = ease(seg(reveal, u * 0.82, u * 0.82 + 0.18));
            if (grow <= 0) break;
            let k = 0;
            cuts.forEach((cu) => {
              if (u >= cu) k++;
            });
            const off2 = (k - 2) * 16 * b * (1 - bOut),
              yo = (k % 2 ? -1 : 1) * 16 * b * (1 - bOut);
            let hgt = amp * S.mix[i];
            if (m > 0) {
              const lim = amp * 0.62;
              const boosted = Math.min(lim, Math.pow(S.mix[i], 0.55) * amp * 0.72);
              hgt = lerp(hgt, boosted, m);
            }
            const segStart = k === 0 ? 0 : cuts[k - 1],
              segEnd = k === 4 ? 1 : cuts[k],
              fade = b * (1 - bOut) * Math.min(1, Math.min(u - segStart, segEnd - u) / 0.012);
            const scale = b * (1 - bOut) > 0 ? lerp(1, fade, b * (1 - bOut)) : 1;
            x.globalAlpha = clamp(waveA);
            x.fillStyle = m > 0 ? paper : paper;
            const yy = cy + yo,
              hh = Math.max(0.75, hgt * scale) * grow;
            x.fillRect(ml + i * bw + off2, yy - hh, Math.max(1, bw * 0.62), hh * 2);
          }
          x.globalAlpha = 1;
          if (a > 0 && a < 1) {
            x.globalAlpha = Math.min(seg(a, 0, 0.06), 1 - seg(a, 0.9, 1));
            x.fillStyle = ember;
            x.fillRect(ml + mw * Math.min(1, reveal / 0.82), cy - amp * 1.15, 2, amp * 2.3);
            x.globalAlpha = 1;
          }
          if (b > 0 && bOut < 1) {
            x.fillStyle = ember;
            x.font = '500 11px "Geist Mono",monospace';
            x.globalAlpha = b * (1 - bOut);
            cuts.forEach((cu, i) => {
              const k = i + 1,
                cxx = ml + mw * cu + (k - 2.5) * 16 * b;
              x.fillRect(cxx, cy - amp * 1.1, 1.5, amp * 2.2);
              x.fillText(String((cu * 512.4).toFixed(3)) + " s", cxx + 5, cy - amp * 1.1 + 10);
            });
            x.globalAlpha = 1;
          }
        }
        if (m > 0) {
          const ceil = cy - amp * 0.62;
          x.globalAlpha = m;
          x.fillStyle = ember;
          x.fillRect(ml, ceil, mw, 1.5);
          x.fillRect(ml, cy + amp * 0.62, mw, 1.5);
          x.font = '500 12px "Geist Mono",monospace';
          x.fillText("Ceiling −1.0 dBTP", ml, ceil - 8);
          x.globalAlpha = 1;
        }
        /* captions */
        const st = p < 0.2 ? 0 : p < 0.46 ? 1 : p < 0.74 ? 2 : 3;
        steps.forEach((el, i) => {
          const on = i === st,
            ty = on ? 0 : i < st ? -28 : 28;
          if (el._on !== on) {
            el._on = on;
            if (hasG)
              gsap.to(el, {
                opacity: on ? 1 : 0,
                y: ty,
                duration: 0.55,
                ease: "expo.out",
                overwrite: true,
              });
          }
        });
        rail.forEach((r, i) => r.classList.toggle("on", i === st));
        if (lufs) lufs.textContent = "−" + lerp(23.4, 14, m).toFixed(1);
      },
      0.19,
    );
    if (calm || !hasG) steps.forEach((s) => (s.style.opacity = 1));
  })();

  /* -------- Scene: 8D orbit, source passing behind and in front of the type -------- */
  (function orbitScene() {
    const bc = $("#back8"),
      fc = $("#front8");
    if (!bc) return;
    const pathsEl = $$("#paths span"),
      rAz = $("#rAz"),
      rEl = $("#rEl"),
      rDi = $("#rDi"),
      t8 = $("#t8"),
      tD = $("#tD");
    const ink = "#090A12",
      paper = "#F6F4EC";
    const P = [
      (a) => [Math.cos(a), 0, Math.sin(a)],
      (a) => {
        const r = 0.55 + 0.45 * Math.sin(a * 0.37);
        return [Math.cos(a) * r, Math.sin(a * 0.37) * 0.55, Math.sin(a) * r];
      },
      (a) => [Math.sin(a), Math.sin(a * 2) * 0.22, Math.sin(a * 2) * 0.62],
      (a) => [Math.sin(a * 0.5) * 0.18, Math.sin(a), Math.cos(a)],
      (a) => [
        Math.sin(a * 1.3) * 0.8 + Math.sin(a * 3.1) * 0.2,
        Math.sin(a * 1.9) * 0.55 + Math.cos(a * 4.3) * 0.12,
        Math.cos(a * 1.7) * 0.8 + Math.sin(a * 2.3) * 0.2,
      ],
    ];
    const at = (mode, a) => {
      const i = Math.floor(mode),
        f = ease(mode - i),
        A0 = P[i](a),
        B0 = P[Math.min(4, i + 1)](a);
      return [lerp(A0[0], B0[0], f), lerp(A0[1], B0[1], f), lerp(A0[2], B0[2], f)];
    };
    let lastA = 0;
    scene(
      $("#s8d"),
      4.4,
      (p, t) => {
        const bx = fit(bc),
          fx = fit(fc),
          W = bc.W,
          H = bc.H;
        bx.clearRect(0, 0, W, H);
        fx.clearRect(0, 0, W, H);
        const mode = clamp(p * 5.2 - 0.3, 0, 4),
          pitch = lerp(0.52, 1.02, Math.sin(p * Math.PI)),
          R = Math.min(W * 0.44, H * 0.46),
          cx = W / 2,
          cy = H / 2,
          per = 3.1;
        const a = p * 34 + (t || 0) * 0.55;
        lastA = a;
        const cp = Math.cos(pitch),
          sp = Math.sin(pitch);
        const proj = (X, Y, Z) => {
          const y2 = Y * cp - Z * sp,
            z2 = Y * sp + Z * cp,
            f = per / (per + z2);
          return [cx + X * f * R, cy - y2 * f * R, f, z2];
        };
        /* floor rings */
        [0.34, 0.67, 1].forEach((rr, ri) => {
          const pass = (ctx, front) => {
            ctx.beginPath();
            let st = false;
            for (let i = 0; i <= 96; i++) {
              const th = (i / 96) * Math.PI * 2,
                q = proj(Math.cos(th) * rr, 0, Math.sin(th) * rr);
              if (q[3] < 0 === front) {
                if (!st) {
                  ctx.moveTo(q[0], q[1]);
                  st = true;
                } else ctx.lineTo(q[0], q[1]);
              } else st = false;
            }
            ctx.strokeStyle = ink;
            ctx.lineWidth = ri === 2 ? 1.6 : 1;
            ctx.globalAlpha = ri === 2 ? 0.85 : 0.38;
            ctx.stroke();
            ctx.globalAlpha = 1;
          };
          pass(bx, false);
          pass(fx, true);
        });
        /* degree ticks */
        for (let d = 0; d < 360; d += 30) {
          const th = (d * Math.PI) / 180,
            q1 = proj(Math.sin(th), 0, -Math.cos(th)),
            q2 = proj(Math.sin(th) * 1.05, 0, -Math.cos(th) * 1.05);
          const ctx = q1[3] < 0 ? fx : bx;
          ctx.strokeStyle = ink;
          ctx.lineWidth = 1.4;
          ctx.beginPath();
          ctx.moveTo(q1[0], q1[1]);
          ctx.lineTo(q2[0], q2[1]);
          ctx.stroke();
        }
        /* trail */
        const TN = 90;
        for (let i = TN; i > 0; i--) {
          const pos = at(mode, a - i * 0.045),
            q = proj(pos[0], pos[1], pos[2]),
            ctx = q[3] < 0 ? fx : bx,
            al = 1 - i / TN;
          ctx.globalAlpha = al * al * 0.9;
          ctx.fillStyle = paper;
          ctx.beginPath();
          ctx.arc(q[0], q[1], Math.max(0.8, 13 * q[2] * al), 0, 7);
          ctx.fill();
        }
        bx.globalAlpha = 1;
        fx.globalAlpha = 1;
        /* head (top-down silhouette, always at centre) */
        const hq = proj(0, 0, 0),
          hr = R * 0.115;
        fx.fillStyle = paper;
        fx.strokeStyle = ink;
        fx.lineWidth = 2;
        fx.beginPath();
        fx.ellipse(hq[0], hq[1], hr, hr * (0.72 + 0.28 * cp), 0, 0, 7);
        fx.fill();
        fx.stroke();
        const nose = proj(0, 0, -0.17);
        fx.beginPath();
        fx.moveTo(nose[0], nose[1]);
        fx.lineTo(hq[0] - hr * 0.18, hq[1] + 2 - hr * 0.55 * cp);
        fx.lineTo(hq[0] + hr * 0.18, hq[1] + 2 - hr * 0.55 * cp);
        fx.closePath();
        fx.fillStyle = ink;
        fx.fill();
        fx.fillStyle = ink;
        fx.fillRect(hq[0] - hr - 5, hq[1] - hr * 0.2, 5, hr * 0.4);
        fx.fillRect(hq[0] + hr, hq[1] - hr * 0.2, 5, hr * 0.4);
        /* source */
        const pos = at(mode, a),
          q = proj(pos[0], pos[1], pos[2]),
          ctx = q[3] < 0 ? fx : bx,
          sr = Math.max(9, R * 0.075 * q[2]);
        ctx.fillStyle = paper;
        ctx.strokeStyle = ink;
        ctx.lineWidth = 2.5;
        ctx.beginPath();
        ctx.arc(q[0], q[1], sr, 0, 7);
        ctx.fill();
        ctx.stroke();
        ctx.fillStyle = ink;
        ctx.beginPath();
        ctx.arc(q[0], q[1], sr * 0.32, 0, 7);
        ctx.fill();
        /* type breathes with depth */
        const depth = pos[2];
        if (t8) {
          t8.style.transform = `translateX(${pos[0] * -2.2}%) scaleY(${1 + (pos[0] < 0 ? -pos[0] : 0) * 0.05})`;
          tD.style.transform = `translateX(${pos[0] * 2.2}%) scaleY(${1 + (pos[0] > 0 ? pos[0] : 0) * 0.05})`;
        }
        const az = (Math.atan2(pos[0], -pos[2]) * 180) / Math.PI,
          el = (Math.atan2(pos[1], Math.hypot(pos[0], pos[2])) * 180) / Math.PI,
          di = Math.hypot(pos[0], pos[1], pos[2]) * 1.8;
        rAz.textContent = (az >= 0 ? "+" : "−") + Math.abs(az).toFixed(0).padStart(3, "0") + "°";
        rEl.textContent = (el >= 0 ? "+" : "−") + Math.abs(el).toFixed(0).padStart(2, "0") + "°";
        rDi.textContent = di.toFixed(1) + " m";
        const ai = Math.round(mode);
        pathsEl.forEach((s, i) => s.classList.toggle("on", i === ai));
        if (A.on && A.pan) {
          const T = A.ctx.currentTime,
            sc = 2.2;
          if (A.pan.positionX) {
            A.pan.positionX.setTargetAtTime(pos[0] * sc, T, 0.02);
            A.pan.positionY.setTargetAtTime(pos[1] * sc, T, 0.02);
            A.pan.positionZ.setTargetAtTime(pos[2] * sc, T, 0.02);
          } else A.pan.setPosition(pos[0] * sc, pos[1] * sc, pos[2] * sc);
        }
      },
      0.08,
    );
  })();

  /* -------- Scene: edit audio by editing text -------- */
  (function textScene() {
    const para = $("#para"),
      c = $("#clipwave"),
      dur = $("#dur");
    if (!para) return;
    const words = [
      ["So, ", 1],
      ["um, ", 1],
      ["the ", 1],
      ["the ", 0],
      ["thing ", 0],
      ["I ", 0],
      ["keep ", 0],
      ["telling ", 0],
      ["people ", 0],
      ["is, ", 0],
      ["you know, ", 1],
      ["that ", 0],
      ["audio ", 0],
      ["should ", 0],
      ["be ", 0],
      ["as ", 0],
      ["easy ", 0],
      ["to ", 0],
      ["edit ", 0],
      ["as ", 0],
      ["a ", 0],
      ["document.", 0],
    ];
    const lens = [
      0.34, 0.46, 0.2, 0.2, 0.3, 0.1, 0.26, 0.36, 0.36, 0.3, 0.62, 0.22, 0.4, 0.32, 0.14, 0.14,
      0.32, 0.14, 0.3, 0.14, 0.1, 0.68,
    ];
    const els = words.map((w) => {
      const s = document.createElement("span");
      s.className = "w" + (w[1] ? " fill" : "");
      s.textContent = w[0];
      s.setAttribute("aria-hidden", "true");
      para.appendChild(s);
      return s;
    });
    let wid = [];
    const measure = () => {
      els.forEach((e) => (e.style.width = ""));
      wid = els.map((e) => e.getBoundingClientRect().width);
    };
    const filled = els.map((e, i) => (words[i][1] ? i : -1)).filter((i) => i >= 0);
    const S = makeStems(460),
      total = lens.reduce((a, b) => a + b, 0);
    scene(
      $("#stext"),
      2.4,
      (p) => {
        if (!wid.length) measure();
        const strike = seg(p, 0.08, 0.44),
          coll = ease(seg(p, 0.48, 0.8));
        filled.forEach((i, k) => {
          const s0 = seg(strike, k / filled.length, (k + 1) / filled.length);
          els[i].style.setProperty("--x", s0);
          els[i].style.width = wid[i] * (1 - coll) + "px";
          els[i].style.opacity = 1 - coll;
        });
        const x = fit(c),
          W = c.W,
          H = c.H;
        x.clearRect(0, 0, W, H);
        let px = 0;
        const scale = W / lerp(total, total - filled.reduce((a, i) => a + lens[i], 0), coll);
        let seen = 0;
        words.forEach((w, i) => {
          const span = lens[i] * (w[1] ? 1 - coll : 1) * scale,
            n = Math.max(1, Math.round((lens[i] / total) * 460));
          const bw = span / n,
            s0 = w[1]
              ? seg(
                  strike,
                  filled.indexOf(i) / filled.length,
                  (filled.indexOf(i) + 1) / filled.length,
                )
              : 0;
          for (let j = 0; j < n; j++) {
            const v = S.v[(seen + j) % 460] * 0.7 + 0.08;
            const hh = v * H * 0.46;
            x.fillStyle = s0 > j / n ? "#FF5A1C" : "#090A12";
            x.fillRect(px + j * bw, H / 2 - hh, Math.max(0.8, bw * 0.62), hh * 2);
          }
          seen += n;
          px += span;
        });
        if (dur) {
          const v = lerp(8.4, 6.1, coll);
          dur.textContent = "00:0" + v.toFixed(2);
        }
      },
      0,
    );
    addEventListener("resize", () => {
      wid = [];
    });
  })();

  /* -------- Scene: stems, exploded -------- */
  (function stemScene() {
    const host = $("#plates");
    if (!host) return;
    const S = makeStems(260);
    const defs = [
      ["Other", S.o, "#7B8CFF"],
      ["Bass", S.b, "#FF5A1C"],
      ["Drums", S.d, "#F6F4EC"],
      ["Vocals", S.v, "#FFC533"],
    ];
    const pl = defs.map((d) => {
      const e = document.createElement("div");
      e.className = "plate";
      e.style.color = d[2];
      e.innerHTML = "<canvas></canvas><span>" + d[0] + "</span>";
      host.appendChild(e);
      return { e, c: e.firstChild, s: d[1], col: d[2], lab: e.lastChild };
    });
    function paint() {
      pl.forEach((o) => {
        const x = fit(o.c),
          W = o.c.W,
          H = o.c.H;
        x.clearRect(0, 0, W, H);
        x.fillStyle = "#25103DCC";
        x.fillRect(0, 0, W, H);
        x.fillStyle = o.col;
        const bw = W / 260;
        for (let i = 0; i < 260; i++) {
          const h = o.s[i] * H * 0.4;
          x.fillRect(i * bw, H / 2 - h, Math.max(1, bw * 0.6), h * 2);
        }
      });
    }
    let painted = false;
    addEventListener("resize", () => {
      painted = false;
    });
    scene(
      $("#sstem"),
      2.6,
      (p) => {
        if (!painted) {
          paint();
          painted = true;
        }
        const e = ease(seg(p, 0, 0.42)),
          lift = ease(seg(p, 0.6, 0.9));
        host.style.transform = `translateY(${lerp(8, 12, e)}%) rotateX(${58 * e}deg) rotateZ(${-34 * e}deg)`;
        const gap = Math.min(innerWidth * 0.15, innerWidth >= 1000 ? 112 : 150);
        pl.forEach((o, i) => {
          let z = (i - 1.5) * gap * e,
            lx = 0,
            op = 1;
          if (i === 3) {
            z += (innerWidth >= 1000 ? 150 : 240) * lift;
            lx = 40 * lift;
            op = 1 - lift * 0.25;
          }
          o.e.style.transform = `translate3d(${lx}px,0,${z}px)`;
          o.e.style.opacity = op;
          o.lab.style.opacity = e;
        });
      },
      0.5,
    );
  })();

  /* -------- Scene: simple to pro -------- */
  (function proScene() {
    const bench = $("#bench");
    if (!bench) return;
    const ctl = [
      ["Low-pass", "6.4 kHz", 0.62],
      ["High-pass", "90 Hz", 0.18],
      ["Drive", "+4.0 dB", 0.44],
      ["Bit depth", "12 bit", 0.55],
      ["Sample rate", "22.0 kHz", 0.4],
      ["Crackle", "4.0 /s", 0.46],
      ["Surface", "−48 dB", 0.36],
      ["Wow", "0.6 Hz", 0.32],
      ["Flutter", "7.5 Hz", 0.24],
      ["Pump", "35%", 0.35],
      ["Room", "1.1 s", 0.3],
      ["Noise bed", "−54 dB", 0.22],
    ];
    const els = ctl.map((c) => {
      const d = document.createElement("div");
      d.className = "ctl";
      d.innerHTML = `<label>${c[0]}</label><div class="m"><i style="transform:scaleX(${c[2]})"></i></div><span class="v">${c[1]}</span>`;
      bench.appendChild(d);
      return d;
    });
    const ms = $("#ms"),
      msi = $("#msi"),
      msv = $("#msv");
    scene(
      $("#spro"),
      2.1,
      (p) => {
        const e = ease(seg(p, 0.18, 0.72)),
          v = lerp(38, 64, ease(seg(p, 0, 0.18)));
        msi.style.width = v + "%";
        msv.textContent = Math.round(v) + "%";
        const cols = innerWidth < 760 ? 3 : 6;
        els.forEach((d, i) => {
          const col = i % cols,
            row = Math.floor(i / cols),
            s = seg(e, (i / els.length) * 0.55, (i / els.length) * 0.55 + 0.45);
          d.style.opacity = s;
          d.style.transform = `translate(${(cols / 2 - col - 0.5) * (1 - s) * 40}%,${-(row + 1) * 90 * (1 - s)}%) scale(${lerp(0.3, 1, s)})`;
        });
      },
      1,
    );
  })();

  /* -------- Effect wall and page flow -------- */
  let lenis;
  if (!calm && hasG) {
    if (window.Lenis) {
      lenis = new Lenis({ lerp: 0.09, smoothWheel: true, wheelMultiplier: 0.95 });
      lenis.on("scroll", ScrollTrigger.update);
      gsap.ticker.add((t) => lenis.raf(t * 1000));
      gsap.ticker.lagSmoothing(0);
    }
    const track = $("#track"),
      sfx = $("#sfx");
    const dist = () => Math.max(0, track.scrollWidth - innerWidth);
    pins.push({
      el: sfx,
      make() {
        const wall = gsap.to(track, {
          x: () => -dist(),
          ease: "none",
          scrollTrigger: {
            trigger: sfx,
            start: "top top",
            end: () => "+=" + dist(),
            pin: true,
            scrub: true,
            anticipatePin: 1,
            invalidateOnRefresh: true,
          },
        });
        $$(".fx .word", track).forEach((w) => {
          if (!w.closest(".f-rev,.f-stop,.f-slow,.f-tape,.f-night,.f-bot"))
            gsap.fromTo(
              w,
              { xPercent: 10 },
              {
                xPercent: -10,
                ease: "none",
                scrollTrigger: {
                  trigger: w.closest(".fx"),
                  containerAnimation: wall,
                  start: "left right",
                  end: "right left",
                  scrub: true,
                },
              },
            );
        });
      },
    });
    /* Pins must be created top to bottom, or ScrollTrigger measures the later ones without the earlier spacers and sections land on top of each other. */
    pins
      .sort((a, b) =>
        a.el.compareDocumentPosition(b.el) & Node.DOCUMENT_POSITION_FOLLOWING ? -1 : 1,
      )
      .forEach((q) => q.make());

    /* ground colour: the page itself is in flux */
    const secs = $$("[data-g]");
    const ground = (s) =>
      gsap.to(document.body, {
        "--ground": s.dataset.g,
        "--figure": s.dataset.f,
        duration: 0.7,
        ease: "power2.out",
        overwrite: "auto",
      });
    secs.forEach((s, i) =>
      ScrollTrigger.create({
        trigger: s,
        start: "top 55%",
        end: "max",
        onEnter: () => ground(s),
        onLeaveBack: () => ground(secs[Math.max(0, i - 1)]),
      }),
    );

    /* hero exit */
    gsap.to(heroState, {
      s: 1,
      ease: "none",
      scrollTrigger: { trigger: hero, start: "top top", end: "bottom top", scrub: true },
    });
    gsap.to(".hero h1", {
      yPercent: -28,
      ease: "none",
      scrollTrigger: { trigger: hero, start: "top top", end: "bottom top", scrub: true },
    });
    gsap.to(".herofoot", {
      y: -70,
      opacity: 0,
      ease: "none",
      scrollTrigger: { trigger: hero, start: "top top", end: "55% top", scrub: true },
    });

    /* hero entrance */
    gsap.to(intro, { v: 1, duration: 1.3, ease: "expo.out", stagger: 0.035, delay: 0.1 });
    gsap.from("#seam", { scaleX: 0, duration: 1.4, ease: "expo.inOut" });
    gsap.from(".herofoot>*", {
      y: 34,
      opacity: 0,
      duration: 1,
      ease: "expo.out",
      stagger: 0.08,
      delay: 0.55,
    });

    /* marquee by scroll with velocity skew */
    const mq = $("#mq1");
    let skewTo = gsap.quickTo(mq, "skewX", { duration: 0.5, ease: "power3" });
    gsap.fromTo(
      mq,
      { xPercent: 0 },
      {
        xPercent: -46,
        ease: "none",
        scrollTrigger: {
          trigger: ".mq",
          start: "top bottom",
          end: "bottom top",
          scrub: true,
          onUpdate: (s) => skewTo(clamp(s.getVelocity() / -320, -14, 14)),
        },
      },
    );

    /* numbers and rows */
    gsap.from("#zero", {
      xPercent: -6,
      scaleY: 0.4,
      transformOrigin: "left bottom",
      ease: "none",
      scrollTrigger: { trigger: "#spec", start: "top 85%", end: "top 25%", scrub: true },
    });
    $$(".spec").forEach((r, i) =>
      gsap.from(r, {
        y: 40,
        opacity: 0.15,
        duration: 0.9,
        ease: "expo.out",
        scrollTrigger: { trigger: r, start: "top 92%", toggleActions: "play none none reverse" },
      }),
    );
    $$(".plat").forEach((r, i) =>
      gsap.from(r, {
        y: 60,
        opacity: 0.15,
        duration: 1,
        delay: i * 0.06,
        ease: "expo.out",
        scrollTrigger: { trigger: r, start: "top 92%", toggleActions: "play none none reverse" },
      }),
    );

    /* magnetic buttons */
    if (matchMedia("(hover:hover)").matches)
      $$(".btn").forEach((b) => {
        const mx = gsap.quickTo(b, "x", { duration: 0.6, ease: "elastic.out(1,.4)" }),
          my = gsap.quickTo(b, "y", { duration: 0.6, ease: "elastic.out(1,.4)" });
        b.addEventListener("pointermove", (e) => {
          const r = b.getBoundingClientRect();
          mx((e.clientX - r.left - r.width / 2) * 0.22);
          my((e.clientY - r.top - r.height / 2) * 0.32);
        });
        b.addEventListener("pointerleave", () => {
          mx(0);
          my(0);
        });
      });

    /* in-page anchor */
    $(".wm").addEventListener("click", (e) => {
      e.preventDefault();
      lenis ? lenis.scrollTo(0, { duration: 1.6 }) : scrollTo(0, 0);
    });
  }

  /* -------- The render loop -------- */
  let heroVisible = true,
    markVisible = false;
  if ("IntersectionObserver" in window) {
    new IntersectionObserver((e) => {
      heroVisible = e[0].isIntersecting;
    }).observe(hero);
    new IntersectionObserver((e) => {
      markVisible = e[0].isIntersecting;
    }).observe($("#bigmark"));
  }
  const navL = $("#navl"),
    bigL = $("#bigl"),
    seam = $("#seam");
  const up = $$(".ln.up .c"),
    dn = $$(".ln.dn .c");
  let lastY = 0,
    vel = 0;
  function frame(tms) {
    const t = tms / 1000;
    if (A.live) A.an.getByteFrequencyData(A.data);
    const y = scrollY;
    vel += (y - lastY - vel) * 0.12;
    lastY = y;
    heroState.mx += (heroState.tx - heroState.mx) * 0.06;
    heroState.my += (heroState.ty - heroState.my) * 0.06;
    if (heroVisible) {
      if (heroState.draw) heroState.draw(t);
      const r = heroLetters.length,
        mxp = heroState.mx * innerWidth;
      heroLetters.forEach((c, i) => {
        const dx = Math.abs(heroCx[i] - mxp) / innerWidth,
          near = Math.exp(-dx * dx * 44);
        const v = clamp(spectrum(i, r, t) * 0.62 + near * 0.62);
        c.style.fontWeight = Math.round(lerp(480, 800, v));
        c.style.transform = `scaleY(${(lerp(0.92, 1.42, v) * intro[i].v).toFixed(3)})`;
      });
    }
    if (markVisible) {
      markLetters.forEach((c, i) => {
        const v = spectrum(i * 3, 9, t * 1.2);
        c.style.fontWeight = Math.round(lerp(560, 800, v));
        c.style.transform = `scaleY(${lerp(0.86, 1.08, v).toFixed(3)})`;
      });
      if (bigL)
        bigL.style.transform = `scaleY(${lerp(0.88, 1.12, spectrum(4, 9, t * 2.1)).toFixed(3)})`;
    }
    if (navL)
      navL.style.transform = `scaleY(${lerp(0.88, 1.12, A.live ? spectrum(6, 20, t) : clamp(0.75 + Math.abs(vel) / 40)).toFixed(3)})`;
    if (!calm)
      scenes.forEach((s) => {
        if (s.near) {
          s.p += (s.t - s.p) * 0.13;
          s.draw(s.p, t);
        }
      });
    if (!calm) requestAnimationFrame(frame);
  }
  const boot = () => {
    lockWidths();
    if (hasG) ScrollTrigger.refresh();
    if (!calm) requestAnimationFrame(frame);
    else frame(12000);
  };
  (document.fonts
    ? Promise.race([
        Promise.all([
          document.fonts.load("800 100px 'Bricolage Grotesque'"),
          document.fonts.load("400 13px 'Geist Mono'"),
        ]),
        new Promise((r) => setTimeout(r, 1800)),
      ]).catch(() => {})
    : Promise.resolve()
  ).then(boot);
  addEventListener("resize", () => {
    lockWidths();
  });
})();
