# MixLab — Documentação Completa de Layout, Template e UI

**Versão:** 0.2.0  
**Projeto:** MixLab — Aplicativo de DJ desktop (Tauri 2 + React + Vite + Rust)  
**Referência:** Rekordbox (AlphaTheta) — interface 2Deck Horizontal  
**Data:** 05/11/2026  
**Status:** Layout base implementado; biblioteca (Parte 3) pendente

---

## 1. Visão Geral

O MixLab é um aplicativo de DJ para desktop com engine de áudio em Rust (Rubato para time-stretching/key lock, CPAL para áudio) e frontend em React + Vite + Tailwind CSS. A interface segue a estrutura de três partes da Rekordbox:

1. **Parte 1 — BigWaveforms** (topo): duas waveforms empilhadas com playhead fixo no centro.
2. **Parte 2 — Decks + Mixer** (meio): Deck A | Mixer central | Deck B.
3. **Parte 3 — Biblioteca** (base): navegação em árvore à esquerda + track list à direita (pendente de implementação).

---

## 2. Referência: Interface Rekordbox

Abaixo, a extração exaustiva dos screenshots da Rekordbox utilizada como referência de design.

### 2.1 Layout geral (proporções)

| Seção | Ocupação vertical | Comportamento |
|-------|-------------------|---------------|
| Header / controles globais | ~8–10% | Fixo, altura baixa |
| Waveforms principais | ~15–22% | Duas ondas, cada uma ~half da seção |
| Decks + Mixer | ~55–60% | Maior área; decks laterais largos, mixer central estreito |
| Biblioteca | ~15–20% | Árvore à esquerda (~25–30% largura), track list à direita (~70–75%) |

Largura na seção dos decks: Deck A ~40–45%, Mixer ~10–20%, Deck B ~40–45%.

### 2.2 Cores e paleta (Rekordbox)

| Elemento | Cor |
|----------|-----|
| Fundo principal | Dark charcoal #222222 / #282828 / #1A1A1A |
| Texto padrão | Branco #FFFFFF |
| Texto secundário/inativo | Light gray |
| Item selecionado (fundo) | Azul escuro #004080 |
| Texto selecionado | Branco |
| Waveforms | Ciano/light blue (peaks brancos em transientes) |
| Grid de batidas | Linhas brancas finas (batidas) + vermelhas grossas (downbeats) |
| Playhead | Linha vermelha vertical + triângulos laranja topo/base |
| Hot cues | Paleta de 8 cores: azul, verde, roxo, laranja, teal, rosa, amarelo, vermelho |
| Key display | Texto branco sobre fundo vermelho (ex: "Dm", "E") |
| MASTER ativo | Fundo laranja |
| CUE button (deck) | Laranja |
| CUE button (mixer/PFL) | Azul quando ativo |
| Borda/separador | Cinza sutil |

### 2.3 Waveform (Rekordbox vs MixLab)

| Item | Rekordbox (referência) | MixLab atual |
|------|------------------------|--------------|
| Posição da agulha | Avança da esquerda para direita; onda rola sob ela | Fixa no centro; onda desliza sob a agulha fixa |
| Espaçamento entre barras | Zero gap (barras contíguas); detailed: 2px barra + 1px gap; overview: 1px + 1px | `barGap = barWidth * 0.15` (15%) |
| Cantos das barras | Retos | Arredondados (`cornerRadius`) |
| Grid de batidas | Linhas brancas finas + downbeats vermelhos grossos | Grid sutil de 16 divisões, sem destaque de downbeat |
| Hot cues na onda | Triângulos coloridos nas bordas superior/inferior | Não desenhados |
| Rótulo de compassos | "128.3Bars" próximo à agulha | Centralizado embaixo da agulha |
| Cores da onda | Espectro multicolorido (vermelho, verde, azul, roxo, laranja, amarelo) | 3 camadas espectrais fixas: low (laranja), mid (ciano), high (roxo) |

### 2.4 Deck — Track info (Rekordbox)

| Elemento | Posição | Detalhe |
|----------|---------|---------|
| Número do deck | Topo esquerdo | Branco grande (ex: "1", "2") |
| Artwork | Ao lado do número | Thumb ~30x30px |
| Título | Após artwork | Branco grande |
| Artista | Abaixo do título | Branco menor |
| BPM | Jog wheel centro | Branco grande (ex: "124.0", "125.0") |
| Key | Topo direito | Branco sobre fundo vermelho (ex: "Dm", "C") |
| Tempo decorrido | Topo, centro-direita | Branco (ex: "04:07.2", "00:15.4") |
| Tempo restante / total | Topo direita | Branco |
| SYNC / MASTER / WIDE | Topo direito, coluna vertical | Botões retangulares; MASTER laranja quando ativo |

### 2.5 Hot cue pads (Rekordbox)

- 8 pads por deck, layout 2 colunas × 4 linhas (A–D esquerda, E–H direita; espelhado no Deck 2).
- Cada pad: letra + timestamp quando setado (ex: "B — 01:15").
- Cores: azul (A), verde (B), roxo (C), laranja (D), teal (E), rosa, amarelo, vermelho.
- Pads vazios: fundo cinza escuro, letra em light gray.
- Abas: HOT CUE (ativa), PAD FX, SLICER, BEAT JUMP.
- Botões abaixo: MEMORY CUE, CALL.

### 2.6 Jog wheel / controles de deck (Rekordbox)

| Controle | Detalhe |
|----------|---------|
| BPM | Centro do jog, branco grande |
| Pitch % | Abaixo do BPM, branco menor |
| Pitch range | "±10" ao lado do pitch % |
| CUE button | Esquerda do jog (Deck 1) / Direita (Deck 2), laranja, circular |
| Play/Pause | Abaixo do CUE |
| Loop length | "LOOP 2" / "LOOP 8" acima do CUE |
| Loop in/out | Botões < > para ajustar loop |
| SLIP | Acima do jog, inativo por padrão |
| MT (Master Tempo) | Abaixo do jog, laranja quando ativo |
| Q (Quantize) | Ao lado do jog |
| INT selector | Acima do jog, com setas up/down |
| Pitch fader | Vertical, à direita do jog, handle centro = 0% |

### 2.7 Mixer central (Rekordbox)

Da cima para baixo, por canal:

1. **TRIM knob** — circular, cinza escuro, indicador branco 12h, label "TRIM".
2. **EQ knobs HI / MID / LO** — três knobs empilhados, centro = flat, labels à direita (Deck 1) / esquerda (Deck 2).
3. **Filter knob** — circular, centro = neutro.
4. **CUE button** — quadrado, azul quando ativo (PFL), label "CUE".
5. **Channel fader** — vertical, handle com linha indicadora de posição.
6. **Crossfader assignment** — botão "1" ou "2" abaixo do fader.

Seção fones (inferior central):
- Ícone fone + knob MIXING + knob LEVEL (ambos centro).
- Crossfader horizontal, centro = mix equilibrado.
- Grid 2×2 assignment buttons (1, 2, 3, 4) abaixo do crossfader.

### 2.8 Memory cues (Rekordbox)

- Lista vertical à direita dos controles.
- Cada entrada: ponto colorido + timestamp (ex: "00:15:415") + botão X delete.
- Slots vazios abaixo das entradas ativas.

### 2.9 Biblioteca (Rekordbox) — Parte 3

**Árvore esquerda (~25% largura):**
- Collection (ícone nota musical ♪), não expansível.
- Playlists (ícone lista), expansível ▼ / >.
- Pastas de playlist (ícone pasta, [-] aberto / [+] fechado).
- Playlists individuais (ícone documento com linhas).
- Smart playlists (ícone documento com engrenagem).
- Indentação: nível 1 alinhado esquerda, nível 2 ~10px, nível 3 ~20px com linhas verticais de conexão.
- Item selecionado: fundo azul escuro, texto branco.
- Ícone + à direita do item selecionado (adicionar/criar sub-playlist).

**Track list direita (~75% largura):**
- Cabeçalho: título da playlist + contagem (ex: "Main Floor (30 Tracks)").
- Search bar: lupa esquerda, "Q" + dropdown direita.
- Filter buttons: ícones (nota musical, BPM, genre, rating).
- View modes: List (ativo), Grid, Waveform.
- Colunas: #, BPM, Preview (miniatura onda), Artwork, Track Title, Artist, Album, Key, Time.
- Ordenação: triângulo azul no cabeçalho da coluna ativa.
- Linhas selecionadas: fundo cinza claro, texto branco; seleção única em azul escuro + nome em verde.
- Hot cues na prévia: marcadores coloridos (A–H) na miniatura da onda.
- Status bar: "Selected: 30 Tracks, 3 hours 20 minutes, 425.3 MB".

---

## 3. Estado Atual do MixLab (implementado)

### 3.1 Stack tecnológico

- **Frontend:** React 18, Vite, Tailwind CSS, lucide-react, zustand.
- **Backend:** Tauri 2, Rust, Rubato (time-stretching), CPAL (áudio).
- **Testes:** Vitest (unitário), Playwright (e2e).
- **Build:** `npm run build`, `cargo build`.

### 3.2 Estrutura de pastas

```
src/
  App.tsx          # raiz da aplicação, layout principal
  main.tsx         # entrada Vite
  index.css        # estilos globais/Tailwind
  components/
    Deck.tsx       # componente de deck (A e B)
    BigWaveform.tsx# waveform canvas com 3 camadas espectrais
    Mixer.tsx      # mixer central com EQ, filter, trim, volume, crossfader
    JogWheel.tsx   # jog wheel circular com rotação
    Knob.tsx       # knob rotativo genérico
    VUMeter.tsx    # medidor de nível vertical
    Waveform.tsx   # miniatura de onda (peaks)
  lib/             # utilitários
src-tauri/
  src/
    lib.rs         # entrada Tauri
    state.rs       # estado global
    commands.rs    # comandos (load_track, play_deck, pause_deck, set_cue_point, jump_to_cue, set_deck_pitch, set_deck_eq, set_deck_gain, set_deck_filter, set_crossfader, set_deck_trim, set_key_lock, jog_scrub, etc.)
    audio/
      engine.rs    # engine de áudio: render_deck, decks
      dsp/         # EQ, filter, gain, limiter, crossfader, resample
    analysis/      # BPM, waveform layers
  tauri.conf.json
tests/
  e2e/
    deck-flow.spec.ts
    mock-tauri.js
```

### 3.3 Layout atual (App.tsx)

```tsx
<div className="h-screen w-screen bg-gray-950 text-gray-100 flex flex-col overflow-hidden">
  {/* HEADER */}
  <header className="h-12 bg-gray-900 border-b border-gray-800 flex items-center px-6 justify-between shrink-0 z-10">
    <div className="flex items-center gap-2">
      <div className="w-3 h-3 rounded-full bg-orange-500 shadow-..."></div>
      <h1 className="font-bold text-xl tracking-wider text-gray-200">MIX<span className="text-orange-500">LAB</span></h1>
    </div>
    <div className="text-xs text-gray-500 font-mono">
      ENGINE: RUST + CPAL | STATUS: <span className="text-green-500">ONLINE</span>
    </div>
  </header>

  {/* BIG WAVEFORMS */}
  <div className="h-32 bg-gray-950 border-b border-gray-800 flex flex-col shrink-0 z-0">
    <div className="h-1/2 border-b border-gray-800 relative bg-gray-900 overflow-hidden">
      <BigWaveform low={...} mid={...} high={...} scrollOffset={...} deckLabel="A" />
    </div>
    <div className="h-1/2 relative bg-gray-900 overflow-hidden">
      <BigWaveform low={...} mid={...} high={...} scrollOffset={...} deckLabel="B" />
    </div>
  </div>

  {/* MAIN: DECKS + MIXER */}
  <div className="flex-1 min-h-0 flex flex-col overflow-hidden">
    <main className="flex-1 p-3 grid grid-cols-[1fr_200px_1fr] gap-3 min-h-0 overflow-hidden">
      {/* Deck A (esquerda, 1fr) */}
      <Deck side="A" color="blue" ... />
      {/* Mixer (centro, 200px fixo) */}
      <Mixer vuLevelA={...} vuLevelB={...} />
      {/* Deck B (direita, 1fr) */}
      <Deck side="B" color="red" ... />
    </main>
  </div>

  {/* BIBLIOTECA (Parte 3) - container colapsado */}
  <div className="bg-gray-950 border-t border-gray-800 shrink-0 overflow-hidden transition-all duration-300">
    {/* Shell vazio - preenchido na Fase 2 */}
  </div>

  {/* FOOTER */}
  <footer className="h-8 bg-gray-900 border-t border-gray-800 flex items-center px-6 shrink-0">
    <span className="text-xs text-gray-500">Pronto para mixar.</span>
  </footer>
</div>
```

### 3.4 Componente Deck (estado atual)

| Elemento | Implementação | Cor |
|----------|---------------|-----|
| Cabeçalho | Deck number, título, artista, BPM "---", tempo "00:00 / MM:SS" | Branco / light gray |
| Jog wheel | SVG circular, BPM centro, pitch %, CUE laranja, play/pause | Azul (A) / Vermelho (B) |
| Pitch fader | Input range vertical, 0.8–1.2, 0% centro | Laranja |
| Botões | LOAD, PLAY/PAUSE, CUE (marcar), PITCH (key lock) | Cinza / azul / vermelho |
| Key lock | Toggle com ícone ShieldCheck | Esmeralda quando ativo |
| Mini waveform | Componente Waveform (peaks) | Azul/vermelho por deck |

### 3.5 Componente Mixer (estado atual)

| Elemento | Implementação | Cor |
|----------|---------------|-----|
| EQ HI/MID/LO | Knobs circulares, -12 a +12, padrão 0 | Azul (A) / Vermelho (B) |
| FILTER | Knob, 0–100, padrão 50 | Laranja |
| TRIM | Knob, 0–2, padrão 1 | Verde |
| VOL | Slider vertical, -60 a +12, padrão 0 | Branco |
| Crossfader | Slider horizontal, 0–100, padrão 50 | Branco |
| VU meters | Medidores verticais por deck | Azul (A) / Vermelho (B) |

### 3.6 Cores atuais do MixLab

| Elemento | Cor (Tailwind) |
|----------|----------------|
| Fundo principal | `bg-gray-950` (#0a0a0f) |
| Fundo decks/mixer | `bg-gray-900` (#171717) |
| Borda | `border-gray-800` (#262626) |
| Texto principal | `text-gray-100` / `text-gray-200` |
| Texto secundário | `text-gray-500` / `text-gray-400` |
| Deck A accent | `text-blue-500` / `bg-blue-500/10` |
| Deck B accent | `text-red-500` / `bg-red-500/10` |
| Waveform camadas | Low: laranja `rgba(249,115,22)`, Mid: ciano `rgba(34,211,238)`, High: roxo `rgba(167,139,250)` |
| Playhead | Laranja `#f97316` |
| Status online | Verde `text-green-500` |

---

## 4. Especificações de UI/UX (Template)

### 4.1 Tipografia

- **Fonte:** Sans-serif (sistema/interna do sistema), consistente em toda a interface.
- **Hierarquia:**
  - Título app: `text-xl font-bold tracking-wider`
  - Deck number: `font-bold text-sm tracking-wider`
  - Track title: `font-bold text-sm tracking-wider`
  - Track artist: `text-xs text-gray-400 truncate`
  - BPM: `font-mono text-sm font-bold text-white`
  - Tempo: `font-mono text-xs text-orange-400 font-bold`
  - Labels de controle: `text-[8px] text-gray-500 font-bold`
  - Valores: `text-[9px] text-orange-400 font-mono font-bold`

### 4.2 Espaçamento e grid

- **Grid principal:** `grid-cols-[1fr_200px_1fr]`, `gap-3`.
- **Padding geral:** `p-3` na área principal.
- **Header:** `px-6`, `h-12`.
- **Footer:** `px-6`, `h-8`.
- **Botões:** `height: 40px`, `rounded-lg`, `font-semibold` ou `font-bold`.
- **Jog wheel:** `width: 160px`, `height: 160px`.
- **Pitch fader:** `w-10`, `height: 140px`, input `w-24` rotacionado -90°.
- **Knobs:** `size={36}` (Knob component).
- **VU meters:** `height={280}`, `width={6}`.

### 4.3 Estados visuais

- **Botão ativo:** `bg-blue-600 hover:bg-blue-500` (Deck A) / `bg-red-600 hover:bg-red-500` (Deck B).
- **Botão inativo:** `bg-gray-800 hover:bg-gray-700 text-gray-200 border border-gray-700`.
- **Key lock ativo:** `bg-emerald-600 hover:bg-emerald-500 text-white border border-emerald-400`.
- **CUE set:** `bg-orange-600 hover:bg-orange-500 text-white`.
- **Jog wheel rodando:** Animação `animate-[spin_4s_linear_infinite]` no ícone Disc, `animationPlayState` controlado por `isPlaying`.

---

## 5. Componentes e Props

### 5.1 BigWaveform

```tsx
interface BigWaveformProps {
  low: number[];      // envelope低频 normalizado [0,1]
  mid: number[];      // envelope médio normalizado [0,1]
  high: number[];     // envelope alto normalizado [0,1]
  scrollOffset: number; // 0.0 a 1.0, posição de playback
  deckLabel: string;  // "A" ou "B"
}
```

- Playhead fixo no centro (`width / 2`).
- 3 camadas espectrais desenhadas com `fillRect` simétricas acima/abaixo da linha central.
- Grid vertical sutil (16 divisões).
- Label de barras centralizado embaixo da agulha.
- Suporte DPR (device pixel ratio) para telas Retina/HDPI.
- Sem threshold de redesenho (redesenha a cada mudança de `scrollOffset`).

### 5.2 Deck

```tsx
interface DeckProps {
  side: 'A' | 'B';
  color: 'blue' | 'red';
  scrollOffset: number;
  isPlaying: boolean;
  onPlay: () => void;
  onPause: () => void;
  onLoad: (layers: WaveformLayers) => void;
  onSeek: (newOffset: number) => void;
  bpm: number;
}
```

Estado interno: `trackName`, `pitch`, `duration`, `loading`, `waveformPeaks`, `cueSet`, `keyLock`, `localIsPlaying`.

### 5.3 Mixer

```tsx
interface MixerProps {
  vuLevelA: number;
  vuLevelB: number;
}
```

Estado interno: `eqA`, `eqB` ({low, mid, high, gain, filter}), `crossfader`, `trimA`, `trimB`.

### 5.4 JogWheel

```tsx
interface JogWheelProps {
  color: 'blue' | 'red';
  isPlaying: boolean;
  onJogRotate: (delta: number) => void;
  onJogHold: (isHolding: boolean) => void;
}
```

- Rotação automática baseada em delta time quando `isPlaying`.
- Arrastar para scrub (jog hold).
- SVG com anel de LEDs, disco interno giratório, marcador de posição.

### 5.5 Knob

Knob rotativo genérico com `value`, `min`, `max`, `defaultValue`, `onChange`, `label`, `color`, `size`.

### 5.6 VUMeter

Medidor de nível vertical com `level`, `color`, `height`, `width`.

---

## 6. Comandos Tauri (backend) disponíveis

| Comando | Parâmetros | Descrição |
|---------|------------|-----------|
| `get_deck_status` | `{ deck: number }` | Retorna `{ position, vu_level, is_playing }` |
| `play_deck` | `{ deck: number }` | Inicia reprodução |
| `pause_deck` | `{ deck: number }` | Pausa reprodução |
| `load_track` | `{ deck: number, path: string }` | Carrega faixa, retorna `{ duration_secs }` |
| `get_waveform_peaks` | `{ deck: number }` | Retorna array de peaks |
| `get_waveform_layers` | `{ deck: number }` | Retorna `[low, mid, high]` |
| `set_deck_pitch` | `{ deck: number, pitch: number }` | Ajusta pitch (0.8–1.2) |
| `set_key_lock` | `{ deck: number, enabled: boolean }` | Ativa/desativa key lock |
| `set_cue_point` | `{ deck: number }` | Define ponto de cue |
| `jump_to_cue` | `{ deck: number }` | Salta para ponto de cue |
| `set_deck_eq` | `{ deck: number, low, mid, high }` | Ajusta EQ |
| `set_deck_gain` | `{ deck: number, gainDb: number }` | Ajusta ganho |
| `set_deck_filter` | `{ deck: number, position: number }` | Ajusta filtro (0–1) |
| `set_crossfader` | `{ position: number }` | Ajusta crossfader (0–100) |
| `set_deck_trim` | `{ deck: number, trim: number }` | Ajusta trim (0–2) |
| `jog_scrub` | `{ deck: number, delta: number }` | Scrub via jog wheel |

Evento: `track_analysis_complete` com `{ deck: string|number, bpm: number }`.

---

## 7. Roadmap / Próximos Passos

### Fase 1 — Layout rearranjado (CONCLUÍDO)
- Área de decks/mixer agora flexível (`flex-1`), sem altura fixa.
- Container da biblioteca adicionado na base, colapsado.
- `typecheck` passando.

### Fase 2 — Biblioteca (Parte 3) — PENDENTE
- Árvore de coleção/playlists (esquerda, ~25% largura).
- Track list com colunas Preview, Artwork, Track Title, Artist, Album, Key, BPM, Time (direita, ~75%).
- Busca, filtros, view modes (list/grid/waveform).
- Status bar: "Selected: N Tracks, XhYm, Z MB".
- Toggle expandir/colapsar biblioteca.

### Fase 3 — Recursos de deck (gaps vs Rekordbox) — PENDENTE
- Hot cue pads A–H com timestamp e cores.
- Hot cue markers desenhados sobre o BigWaveform.
- Key display com fundo colorido.
- Botões SYNC / MASTER / WIDE no cabeçalho de cada deck.
- Loop controls: LOOP X beats, in/out, SLIP, MT, Q.
- Memory cues lista vertical com delete.
- Artwork thumb por deck.
- Tempo remaining/elapsed separados.

### Fase 4 — Mixer aprimorado — PENDENTE
- CUE buttons (PFL/headphone monitoring).
- Knobs MIXING / LEVEL.
- Crossfader assignment buttons (1/2).
- Filtros LO CUT / HI CUT.

### Fase 5 — 4 decks — PENDENTE (débito técnico listado no AGENTS.md)

---

## 8. Regras de Implementação (do AGENTS.md)

1. Criar branch a partir de `fix/primeira-correcao`.
2. Executar `cargo check` (Rust) e `npm run check` (frontend) antes de commitar.
3. Executar `npm run test:e2e` para validar fluxo de UI.
4. Executar `npm run test:coverage` e `cargo tarpaulin` para cobertura.
5. Commit com mensagem descritiva em português do Brasil.
6. Não commitar segredos, API keys ou credenciais.
7. Novas funcionalidades devem ter testes correspondentes (unitários ou e2e).
8. Cobertura mínima almejada: 80% (Rust + frontend).
9. O engine de áudio usa `rubato 0.15` para time-stretching (key lock). Ao modificar `render_deck` ou `AudioEngine`, garantir que `cargo check` passe.

---

## 9. Arquitetura de Dados

### 9.1 WaveformLayers (frontend)

```typescript
interface WaveformLayers {
  peaks: number[];
  low: number[];
  mid: number[];
  high: number[];
  duration: number;
}
```

### 9.2 DeckStatus (backend → frontend)

```rust
struct DeckStatus {
  position: f64,      // 0.0 a 1.0
  vu_level: f64,      // nível VU atual
  is_playing: boolean,
}
```

---

## 10. Referências

- Rekordbox interface (screenshots analisados): imagem_a2db81.png, image_bbfe00.png, image_3c6daa.png, image_a8d841.png, image_372767.png, image_7b3f07.png, image_659312.png, image_6ae6b9.png, image_b97d42.png, image_dc6782.png, image_29a5f3.png, image_2f7799.png, image_632a36.png, image_7343f5.png, image_27115d.png, image_eacfc7.png.
- Projeto MixLab: https://github.com/alan-vieira/MixLab
- Skill canvas-waveform-rendering (carregada): regras de renderização de waveform estilo Rekordbox.
- Skill impeccable (carregada): diretrizes de design de UI.

---

**Fim da documentação.**