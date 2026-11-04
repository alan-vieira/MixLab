import { useEffect, useRef } from 'react';

interface WaveformProps {
  peaks: number[];
  color: string;
  scrollOffset: number;
}

export function Waveform({ peaks, color, scrollOffset }: WaveformProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!canvasRef.current || !containerRef.current || peaks.length === 0) return;

    const canvas = canvasRef.current;
    const container = containerRef.current;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // === SUPORTE A TELAS RETINA / HIGH-DPI ===
    const dpr = window.devicePixelRatio || 1;
    const rect = container.getBoundingClientRect();
    canvas.width = rect.width * dpr;
    canvas.height = rect.height * dpr;
    canvas.style.width = `${rect.width}px`;
    canvas.style.height = `${rect.height}px`;
    ctx.scale(dpr, dpr);

    const width = rect.width;
    const height = rect.height;
    const centerY = height / 2;

    ctx.clearRect(0, 0, width, height);

    // Desenha a waveform COMPLETA e parada (todas as barras, sem janela deslizante)
    // A onda carrega completa e fica parada; a agulha é que se move da esquerda para a direita.
    const barWidth = width / peaks.length;
    ctx.fillStyle = color;
    ctx.globalAlpha = 0.7;

    for (let i = 0; i < peaks.length; i++) {
      const peak = peaks[i];
      const barHeight = peak * centerY;
      const x = i * barWidth;
      const y = centerY - barHeight;
      ctx.fillRect(x, y, Math.max(1, barWidth - 0.5), barHeight * 2);
    }

    // Agulha se move da esquerda (scrollOffset=0) para a direita (scrollOffset=1)
    const needleX = scrollOffset * width;
    ctx.globalAlpha = 1.0;
    ctx.fillStyle = '#f97316';
    ctx.shadowBlur = 6;
    ctx.shadowColor = '#f97316';

    ctx.beginPath();
    ctx.moveTo(needleX, 12);
    ctx.lineTo(needleX - 5, 4);
    ctx.lineTo(needleX + 5, 4);
    ctx.closePath();
    ctx.fill();
    ctx.shadowBlur = 0;

    // Linha branca marcando a posição da agulha, começando da ponta do triângulo
    ctx.strokeStyle = '#ffffff';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(needleX, 12); // ponta do triângulo
    ctx.lineTo(needleX, height);
    ctx.stroke();
  }, [peaks, color, scrollOffset]);

  return (
    <div ref={containerRef} className="w-full h-full">
      <canvas
        ref={canvasRef}
        className="w-full h-full"
      />
    </div>
  );
}