// Tiny WebGL 2 renderer for folded boxes: lit two-sided panels plus dark edges.
// No dependencies, so the embeddable web component stays small.

import type { Folded, Vec3 } from './types';

type Mat4 = Float32Array;

function perspective(fovy: number, aspect: number, near: number, far: number): Mat4 {
  const f = 1 / Math.tan(fovy / 2);
  const nf = 1 / (near - far);
  return new Float32Array([
    f / aspect, 0, 0, 0,
    0, f, 0, 0,
    0, 0, (far + near) * nf, -1,
    0, 0, 2 * far * near * nf, 0,
  ]);
}

function lookAt(eye: Vec3, target: Vec3, up: Vec3): Mat4 {
  let zx = eye.x - target.x, zy = eye.y - target.y, zz = eye.z - target.z;
  let l = Math.hypot(zx, zy, zz) || 1;
  zx /= l; zy /= l; zz /= l;
  let xx = up.y * zz - up.z * zy, xy = up.z * zx - up.x * zz, xz = up.x * zy - up.y * zx;
  l = Math.hypot(xx, xy, xz) || 1;
  xx /= l; xy /= l; xz /= l;
  const yx = zy * xz - zz * xy, yy = zz * xx - zx * xz, yz = zx * xy - zy * xx;
  return new Float32Array([
    xx, yx, zx, 0,
    xy, yy, zy, 0,
    xz, yz, zz, 0,
    -(xx * eye.x + xy * eye.y + xz * eye.z),
    -(yx * eye.x + yy * eye.y + yz * eye.z),
    -(zx * eye.x + zy * eye.y + zz * eye.z),
    1,
  ]);
}

function multiply(a: Mat4, b: Mat4): Mat4 {
  const out = new Float32Array(16);
  for (let c = 0; c < 4; c++)
    for (let r = 0; r < 4; r++) {
      let s = 0;
      for (let k = 0; k < 4; k++) s += (a[k * 4 + r] ?? 0) * (b[c * 4 + k] ?? 0);
      out[c * 4 + r] = s;
    }
  return out;
}

const PANEL_VS = `#version 300 es
in vec3 aPos;
in vec3 aNormal;
uniform mat4 uViewProj;
out vec3 vNormal;
out vec3 vWorld;
void main() {
  vNormal = aNormal;
  vWorld = aPos;
  gl_Position = uViewProj * vec4(aPos, 1.0);
}`;

const PANEL_FS = `#version 300 es
precision highp float;
in vec3 vNormal;
in vec3 vWorld;
uniform vec3 uOutside;
uniform vec3 uInside;
uniform vec3 uLight;
uniform vec3 uEye;
out vec4 color;
void main() {
  vec3 n = normalize(vNormal);
  bool outside = dot(n, uEye - vWorld) >= 0.0;   // facing the camera = printed side
  if (!outside) n = -n;
  vec3 base = outside ? uOutside : uInside;
  float diffuse = max(dot(n, normalize(uLight)), 0.0);
  color = vec4(base * (0.42 + 0.58 * diffuse), 1.0);
}`;

const EDGE_VS = `#version 300 es
in vec3 aPos;
uniform mat4 uViewProj;
void main() { gl_Position = uViewProj * vec4(aPos, 1.0); }`;

const EDGE_FS = `#version 300 es
precision highp float;
uniform vec3 uColor;
out vec4 color;
void main() { color = vec4(uColor, 1.0); }`;

function compile(gl: WebGL2RenderingContext, vs: string, fs: string): WebGLProgram {
  const prog = gl.createProgram();
  for (const [type, src] of [
    [gl.VERTEX_SHADER, vs],
    [gl.FRAGMENT_SHADER, fs],
  ] as const) {
    const sh = gl.createShader(type);
    if (!sh) throw new Error('cannot create shader');
    gl.shaderSource(sh, src);
    gl.compileShader(sh);
    if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS))
      throw new Error(gl.getShaderInfoLog(sh) ?? 'shader error');
    gl.attachShader(prog, sh);
  }
  gl.linkProgram(prog);
  if (!gl.getProgramParameter(prog, gl.LINK_STATUS))
    throw new Error(gl.getProgramInfoLog(prog) ?? 'link error');
  return prog;
}

export interface Camera {
  yaw: number;
  pitch: number;
  zoom: number;
}

export interface Palette {
  outside: [number, number, number];
  inside: [number, number, number];
  edge: [number, number, number];
  background: [number, number, number];
}

export class BoxRenderer {
  private gl: WebGL2RenderingContext;
  private panelProg: WebGLProgram;
  private edgeProg: WebGLProgram;
  private panelVao: WebGLVertexArrayObject;
  private edgeVao: WebGLVertexArrayObject;
  private posBuf: WebGLBuffer;
  private nrmBuf: WebGLBuffer;
  private edgeBuf: WebGLBuffer;
  private triCount = 0;
  private edgeCount = 0;

  constructor(private canvas: HTMLCanvasElement) {
    const gl = canvas.getContext('webgl2', { antialias: true, preserveDrawingBuffer: true });
    if (!gl) throw new Error('WebGL 2 is not available');
    this.gl = gl;
    this.panelProg = compile(gl, PANEL_VS, PANEL_FS);
    this.edgeProg = compile(gl, EDGE_VS, EDGE_FS);
    this.posBuf = gl.createBuffer();
    this.nrmBuf = gl.createBuffer();
    this.edgeBuf = gl.createBuffer();

    this.panelVao = gl.createVertexArray();
    gl.bindVertexArray(this.panelVao);
    for (const [buf, name] of [
      [this.posBuf, 'aPos'],
      [this.nrmBuf, 'aNormal'],
    ] as const) {
      const loc = gl.getAttribLocation(this.panelProg, name);
      gl.bindBuffer(gl.ARRAY_BUFFER, buf);
      gl.enableVertexAttribArray(loc);
      gl.vertexAttribPointer(loc, 3, gl.FLOAT, false, 0, 0);
    }
    this.edgeVao = gl.createVertexArray();
    gl.bindVertexArray(this.edgeVao);
    const eloc = gl.getAttribLocation(this.edgeProg, 'aPos');
    gl.bindBuffer(gl.ARRAY_BUFFER, this.edgeBuf);
    gl.enableVertexAttribArray(eloc);
    gl.vertexAttribPointer(eloc, 3, gl.FLOAT, false, 0, 0);
    gl.bindVertexArray(null);
  }

  /** Uploads panel geometry (two triangles and four edges per panel). */
  setGeometry(f: Folded) {
    const pos: number[] = [];
    const nrm: number[] = [];
    const edges: number[] = [];
    for (const p of f.panels) {
      const [a, b, c, d] = p.corners;
      for (const v of [a, b, c, a, c, d]) {
        pos.push(v.x, v.y, v.z);
        nrm.push(p.normal.x, p.normal.y, p.normal.z);
      }
      for (const [u, w] of [
        [a, b],
        [b, c],
        [c, d],
        [d, a],
      ] as const)
        edges.push(u.x, u.y, u.z, w.x, w.y, w.z);
    }
    const gl = this.gl;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.posBuf);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(pos), gl.DYNAMIC_DRAW);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.nrmBuf);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(nrm), gl.DYNAMIC_DRAW);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.edgeBuf);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(edges), gl.DYNAMIC_DRAW);
    this.triCount = pos.length / 3;
    this.edgeCount = edges.length / 3;
  }

  /** Draws with the camera orbiting `target` at a distance scaled from `radius`. */
  draw(target: Vec3, radius: number, cam: Camera, palette: Palette) {
    const gl = this.gl;
    const dpr = window.devicePixelRatio || 1;
    const w = Math.max(1, Math.round(this.canvas.clientWidth * dpr));
    const h = Math.max(1, Math.round(this.canvas.clientHeight * dpr));
    if (this.canvas.width !== w || this.canvas.height !== h) {
      this.canvas.width = w;
      this.canvas.height = h;
    }
    gl.viewport(0, 0, w, h);
    gl.clearColor(...palette.background, 1);
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
    gl.enable(gl.DEPTH_TEST);

    const dist = radius * 2.6 * cam.zoom;
    const eye = {
      x: target.x + dist * Math.cos(cam.pitch) * Math.sin(cam.yaw),
      y: target.y + dist * Math.sin(cam.pitch),
      z: target.z + dist * Math.cos(cam.pitch) * Math.cos(cam.yaw),
    };
    const viewProj = multiply(
      perspective((40 * Math.PI) / 180, w / h, dist / 50, dist * 10),
      lookAt(eye, target, { x: 0, y: 1, z: 0 }),
    );

    gl.useProgram(this.panelProg);
    gl.uniformMatrix4fv(gl.getUniformLocation(this.panelProg, 'uViewProj'), false, viewProj);
    gl.uniform3fv(gl.getUniformLocation(this.panelProg, 'uOutside'), palette.outside);
    gl.uniform3fv(gl.getUniformLocation(this.panelProg, 'uInside'), palette.inside);
    gl.uniform3f(gl.getUniformLocation(this.panelProg, 'uLight'), 0.4, 0.8, 0.6);
    gl.uniform3f(gl.getUniformLocation(this.panelProg, 'uEye'), eye.x, eye.y, eye.z);
    gl.enable(gl.POLYGON_OFFSET_FILL);
    gl.polygonOffset(1, 1);
    gl.bindVertexArray(this.panelVao);
    gl.drawArrays(gl.TRIANGLES, 0, this.triCount);
    gl.disable(gl.POLYGON_OFFSET_FILL);

    gl.useProgram(this.edgeProg);
    gl.uniformMatrix4fv(gl.getUniformLocation(this.edgeProg, 'uViewProj'), false, viewProj);
    gl.uniform3fv(gl.getUniformLocation(this.edgeProg, 'uColor'), palette.edge);
    gl.bindVertexArray(this.edgeVao);
    gl.drawArrays(gl.LINES, 0, this.edgeCount);
    gl.bindVertexArray(null);
  }

  /** PNG of the current frame. */
  snapshot(): string {
    return this.canvas.toDataURL('image/png');
  }
}
