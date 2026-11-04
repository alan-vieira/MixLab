import { useEffect, useRef } from 'react';

interface VUMeterProps {
  level: number; // 0.0 a 1.0
  color: 'blue' | 'red';
  height?: number;
  width?: number;
}

export function VUMeter({ level, color, height = 280, width = 6 }: VUMeterProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const levelRef = useRef(level);

  // Mantém o levelRef sempre atualizado
  levelRef.current = level;

  // Loop de desenho contínuo (60fps) — garante que o VU reaja a qualquer mudança
  useEffect(() => {
    if (!canvasRef.current) return;
    const canvas = canvasRef.current;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    canvas.style.width = `${width}px`;
    canvas.style.height = `${height}px`;
    ctx.scale(dpr, dpr);

    const w = width;
    const h = height;
    const segments = 20;
    const segmentHeight = h / segments;
    const gap = 2;

    const draw = () => {
      const currentLevel = levelRef.current;
      ctx.clearRect(0, 0, w, h);

      const filledSegments = Math.floor(currentLevel * segments);

      for (let i = 0; i < segments; i++) {
        const y = h - (i + 1) * segmentHeight + gap / 2;
        const isFilled = i < filledSegments;

        let segmentColor: string;
        if (i < segments * 0.6) {
          segmentColor = '#22c55e'; // Verde
        } else if (i < segments * 0.85) {
          segmentColor = '#eab308'; // Amarelo
        } else {
          segmentColor = '#ef4444'; // Vermelho
        }

        ctx.fillStyle = isFilled ? segmentColor : '#1a1a2e';
        ctx.fillRect(1, y, w - 2, segmentHeight - gap);

        if (isFilled) {
          ctx.shadowBlur = 6;
          ctx.shadowColor = segmentColor;
          ctx.fillRect(1, y, w - 2, segmentHeight - gap);
          ctx.shadowBlur = 0;
        }
      }
    };

    draw();
    const raf = requestAnimationFrame(function loop() {
      draw();
      requestAnimationFrame(loop);
    });
    return () => cancelAnimationFrame(raf);
  }, [level, color, width, height]);

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