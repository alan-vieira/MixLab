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
  const [vuLevelA, setVuLevelA] = useState(0);
  const [vuLevelB, setVuLevelB] = useState(0);

  // Loop de sincronização contínuo - polling do motor Rust a cada frame
  useEffect(() => {
    let animId: number;
    let mounted = true;

    const syncWithAudioEngine = async () => {
      try {
        const [statusA, statusB] = await Promise.all([
          invoke<DeckStatus>('get_deck_status', { deck: 'A' }),
          invoke<DeckStatus>('get_deck_status', { deck: 'B' }),
        ]);

        if (!mounted) return;

        setOffsetA(statusA.position);
        setVuLevelA(statusA.vu_level);

        setOffsetB(statusB.position);
        setVuLevelB(statusB.vu_level);
      } catch (err) {
        console.error("Erro ao sincronizar com o motor de áudio:", err);
      }
      
      if (mounted) {
        animId = requestAnimationFrame(syncWithAudioEngine);
      }
    };

    animId = requestAnimationFrame(syncWithAudioEngine);
    return () => {
      mounted = false;
      cancelAnimationFrame(animId);
    };
  }, []);

  // Escuta evento de análise completa do backend
  useEffect(() => {
    const unlisten = listen<{ deck: string; bpm: number }>('track_analysis_complete', (event) => {
      if (event.payload.deck === 'A') {
        setBpmA(event.payload.bpm);
      } else if (event.payload.deck === 'B') {
        setBpmB(event.payload.bpm);
      }
    });

    return () => {
      unlisten.then(fn => fn());
    };
  }, []);

  const handlePlayA = async () => {
    await invoke('play_deck', { deck: 'A' });
  };
  
  const handlePauseA = async () => {
    await invoke('pause_deck', { deck: 'A' });
  };
  
  const handleLoadA = (layers: WaveformLayers) => {
    setLayersA(layers);
    setOffsetA(0);
  };

  const handleSeekA = async (newOffset: number) => {
    // O próximo frame de sincronização já vai pegar a nova posição do backend
    setOffsetA(newOffset);
  };

  const handlePlayB = async () => {
    await invoke('play_deck', { deck: 'B' });
  };
  
  const handlePauseB = async () => {
    await invoke('pause_deck', { deck: 'B' });
  };
  
  const handleLoadB = (layers: WaveformLayers) => {
    setLayersB(layers);
    setOffsetB(0);
  };

  const handleSeekB = async (newOffset: number) => {
    setOffsetB(newOffset);
  };

  return (
    <div className="h-screen w-screen bg-gray-950 text-gray-100 flex flex-col overflow-hidden">
      <header className="h-12 bg-gray-900 border-b border-gray-800 flex items-center px-6 justify-between shrink-0 z-10">
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 rounded-full bg-orange-500 shadow-[0_0_8px_rgba(249,115,22,0.8)]"></div>
          <h1 className="font-bold text-xl tracking-wider text-gray-200">MIX<span className="text-orange-500">LAB</span></h1>
        </div>
        <div className="text-xs text-gray-500 font-mono">
          ENGINE: RUST + CPAL | STATUS: <span className="text-green-500">ONLINE</span>
        </div>
      </header>

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

      <main className="flex-1 p-3 grid grid-cols-[1fr_200px_1fr] gap-3 min-h-0 overflow-hidden">
        <div className="min-h-0 flex flex-col">
          <Deck 
            side="A" 
            color="blue" 
            scrollOffset={offsetA} 
            onPlay={handlePlayA} 
            onPause={handlePauseA} 
            onLoad={handleLoadA}
            onSeek={handleSeekA}
            bpm={bpmA}
            vuLevel={vuLevelA}
          />
        </div>
        <div className="min-h-0">
          <Mixer />
        </div>
        <div className="min-h-0 flex flex-col">
          <Deck 
            side="B" 
            color="red" 
            scrollOffset={offsetB} 
            onPlay={handlePlayB} 
            onPause={handlePauseB} 
            onLoad={handleLoadB}
            onSeek={handleSeekB}
            bpm={bpmB}
            vuLevel={vuLevelB}
          />
        </div>
      </main>

      <footer className="h-8 bg-gray-900 border-t border-gray-800 flex items-center px-6 shrink-0">
        <span className="text-xs text-gray-500">Pronto para mixar.</span>
      </footer>
    </div>
  );
}

export default App;