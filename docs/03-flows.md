# 03 — Flows

## 1. User flow: design → export

```mermaid
flowchart LR
  A[Open app / page] --> B[Library<br/>browse · search]
  B --> C[Pick style<br/>e.g. FEFCO 0201]
  C --> D[Parameter panel<br/>L × W × H, board, options]
  D -->|every change| E[render&#40;project&#41;<br/>core, &lt;16 ms]
  E --> F[2D die-line view]
  E --> G[3D fold view]
  E --> H[Stats<br/>blank, area, lengths]
  F & G & H --> D
  D --> I{Export}
  I -->|SVG / PDF watermark| J[Free: local]
  I -->|DXF · CF2 · HPGL · clean PDF| K{Entitled?}
  K -->|yes| L[File saved / downloaded]
  K -->|no| M[Upgrade prompt]
```

## 2. Data flow inside the core

```mermaid
flowchart LR
  P[Project<br/>component id + params + board] --> V[Validate params<br/>types, ranges, rules]
  LIB[(Component file<br/>*.pfc.toml)] --> PARSE[Parse + schema check]
  PARSE --> EVAL
  V --> EVAL[Evaluate formulas<br/>pf-expr]
  EVAL --> GEO[Build geometry<br/>panels, flaps, lines]
  GEO --> DL[DieLine<br/>entities + line types]
  GEO --> FM[FoldModel<br/>panels + hinges]
  DL --> ST[Stats]
  DL --> EX[Exporters<br/>SVG · PDF · DXF · CF2 · HPGL]
  FM --> M3[Mesh / glTF]
  EX --> OUT[bytes + mime]
```

## 3. Web embed in Laravel (browser-only, Free tier)

```mermaid
sequenceDiagram
  participant U as User browser
  participant L as Laravel (Blade page)
  participant W as packforge-designer (WASM)
  U->>L: GET /configurator
  L-->>U: HTML + <script src=".../packforge-designer.js">
  U->>W: load WASM core + library index
  U->>W: change dimensions
  W-->>U: live 2D/3D preview (no server calls)
  U->>W: Save design
  W-->>U: emits event "pf:save" {project JSON}
  U->>L: POST /designs (project JSON)
  L-->>U: 201 Created
```

## 4. Server-side export from Laravel (Premium / API)

```mermaid
sequenceDiagram
  participant L as Laravel app
  participant Q as Laravel queue job
  participant A as pf-api (Docker)
  participant S as Storage (S3/local)
  L->>Q: dispatch GenerateDieline(order)
  Q->>A: POST /v1/export {project, format:"dxf"}<br/>Authorization: Bearer API_KEY
  A->>A: verify key → plan → entitlement
  A->>A: pf-core render + export (deterministic)
  A-->>Q: 200 application/dxf (bytes)
  Q->>S: store orders/123/box.dxf
  Q-->>L: event DielineGenerated
```

## 5. Premium operation from embedded designer

```mermaid
sequenceDiagram
  participant W as packforge-designer
  participant B as Your backend (Laravel)
  participant A as pf-api
  W->>B: POST /packforge/export {project, format}
  Note over W,B: Browser never sees the API key
  B->>A: POST /v1/export (Bearer key)
  A-->>B: file bytes
  B-->>W: file download
```

## 6. Desktop licensing

```mermaid
flowchart LR
  A[App start] --> B{License token<br/>stored?}
  B -->|no| F[Free tier]
  B -->|yes| C[pf-license verify<br/>Ed25519 signature,<br/>expiry, product, seats]
  C -->|valid| P[Premium entitlements]
  C -->|expired / invalid| F
  F --> G[Activate: paste key or sign in]
  G --> H[Activation server<br/>issues signed token]
  H --> C
  P --> R[Periodic refresh<br/>when online, grace 14 days offline]
```

## 7. Custom component authoring (Premium)

```mermaid
flowchart LR
  A[New component] --> B[Code editor<br/>*.pfc.toml]
  B -->|on save| C[Parse + validate]
  C -->|errors| B
  C --> D[Render with test params]
  D --> E[Preview 2D/3D]
  E --> F[Run checks<br/>closed outline, no overlaps,<br/>min flap sizes]
  F --> G[Publish to library<br/>versioned]
```
