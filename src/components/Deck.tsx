      import { Play, Pause, FolderOpen, Disc, Flag, RotateCcw } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { useState, useCallback, useRef } from 'react';
import { Waveform } from './Waveform';
import { JogWheel } from './JogWheel';

interface WaveformLayers {
  peaks: number[];
  low: number[];
  mid: number[];
  high: number[];
  duration: number;
}

interface DeckProps {
  side: 'A' | 'B';
  color: 'blue' | 'red';
  scrollOffset: number;
  isPlaying: boolean;    // ✅ Vem do App via polling
  pitch: number;         // ✅ Vem do App via polling
  onPlay: () => void;
  onPause: () => void;
  onLoad: (layers: WaveformLayers) => void;
  onSeek: (newOffset: number) => void;
  bpm: number;
  vuLevel: number;       // ✅ Vem do App via polling (para uso futuro se necessário)
}

const formatTime = (secs: number) => {
  if (!secs || isNaN(secs)) return "00:00";
  const minutes = Math.floor(secs / 60);
  const seconds = Math.floor(secs % 60);
  return `${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`;
};

export function Deck({ side, color, scrollOffset, isPlaying, pitch, onPlay, onPause, onLoad, onSeek, bpm, vuLevel }: DeckProps) {
  const [trackName, setTrackName] = useState('Nenhuma faixa carregada');
  const [duration, setDuration] = useState(0);
  const [loading, setLoading] = useState(false);
  const [waveformPeaks, setWaveformPeaks] = useState<number[]>([]);
  const [cueSet, setCueSet] = useState(false);

  // ❌ REMOVIDO: const [isPlaying, setIsPlaying] = useState(false);
  // ❌ REMOVIDO: const [pitch, setPitch] = useState(1.0);

  const accentColor = color === 'blue' ? 'text-blue-500 border-blue-500/30 bg-blue-500/10' : 'text-red-500 border-red-500/30 bg-red-500/10';
  const btnColor = color === 'blue' ? 'bg-blue-600 hover:bg-blue-500' : 'bg-red-600 hover:bg-red-500';
  const waveformColor = color === 'blue' ? 'rgba(59, 130, 246, 0.7)' : 'rgba(239, 68, 68, 0.7)';

  const currentBpm = (bpm * pitch).toFixed(1);

  const handleLoad = async () => {
    setLoading(true);
    try {
      const selected = await open({
        title: `Selecionar música para Deck ${side}`,
        multiple: false,
        filters: [{ name: 'Audio', extensions: ['mp3', 'wav', 'flac', 'ogg', 'm4a'] }]
      });

      if (selected === null) {
        setLoading(false);
        return;
      }

      const filePath = selected.toString();
      const fileName = filePath.split('\\').pop() || filePath.split('/').pop() || 'Faixa desconhecida';
      
      const trackInfo: any = await invoke('load_track', { deck: side, path: filePath });
      
      setTrackName(fileName);
      setDuration(trackInfo.duration_secs);
      // ❌ REMOVIDO: setPitch(1.0); - o backend já reseta o pitch ao carregar
      setCueSet(false);

      const peaks = trackInfo.peaks || [];
      const low = trackInfo.waveform_low || [];
      const mid = trackInfo.waveform_mid || [];

      const high = trackInfo.waveform_high || [];


      setWaveformPeaks(peaks);
      onLoad({ peaks, low, mid, high, duration: trackInfo.duration_secs });
      
      setLoading(false);

    } catch (err) {
      console.error("Erro ao carregar:", err);
      setTrackName('Erro ao carregar');
      setLoading(false);
    }
  };

  const handlePlayPause = async () => {
    if (isPlaying) {
      await invoke('pause_deck', { deck: side });
      // ❌ REMOVIDO: setIsPlaying(false) - o polling do App atualiza
      onPause();
    } else {
      await invoke('play_deck', { deck: side });
      //  REMOVIDO: setIsPlaying(true) - o polling do App atualiza
      onPlay();
    }
  };

  const handleCue = async () => {
    if (cueSet) {
      await invoke('jump_to_cue', { deck: side });
      await invoke('pause_deck', { deck: side });
      // ❌ REMOVIDO: setIsPlaying(false)
      onPause();
      onSeek(0);
    } else {
      await invoke('set_cue_point', { deck: side });
      setCueSet(true);
    }
  };

  const handlePitchChange = (value: number) => {
    // Debounce: evita saturar o IPC Tauri com chamadas repetidas durante o arrasto do slider
    if (pitchDebounceRef.current) {
      clearTimeout(pitchDebounceRef.current);
    }
    pitchDebounceRef.current = setTimeout(() => {
      void invoke('set_deck_pitch', { deck: side, pitch: value }).catch((err) =>
        console.error("Erro ao ajustar pitch:", err)
      );
    }, 150);
  };

  const pitchDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleJogRotate = useCallback(async (delta: number) => {
    try {
      const secondsDelta = (delta / 360) * 2;
      const offsetDelta = secondsDelta / duration;
      const newOffset = Math.max(0, Math.min(1, scrollOffset + offsetDelta));
      await invoke('jog_scrub', { deck: side, delta: delta / 180 });
      onSeek(newOffset);
    } catch (err) {
      console.error("Erro no jog:", err);
    }
  }, [side, scrollOffset, duration, onSeek]);

  const wasPlayingBeforeJog = useRef(false);

  const handleJogHold = useCallback(async (isHolding: boolean) => {
    if (isHolding) {
      wasPlayingBeforeJog.current = isPlaying;
      await invoke('pause_deck', { deck: side });
      // ❌ REMOVIDO: setIsPlaying(false)
      onPause();
    } else {
      if (wasPlayingBeforeJog.current) {
        await invoke('play_deck', { deck: side });
        // ❌ REMOVIDO: setIsPlaying(true)
        onPlay();
      }
    }
  }, [side, isPlaying, onPlay, onPause]);

  const elapsedTime = scrollOffset * duration;
  const pitchPercentage = Math.round((pitch - 1.0) * 100);
  const pitchDisplay = pitchPercentage > 0 ? `+${pitchPercentage}%` : `${pitchPercentage}%`;

  return (
    <div className="flex flex-col h-full bg-gray-900 rounded-xl border border-gray-800 shadow-2xl overflow-hidden">
      {/* CABEÇALHO */}
      <div className={`flex items-center justify-between px-3 border-b border-gray-800 ${accentColor}`} style={{ height: '48px' }}>
        <div className="flex items-center gap-2 flex-1 min-w-0">
          <Disc className="w-4 h-4 animate-[spin_4s_linear_infinite] flex-shrink-0" style={{ animationPlayState: isPlaying ? 'running' : 'paused' }} />
          <span className="font-bold text-sm tracking-wider flex-shrink-0">DECK {side}</span>
          <span className="text-xs text-gray-400 truncate" title={trackName}>{trackName}</span>
        </div>
        
        <div className="flex flex-col items-end flex-shrink-0 ml-2">
          <div className="font-mono text-sm font-bold text-white">
            {bpm > 0 ? currentBpm : '---'} <span className="text-[10px] text-gray-400">BPM</span>
          </div>
          <div className="font-mono text-xs text-orange-400 font-bold">
            {formatTime(elapsedTime)} <span className="text-gray-500">/ {formatTime(duration)}</span>
          </div>
        </div>
      </div>

      {/* MINI WAVEFORM */}
      <div className="bg-gray-950 border-b border-gray-800 relative overflow-hidden flex-shrink-0" style={{ height: '60px' }}>
        {waveformPeaks.length > 0 && (
          <Waveform peaks={waveformPeaks} color={waveformColor} scrollOffset={scrollOffset} />
        )}
      </div>

      {/* ÁREA FLEXÍVEL */}
      <div className="flex-1 bg-gray-900 min-h-0"></div>

      {/* CONTROLES INFERIORES */}
      <div className="bg-gray-900 border-t border-gray-800 flex-shrink-0 p-3">
        <div className="flex items-center gap-3">
          
          {/* Jog Wheel + Pitch Fader (SEM VU METER - conforme solicitado) */}
          <div className="flex items-center gap-2 flex-shrink-0">
            <JogWheel 
              color={color} 
              isPlaying={isPlaying}
              onJogRotate={handleJogRotate}
              onJogHold={handleJogHold}
            />
            
            {/* Pitch Fader */}
            <div className="flex flex-col items-center justify-center w-10 bg-gray-950 rounded border border-gray-800 py-2" style={{ height: '140px' }}>
              <span className="text-[8px] text-gray-500 font-bold mb-1">PITCH</span>
              <div className="relative flex items-center justify-center flex-1" style={{ width: '24px' }}>
                <input 
                  type="range" 
                  min="0.8" 
                  max="1.2" 
                  step="0.01"
                  value={pitch}
                  onChange={(e) => handlePitchChange(parseFloat(e.target.value))}
                  className="w-24 h-1 appearance-none bg-gray-800 rounded-full outline-none cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-3 [&::-webkit-slider-thumb]:h-4 [&::-webkit-slider-thumb]:bg-orange-500 [&::-webkit-slider-thumb]:rounded-sm [&::-webkit-slider-thumb]:shadow-md [&::-webkit-slider-thumb]:cursor-ew-resize [&::-webkit-slider-thumb]:border [&::-webkit-slider-thumb]:border-orange-400 rotate-[-90deg]"
                />
              </div>
              <span className="text-[9px] text-orange-400 font-mono font-bold mt-1">{pitchDisplay}</span>
            </div>
          </div>

          {/* Botões de controle */}
          <div className="flex-1 flex flex-col gap-2">
            <div className="grid grid-cols-2 gap-2">
              <button 
                onClick={handleLoad}
                disabled={loading}
                className="flex items-center justify-center gap-2 bg-gray-800 hover:bg-gray-700 disabled:bg-gray-800/50 text-gray-200 rounded-lg font-semibold transition-colors border border-gray-700 text-sm"
                style={{ height: '40px' }}
              >
                <FolderOpen className="w-4 h-4" />
                {loading ? '...' : 'LOAD'}
              </button>
              
              <button 
                onClick={handlePlayPause}
                disabled={!waveformPeaks.length}
                className={`flex items-center justify-center gap-2 ${btnColor} text-white rounded-lg font-bold transition-all shadow-lg active:scale-95 disabled:opacity-50 disabled:cursor-not-allowed text-sm`}
                style={{ height: '40px' }}
              >
                {isPlaying ? <Pause className="w-4 h-4 fill-current" /> : <Play className="w-4 h-4 fill-current" />}
                {isPlaying ? 'PAUSE' : 'PLAY'}
              </button>
            </div>

            <button
              onClick={handleCue}
              disabled={!waveformPeaks.length}
              className={`flex items-center justify-center gap-2 rounded-lg font-bold transition-all shadow-lg active:scale-95 disabled:opacity-50 disabled:cursor-not-allowed text-sm ${cueSet ? 'bg-orange-600 hover:bg-orange-500 text-white' : 'bg-gray-800 hover:bg-gray-700 text-gray-200 border border-gray-700'}`}
              style={{ height: '40px' }}
            >
              {cueSet ? <RotateCcw className="w-4 h-4" /> : <Flag className="w-4 h-4" />}
              {cueSet ? 'CUE (retorno)' : 'CUE (marcar)'}
            </button>
          </div>

        </div>
      </div>
    </div>
  );
}