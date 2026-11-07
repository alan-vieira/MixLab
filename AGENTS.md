# AGENTS.md — MixLab

## Visão geral
MixLab é um aplicativo de DJ (desktop, Tauri 2 + React + Vite) com engine de áudio em Rust.  
Branch de trabalho: `fix/primeira-correcao`.

## Comandos úteis

### Rust (src-tauri)
- `cargo check` — verifica compilação
- `cargo test` — testes unitários Rust
- `cargo clippy` — linter Rust
- `cargo fmt` — formatação Rust
- `cargo tarpaulin` — cobertura de código Rust (instalar via `cargo install cargo-tarpaulin`)

### Frontend (raiz)
- `npm run dev` — servidor de desenvolvimento Vite
- `npm run build` — build de produção (typecheck + vite build)
- `npm run typecheck` — verificação de tipos TypeScript
- `npm run lint` — linter (oxlint)
- `npm run test` — testes unitários (vitest)
- `npm run test:coverage` — testes com cobertura (v8)
- `npm run test:e2e` — testes e2e (Playwright)
- `npm run check` — typecheck + lint + build

## Estrutura do projeto
```
src-tauri/
  Cargo.toml          # dependências Rust (inclui rubato, audioadapter-buffers)
  src/
    lib.rs            # ponto de entrada Tauri
    state.rs          # estado global da aplicação
    commands.rs       # comandos Tauri (load_track, set_key_lock, etc.)
    audio/
      engine.rs       # engine de áudio: render_deck, key_lock (Rubato), decks
      dsp/mod.rs      # DSP utilitário
  capabilities/
    default.json      # capabilities reduzidas (mínimo necessário)
  tauri.conf.json     # config Tauri (CSP aplicado)
src/
  App.tsx             # raiz da aplicação React
  main.tsx            # entrada Vite
  components/         # componentes React (Deck, Mixer, VU, Waveform, etc.)
  lib/                # utilitários
tests/
  e2e/
    deck-flow.spec.ts # testes e2e Playwright (fluxo crítico)
    mock-tauri.js     # mock da API Tauri para e2e sem backend Rust
.github/
  workflows/
    ci.yml            # CI: testes Rust, clippy, fmt, typecheck, build, e2e
```

## Fluxo de desenvolvimento
1. Crie uma branch a partir de `fix/primeira-correcao`.
2. Faça as alterações no código.
3. Execute `cargo check` (Rust) e `npm run check` (frontend) antes de commitar.
4. Execute `npm run test:e2e` para validar o fluxo de UI.
5. Execute `npm run test:coverage` e `cargo tarpaulin` para verificar cobertura.
6. Commit com mensagem descritiva em português do Brasil.

## Regras importantes
- Não commite segredos, API keys ou credenciais (use variáveis de ambiente).
- Mantenha o CI passando em todo push/PR.
- Novas funcionalidades devem ter testes correspondentes (unitários ou e2e).
- Cobertura mínima almejada: 80% (Rust + frontend).
- O engine de áudio usa `rubato 0.15` para time-stretching (key lock). Ao modificar `render_deck` ou `AudioEngine`, garanta que `cargo check` passe.

## Débito técnico restante (Fases 2 e 3)
- Beatgrid e sincronização visual sobre waveforms (v0.4)
- 4 decks (atualmente 2)
- MIDI controller support
- Gravação de mix
- Zoom de waveform

## Contato
Projeto: MixLab — https://github.com/alan-vieira/MixLab
