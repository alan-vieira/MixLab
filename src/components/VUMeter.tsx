import { useEffect, useRef } from 'react';

interface VUMeterProps {
  level: number; // 0.0 a 1.0
  color: 'blue' | 'red';
  height?: number;
  width?: number;
}

export function VUMeter({ level, color, height = 200, width = 16 }: VUMeterProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    if (!canvasRef.current) return;
    const canvas = canvasRef.current;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const w = canvas.width;
    const h = canvas.height;
    ctx.clearRect(0, 0, w, h);

    // Número de segmentos LED
    const segments = 20;
    const segmentHeight = h / segments;
    const gap = 2;
    const filledSegments = Math.floor(level * segments);

    for (let i = 0; i < segments; i++) {
      const y = h - (i + 1) * segmentHeight + gap / 2;
      const isFilled = i < filledSegments;
      
      // Cor baseada na posição (verde → amarelo → vermelho)
      let segmentColor: string;
      if (i < segments * 0.6) {
        segmentColor = color === 'blue' ? '#22c55e' : '#22c55e'; // Verde
      } else if (i < segments * 0.85) {
        segmentColor = '#eab308'; // Amarelo
      } else {
        segmentColor = '#ef4444'; // Vermelho
      }

      // Desenha o segmento
      ctx.fillStyle = isFilled ? segmentColor : '#1a1a2e';
      ctx.fillRect(0, y, w, segmentHeight - gap);

      // Brilho nos segmentos ativos
      if (isFilled) {
        ctx.shadowBlur = 4;
        ctx.shadowColor = segmentColor;
        ctx.fillRect(0, y, w, segmentHeight - gap);
        ctx.shadowBlur = 0;
      }
    }
  }, [level, color]);

  return (
    <canvas
      ref={canvasRef}
      width={width}
      height={height}
      className="rounded-sm"
      style={{ width: `${width}px`, height: `${height}px` }}
    />
  );
}