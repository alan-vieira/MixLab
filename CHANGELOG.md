# Changelog

Todas as alterações notáveis neste projeto serão documentadas neste arquivo.

O formato é baseado em [Keep a Changelog](https://keepachangelog.com/pt-BR/1.0.0/),
e este projeto adere ao [Versionamento Semântico](https://semver.org/lang/pt-BR/).

## [0.1.0] - 2025-10-26

### Adicionado
- **Decks Duplos (A e B)**
  - Carregamento de arquivos via seletor nativo (MP3, WAV, FLAC, OGG, M4A)
  - Jog Wheels interativos com scrubbing e hold-to-pause
  - Pitch Fader (-20% a +20%) com BPM dinâmico
  - Sistema CUE inteligente (marcar/retornar)
  - Play/Pause unificado com sincronização Rust
  - Display de tempo MM:SS / MM:SS

- **Visualização de Áudio**
  - Big Waveforms de 3 camadas (Low/Mid/High) via FFT
  - Mini Waveforms com agulha móvel sincronizada
  - Scroll e zoom para análise detalhada

- **Mixer Profissional**
  - EQ 3 bandas (HI/MID/LO) -12 dB a +12 dB (biquad Butterworth)
  - Filter Knob LPF/HPF combinado (0-100, 50=neutro)
  - Faders de volume verticais (-60 dB a +12 dB)
  - VU Meters LED 20 FPS (verde/amarelo/vermelho)
  - Crossfader equal-power suave

- **Motor de Áudio Rust**
  - CPAL para I/O de áudio cross-platform baixa latência
  - Symphonia para decodificação multi-codec
  - realfft para análise espectral FFT
  - DSP pipeline zero-allocation (tempo real garantido)
  - Pipeline: Filter → EQ → Gain → Crossfader
  - Detecção BPM via autocorrelação
  - Geração de waveforms (peaks + layers FFT)

- **Infraestrutura**
  - Tauri v2 + React 19 + TypeScript + Vite 8
  - Tailwind CSS 3.4 (utility-first)
  - Zustand para estado global
  - Lucide React para ícones
  - Build nativo multiplataforma (Windows/macOS/Linux)
  - Release profile otimizado (LTO, strip, panic=abort)

### Corrigido
- Borrow checker issues no `AudioEngine::fill_buffer` (calculate_rms como fn associada)
- Stream CPAL config borrow no `state.rs` (clone antes de into)
- Position calculation usando `src_index` em vez de `current_frame`

### Segurança
- Arquitetura Tauri v2 com CSP e IPC type-safe
- Non-root containers (futuro Docker)
- Validação de inputs em todos os comandos Tauri

---

## [Unreleased]

### Planejado para v0.2.0
- [ ] Biblioteca de faixas com metadados (ID3 tags)
- [ ] Hot cues (múltiplos pontos CUE por deck)
- [ ] Loop automático (auto-loop 1/2/4/8/16/32 beats)
- [ ] Sync button (beat sync automático)
- [ ] Efeitos: Reverb, Delay, Flanger, Phaser
- [ ] Gravação de sessão (WAV/MP3)
- [ ] MIDI mapping para controllers externos
- [ ] Waveform overview (scrollbar com waveform completa)
- [ ] Key detection (tonalidade) + key shift
- [ ] Testes unitários Rust (>80% cobertura)
- [ ] Testes frontend (Vitest + Playwright)
- [ ] CI/CD GitHub Actions

### Planejado para v0.3.0
- [ ] 4 decks mode
- [ ] Sampler/Remix decks
- [ ] Video mixing support
- [ ] Streaming integration (SoundCloud, Beatport Link)
- [ ] Cloud sync de biblioteca/cues
- [ ] Plugin architecture (VST3/CLAP)

---

## Legenda

- `Adicionado` para novas funcionalidades
- `Alterado` para mudanças em funcionalidades existentes
- `Obsoleto` para funcionalidades que serão removidas
- `Removido` para funcionalidades removidas
- `Corrigido` para correções de bugs
- `Segurança` para correções de vulnerabilidades
