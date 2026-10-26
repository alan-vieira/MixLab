import { Deck } from './components/Deck';
import { Mixer } from './components/Mixer';
import { BigWaveform } from './components/BigWaveform';
import { useState, useEffect, useRef } from 'react';
import './index.css';

interface WaveformLayers {
  peaks: number[];
  low: number[];
  mid: number[];
  high: number[];
  duration: number;
}

function App() {
  const [layersA, setLayersA] = useState<WaveformLayers>({ peaks: [], low: [], mid: [], high: [], duration: 0 });
  const [layersB, setLayersB] = useState<WaveformLayers>({ peaks: [], low: [], mid: [], high: [], duration: 0 });
  
  const [offsetA, setOffsetA] = useState(0);
  const [offsetB, setOffsetB] = useState(0);
  const [isPlayingA, setIsPlayingA] = useState(false);
  const [isPlayingB, setIsPlayingB] = useState(false);
  
  const startTimeA = useRef<number>(0);
  const startTimeB = useRef<number>(0);
  const startOffsetA = useRef<number>(0);
  const startOffsetB = useRef<number>(0);

  useEffect(() => {
    if (!isPlayingA && !isPlayingB) return;

    let animId: number;
    
    const animate = () => {
      const now = Date.now();
      
      if (isPlayingA && layersA.duration > 0) {
        const elapsed = (now - startTimeA.current) / 1000;
        const newOffset = startOffsetA.current + (elapsed / layersA.duration);
        if (newOffset >= 1.0) {
          setOffsetA(1.0);
          setIsPlayingA(false);
        } else {
          setOffsetA(newOffset);
        }
      }
      
      if (isPlayingB && layersB.duration > 0) {
        const elapsed = (now - startTimeB.current) / 1000;
        const newOffset = startOffsetB.current + (elapsed / layersB.duration);
        if (newOffset >= 1.0) {
          setOffsetB(1.0);
          setIsPlayingB(false);
        } else {
          setOffsetB(newOffset);
        }
      }
      
      animId = requestAnimationFrame(animate);
    };

    animId = requestAnimationFrame(animate);
    return () => cancelAnimationFrame(animId);
  }, [isPlayingA, isPlayingB, layersA.duration, layersB.duration]);

  const handlePlayA = () => {
    startTimeA.current = Date.now();
    startOffsetA.current = offsetA;
    setIsPlayingA(true);
  };
  
  const handlePauseA = () => setIsPlayingA(false);
  
  const handleLoadA = (layers: WaveformLayers) => {
    setLayersA(layers);
    setOffsetA(0);
    setIsPlayingA(false);
  };

  const handleSeekA = (newOffset: number) => {
    setOffsetA(newOffset);
    startOffsetA.current = newOffset;
    startTimeA.current = Date.now();
  };

  const handlePlayB = () => {
    startTimeB.current = Date.now();
    startOffsetB.current = offsetB;
    setIsPlayingB(true);
  };
  
  const handlePauseB = () => setIsPlayingB(false);
  
  const handleLoadB = (layers: WaveformLayers) => {
    setLayersB(layers);
    setOffsetB(0);
    setIsPlayingB(false);
  };

  const handleSeekB = (newOffset: number) => {
    setOffsetB(newOffset);
    startOffsetB.current = newOffset;
    startTimeB.current = Date.now();
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