# Changelog

Todas as alterações notáveis neste projeto serão documentadas neste arquivo.

O formato é baseado em [Keep a Changelog](https://keepachangelog.com/pt-BR/1.0.0/),
e este projeto adere ao [Versionamento Semântico](https://semver.org/lang/pt-BR/).

## [0.1.1] - 2025-10-27

### Adicionado
- Comando unificado `get_deck_status()` para sincronização perfeita (posição + VU + estado de play)
- Loop de sincronização no frontend com `requestAnimationFrame` polling a 60fps

### Corrigido
- **Sincronização perfeita**: Eliminação do drift temporal — `Date.now()` substituído por polling atômico do motor Rust
- **Jog Wheel, waveform scroll e VU Meters**: Agora 100% sincronizados com a posição real do áudio
- Detecção BPM com validação de half-time/double-time (scoring musical)
- BigWaveform usa 3 camadas espectrais sobrepostas (Low/Mid/High)
- Normalização de waveform com percentil 95 + gamma 0.5 (padrão Rekordbox)
- Database movido para fora de `src-tauri` (evita loop de rebuild)
- Análise de BPM/waveform em `spawn_blocking` (não trava UI)
- `package-lock.json` sincronizado para CI/CD

---

## [0.1.0] - 2025-10-26

### Adicionado

#### 🎛️ Decks Duplos (A e B)
- Carregamento de arquivos via seletor nativo (MP3, WAV, FLAC, OGG, M4A) usando `tauri-plugin-dialog`
- Jog Wheels interativos com scrubbing (drag) e hold-to-pause (segure para pausar)
- Pitch Fader com range de **-20% a +20%** (0.8x–1.2x) e cálculo dinâmico de BPM em tempo real
- Sistema CUE inteligente: primeiro clique marca ponto, segundo clique retorna ao ponto
- Play/Pause unificado com sincronização imediata com engine Rust via IPC
- Display de tempo no formato `MM:SS / MM:SS` sincronizado com duração real da faixa

#### 🎨 Visualização de Áudio Avançada
- **Big Waveforms de 3 Camadas** (Header dos Decks): Análise espectral via FFT (tamanho 2048) separando:
  - 🟠 **Low (Graves)**: 20–300 Hz — bumbo, sub-bass, linha de baixo
  - 🔵 **Mid (Médios)**: 300–4000 Hz — voz, sintetizadores, leads, pianos
  - 🟣 **High (Agudos)**: 4000 Hz+ — pratos, hi-hats, percussão, transientes
- **Mini Waveforms** (Controles do Deck): Versão compacta baseada em RMS blocks com agulha móvel (playhead) sincronizada
- **Normalização estilo Rekordbox**: Percentil 95 + gamma correction (√) para preservar dinâmica e contraste visual
- **Cores sobrepostas**: Mistura aditiva das 3 camadas revela conteúdo musical (ex: roxo sobre laranja = bumbo + pratos)
- Scroll horizontal sincronizado com posição de reprodução

#### 🎚️ Mixer Profissional Central
- **EQ 3 Bandas**: Knobs rotativos HI/MID/LO (-12 dB a +12 dB) com filtros biquad padrão **Audio EQ Cookbook** (Butterworth)
  - Low: Low-shelf 320 Hz, Q=1.0
  - Mid: Peaking 1000 Hz, Q=1.0
  - High: High-shelf 3200 Hz, Q=1.0
- **Filter Knob**: Filtro combinado Low-Pass/High-Pass estilo CDJ (0=LPF máximo, 50=neutro, 100=HPF máximo)
  - Interpolação logarítmica suave de frequência (20 Hz – 20 kHz)
  - Resonance controlada, stateful entre buffers
- **Faders de Volume**: Sliders verticais com indicador numérico em dB (-60 dB a +12 dB, escala logarítmica)
- **VU Meters**: Medidores LED (verde/amarelo/vermelho) baseados em RMS, atualizados a ~20 FPS via polling
- **Crossfader**: Mixagem suave entre os decks A e B com curva **equal-power** (cos/sin π/2) — loudness constante

#### ⚙️ Motor de Áudio (Rust + CPAL)
- **CPAL** para I/O de áudio cross-platform (WASAPI/ALSA/CoreAudio/CoreAudio) com latência < 10ms
- **Symphonia** para decodificação multi-codec puro Rust (MP3, WAV, FLAC, AAC/M4A, OGG) — sem FFmpeg
- **realfft** (baseado em RustFFT) para análise espectral FFT rápida
- **DSP Pipeline Zero-Allocation**: Nenhuma alocação na thread de áudio (tempo real garantido)
  - Pipeline: **Filter → EQ (3 bandas) → Gain → Crossfader**
  - Buffers pré-alocados (`temp_buffer_a`, `temp_buffer_b`, `output_buffer`)
  - `src_index` fracionário para pitch shifting suave
- **Detecção BPM**: Autocorrelação via FFT em diferenças de energia espectral (range 60–200 BPM)
- **Geração de Waveforms**: 
  - Peaks: RMS por bloco (mini waveform)
  - Layers: FFT 3 bandas por bloco (big waveform) — 1000 pontos máx
- **Cache SQLite** (rusqlite bundled): Persistência de BPM, duração e camadas de waveform por arquivo

#### 🏗️ Infraestrutura e Qualidade
- **Tauri v2** + **React 19** + **TypeScript 5.6** + **Vite 8** + **Tailwind CSS 3.4**
- **Zustand** para estado global leve (fora da árvore React)
- **Lucide React** para ícones vetoriais tree-shakable
- **Build nativo multiplataforma**:
  - Windows: .msi (NSIS)
  - macOS: .dmg + .app (universal)
  - Linux: .AppImage + .deb + .rpm
- **Release Profile Otimizado**: LTO, strip, panic=abort, codegen-units=1, opt-level=3
- **TypeScript Strict Mode**: Type safety completa, sem `any` em código novo
- **ESLint + oxlint + Prettier**: Linting rápido e formatação consistente

### Corrigido
- Borrow checker issues no `AudioEngine::fill_buffer` — `calculate_rms` como associated function para evitar conflito de borrow
- Stream CPAL config borrow no `state.rs` — clone de config antes de `into_stream`
- Position calculation usando `src_index` fracionário em vez de `current_frame` inteiro
- BPM detection range corrigido: `min_lag_frames=26` (~200 BPM), `max_lag_frames=86` (~60 BPM)
- Normalização de waveform alterada de escala logarítmica (dB) para **percentil 95 + gamma 0.5** (padrão Rekordbox)

### Segurança
- Arquitetura Tauri v2 com CSP e IPC type-safe (`invoke` tipado)
- Validação de inputs em todos os comandos Tauri (clamp, bounds check)
- Non-root containers (preparação Docker futura)
- `serde` para serialização type-safe entre frontend/backend

---

## [Unreleased]

### Planejado para v0.2.0

#### 🎵 Funcionalidades de DJ
- [ ] Biblioteca de faixas com metadados (ID3 tags: título, artista, BPM, key, artwork)
- [ ] **Hot Cues**: Múltiplos pontos CUE por deck (A–H) com pads coloridos
- [ ] **Auto Loop**: Loop automático beat-sync (1/2/4/8/16/32 beats) com quantize
- [ ] **Sync Button**: Beat sync automático (alinha phase + BPM)
- [ ] **Key Detection**: Detecção de tonalidade (Camelot/Key notation) + key shift
- [ ] **Pitch/Time-stretch Avançado**: Integração Rubato (WSOLA/FFT-based) para qualidade profissional

#### 🎛️ Efeitos e Performance
- [ ] Efeitos built-in: Reverb, Delay, Flanger, Phaser, Echo
- [ ] **Gravação de Sessão**: Export WAV/MP3 da mixagem completa
- [ ] **MIDI Mapping**: Mapeamento para controllers externos (Pioneer DDJ, Numark, etc.)
- [ ] **Waveform Overview**: Scrollbar com waveform completa da faixa para navegação rápida

#### 🧪 Qualidade e Testes
- [ ] Testes unitários Rust (>80% cobertura: DSP, analysis, engine)
- [ ] Testes frontend: Vitest + React Testing Library (componentes, hooks)
- [ ] Testes E2E: Playwright (fluxos críticos: load → play → mix → record)
- [ ] Benchmarks de áudio: SNR, THD, latency measurement
- [ ] CI/CD GitHub Actions completo (já configurado, aguardando push)

### Planejado para v0.3.0

#### 🚀 Expansão Avançada
- [ ] **4 Decks Mode**: Suporte a 4 decks simultâneos
- [ ] **Sampler/Remix Decks**: Slots de samples disparáveis (one-shots, loops)
- [ ] **Video Mixing Support**: Visuals sincronizados com áudio
- [ ] **Streaming Integration**: SoundCloud, Beatport Link, Beatsource
- [ ] **Cloud Sync**: Sincronização de biblioteca, cues, loops via backend
- [ ] **Plugin Architecture**: Suporte a VST3/CLAP para efeitos de terceiros

---

## Legenda

- `Adicionado` para novas funcionalidades
- `Alterado` para mudanças em funcionalidades existentes
- `Obsoleto` para funcionalidades que serão removidas
- `Removido` para funcionalidades removidas
- `Corrigido` para correções de bugs
- `Segurança` para correções de vulnerabilidades

---

## Comparação com Referências da Indústria

| Feature | MixLab v0.1.0 | Rekordbox 7 | Serato DJ Pro | Traktor Pro 4 |
|---|---|---|---|---|
| Decks | 2 | 2/4 | 2/4 | 2/4 |
| Waveform 3-camadas | ✅ | ✅ | ✅ | ❌ (2 camadas) |
| EQ 3 bandas biquad | ✅ | ✅ | ✅ | ✅ |
| Filter LPF/HPF | ✅ | ✅ | ✅ | ✅ |
| Hot Cues | 🔄 v0.2 | ✅ | ✅ | ✅ |
| Auto Loop | 🔄 v0.2 | ✅ | ✅ | ✅ |
| Beat Sync | 🔄 v0.2 | ✅ | ✅ | ✅ |
| Key Detection | 🔄 v0.2 | ✅ | ✅ | ✅ |
| MIDI Mapping | 🔄 v0.2 | ✅ | ✅ | ✅ |
| Gravação | 🔄 v0.2 | ✅ | ✅ | ✅ |
| 4 Decks | 🔄 v0.3 | ✅ | ✅ | ✅ |
| Video | 🔄 v0.3 | ✅ | ❌ | ❌ |
| Streaming | 🔄 v0.3 | ✅ | ✅ | ❌ |
| Plugins (VST/CLAP) | 🔄 v0.3 | ❌ | ❌ | ✅ |

---

## Como Contribuir com o Changelog

Ao fazer PRs, adicione entradas na seção `[Unreleased]` seguindo o formato:

```markdown
### Adicionado
- Descrição clara da funcionalidade

### Corrigido
- Descrição do bug e solução

### Alterado
- O que mudou e porquê
```

Na release, mova as entradas de `[Unreleased]` para a nova versão com data.
