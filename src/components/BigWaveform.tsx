import { useEffect, useRef } from 'react';

interface BigWaveformProps {
  low: number[];    // Graves
  mid: number[];    // Médios
  high: number[];   // Agudos
  scrollOffset: number;
  deckLabel: string;
}

export function BigWaveform({ low, mid, high, scrollOffset, deckLabel }: BigWaveformProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    if (!canvasRef.current || low.length === 0) return;

    const canvas = canvasRef.current;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const width = canvas.width;
    const height = canvas.height;
    const centerY = height / 2;

    ctx.clearRect(0, 0, width, height);

    // Fundo escuro
    ctx.fillStyle = '#0a0a0a';
    ctx.fillRect(0, 0, width, height);

    const visiblePeaks = 200;
    const currentPeak = Math.floor(scrollOffset * low.length);
    const startPeak = currentPeak - Math.floor(visiblePeaks / 2);
    const barWidth = width / visiblePeaks;

    // === GRID VERTICAL ===
    ctx.strokeStyle = '#1a1a1a';
    ctx.lineWidth = 1;
    const gridSpacing = width / 8;
    for (let x = 0; x < width; x += gridSpacing) {
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x, height);
      ctx.stroke();
    }

    // === LINHA HORIZONTAL CENTRAL ===
    ctx.strokeStyle = '#333333';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, centerY);
    ctx.lineTo(width, centerY);
    ctx.stroke();

    // Helper para desenhar uma camada
    const drawLayer = (data: number[], color: string, alpha: number) => {
      ctx.fillStyle = color;
      ctx.globalAlpha = alpha;

      for (let i = 0; i < visiblePeaks; i++) {
        const peakIndex = startPeak + i;
        if (peakIndex >= 0 && peakIndex < data.length) {
          const peak = data[peakIndex];
          const barHeight = peak * centerY * 0.95;
          const x = i * barWidth;
          
          // Simétrico (cima e baixo)
          ctx.fillRect(x, centerY - barHeight, Math.max(1, barWidth - 0.5), barHeight);
          ctx.fillRect(x, centerY, Math.max(1, barWidth - 0.5), barHeight);
        }
      }
      ctx.globalAlpha = 1.0;
    };

    // Desenha as 3 camadas sobrepostas (ordem importa!)
    drawLayer(low, '#f97316', 0.9);   // Laranja - Graves (BUMBO)
    drawLayer(mid, '#22d3ee', 0.7);   // Ciano - Médios (VOZ)
    drawLayer(high, '#a78bfa', 0.5);  // Roxo - Agudos (PRATOS)

    // === AGULHA ===
    ctx.strokeStyle = '#f97316';
    ctx.lineWidth = 2;
    ctx.shadowBlur = 8;
    ctx.shadowColor = '#f97316';
    ctx.beginPath();
    ctx.moveTo(width / 2, 0);
    ctx.lineTo(width / 2, height);
    ctx.stroke();
    ctx.shadowBlur = 0;

    // Triângulo embaixo
    ctx.fillStyle = '#f97316';
    ctx.beginPath();
    ctx.moveTo(width / 2, height - 5);
    ctx.lineTo(width / 2 - 6, height);
    ctx.lineTo(width / 2 + 6, height);
    ctx.closePath();
    ctx.fill();

    // Etiqueta
    const bars = ((scrollOffset * low.length) / (low.length / 16)).toFixed(1);
    ctx.fillStyle = '#60a5fa';
    ctx.font = 'bold 11px monospace';
    ctx.textAlign = 'center';
    ctx.fillText(`${bars}Bars ${deckLabel}`, width / 2, 14);
    ctx.textAlign = 'left';

  }, [low, mid, high, scrollOffset, deckLabel]);

  return (
    <canvas 
      ref={canvasRef} 
      width={1600} 
      height={140} 
      className="w-full h-full"
    />
  );
}