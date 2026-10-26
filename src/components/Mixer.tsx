import { useState, useCallback, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Knob } from './Knob';
import { VUMeter } from './VUMeter';

interface EqValues {
  low: number;
  mid: number;
  high: number;
  gain: number;
  filter: number;
}

export function Mixer() {
  const [eqA, setEqA] = useState<EqValues>({ low: 0, mid: 0, high: 0, gain: 0, filter: 50 });
  const [eqB, setEqB] = useState<EqValues>({ low: 0, mid: 0, high: 0, gain: 0, filter: 50 });
  const [crossfader, setCrossfader] = useState(50);
  
  const [vuA, setVuA] = useState(0);
  const [vuB, setVuB] = useState(0);
  const vuIntervalRef = useRef<number>();

  const handleEqChange = useCallback(async (
    deck: 'A' | 'B',
    band: 'low' | 'mid' | 'high',
    value: number
  ) => {
    const newEq = deck === 'A' ? { ...eqA } : { ...eqB };
    newEq[band] = value;
    
    if (deck === 'A') setEqA(newEq);
    else setEqB(newEq);

    try {
      await invoke('set_deck_eq', { deck, low: newEq.low, mid: newEq.mid, high: newEq.high });
    } catch (err) {
      console.error(`Erro ao ajustar EQ ${deck}:`, err);
    }
  }, [eqA, eqB]);

  const handleGainChange = useCallback(async (deck: 'A' | 'B', value: number) => {
    const newEq = deck === 'A' ? { ...eqA } : { ...eqB };
    newEq.gain = value;
    
    if (deck === 'A') setEqA(newEq);
    else setEqB(newEq);

    try {
      await invoke('set_deck_gain', { deck, gainDb: value });
    } catch (err) {
      console.error(`Erro ao ajustar gain ${deck}:`, err);
    }
  }, [eqA, eqB]);

  const handleFilterChange = useCallback(async (deck: 'A' | 'B', value: number) => {
    const newEq = deck === 'A' ? { ...eqA } : { ...eqB };
    newEq.filter = value;
    
    if (deck === 'A') setEqA(newEq);
    else setEqB(newEq);

    try {
      const position = value / 100.0;
      await invoke('set_deck_filter', { deck, position });
    } catch (err) {
      console.error(`Erro ao ajustar filtro ${deck}:`, err);
    }
  }, [eqA, eqB]);

  const handleCrossfaderChange = useCallback(async (value: number) => {
    setCrossfader(value);
    try {
      await invoke('set_crossfader', { position: value });
    } catch (err) {
      console.error('Erro ao ajustar crossfader:', err);
    }
  }, []);

  // Polling dos níveis VU a cada 50ms
  useEffect(() => {
    const pollVu = async () => {
      try {
        const levels = await invoke<[number, number]>('get_vu_levels');
        setVuA(levels[0]);
        setVuB(levels[1]);
      } catch (err) {
        // Silencioso
      }
    };

    vuIntervalRef.current = window.setInterval(pollVu, 50);
    return () => {
      if (vuIntervalRef.current) {
        window.clearInterval(vuIntervalRef.current);
      }
    };
  }, []);

  const volumeSliderClass = "w-2 h-32 appearance-none bg-gray-800 rounded-full outline-none cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-6 [&::-webkit-slider-thumb]:h-4 [&::-webkit-slider-thumb]:bg-white [&::-webkit-slider-thumb]:rounded-sm [&::-webkit-slider-thumb]:shadow-md [&::-webkit-slider-thumb]:border [&::-webkit-slider-thumb]:border-gray-600 [&::-webkit-slider-thumb]:cursor-ns-resize";

  return (
    <div className="h-full bg-gray-950 rounded-xl border border-gray-800 shadow-2xl p-3 flex flex-col">
      <h3 className="text-center text-gray-500 text-xs font-bold tracking-widest mb-3">MIXER</h3>
      
      <div className="flex-1 flex justify-between items-start gap-2 min-h-0">
        {/* Deck A - VU fino + Knobs + Volume */}
        <div className="flex items-start gap-1">
          <VUMeter level={vuA} color="blue" height={280} width={6} />
          
          <div className="flex flex-col items-center gap-2">
            <Knob value={eqA.high} min={-12} max={12} defaultValue={0} onChange={(v) => handleEqChange('A', 'high', v)} label="HI" color="#3b82f6" size={36} />
            <Knob value={eqA.mid} min={-12} max={12} defaultValue={0} onChange={(v) => handleEqChange('A', 'mid', v)} label="MID" color="#3b82f6" size={36} />
            <Knob value={eqA.low} min={-12} max={12} defaultValue={0} onChange={(v) => handleEqChange('A', 'low', v)} label="LO" color="#3b82f6" size={36} />
            <Knob value={eqA.filter} min={0} max={100} defaultValue={50} onChange={(v) => handleFilterChange('A', v)} label="FILTER" color="#f59e0b" size={36} />
            
            <div className="flex flex-col items-center gap-1 mt-2">
              <span className="text-[9px] text-blue-500 font-bold">VOL</span>
              <input 
                type="range" 
                min="-60" 
                max="12" 
                value={eqA.gain}
                onChange={(e) => handleGainChange('A', parseFloat(e.target.value))}
                className={volumeSliderClass}
                style={{ writingMode: 'vertical-lr', direction: 'rtl' }}
              />
              <span className="text-[8px] text-gray-400 font-mono">{eqA.gain > 0 ? '+' : ''}{eqA.gain.toFixed(0)}</span>
            </div>
          </div>
        </div>

        {/* Deck B - VU fino + Knobs + Volume */}
        <div className="flex items-start gap-1">
          <VUMeter level={vuB} color="red" height={280} width={6} />
          
          <div className="flex flex-col items-center gap-2">
            <Knob value={eqB.high} min={-12} max={12} defaultValue={0} onChange={(v) => handleEqChange('B', 'high', v)} label="HI" color="#ef4444" size={36} />
            <Knob value={eqB.mid} min={-12} max={12} defaultValue={0} onChange={(v) => handleEqChange('B', 'mid', v)} label="MID" color="#ef4444" size={36} />
            <Knob value={eqB.low} min={-12} max={12} defaultValue={0} onChange={(v) => handleEqChange('B', 'low', v)} label="LO" color="#ef4444" size={36} />
            <Knob value={eqB.filter} min={0} max={100} defaultValue={50} onChange={(v) => handleFilterChange('B', v)} label="FILTER" color="#f59e0b" size={36} />
            
            <div className="flex flex-col items-center gap-1 mt-2">
              <span className="text-[9px] text-red-500 font-bold">VOL</span>
              <input 
                type="range" 
                min="-60" 
                max="12" 
                value={eqB.gain}
                onChange={(e) => handleGainChange('B', parseFloat(e.target.value))}
                className={volumeSliderClass}
                style={{ writingMode: 'vertical-lr', direction: 'rtl' }}
              />
              <span className="text-[8px] text-gray-400 font-mono">{eqB.gain > 0 ? '+' : ''}{eqB.gain.toFixed(0)}</span>
            </div>
          </div>
        </div>
      </div>

      {/* Crossfader */}
      <div className="mt-4 pt-3 border-t border-gray-800 flex-shrink-0">
        <div className="flex justify-center mb-2">
          <input 
            type="range" 
            min="0" 
            max="100" 
            value={crossfader}
            onChange={(e) => handleCrossfaderChange(parseFloat(e.target.value))}
            className="w-full h-2 appearance-none bg-gray-800 rounded-full outline-none cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-5 [&::-webkit-slider-thumb]:h-5 [&::-webkit-slider-thumb]:bg-white [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:shadow-lg [&::-webkit-slider-thumb]:border-2 [&::-webkit-slider-thumb]:border-gray-700"
          />
        </div>
        <div className="flex justify-between text-[9px] text-gray-600 font-mono px-1">
          <span className="text-blue-500 font-bold">A</span>
          <span className="text-gray-400">{crossfader}</span>
          <span className="text-red-500 font-bold">B</span>
        </div>
        <div className="text-center text-gray-500 text-[8px] mt-1 tracking-wider">CROSSFADER</div>
      </div>
    </div>
  );
}