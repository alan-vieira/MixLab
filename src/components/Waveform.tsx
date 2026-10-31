import { useEffect, useRef } from 'react';

interface WaveformProps {
  peaks: number[];
  color: string;
  scrollOffset: number;
}

export function Waveform({ peaks, color, scrollOffset }: WaveformProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    if (!canvasRef.current || peaks.length === 0) return;

    const canvas = canvasRef.current;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const width = canvas.width;
    const height = canvas.height;
    const centerY = height / 2;

    ctx.clearRect(0, 0, width, height);

    // Desenha TODA a waveform completa
    const barWidth = width / peaks.length;
    ctx.fillStyle = color;
    ctx.globalAlpha = 0.7;

    peaks.forEach((peak, i) => {
      const barHeight = peak * centerY;
      const x = i * barWidth;
      const y = centerY - barHeight;
      ctx.fillRect(x, y, Math.max(1, barWidth - 0.5), barHeight * 2);
    });

    // APENAS o triângulo (sem linha)
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
  }, [peaks, color, scrollOffset]);

  return (
    <>
      <span className="sr-only">
        Waveform de áudio com {peaks.length} pontos de amostra
      </span>
      <canvas 
        ref={canvasRef} 
        width={600} 
        height={48} 
        role="img"
        aria-label="Waveform de áudio"
        className="w-full h-full"
      />
    </>
  );
}