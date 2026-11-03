/* Flux Download Page: WebGL2 flow field, interactive audio synth, kinetic type, ground shifts. */
(function () {
  const $ = (s, r) => (r || document).querySelector(s),
    $$ = (s, r) => [...(r || document).querySelectorAll(s)];
  const calm = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const hasG = !!(window.gsap && window.ScrollTrigger);
  if (hasG) gsap.registerPlugin(ScrollTrigger);

  const clamp = (v, a = 0, b = 1) => Math.min(b, Math.max(a, v));

  /* -------- Kinetic Type Physics -------- */
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

  /* -------- Web Audio Synthesizer -------- */
  const A = {
    ctx: null,
    an: null,
    data: null,
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

    const bus = ctx.createGain();
    bus.connect(A.master);
    A.master.connect(A.an);
    A.an.connect(ctx.destination);

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

    A.timer = setInterval(() => {
      while (next < ctx.currentTime + 0.25) {
        const bar = Math.floor(step / 16) % 4,
          s16 = step % 16,
          ch = chords[bar];
        if (s16 % 8 === 0) ch.forEach((f) => note(f, next, 2.6, 0.04, "sawtooth"));
        if ([0, 4, 8, 12].includes(s16)) note(ch[(s16 * 3) % 4] * 2, next, 0.55, 0.06, "triangle");
        next += beat / 4;
        step++;
      }
    }, 60);

    ctx.onstatechange = paintSound;
  }

  function playClick(f = 880, d = 0.04) {
    if (!A.on || !A.ctx || A.ctx.state !== "running") return;
    try {
      const o = A.ctx.createOscillator(),
        g = A.ctx.createGain();
      o.frequency.setValueAtTime(f, A.ctx.currentTime);
      g.gain.setValueAtTime(0.08, A.ctx.currentTime);
      g.gain.exponentialRampToValueAtTime(0.001, A.ctx.currentTime + d);
      o.connect(g);
      g.connect(A.master);
      o.start();
      o.stop(A.ctx.currentTime + d);
    } catch {
      // AudioContext may be suspended or blocked
    }
  }

  const sb = $("#snd"),
    sndl = $("#sndl");
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
    if (sb) {
      sb.classList.toggle("on", A.live);
      sb.setAttribute("aria-checked", A.on);
    }
    if (sndl) {
      sndl.textContent = A.on ? "Sound on" : "Sound off";
    }
    if (A.live) gestures.forEach((g) => removeEventListener(g, firstGesture, true));
  }

  function wake() {
    if (!A.on) return;
    if (!A.ctx) startAudio();
    if (!A.ctx) return;
    A.ctx.resume().then(() => {
      if (A.on) fadeTo(0.45, 1.2);
      paintSound();
    });
  }

  function firstGesture(e) {
    if (!A.on) return;
    if (sb && sb.contains(e.target)) {
      A.skip = true;
      setTimeout(() => {
        A.skip = false;
      }, 1000);
    }
    wake();
  }

  gestures.forEach((g) => addEventListener(g, firstGesture, true));

  if (sb) {
    sb.addEventListener("click", () => {
      if (A.skip) {
        A.skip = false;
        return;
      }
      A.on = !A.on;
      try {
        localStorage.setItem("flux-sound", A.on ? "on" : "off");
      } catch {
        // AudioContext may be suspended or blocked
      }
      if (A.on) wake();
      else if (A.ctx) {
        fadeTo(0, 0.3);
        setTimeout(() => {
          if (!A.on && A.ctx.state === "running") A.ctx.suspend();
        }, 350);
      }
      paintSound();
    });
  }

  document.addEventListener("visibilitychange", () => {
    if (!A.ctx) return;
    if (document.hidden) A.ctx.suspend();
    else if (A.on) A.ctx.resume();
  });

  wake();
  paintSound();

  /* -------- WebGL2 Flow Lines Canvas -------- */
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
 vec3 ground=vec3(.035,.039,.071),ink=vec3(.965,.957,.925),hot=vec3(1.,.353,.11);
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

    let start = performance.now();
    function loop(now) {
      const t = (now - start) * 0.001;
      heroState.mx += (heroState.tx - heroState.mx) * 0.05;
      heroState.my += (heroState.ty - heroState.my) * 0.05;
      gl.uniform2f(U.r, c.width, c.height);
      gl.uniform1f(U.t, t);
      gl.uniform1f(U.s, heroState.s);
      gl.uniform2f(U.m, heroState.mx, heroState.my);
      gl.drawArrays(gl.TRIANGLES, 0, 3);
      if (!calm) requestAnimationFrame(loop);
    }
    if (!calm) requestAnimationFrame(loop);
    else {
      gl.uniform2f(U.r, c.width, c.height);
      gl.uniform1f(U.t, 12);
      gl.uniform1f(U.s, 0);
      gl.uniform2f(U.m, 0.5, 0.5);
      gl.drawArrays(gl.TRIANGLES, 0, 3);
    }
  })();

  addEventListener(
    "pointermove",
    (e) => {
      heroState.tx = e.clientX / innerWidth;
      heroState.ty = 1 - e.clientY / innerHeight;

      // Kinetic letter proximity
      if (heroCx.length && !calm) {
        heroLetters.forEach((c, i) => {
          const dx = e.clientX - heroCx[i];
          const dist = Math.abs(dx);
          if (dist < 180) {
            const pull = (1 - dist / 180) * 8;
            c.style.transform = `translateY(${dx > 0 ? -pull : pull}px) rotate(${dx * 0.04}deg)`;
          } else {
            c.style.transform = "";
          }
        });
      }
    },
    { passive: true },
  );

  /* -------- Lenis Smooth Scroll & Ground Color Shift -------- */
  if (hasG) {
    if (window.Lenis && !calm) {
      const lenis = new Lenis({ lerp: 0.09, smoothWheel: true, wheelMultiplier: 0.95 });
      lenis.on("scroll", ScrollTrigger.update);
      gsap.ticker.add((t) => lenis.raf(t * 1000));
      gsap.ticker.lagSmoothing(0);
    }

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

    // Hero entrance animations
    gsap.from(".hero .c", {
      y: 60,
      opacity: 0,
      stagger: 0.03,
      duration: 1,
      ease: "power4.out",
    });
    gsap.from("#seam", { scaleX: 0, duration: 1.2, ease: "expo.inOut" });
    gsap.from(".herofoot > *", {
      y: 30,
      opacity: 0,
      duration: 0.9,
      ease: "expo.out",
      stagger: 0.1,
      delay: 0.3,
    });
  }

  /* -------- Copy to Clipboard Action -------- */
  $$(".btn-copy").forEach((btn) => {
    btn.addEventListener("click", () => {
      const text = btn.dataset.copy;
      if (!text) return;
      navigator.clipboard.writeText(text).then(() => {
        playClick(1200, 0.08);
        const original = btn.textContent;
        btn.textContent = "COPIED!";
        btn.style.background = "rgba(121, 255, 225, 0.25)";
        btn.style.borderColor = "var(--mint)";
        setTimeout(() => {
          btn.textContent = original;
          btn.style.background = "";
          btn.style.borderColor = "";
        }, 1800);
      });
    });
  });

  // Sound feedback on button hovers
  $$(".btn").forEach((b) => {
    b.addEventListener("mouseenter", () => playClick(640, 0.02));
  });

  addEventListener("load", lockWidths);
  addEventListener("resize", lockWidths);
  lockWidths();
})();
