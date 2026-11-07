// Script injetado ANTES da aplicação carregar para mockar a API Tauri.
// Isso permite testar a UI no Vite dev server sem o backend Rust rodando.
(function () {
  const calls = [];

  const mockInvoke = async (cmd, args) => {
    calls.push({ cmd, args });
    switch (cmd) {
      case 'get_deck_status':
        return { position: 0, vu_level: 0, is_playing: false };
      case 'get_waveform_peaks':
        return Array(1000).fill(0.5);
      case 'get_waveform_layers':
        return [
          Array(1000).fill(0.3),
          Array(1000).fill(0.5),
          Array(1000).fill(0.2),
        ];
      case 'load_track':
        return { duration_secs: 240, bpm: 120 };
      case 'play_deck':
      case 'pause_deck':
      case 'set_deck_eq':
      case 'set_deck_gain':
      case 'set_crossfader':
      case 'set_deck_pitch':
      case 'set_deck_trim':
      case 'set_key_lock':
      case 'set_loop_in':
      case 'set_loop_out':
      case 'toggle_loop':
      case 'loop_reloop':
      case 'clear_loop':
      case 'set_cue_point':
      case 'jump_to_cue':
      case 'jog_scrub':
      case 'set_deck_filter':
      case 'get_vu_levels':
        return undefined;
      default:
        return undefined;
    }
  };

  // Cria a estrutura __TAURI__ esperada pelo @tauri-apps/api/core
  // antes que qualquer módulo da aplicação seja carregado.
  if (!window.__TAURI__) {
    window.__TAURI__ = {
      core: {
        invoke: mockInvoke,
      },
      event: {
        listen: async () => Promise.resolve(() => {}),
        unlisten: async () => {},
        once: async () => Promise.resolve(() => {}),
        emit: async () => {},
        emitTo: async () => {},
      },
      http: {},
      os: {},
      window: {
        getCurrentWebviewWindow: () => ({
          emit: async () => {},
          emitTo: async () => {},
          once: async () => Promise.resolve(() => {}),
          listen: async () => Promise.resolve(() => {}),
          unlisten: async () => {},
          innerText: async () => '',
          innerHTML: async () => '',
          resize: async () => {},
          maximize: async () => {},
          unmaximize: async () => {},
          minimize: async () => {},
          show: async () => {},
          hide: async () => {},
          close: async () => {},
          setFocus: async () => {},
          print: async () => {},
          setTitle: async () => {},
          setAlwaysOnTop: async () => {},
          setResizable: async () => {},
          setFullscreen: async () => {},
          setSkipTaskbar: async () => {},
          setIgnoreShadow: async () => {},
          setProgressBar: async () => {},
          setContentSecurityPolicy: async () => {},
        }),
      },
      dialog: {
        open: async () => null,
        save: async () => null,
        message: async () => undefined,
      },
    };
  }

  window.__TAURI_MOCK__ = { calls, invoke: mockInvoke };
})();

// Garante que o invoke do @tauri-apps/api/core encontre a função
if (!window.__TAURI_INTERNALS__) {
  window.__TAURI_INTERNALS__ = { invoke: mockInvoke };
}
if (typeof window.__TAURI_INTERNALS__.invoke !== 'function') {
  window.__TAURI_INTERNALS__.invoke = mockInvoke;
}
if (typeof window.__TAURI_INTERNALS__.transformCallback !== 'function') {
  window.__TAURI_INTERNALS__.transformCallback = (cb, once) => cb;
}

// Garante que o listen do @tauri-apps/api/event encontre a função
if (!window.__TAURI__.event) {
  window.__TAURI__.event = { listen: async () => Promise.resolve(() => {}) };
}
if (typeof window.__TAURI__.event.listen !== 'function') {
  window.__TAURI__.event.listen = async () => Promise.resolve(() => {});
}
