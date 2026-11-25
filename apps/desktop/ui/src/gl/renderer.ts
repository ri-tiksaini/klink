import type { ClusterInfo } from '../state/engine';

const MASK_W = 256;
const MASK_H = 8;

const NODE_VS = `#version 300 es
precision highp float;
precision highp int;
uniform vec2 u_res;
uniform vec2 u_cam;
uniform float u_zoom;
uniform float u_time;
uniform float u_motion;
uniform float u_hasSel;
uniform int u_hoverId;
uniform int u_selId;
uniform int u_langMask;
uniform highp usampler2D u_mask;
in vec2 a_quad;
in vec2 a_pos;
in float a_rad;
in vec4 a_rgba;
in float a_phase;
in float a_amp;
in uint a_meta;
out vec2 v_uv;
out vec3 v_col;
out float v_alpha;
out float v_glow;
out float v_ring;

bool isImpacted(uint id) {
  uint wi = id >> 5u;
  uint bi = id & 31u;
  ivec2 tc = ivec2(int(wi % 256u), int(wi / 256u));
  return ((texelFetch(u_mask, tc, 0).r >> bi) & 1u) != 0u;
}

void main() {
  uint id = a_meta & 0xFFFFFFu;
  uint lang = (a_meta >> 24u) & 0xFu;
  bool inCycle = ((a_meta >> 28u) & 1u) != 0u;
  if (((u_langMask >> int(lang)) & 1) == 0) {
    gl_Position = vec4(2.0, 2.0, 0.0, 1.0);
    v_uv = vec2(0.0); v_col = vec3(0.0); v_alpha = 0.0; v_glow = 0.0; v_ring = 0.0;
    return;
  }
  bool sel = (int(id) == u_selId);
  bool hov = (int(id) == u_hoverId);
  bool imp = (u_hasSel > 0.5) && isImpacted(id);
  float t = u_time * 0.28;
  vec2 drift = vec2(sin(t + a_phase), cos(t * 0.83 + a_phase)) * a_amp * u_motion * ((sel || hov || imp) ? 0.15 : 1.0);
  float pad = (sel || hov || imp) ? 2.1 : 1.25;
  float screenR = clamp(a_rad * pow(max(u_zoom, 0.01), 0.52), 2.4, 16.0);
  float r = (screenR / max(u_zoom, 0.0005)) * (sel ? 1.55 : (hov ? 1.35 : (imp ? 1.22 : 1.0))) * pad;
  vec2 ndc = (((a_pos + drift + a_quad * r) - u_cam) * u_zoom) / (u_res * 0.5);
  ndc.y = -ndc.y;
  gl_Position = vec4(ndc, 0.0, 1.0);
  v_uv = a_quad * pad;
  v_col = sel ? vec3(0.30, 0.76, 1.0) : (imp ? vec3(1.0, 0.71, 0.33) : (hov ? mix(a_rgba.rgb, vec3(1.0), 0.35) : a_rgba.rgb));
  v_alpha = (sel || hov || imp) ? 1.0 : ((u_hasSel > 0.5) ? 0.22 : 0.92);
  v_glow = sel ? 1.0 : (imp ? 0.78 : (hov ? 0.65 : 0.0));
  v_ring = inCycle ? 1.0 : 0.0;
}`;

const NODE_FS = `#version 300 es
precision highp float;
in vec2 v_uv;
in vec3 v_col;
in float v_alpha;
in float v_glow;
in float v_ring;
out vec4 fragColor;
void main() {
  float d = length(v_uv);
  float core = 1.0 - smoothstep(0.75, 1.0, d);
  float glow = v_glow * exp(-d * d * 1.45) * 0.65;
  float ring = v_ring * smoothstep(0.88, 0.98, d) * (1.0 - smoothstep(1.04, 1.16, d)) * 0.85;
  float a = clamp(core * v_alpha + glow + ring * v_alpha, 0.0, 1.0);
  if (a < 0.01) discard;
  fragColor = vec4(v_col * (core + glow * 1.25) + vec3(1.0, 0.42, 0.42) * ring, a);
}`;

const EDGE_VS = `#version 300 es
precision highp float;
precision highp int;
uniform vec2 u_res;
uniform vec2 u_cam;
uniform float u_zoom;
uniform float u_hasSel;
uniform int u_confMask;
uniform highp usampler2D u_mask;
in float a_t;
in vec2 a_src;
in vec2 a_dst;
in uint a_srcId;
in uint a_dstId;
in uint a_conf;
out vec4 v_col;
out float v_t;
flat out uint v_conf;

bool isImpacted(uint id) {
  uint wi = id >> 5u;
  uint bi = id & 31u;
  ivec2 tc = ivec2(int(wi % 256u), int(wi / 256u));
  return ((texelFetch(u_mask, tc, 0).r >> bi) & 1u) != 0u;
}

void main() {
  if (((u_confMask >> int(a_conf)) & 1) == 0) {
    gl_Position = vec4(2.0, 2.0, 0.0, 1.0);
    v_col = vec4(0.0); v_t = 0.0; v_conf = 0u;
    return;
  }
  vec2 world = mix(a_src, a_dst, a_t);
  vec2 ndc = ((world - u_cam) * u_zoom) / (u_res * 0.5);
  ndc.y = -ndc.y;
  gl_Position = vec4(ndc, 0.0, 1.0);
  bool hot = (u_hasSel > 0.5) && isImpacted(a_srcId) && isImpacted(a_dstId);
  vec3 base = (a_conf == 0u) ? vec3(0.32, 0.58, 0.80) : (a_conf == 1u ? vec3(0.95, 0.68, 0.30) : vec3(0.95, 0.38, 0.40));
  v_col = hot ? vec4(1.0, 0.71, 0.33, 0.85) : (u_hasSel > 0.5 ? vec4(base, 0.06) : vec4(base, 0.28));
  v_t = a_t;
  v_conf = a_conf;
}`;

const EDGE_FS = `#version 300 es
precision highp float;
precision highp int;
in vec4 v_col;
in float v_t;
flat in uint v_conf;
out vec4 fragColor;
void main() {
  if (v_col.a < 0.01) discard;
  if (v_conf == 1u && fract(v_t * 14.0) > 0.55) discard;
  if (v_conf == 2u && fract(v_t * 26.0) > 0.32) discard;
  fragColor = v_col;
}`;

const PICK_VS = `#version 300 es
precision highp float;
precision highp int;
uniform vec2 u_res;
uniform vec2 u_cam;
uniform float u_zoom;
uniform int u_langMask;
in vec2 a_quad;
in vec2 a_pos;
in float a_rad;
in uint a_meta;
flat out uint v_id;
out vec2 v_uv;
void main() {
  uint lang = (a_meta >> 24u) & 0xFu;
  if (((u_langMask >> int(lang)) & 1) == 0) {
    gl_Position = vec4(2.0, 2.0, 0.0, 1.0);
    v_id = 0u; v_uv = vec2(2.0); return;
  }
  float screenR = clamp(a_rad * pow(max(u_zoom, 0.01), 0.52), 4.0, 18.0);
  float r = (screenR / max(u_zoom, 0.0005)) * 1.35;
  vec2 ndc = (((a_pos + a_quad * r) - u_cam) * u_zoom) / (u_res * 0.5);
  ndc.y = -ndc.y;
  gl_Position = vec4(ndc, 0.0, 1.0);
  v_id = (a_meta & 0xFFFFFFu) + 1u;
  v_uv = a_quad;
}`;

const PICK_FS = `#version 300 es
precision highp float;
precision highp int;
flat in uint v_id;
in vec2 v_uv;
out vec4 fragColor;
void main() {
  if (length(v_uv) > 1.0) discard;
  fragColor = vec4(float(v_id & 255u)/255.0, float((v_id >> 8u) & 255u)/255.0, float((v_id >> 16u) & 255u)/255.0, 1.0);
}`;

export class KlinkRenderer {
  private gl: WebGL2RenderingContext;
  private ctx2d: CanvasRenderingContext2D;
  private nodeProg: WebGLProgram;
  private edgeProg: WebGLProgram;
  private pickProg: WebGLProgram;
  private nodeVao: WebGLVertexArrayObject;
  private edgeVao: WebGLVertexArrayObject;
  private pickVao: WebGLVertexArrayObject;
  private nodeBuf: WebGLBuffer;
  private edgeBuf: WebGLBuffer;
  private maskTex: WebGLTexture;
  private pickFbo: WebGLFramebuffer;
  private pickTex: WebGLTexture;

  private nodeCount = 0;
  private edgeCount = 0;
  private selectedId = -1;
  private hoverId = -1;
  private motion = 1.0;
  private showEdges = true;
  private showClusters = true;
  private langMask = 0x1FF;
  private confMask = 0x7;
  private clusters: ClusterInfo[] = [];

  private cam = { x: 0, y: 0, zoom: 0.75, tx: 0, ty: 0, tzoom: 0.75 };
  private pickDirty = true;
  private rafId = 0;
  private prevTs = performance.now();
  private fpsAccum = 0;
  private fpsFrames = 0;
  private lastHud = performance.now();

  constructor(
    private canvas: HTMLCanvasElement,
    private overlay: HTMLCanvasElement,
    private onSelect: (id: number) => void,
    private onHud: (fps: number, frameMs: number) => void
  ) {
    const gl = canvas.getContext('webgl2', { antialias: true, alpha: false, powerPreference: 'high-performance' });
    if (!gl) throw new Error('WebGL2 unavailable');
    this.gl = gl;
    this.ctx2d = overlay.getContext('2d')!;

    this.nodeProg = this.program(NODE_VS, NODE_FS);
    this.edgeProg = this.program(EDGE_VS, EDGE_FS);
    this.pickProg = this.program(PICK_VS, PICK_FS);

    const quad = gl.createBuffer()!;
    gl.bindBuffer(gl.ARRAY_BUFFER, quad);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]), gl.STATIC_DRAW);

    const lineT = gl.createBuffer()!;
    gl.bindBuffer(gl.ARRAY_BUFFER, lineT);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([0, 1]), gl.STATIC_DRAW);

    this.nodeBuf = gl.createBuffer()!;
    this.edgeBuf = gl.createBuffer()!;

    // Node VAO (32B stride)
    this.nodeVao = gl.createVertexArray()!;
    gl.bindVertexArray(this.nodeVao);
    gl.bindBuffer(gl.ARRAY_BUFFER, quad);
    const lq = gl.getAttribLocation(this.nodeProg, 'a_quad');
    gl.enableVertexAttribArray(lq);
    gl.vertexAttribPointer(lq, 2, gl.FLOAT, false, 0, 0);

    gl.bindBuffer(gl.ARRAY_BUFFER, this.nodeBuf);
    const fAttr = (n: string, sz: number, off: number, norm = false, typ = gl.FLOAT) => {
      const l = gl.getAttribLocation(this.nodeProg, n);
      if (l < 0) return;
      gl.enableVertexAttribArray(l);
      gl.vertexAttribPointer(l, sz, typ, norm, 32, off);
      gl.vertexAttribDivisor(l, 1);
    };
    fAttr('a_pos', 2, 0);
    fAttr('a_rad', 1, 8);
    fAttr('a_rgba', 4, 12, true, gl.UNSIGNED_BYTE);
    fAttr('a_phase', 1, 16);
    fAttr('a_amp', 1, 20);
    const lm = gl.getAttribLocation(this.nodeProg, 'a_meta');
    gl.enableVertexAttribArray(lm);
    gl.vertexAttribIPointer(lm, 1, gl.UNSIGNED_INT, 32, 24);
    gl.vertexAttribDivisor(lm, 1);

    // Edge VAO (28B stride)
    this.edgeVao = gl.createVertexArray()!;
    gl.bindVertexArray(this.edgeVao);
    gl.bindBuffer(gl.ARRAY_BUFFER, lineT);
    const et = gl.getAttribLocation(this.edgeProg, 'a_t');
    gl.enableVertexAttribArray(et);
    gl.vertexAttribPointer(et, 1, gl.FLOAT, false, 0, 0);

    gl.bindBuffer(gl.ARRAY_BUFFER, this.edgeBuf);
    const es = gl.getAttribLocation(this.edgeProg, 'a_src');
    gl.enableVertexAttribArray(es);
    gl.vertexAttribPointer(es, 2, gl.FLOAT, false, 28, 0);
    gl.vertexAttribDivisor(es, 1);
    const ed = gl.getAttribLocation(this.edgeProg, 'a_dst');
    gl.enableVertexAttribArray(ed);
    gl.vertexAttribPointer(ed, 2, gl.FLOAT, false, 28, 8);
    gl.vertexAttribDivisor(ed, 1);
    const esi = gl.getAttribLocation(this.edgeProg, 'a_srcId');
    gl.enableVertexAttribArray(esi);
    gl.vertexAttribIPointer(esi, 1, gl.UNSIGNED_INT, 28, 16);
    gl.vertexAttribDivisor(esi, 1);
    const edi = gl.getAttribLocation(this.edgeProg, 'a_dstId');
    gl.enableVertexAttribArray(edi);
    gl.vertexAttribIPointer(edi, 1, gl.UNSIGNED_INT, 28, 20);
    gl.vertexAttribDivisor(edi, 1);
    const ec = gl.getAttribLocation(this.edgeProg, 'a_conf');
    gl.enableVertexAttribArray(ec);
    gl.vertexAttribIPointer(ec, 1, gl.UNSIGNED_INT, 28, 24);
    gl.vertexAttribDivisor(ec, 1);

    // Pick VAO
    this.pickVao = gl.createVertexArray()!;
    gl.bindVertexArray(this.pickVao);
    gl.bindBuffer(gl.ARRAY_BUFFER, quad);
    const pq = gl.getAttribLocation(this.pickProg, 'a_quad');
    gl.enableVertexAttribArray(pq);
    gl.vertexAttribPointer(pq, 2, gl.FLOAT, false, 0, 0);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.nodeBuf);
    const pp = gl.getAttribLocation(this.pickProg, 'a_pos');
    gl.enableVertexAttribArray(pp);
    gl.vertexAttribPointer(pp, 2, gl.FLOAT, false, 32, 0);
    gl.vertexAttribDivisor(pp, 1);
    const pr = gl.getAttribLocation(this.pickProg, 'a_rad');
    gl.enableVertexAttribArray(pr);
    gl.vertexAttribPointer(pr, 1, gl.FLOAT, false, 32, 8);
    gl.vertexAttribDivisor(pr, 1);
    const pm = gl.getAttribLocation(this.pickProg, 'a_meta');
    gl.enableVertexAttribArray(pm);
    gl.vertexAttribIPointer(pm, 1, gl.UNSIGNED_INT, 32, 24);
    gl.vertexAttribDivisor(pm, 1);
    gl.bindVertexArray(null);

    this.maskTex = gl.createTexture()!;
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.maskTex);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.R32UI, MASK_W, MASK_H, 0, gl.RED_INTEGER, gl.UNSIGNED_INT, new Uint32Array(MASK_W * MASK_H));

    this.pickFbo = gl.createFramebuffer()!;
    this.pickTex = gl.createTexture()!;
    this.setupInput();
    this.resize();
    this.loop(performance.now());
  }

  private program(vs: string, fs: string): WebGLProgram {
    const gl = this.gl;
    const mk = (t: number, s: string) => {
      const sh = gl.createShader(t)!;
      gl.shaderSource(sh, s);
      gl.compileShader(sh);
      if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(sh) || 'shader err');
      return sh;
    };
    const p = gl.createProgram()!;
    gl.attachShader(p, mk(gl.VERTEX_SHADER, vs));
    gl.attachShader(p, mk(gl.FRAGMENT_SHADER, fs));
    gl.linkProgram(p);
    return p;
  }

  public uploadGraph(nodeBuf: ArrayBuffer, edgeBuf: ArrayBuffer, clusters: ClusterInfo[]) {
    this.nodeCount = Math.floor(nodeBuf.byteLength / 32);
    this.edgeCount = Math.floor(edgeBuf.byteLength / 28);
    this.clusters = clusters;
    this.gl.bindBuffer(this.gl.ARRAY_BUFFER, this.nodeBuf);
    this.gl.bufferData(this.gl.ARRAY_BUFFER, nodeBuf, this.gl.STATIC_DRAW);
    this.gl.bindBuffer(this.gl.ARRAY_BUFFER, this.edgeBuf);
    this.gl.bufferData(this.gl.ARRAY_BUFFER, edgeBuf, this.gl.STATIC_DRAW);
    this.fitAll();
  }

  public fitAll() {
    if (this.clusters.length === 0) {
      this.cam.tx = 0; this.cam.ty = 0; this.cam.tzoom = 0.8;
      return;
    }
    let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
    for (const c of this.clusters) {
      minX = Math.min(minX, c.cx - c.radius);
      maxX = Math.max(maxX, c.cx + c.radius);
      minY = Math.min(minY, c.cy - c.radius);
      maxY = Math.max(maxY, c.cy + c.radius);
    }
    const w = Math.max(120, maxX - minX);
    const h = Math.max(120, maxY - minY);
    const zx = (this.canvas.clientWidth * 0.62) / w;
    const zy = (this.canvas.clientHeight * 0.72) / h;
    this.cam.tx = (minX + maxX) * 0.5;
    this.cam.ty = (minY + maxY) * 0.5;
    this.cam.tzoom = Math.max(0.08, Math.min(2.5, Math.min(zx, zy)));
    this.pickDirty = true;
  }

  public flyTo(x: number, y: number, zoom = 1.6) {
    this.cam.tx = x;
    this.cam.ty = y;
    this.cam.tzoom = Math.max(0.2, Math.min(6.0, zoom));
    this.pickDirty = true;
  }

  public zoomBy(factor: number) {
    this.cam.tzoom = Math.max(0.05, Math.min(12.0, this.cam.tzoom * factor));
    this.pickDirty = true;
  }

  public uploadImpactMask(words: Uint32Array, selectedId: number) {
    this.selectedId = selectedId;
    const gl = this.gl;
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.maskTex);
    gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, MASK_W, MASK_H, gl.RED_INTEGER, gl.UNSIGNED_INT, words);
  }

  public setMotion(on: boolean) { this.motion = on ? 1.0 : 0.0; }
  public setShowEdges(on: boolean) { this.showEdges = on; }
  public setShowClusters(on: boolean) { this.showClusters = on; }
  public setLangMask(mask: number) { this.langMask = mask; this.pickDirty = true; }
  public setConfMask(mask: number) { this.confMask = mask; }

  public resize() {
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const cw = Math.max(1, this.canvas.clientWidth);
    const ch = Math.max(1, this.canvas.clientHeight);
    this.canvas.width = Math.floor(cw * dpr);
    this.canvas.height = Math.floor(ch * dpr);
    this.overlay.width = Math.floor(cw * dpr);
    this.overlay.height = Math.floor(ch * dpr);
    this.ctx2d.setTransform(dpr, 0, 0, dpr, 0, 0);

    const gl = this.gl;
    gl.bindTexture(gl.TEXTURE_2D, this.pickTex);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, Math.max(1, cw >> 1), Math.max(1, ch >> 1), 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.pickFbo);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, this.pickTex, 0);
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    this.pickDirty = true;
  }

  private drawClusterOverlay() {
    const ctx = this.ctx2d;
    const cw = this.canvas.clientWidth;
    const ch = this.canvas.clientHeight;
    ctx.clearRect(0, 0, cw, ch);
    if (!this.showClusters || this.clusters.length === 0) return;

    ctx.textAlign = 'center';
    ctx.textBaseline = 'bottom';
    ctx.font = '600 11px "JetBrains Mono", monospace';

    for (const c of this.clusters) {
      const sx = (c.cx - this.cam.x) * this.cam.zoom + cw * 0.5;
      const sy = (c.cy - this.cam.y) * this.cam.zoom + ch * 0.5;
      const sr = c.radius * this.cam.zoom;
      if (sx + sr < -40 || sx - sr > cw + 40 || sy + sr < -40 || sy - sr > ch + 40) continue;

      ctx.beginPath();
      ctx.arc(sx, sy, sr, 0, Math.PI * 2);
      ctx.fillStyle = 'rgba(76, 194, 255, 0.035)';
      ctx.fill();
      ctx.strokeStyle = 'rgba(76, 194, 255, 0.18)';
      ctx.lineWidth = 1;
      ctx.setLineDash([4, 4]);
      ctx.stroke();
      ctx.setLineDash([]);

      if (sr > 24) {
        const label = `${c.name} (${c.member_count})`;
        ctx.fillStyle = 'rgba(20, 25, 34, 0.78)';
        const tw = ctx.measureText(label).width + 12;
        ctx.fillRect(sx - tw * 0.5, sy - sr - 20, tw, 17);
        ctx.fillStyle = '#9EC5E6';
        ctx.fillText(label, sx, sy - sr - 6);
      }
    }
  }

  private pickAt(clientX: number, clientY: number): number {
    const gl = this.gl;
    const pw = Math.max(1, this.canvas.clientWidth >> 1);
    const ph = Math.max(1, this.canvas.clientHeight >> 1);
    if (this.pickDirty) {
      gl.bindFramebuffer(gl.FRAMEBUFFER, this.pickFbo);
      gl.viewport(0, 0, pw, ph);
      gl.clearColor(0, 0, 0, 1);
      gl.clear(gl.COLOR_BUFFER_BIT);
      gl.disable(gl.BLEND);
      gl.useProgram(this.pickProg);
      gl.uniform2f(gl.getUniformLocation(this.pickProg, 'u_res'), this.canvas.clientWidth, this.canvas.clientHeight);
      gl.uniform2f(gl.getUniformLocation(this.pickProg, 'u_cam'), this.cam.x, this.cam.y);
      gl.uniform1f(gl.getUniformLocation(this.pickProg, 'u_zoom'), this.cam.zoom);
      gl.uniform1i(gl.getUniformLocation(this.pickProg, 'u_langMask'), this.langMask);
      gl.bindVertexArray(this.pickVao);
      gl.drawArraysInstanced(gl.TRIANGLE_STRIP, 0, 4, this.nodeCount);
      gl.bindFramebuffer(gl.FRAMEBUFFER, null);
      this.pickDirty = false;
    }
    const rect = this.canvas.getBoundingClientRect();
    const px = Math.floor(((clientX - rect.left) / rect.width) * pw);
    const py = Math.floor((1 - (clientY - rect.top) / rect.height) * ph);
    const pix = new Uint8Array(4);
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.pickFbo);
    gl.readPixels(px, py, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, pix);
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    const v = pix[0] | (pix[1] << 8) | (pix[2] << 16);
    return v === 0 ? -1 : v - 1;
  }

  private setupInput() {
    let dragging = false, moved = false, lx = 0, ly = 0;
    this.canvas.addEventListener('pointerdown', (e) => {
      dragging = true; moved = false; lx = e.clientX; ly = e.clientY;
      this.canvas.classList.add('dragging');
    });
    this.canvas.addEventListener('pointermove', (e) => {
      if (dragging) {
        const dx = e.clientX - lx, dy = e.clientY - ly;
        if (Math.abs(dx) > 2 || Math.abs(dy) > 2) moved = true;
        this.cam.tx -= dx / this.cam.zoom;
        this.cam.ty -= dy / this.cam.zoom;
        this.cam.x = this.cam.tx;
        this.cam.y = this.cam.ty;
        lx = e.clientX; ly = e.clientY;
        this.pickDirty = true;
        return;
      }
      if (this.nodeCount > 0) {
        this.hoverId = this.pickAt(e.clientX, e.clientY);
        this.canvas.classList.toggle('hovering', this.hoverId >= 0);
      }
    });
    this.canvas.addEventListener('pointerup', (e) => {
      dragging = false;
      this.canvas.classList.remove('dragging');
      if (!moved && this.nodeCount > 0) this.onSelect(this.pickAt(e.clientX, e.clientY));
    });
    this.canvas.addEventListener('wheel', (e) => {
      e.preventDefault();
      const rect = this.canvas.getBoundingClientRect();
      const mx = e.clientX - rect.left - rect.width * 0.5;
      const my = e.clientY - rect.top - rect.height * 0.5;
      const wx = this.cam.tx + mx / this.cam.tzoom;
      const wy = this.cam.ty + my / this.cam.tzoom;
      this.cam.tzoom = Math.max(0.05, Math.min(12, this.cam.tzoom * Math.exp(-e.deltaY * 0.0014)));
      this.cam.tx = wx - mx / this.cam.tzoom;
      this.cam.ty = wy - my / this.cam.tzoom;
      this.pickDirty = true;
    }, { passive: false });
  }

  private loop = (now: number) => {
    const rawMs = now - this.prevTs;
    const dt = Math.min(0.05, rawMs * 0.001);
    this.prevTs = now;

    const ease = 1.0 - Math.exp(-dt * 12.0);
    if (Math.abs(this.cam.tx - this.cam.x) > 0.05 || Math.abs(this.cam.ty - this.cam.y) > 0.05 || Math.abs(this.cam.tzoom - this.cam.zoom) > 0.0005) {
      this.cam.x += (this.cam.tx - this.cam.x) * ease;
      this.cam.y += (this.cam.ty - this.cam.y) * ease;
      this.cam.zoom += (this.cam.tzoom - this.cam.zoom) * ease;
      this.pickDirty = true;
    }

    const gl = this.gl;
    gl.viewport(0, 0, this.canvas.width, this.canvas.height);
    gl.clearColor(0.043, 0.055, 0.078, 1);
    gl.clear(gl.COLOR_BUFFER_BIT);

    if (this.nodeCount > 0) {
      gl.enable(gl.BLEND);
      gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, this.maskTex);

      if (this.showEdges && this.edgeCount > 0) {
        gl.useProgram(this.edgeProg);
        gl.uniform2f(gl.getUniformLocation(this.edgeProg, 'u_res'), this.canvas.clientWidth, this.canvas.clientHeight);
        gl.uniform2f(gl.getUniformLocation(this.edgeProg, 'u_cam'), this.cam.x, this.cam.y);
        gl.uniform1f(gl.getUniformLocation(this.edgeProg, 'u_zoom'), this.cam.zoom);
        gl.uniform1f(gl.getUniformLocation(this.edgeProg, 'u_hasSel'), this.selectedId >= 0 ? 1 : 0);
        gl.uniform1i(gl.getUniformLocation(this.edgeProg, 'u_confMask'), this.confMask);
        gl.uniform1i(gl.getUniformLocation(this.edgeProg, 'u_mask'), 0);
        gl.bindVertexArray(this.edgeVao);
        gl.drawArraysInstanced(gl.LINES, 0, 2, this.edgeCount);
      }

      gl.useProgram(this.nodeProg);
      gl.uniform2f(gl.getUniformLocation(this.nodeProg, 'u_res'), this.canvas.clientWidth, this.canvas.clientHeight);
      gl.uniform2f(gl.getUniformLocation(this.nodeProg, 'u_cam'), this.cam.x, this.cam.y);
      gl.uniform1f(gl.getUniformLocation(this.nodeProg, 'u_zoom'), this.cam.zoom);
      gl.uniform1f(gl.getUniformLocation(this.nodeProg, 'u_time'), now * 0.001);
      gl.uniform1f(gl.getUniformLocation(this.nodeProg, 'u_motion'), this.motion);
      gl.uniform1f(gl.getUniformLocation(this.nodeProg, 'u_hasSel'), this.selectedId >= 0 ? 1 : 0);
      gl.uniform1i(gl.getUniformLocation(this.nodeProg, 'u_hoverId'), this.hoverId);
      gl.uniform1i(gl.getUniformLocation(this.nodeProg, 'u_selId'), this.selectedId);
      gl.uniform1i(gl.getUniformLocation(this.nodeProg, 'u_langMask'), this.langMask);
      gl.uniform1i(gl.getUniformLocation(this.nodeProg, 'u_mask'), 0);
      gl.bindVertexArray(this.nodeVao);
      gl.drawArraysInstanced(gl.TRIANGLE_STRIP, 0, 4, this.nodeCount);
      gl.bindVertexArray(null);
    }

    this.drawClusterOverlay();

    this.fpsAccum += rawMs;
    this.fpsFrames++;
    if (now - this.lastHud > 300) {
      const avg = this.fpsAccum / Math.max(1, this.fpsFrames);
      this.onHud(Math.round(1000 / Math.max(0.1, avg)), avg);
      this.fpsAccum = 0;
      this.fpsFrames = 0;
      this.lastHud = now;
    }
    this.rafId = requestAnimationFrame(this.loop);
  };

  public destroy() { cancelAnimationFrame(this.rafId); }
}
