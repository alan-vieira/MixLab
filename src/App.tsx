import { Deck } from './components/Deck';
import { Mixer } from './components/Mixer';
import { BigWaveform } from './components/BigWaveform';
import { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import './index.css';

interface WaveformLayers {
  peaks: number[];
  low: number[];
  mid: number[];
  high: number[];
  duration: number;
}

interface DeckStatus {
  position: number;
  vu_level: number;
  is_playing: boolean;
}

function App() {
  const [layersA, setLayersA] = useState<WaveformLayers>({ peaks: [], low: [], mid: [], high: [], duration: 0 });
  const [layersB, setLayersB] = useState<WaveformLayers>({ peaks: [], low: [], mid: [], high: [], duration: 0 });
  const [bpmA, setBpmA] = useState(0);
  const [bpmB, setBpmB] = useState(0);

  const [offsetA, setOffsetA] = useState(0);
  const [offsetB, setOffsetB] = useState(0);

  const [isPlayingA, setIsPlayingA] = useState(false);
  const [isPlayingB, setIsPlayingB] = useState(false);
  const [vuLevelA, setVuLevelA] = useState(0);
  const [vuLevelB, setVuLevelB] = useState(0);

  // Detecta se estamos rodando dentro do Tauri (não no Vite dev server puro)
  const isTauri = typeof window !== 'undefined' && !!(window as any).__TAURI__;
  const [backendConnected, setBackendConnected] = useState(false);

  // ⚡ Loop de sincronização com o motor de áudio (10fps)
  // Tenta conectar ao backend Tauri sempre; se não houver backend (dev puro no Vite),
  // falha silenciosamente e a barra permanece em 0.
  useEffect(() => {
    let intervalId: ReturnType<typeof setInterval>;
    let mounted = true;

    const syncWithAudioEngine = async () => {
      try {
        const [statusA, statusB] = await Promise.all([
          invoke<DeckStatus>('get_deck_status', { deck: 0 }),
          invoke<DeckStatus>('get_deck_status', { deck: 1 }),
        ]);
        if (!mounted) return;

        if (!backendConnected) setBackendConnected(true);

        setOffsetA(statusA.position);
        setVuLevelA(statusA.vu_level);
        if (statusA.vu_level > 0.001) console.log('[VU-DEBUG] App: Deck A vu_level=', statusA.vu_level, 'playing=', statusA.is_playing);
        setIsPlayingA(statusA.is_playing);

        setOffsetB(statusB.position);
        setVuLevelB(statusB.vu_level);
        if (statusB.vu_level > 0.001) console.log('[VU-DEBUG] App: Deck B vu_level=', statusB.vu_level, 'playing=', statusB.is_playing);
        setIsPlayingB(statusB.is_playing);
      } catch (err) {
        // Backend não disponível (dev puro no Vite) — silencioso
        if (backendConnected) setBackendConnected(false);
      }
    };

    syncWithAudioEngine();
    intervalId = setInterval(syncWithAudioEngine, 100); // 10fps
    return () => {
      mounted = false;
      clearInterval(intervalId);
    };
  }, [backendConnected]);

  // Escuta evento de análise completa do backend
  useEffect(() => {
    if (!isTauri) return;

    const unlisten = listen<{ deck: string | number; bpm: number }>('track_analysis_complete', (event) => {
      const deckLabel = typeof event.payload.deck === 'number'
        ? (event.payload.deck === 0 ? 'A' : event.payload.deck === 1 ? 'B' : event.payload.deck.toString())
        : (event.payload.deck === '0' ? 'A' : event.payload.deck === '1' ? 'B' : event.payload.deck);
      if (deckLabel === 'A') {
        setBpmA(event.payload.bpm);
      } else if (deckLabel === 'B') {
        setBpmB(event.payload.bpm);
      }
    });

    return () => {
      unlisten.then(fn => fn());
    };
  }, [isTauri]);

  const handlePlayA = async () => {
    if (!isTauri) return;
    try {
      await invoke('play_deck', { deck: 0 });
    } catch (err) {
      console.error('[App] Erro ao reproduzir Deck A:', err);
    }
  };

  const handlePauseA = async () => {
    if (!isTauri) return;
    try {
      await invoke('pause_deck', { deck: 0 });
    } catch (err) {
      console.error('[App] Erro ao pausar Deck A:', err);
    }
  };

  const handleLoadA = async (layers: WaveformLayers) => {
    setLayersA(layers);
    setOffsetA(0); // agulha inicia no início (esquerda)
  };

  const handleSeekA = async (newOffset: number) => {
    setOffsetA(newOffset);
  };

  const handlePlayB = async () => {
    if (!isTauri) return;
    try {
      await invoke('play_deck', { deck: 1 });
    } catch (err) {
      console.error('[App] Erro ao reproduzir Deck B:', err);
    }
  };

  const handlePauseB = async () => {
    if (!isTauri) return;
    try {
      await invoke('pause_deck', { deck: 1 });
    } catch (err) {
      console.error('[App] Erro ao pausar Deck B:', err);
    }
  };

  const handleLoadB = async (layers: WaveformLayers) => {
    setLayersB(layers);
    setOffsetB(0); // agulha inicia no início (esquerda)
  };

  // ⚡ Re-sincronização imediata quando o backend emite track_ended (EOF).
  // Corrige a desincronização: o Rust desliga is_playing no EOF e o frontend
  // precisava esperar o próximo poll de 100ms — agora atualiza imediatamente.
  useEffect(() => {
    if (!isTauri) return;

    const unlisten = listen<{ deck: string | number; deck_idx: number }>('track_ended', async (event) => {
      const deckIdx = typeof event.payload.deck_idx === 'number' ? event.payload.deck_idx : -1;
      if (deckIdx < 0) return;

      try {
        const status = await invoke<DeckStatus>('get_deck_status', { deck: deckIdx });
        if (deckIdx === 0) {
          setIsPlayingA(status.is_playing);
          setOffsetA(status.position);
        } else if (deckIdx === 1) {
          setIsPlayingB(status.is_playing);
          setOffsetB(status.position);
        }
        console.log('[track_ended] deck', deckIdx, 'is_playing=', status.is_playing);
      } catch (err) {
        console.error('[track_ended] Erro ao re-sincronizar deck', deckIdx, ':', err);
      }
    });

    return () => {
      unlisten.then(fn => fn());
    };
  }, [isTauri]);

  const handleSeekB = async (newOffset: number) => {
    setOffsetB(newOffset);
  };

  return (
    <div className="h-screen w-screen bg-gray-950 text-gray-100 flex flex-col overflow-hidden">
      {/* HEADER */}
      <header className="h-12 bg-gray-900 border-b border-gray-800 flex items-center px-6 justify-between shrink-0 z-10">
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 rounded-full bg-orange-500 shadow-[0_0_8px_rgba(249,115,22,0.8)]"></div>
          <h1 className="font-bold text-xl tracking-wider text-gray-200">MIX<span className="text-orange-500">LAB</span></h1>
        </div>
        <div className="text-xs text-gray-500 font-mono">
          ENGINE: RUST + CPAL | STATUS: <span className="text-green-500">ONLINE</span>
        </div>
      </header>

      {/* BIG WAVEFORMS */}
      <div className="h-32 bg-gray-950 border-b border-gray-800 flex flex-col shrink-0 z-0">
        <div className="h-1/2 border-b border-gray-800 relative bg-gray-900 overflow-hidden">
          {layersA.low.length > 0 ? (
            <BigWaveform low={layersA.low} mid={layersA.mid} high={layersA.high} scrollOffset={offsetA} deckLabel="A" />
          ) : (
            <div className="flex items-center justify-center h-full text-gray-600 text-sm">Deck A - Carregue uma faixa</div>
          )}
        </div>
        <div className="h-1/2 relative bg-gray-900 overflow-hidden">
          {layersB.low.length > 0 ? (
            <BigWaveform low={layersB.low} mid={layersB.mid} high={layersB.high} scrollOffset={offsetB} deckLabel="B" />
          ) : (
            <div className="flex items-center justify-center h-full text-gray-600 text-sm">Deck B - Carregue uma faixa</div>
          )}
        </div>
      </div>

      {/* MAIN: DECKS + MIXER */}
      <div className="flex-1 min-h-0 flex flex-col overflow-hidden">
      <main className="flex-1 p-3 grid grid-cols-[1fr_200px_1fr] gap-3 min-h-0 overflow-hidden">
                    <div className="min-h-0 h-[690px] flex flex-col">
                      <Deck
                        side="A"
                        color="blue"
                        scrollOffset={offsetA}
                        isPlaying={isPlayingA}
                        onPlay={handlePlayA}
                        onPause={handlePauseA}
                        onLoad={handleLoadA}
                        onSeek={handleSeekA}
                        bpm={bpmA}
                      />
                    </div>
                    <div className="min-h-0 h-[690px]">
                      <Mixer 
                        vuLevelA={vuLevelA}
                        vuLevelB={vuLevelB}
                      />
                    </div>
                    <div className="min-h-0 h-[690px] flex flex-col">
                      <Deck
                        side="B"
                        color="red"
                        scrollOffset={offsetB}
                        isPlaying={isPlayingB}
                        onPlay={handlePlayB}
                        onPause={handlePauseB}
                        onLoad={handleLoadB}
                        onSeek={handleSeekB}
                        bpm={bpmB}
                      />
                    </div>
                  </main>
      </div>

      {/* BIBLIOTECA (Parte 3) - container colapsado, expande via toggle */}
      <div className="bg-gray-950 border-t border-gray-800 shrink-0 overflow-hidden transition-all duration-300">
        {/* Shell da biblioteca - preenchido na Fase 2 */}
      </div>

      {/* FOOTER */}
      <footer className="h-8 bg-gray-900 border-t border-gray-800 flex items-center px-6 shrink-0">
        <span className="text-xs text-gray-500">Pronto para mixar.</span>
      </footer>
    </div>
  );
}

export default App;
