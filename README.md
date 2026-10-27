# 🎧 MixLab

<div align="center">

**Software de DJ Profissional — Construído com Tauri, Rust e React**

[![Tauri](https://img.shields.io/badge/Tauri-v2.0-FFC131?style=for-the-badge&logo=tauri&logoColor=white)](https://tauri.app)
[![Rust](https://img.shields.io/badge/Rust-1.80+-DEA584?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![React](https://img.shields.io/badge/React-19-61DAFB?style=for-the-badge&logo=react&logoColor=white)](https://react.dev)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.6-3178C6?style=for-the-badge&logo=typescript&logoColor=white)](https://www.typescriptlang.org)
[![Vite](https://img.shields.io/badge/Vite-8-646CFF?style=for-the-badge&logo=vite&logoColor=white)](https://vite.dev)
[![Tailwind](https://img.shields.io/badge/Tailwind_CSS-3.4-06B6D4?style=for-the-badge&logo=tailwindcss&logoColor=white)](https://tailwindcss.com)
[![License](https://img.shields.io/badge/License-MIT-green?style=for-the-badge)](LICENSE)
[![Version](https://img.shields.io/badge/Version-0.1.0-orange?style=for-the-badge)](CHANGELOG.md)

[Funcionalidades](#-funcionalidades) • [Instalação](#-instalação) • [Uso](#-uso) • [Arquitetura](#-arquitetura) • [Desenvolvimento](#-desenvolvimento) • [Testes](#-testes-e-qualidade) • [Changelog](#-changelog)

</div>

---

## 📖 Sobre o Projeto

**MixLab** é um software de DJ profissional desenvolvido com foco em **performance**, **baixa latência** e **precisão de áudio**. Utilizando **Rust** no backend para processamento de áudio em tempo real sem garbage collection, combinado com uma interface moderna construída em **React 19**, **TypeScript** e **Tailwind CSS**, o MixLab oferece uma experiência de mixagem profissional e responsiva rodando nativamente no seu sistema operacional via **Tauri v2**.

### 🎯 Por que MixLab?

| Característica | Benefício |
|---|---|
| **Rust + CPAL** | Áudio em tempo real sem GC pauses, latência < 10ms |
| **Tauri v2** | Binários nativos pequenos (~10MB), segurança por design, WebView2/WebKit |
| **React 19 + Vite 8** | HMR instantâneo, DX moderna, bundle otimizado, Server Components ready |
| **TypeScript Strict** | Type safety completa, refatoração segura |
| **Tailwind CSS 3.4** | Design system utility-first, dark mode nativo, JIT compiler |
| **Zero dependências pesadas** | Sem Electron, sem Chromium bundleado, menor footprint |

---

## ✨ Funcionalidades

### 🎛️ Decks Duplos (A e B)

| Funcionalidade | Descrição |
|---|---|
| **Carregamento de Arquivos** | Suporte nativo a MP3, WAV, FLAC, OGG, M4A via seletor do sistema (`tauri-plugin-dialog`) |
| **Jog Wheels Interativos** | Discos giratórios com *scrubbing* (avançar/retroceder) e pausa ao segurar (hold-to-pause) com feedback visual |
| **Pitch Fader** | Controle de velocidade de **-20% a +20%** (0.8x–1.2x) com cálculo dinâmico do BPM em tempo real |
| **Sistema CUE Inteligente** | Marcação e retorno a pontos específicos da faixa (toggle marcar/retornar) |
| **Play/Pause Unificado** | Controle intuitivo com feedback visual imediato e sincronização com engine Rust |
| **Display de Tempo** | Formato `MM:SS / MM:SS` sincronizado com a duração real da faixa |

### 🎨 Visualização de Áudio Avançada

- **Big Waveforms de 3 Camadas** (Deck Header): Análise espectral via FFT separando:
  - 🟠 **Graves (Low)**: 20–300 Hz — bumbo, sub-bass, linha de baixo
  - 🔵 **Médios (Mid)**: 300–4000 Hz — voz, sintetizadores, leads, pianos
  - 🟣 **Agudos (High)**: 4000 Hz+ — pratos, hi-hats, percussão, transientes
- **Mini Waveforms** (Deck Controls): Versão compacta em cada deck com agulha móvel sincronizada (RMS blocks)
- **Scroll horizontal**: Navegação sincronizada com posição de reprodução
- **Cores sobrepostas estilo Rekordbox**: A mistura das 3 camadas revela o conteúdo musical (ex: roxo sobre laranja = bumbo + pratos)

> **Nota**: A detecção de BPM é otimizada para gêneros populares (Pop, House, Hip-Hop). Gêneros com ritmos complexos (Trance, Progressive, Jazz) podem requerer ajuste manual.

### 🎚️ Mixer Profissional Central

| Componente | Especificação |
|---|---|
| **EQ 3 Bandas** | Knobs rotativos HI/MID/LO (-12 dB a +12 dB) com filtros biquad padrão Butterworth (Audio EQ Cookbook) |
| **Filter Knob** | Filtro combinado Low-Pass/High-Pass estilo CDJ (0=LPF max, 50=neutro, 100=HPF max) |
| **Faders de Volume** | Sliders verticais com indicador numérico em dB (-60 dB a +12 dB, escala logarítmica) |
| **VU Meters** | Medidores LED (verde/amarelo/vermelho) atualizados a ~20 FPS via polling Rust |
| **Crossfader** | Mixagem suave entre os decks A e B com curva **equal-power** (cos/sin π/2) |

### ⚙️ Motor de Áudio (Rust + CPAL)

- **Latência Ultrabaixa**: Processamento em tempo real com buffers pré-alocados (512 frames CPAL, 2048 FFT)
- **DSP Otimizado**: Zero alocação de memória na thread de áudio (tempo real garantido — sem GC)
- **Pipeline DSP** (ordem profissional): **Filter → EQ (3 bandas) → Gain → Crossfader**
- **Análise de Faixa**: Detecção automática de BPM (autocorrelação via FFT) e geração de waveforms no carregamento
- **Decodificação Multi-codec**: Symphonia com suporte a MP3, WAV, FLAC, AAC/M4A, OGG — sem FFmpeg
- **Pitch/Time-stretch**: Interpolação linear com `src_index` fracionário (preparado para Rubato/WSOLA futuro)

---

## 🛠️ Stack Tecnológica

| Camada | Tecnologia | Versão | Finalidade |
|---|---|---|---|
| **Frontend Framework** | React | 19.1.0 | Interface reativa, hooks modernos, concurrent features |
| **Language** | TypeScript | 5.6 | Type safety estrita, inference avançada |
| **Build Tool** | Vite | 8.0.16 | Dev server rápido, build otimizado, code splitting |
| **Estilização** | Tailwind CSS | 3.4.19 | Design system utility-first, dark mode, JIT |
| **Ícones** | Lucide React | 1.61.0 | Ícones vetoriais consistentes, tree-shakable |
| **Estado Global** | Zustand | 5.0.16 | Estado leve e performático (fora do React tree) |
| **Backend/Runtime** | Rust | 1.80+ | Performance, segurança de memória, zero-cost abstractions |
| **Framework Desktop** | Tauri | 2.x | Ponte Rust ↔ WebView, binários nativos, IPC seguro |
| **Áudio I/O** | CPAL | 0.18.2 | Playback de baixa latência cross-platform |
| **DSP/FFT** | realfft | 3.5.0 | Análise espectral rápida (FFT baseada em RustFFT) |
| **Decodificação** | Symphonia | 0.5 | Suporte a múltiplos codecs puro Rust |
| **Serialização** | Serde | 1.0 | JSON type-safe entre frontend/backend |
| **Math/Audio** | num-complex | 0.4.6 | Números complexos para FFT |
| **DB Local** | rusqlite | 0.31 | Cache de análise (BPM, waveforms) com bundled SQLite |

---

## 📦 Instalação

### Pré-requisitos

Certifique-se de ter instalados:

- **Node.js** 18+ ([download](https://nodejs.org))
- **Rust** 1.80+ via [rustup](https://rustup.rs) (`rustup update stable`)
- **Git** ([git-scm.com](https://git-scm.com))

> **Windows**: Requer **Visual Studio Build Tools** com "Desktop development with C++" para compilar o backend Rust.
>
> **Linux**: `sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev` (ou equivalentes da distro)
>
> **macOS**: Xcode Command Line Tools (`xcode-select --install`)

### Passo a Passo

```bash
# 1. Clone o repositório
git clone https://github.com/<SEU_USUARIO>/MixLab.git
cd MixLab

# 2. Instale as dependências do frontend
npm install

# 3. (Opcional) Verifique a configuração do Tauri
npx tauri info

# 4. Inicie o ambiente de desenvolvimento
npm run tauri dev
```

### Build para Produção

```bash
# Gera o instalador nativo
# Windows: .msi (NSIS)
# macOS:   .dmg + .app
# Linux:   .AppImage + .deb + .rpm
npm run tauri build
```

Os instaladores serão gerados em `src-tauri/target/release/bundle/`.

---

## 🎮 Uso

### Fluxo Básico de Mixagem

#### 1. Carregar Faixas
- Clique em **LOAD** no Deck A ou B
- Selecione um arquivo de áudio do seu computador
- A waveform e o BPM serão calculados automaticamente (pode levar alguns segundos em arquivos longos)

#### 2. Preparar a Mixagem
- Ajuste o **Pitch** para igualar os BPMs dos dois decks (objetivo: BPMs idênticos)
- Use o **CUE** para marcar pontos de entrada (primeiro clique = marcar, segundo = retornar)
- Utilize o **Jog Wheel** para alinhar as batidas manualmente (beatmatch)

#### 3. Mixar
- Suba o **Volume** do deck que está entrando
- Ajuste o **EQ** para evitar sobreposição de frequências (ex: corte LO no deck entrante)
- Use o **Crossfader** para transições suaves entre decks
- Aplique o **Filter** para efeitos de transição (sweep LPF/HPF)

### Controles Principais

| Controle | Função | Atalho / Interação |
|---|---|---|
| **LOAD** | Carrega uma faixa do sistema de arquivos | Clique |
| **PLAY/PAUSE** | Inicia/pausa a reprodução da faixa | Clique |
| **CUE** | Marca ou retorna ao ponto de referência | Clique (toggle) |
| **Jog Wheel** | Gire para scrubbing, segure para pausar | Drag / Hold |
| **Pitch Fader** | Ajusta a velocidade da faixa (-20% a +20%) | Slider vertical |
| **EQ (HI/MID/LO)** | Ajusta frequências (-12 dB a +12 dB) | Knobs rotativos |
| **FILTER** | Filtro passa-baixa/passa-alta combinado | Knob (0=LPF, 50=neutro, 100=HPF) |
| **VOL** | Controle de volume do deck (-60 dB a +12 dB) | Slider vertical |
| **CROSSFADER** | Mixagem entre os decks A e B | Slider horizontal |

---

## 🏗️ Arquitetura

### Visão Geral do Sistema

```
┌─────────────────────────────────────────────────────────────────────┐
│                           MIXLAB                                     │
├──────────────────────────────────┬──────────────────────────────────┤
│       FRONTEND (React + TS)      │        BACKEND (Rust)            │
│                                  │                                  │
│  ┌──────────────────────────┐    │  ┌──────────────────────────┐   │
│  │     Deck A / Deck B      │    │  │      AudioEngine         │   │
│  │  • JogWheel (canvas)     │◄───┼──┤  • CPAL Output Stream    │   │
│  │  • Waveform (SVG/Canvas) │    │  │  • Pitch/Time-stretch    │   │
│  │  • Controls (UI)         │    │  └───────────┬──────────────┘   │
│  └──────────────────────────┘    │              │                  │
│           │ Tauri IPC (invoke)   │  ┌───────────▼──────────────┐   │
│  ┌────────▼────────────────┐    │  │       DSP Pipeline       │   │
│  │        Mixer           │    │  │  • Filter (Butterworth)  │   │
│  │  • EQ Knobs (biquad)   │◄───┼──┤  • EQ 3 bandas (biquad)  │   │
│  │  • Filter Knob         │    │  │  • Gain (linear/log)     │   │
│  │  • Volume Faders       │    │  │  • Crossfader (equal-pow)│   │
│  │  • VU Meters (20fps)   │    │  └──────────────────────────┘   │
│  └────────────────────────┘    │              │                  │
│                                │  ┌───────────▼──────────────┐   │
│  ┌────────────────────────┐    │  │       Analysis           │   │
│  │    Waveform Display    │    │  │  • BPM Detection         │   │
│  │  • Peaks (RMS blocks)  │◄───┼──┤  • Waveform Layers (FFT) │   │
│  │  • Low/Mid/High FFT    │    │  │  • Peak Extraction       │   │
│  └────────────────────────┘    │  └──────────────────────────┘   │
│                                │                                  │
└────────────────────────────────┴──────────────────────────────────┘
```

### Fluxo de Dados

1. **Carregamento**: Frontend → `invoke('load_track')` → Rust decodifica (Symphonia) → FFT/BPM → Retorna `TrackInfo` + waveforms → Cache SQLite
2. **Playback**: Frontend → `invoke('play_deck')` → Engine marca `is_playing=true` → CPAL callback consome samples com DSP
3. **DSP Tempo Real**: CPAL callback (thread áudio) → Filter → EQ → Gain → Crossfader → Output buffer (zero alloc)
4. **UI Sync**: Frontend polling `get_vu_levels` (50ms) + `requestAnimationFrame` para position/playhead
5. **Eventos**: Backend emite `track_analysis_complete` → Frontend atualiza BPM display

### Estrutura de Diretórios

```
MixLab/
├── .github/
│   └── workflows/              # GitHub Actions CI/CD
├── public/                     # Assets estáticos
├── src/                        # Frontend React + TypeScript
│   ├── components/
│   │   ├── Deck.tsx            # Componente de deck (controles, jog, waveform)
│   │   ├── Mixer.tsx           # Mixer central (EQ, faders, VU, crossfader)
│   │   ├── JogWheel.tsx        # Disco giratório interativo (canvas)
│   │   ├── Knob.tsx            # Knob rotativo reutilizável (SVG)
│   │   ├── VUMeter.tsx         # Medidor de nível LED (canvas)
│   │   ├── Waveform.tsx        # Mini waveform (canvas, RMS)
│   │   └── BigWaveform.tsx     # Waveform 3 camadas header (canvas, FFT)
│   ├── App.tsx                 # Layout principal, estado global, IPC bridge, event listeners
│   ├── main.tsx                # Entry point React
│   ├── index.css               # Tailwind + variáveis CSS custom + design tokens
│   └── vite-env.d.ts           # Types Tauri
├── src-tauri/                  # Backend Rust
│   ├── src/
│   │   ├── audio/
│   │   │   ├── engine.rs       # Motor principal (fill_buffer, load_track, DSP)
│   │   │   ├── decoder.rs      # Decodificação Symphonia multi-formato
│   │   │   └── dsp/
│   │   │       ├── eq.rs       # Equalizador 3 bandas (biquad Audio EQ Cookbook)
│   │   │       ├── filter.rs   # Filtro LPF/HPF combinado (Butterworth)
│   │   │       ├── gain.rs     # Gain linear/logarítmico com clamp
│   │   │       ├── crossfader.rs # Crossfader equal-power curve
│   │   │       └── mod.rs      # Exports DSP
│   │   ├── analysis/
│   │   │   ├── bpm.rs          # Detecção BPM (autocorrelação via FFT)
│   │   │   ├── waveform.rs     # Geração peaks (RMS) + layers FFT (3 bandas)
│   │   │   └── mod.rs          # Exports analysis
│   │   ├── commands.rs         # Comandos Tauri (IPC surface - 14 comandos)
│   │   ├── state.rs            # AppState, Arc<Mutex<AudioEngine>>, DeckState, DB pool
│   │   ├── database.rs         # Cache SQLite (análises BPM/waveform)
│   │   ├── lib.rs              # Module declarations + re-exports
│   │   └── main.rs             # Entry point Tauri (minimal)
│   ├── Cargo.toml              # Dependências Rust + profiles
│   ├── tauri.conf.json         # Configuração Tauri v2
│   └── icons/                  # Ícones da aplicação (png/ico/icns)
├── package.json                # Dependências Node + scripts
├── vite.config.ts              # Configuração Vite + plugins + bundle analysis
├── tailwind.config.cjs         # Configuração Tailwind
├── tsconfig.json               # TypeScript config (strict mode)
├── README.md                   # Este arquivo
├── CHANGELOG.md                # Histórico de versões
└── LICENSE                     # Licença MIT
```

### Módulos Principais do Backend

| Módulo | Responsabilidade | Linhas aprox. |
|---|---|---|
| `engine.rs` | AudioEngine, CPAL stream, fill_buffer, TrackData, DeckState | 380 |
| `decoder.rs` | AudioDecoder (Symphonia), format detection, downmix mono | 200 |
| `dsp/eq.rs` | Equalizer 3 bandas, Biquad filters, Audio EQ Cookbook | 200 |
| `dsp/filter.rs` | Filter LPF/HPF combinado, stateful, smooth interpolation | 110 |
| `dsp/gain.rs` | GainNode linear, dB↔linear, clamp anti-clipping | 70 |
| `dsp/crossfader.rs` | Equal-power curve, mix buffers, gain calc | 100 |
| `analysis/bpm.rs` | BPM detection via autocorrelação + spectral flux | 180 |
| `analysis/waveform.rs` | 3-layer FFT (2048), peak extraction, normalize (p95+gamma) | 280 |
| `commands.rs` | 14 Tauri commands (IPC), async load_track, DB cache | 400 |
| `state.rs` | AppState, Arc<Mutex<AudioEngine>>, DeckState, DB pool | 120 |
| `database.rs` | SQLite cache (path, BPM, duration, waveform layers) | 150 |

---

## 🧪 Testes e Qualidade

### Backend (Rust)

```bash
cd src-tauri

# Testes unitários + integração
cargo test

# Testes com output detalhado
cargo test -- --nocapture

# Testes específicos
cargo test test_eq_          # Testes do EQ
cargo test test_bpm_         # Testes BPM
cargo test test_waveform_    # Testes waveforms
cargo test test_crossfader_  # Testes crossfader

# Linting (requer rustfmt + clippy)
cargo fmt --check
cargo clippy -- -D warnings

# Security audit
cargo audit
```

**Cobertura atual**: 25+ testes de integração cobrindo engine, DSP components, BPM detection, waveform generation, EQ/gain/filter/crossfader, pitch clamping, cue/jog/VU, concorrência, isolamento de estado.

### Frontend (React + TypeScript)

```bash
# Type checking (strict mode)
npm run typecheck   # npx tsc --noEmit

# Linting (oxlint - 10-100x mais rápido que ESLint)
npm run lint
npm run lint:fix    # Auto-fix

# Formatação
npm run format      # Prettier + Tailwind plugin

# Build de produção (valida tipos + build)
npm run build

# Bundle analysis (abre no browser)
npm run build && open dist/bundle-analysis.html
```

### CI/CD Pipeline (GitHub Actions)

| Job | Plataforma | Verificações |
|---|---|---|
| `rust-checks` | ubuntu-latest | `fmt`, `clippy`, `test`, `build --release` |
| `frontend-checks` | ubuntu-latest | `typecheck`, `lint`, `build` |
| `tauri-build-linux` | ubuntu-latest | `tauri-action` → upload artifacts |
| `tauri-build-windows` | windows-latest | `tauri-action` → upload artifacts |
| `tauri-build-macos` | macos-latest | `tauri-action` → upload artifacts |
| `release` | ubuntu-latest | Tag `v*` → download artifacts → create release |

---

## 🚀 Desenvolvimento

### Comandos Úteis

```bash
# Desenvolvimento (frontend + backend com hot reload)
npm run tauri dev

# Apenas frontend (Vite dev server)
npm run dev

# Build frontend apenas
npm run build

# Preview build produção
npm run preview

# Verificar tipos TypeScript
npx tsc --noEmit

# Formatar código Rust
cd src-tauri && cargo fmt

# Lint Rust
cd src-tauri && cargo clippy

# Atualizar dependências Rust
cd src-tauri && cargo update

# Atualizar dependências Node
npm update

# Verificar vulnerabilidades
npm audit
cargo audit
```

### Variáveis de Ambiente

Crie `.env.local` na raiz (não versionado):

```env
# Opcional: configurar dispositivo de áudio específico
# CPAL_DEFAULT_DEVICE=nome_do_dispositivo

# Debug áudio
# RUST_LOG=debug,mixlab=trace
```

### Debugging

**Frontend**: DevTools do navegador (WebView) — `F12` ou `Ctrl+Shift+I`

**Backend Rust**:
```bash
# Logs no terminal onde rodou `tauri dev`
RUST_LOG=debug npm run tauri dev

# VS Code: extensão "rust-analyzer" + "CodeLLDB" para debug nativo
```

**Perfil de Performance**:
```bash
# Build release + profile
cd src-tauri && cargo build --release
./target/release/mixlab
```

---

## 📋 Changelog

Todas as alterações notáveis são documentadas em [CHANGELOG.md](CHANGELOG.md).

### v0.1.0 (2025-10-26) — Versão Inicial
- ✅ Decks duplos com jog wheels, pitch fader, cue points
- ✅ Waveforms 3-camadas (Low/Mid/High) via FFT + mini waveforms
- ✅ Mixer completo: EQ 3 bandas, filter, volume, crossfader, VU meters
- ✅ Motor Rust: CPAL, Symphonia, realfft, DSP pipeline zero-allocation
- ✅ Tauri v2 + React 19 + TypeScript + Vite 8 + Tailwind CSS
- ✅ Build nativo multiplataforma + CI/CD GitHub Actions
- ✅ Cache SQLite para análises (BPM + waveforms)
- ✅ Normalização waveform: percentil 95 + gamma correction (padrão Rekordbox)

---

## 📄 Licença

Distribuído sob a licença **MIT**. Veja [LICENSE](LICENSE) para mais informações.

---

## 🙏 Agradecimentos

- **Tauri Team** — Framework incrível para apps desktop nativos
- **Rust Audio Community** — CPAL, Symphonia, realfft, Rubato
- **React & Vite Teams** — DX excepcional
- **Tailwind CSS** — Utility-first que acelera desenvolvimento
- **Lucide** — Ícones limpos e consistentes
- **Comunidade DJ/Áudio** — Pelo feedback e inspiração

---

<div align="center">

**Feito com ❤️ e Rust para a comunidade DJ**

[⬆ Voltar ao topo](#-mixlab)

</div>