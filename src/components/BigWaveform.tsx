import { useEffect, useRef } from 'react';

interface BigWaveformProps {
  low: number[];
  mid: number[];
  high: number[];
  scrollOffset: number;
  deckLabel: string;
}

export function BigWaveform({ low, mid, high, scrollOffset, deckLabel }: BigWaveformProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const lastOffsetRef = useRef<number>(-1);

  useEffect(() => {
      if (!canvasRef.current || !containerRef.current || low.length === 0) return;

      const canvas = canvasRef.current;
      const container = containerRef.current;
      const ctx = canvas.getContext('2d');
      if (!ctx) return;

      // ⚡ Sem threshold - redesenha sempre que scrollOffset mudar
      // (jog reverso precisa de precisão sub-pixel)
      lastOffsetRef.current = scrollOffset;

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

    // Anti-aliasing suave
    ctx.imageSmoothingEnabled = true;
    ctx.imageSmoothingQuality = 'high';

    // === FUNDO ===
    ctx.clearRect(0, 0, width, height);
    const bgGradient = ctx.createLinearGradient(0, 0, 0, height);
    bgGradient.addColorStop(0, '#0a0a0f');
    bgGradient.addColorStop(0.5, '#0f0f15');
    bgGradient.addColorStop(1, '#0a0a0f');
    ctx.fillStyle = bgGradient;
    ctx.fillRect(0, 0, width, height);

    const visiblePeaks = Math.max(100, Math.floor(width / 3));
    const currentPeak = Math.floor(scrollOffset * low.length);
    const startPeak = currentPeak - Math.floor(visiblePeaks / 2);
    const barWidth = width / visiblePeaks;
    const barGap = Math.max(0.5, barWidth * 0.15);
    const drawWidth = barWidth - barGap;
    const cornerRadius = Math.min(drawWidth * 0.5, 2.5);

    // === GRID VERTICAL SUTIL ===
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.04)';
    ctx.lineWidth = 1;
    const gridSpacing = width / 16;
    for (let x = 0; x < width; x += gridSpacing) {
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x, height);
      ctx.stroke();
    }

    // === LINHA HORIZONTAL CENTRAL ===
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.08)';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, centerY);
    ctx.lineTo(width, centerY);
    ctx.stroke();

    // === HELPER: DESENHA UMA CAMADA COM GRADIENTE E BARRAS ARREDONDADAS ===
    const drawLayer = (
      data: number[],
      baseColor: [number, number, number],
      alpha: number,
      intensity: number
    ) => {
      for (let i = 0; i < visiblePeaks; i++) {
        const peakIndex = startPeak + i;
        if (peakIndex < 0 || peakIndex >= data.length) continue;

        const peak = Math.min(1, Math.max(0, data[peakIndex]));
        if (peak < 0.01) continue; // ⚡ Skip barras quase invisíveis

        const barHeight = peak * centerY * 0.92 * intensity;
        const x = i * barWidth + barGap / 2;

        const topY = centerY - barHeight;
        const bottomY = centerY + barHeight;

        // Gradiente superior
        const gradTop = ctx.createLinearGradient(0, topY, 0, centerY);
        gradTop.addColorStop(0, `rgba(${baseColor[0]}, ${baseColor[1]}, ${baseColor[2]}, ${alpha * 0.3})`);
        gradTop.addColorStop(0.5, `rgba(${baseColor[0]}, ${baseColor[1]}, ${baseColor[2]}, ${alpha * 0.7})`);
        gradTop.addColorStop(1, `rgba(${baseColor[0]}, ${baseColor[1]}, ${baseColor[2]}, ${alpha})`);

        // Gradiente inferior
        const gradBottom = ctx.createLinearGradient(0, centerY, 0, bottomY);
        gradBottom.addColorStop(0, `rgba(${baseColor[0]}, ${baseColor[1]}, ${baseColor[2]}, ${alpha})`);
        gradBottom.addColorStop(0.5, `rgba(${baseColor[0]}, ${baseColor[1]}, ${baseColor[2]}, ${alpha * 0.7})`);
        gradBottom.addColorStop(1, `rgba(${baseColor[0]}, ${baseColor[1]}, ${baseColor[2]}, ${alpha * 0.3})`);

        // Barra superior arredondada
        ctx.fillStyle = gradTop;
        if (cornerRadius > 0 && drawWidth > cornerRadius * 2) {
          ctx.beginPath();
          ctx.moveTo(x, centerY);
          ctx.lineTo(x, topY + cornerRadius);
          ctx.quadraticCurveTo(x, topY, x + cornerRadius, topY);
          ctx.lineTo(x + drawWidth - cornerRadius, topY);
          ctx.quadraticCurveTo(x + drawWidth, topY, x + drawWidth, topY + cornerRadius);
          ctx.lineTo(x + drawWidth, centerY);
          ctx.closePath();
          ctx.fill();
        } else {
          ctx.fillRect(x, topY, drawWidth, barHeight);
        }

        // Barra inferior arredondada
        ctx.fillStyle = gradBottom;
        if (cornerRadius > 0 && drawWidth > cornerRadius * 2) {
          ctx.beginPath();
          ctx.moveTo(x, centerY);
          ctx.lineTo(x, bottomY - cornerRadius);
          ctx.quadraticCurveTo(x, bottomY, x + cornerRadius, bottomY);
          ctx.lineTo(x + drawWidth - cornerRadius, bottomY);
          ctx.quadraticCurveTo(x + drawWidth, bottomY, x + drawWidth, bottomY - cornerRadius);
          ctx.lineTo(x + drawWidth, centerY);
          ctx.closePath();
          ctx.fill();
        } else {
          ctx.fillRect(x, centerY, drawWidth, barHeight);
        }
      }
    };

    // === DESENHA AS 3 CAMADAS ===
    drawLayer(low, [249, 115, 22], 0.85, 1.0);
    drawLayer(mid, [34, 211, 238], 0.6, 0.95);
    drawLayer(high, [167, 139, 250], 0.45, 0.9);

    // === AGULHA ELEGANTE ===
    const needleX = width / 2;

    ctx.shadowBlur = 12;
    ctx.shadowColor = '#f97316';
    ctx.strokeStyle = '#f97316';
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(needleX, 0);
    ctx.lineTo(needleX, height);
    ctx.stroke();
    ctx.shadowBlur = 0;

    // Triângulo superior
    ctx.fillStyle = '#f97316';
    ctx.beginPath();
    ctx.moveTo(needleX, 5);
    ctx.lineTo(needleX - 6, 0);
    ctx.lineTo(needleX + 6, 0);
    ctx.closePath();
    ctx.fill();

    // Triângulo inferior
    ctx.beginPath();
    ctx.moveTo(needleX, height - 5);
    ctx.lineTo(needleX - 6, height);
    ctx.lineTo(needleX + 6, height);
    ctx.closePath();
    ctx.fill();

    // === ETIQUETA DE BARRAS ===
    const bars = ((scrollOffset * low.length) / (low.length / 16)).toFixed(1);
    ctx.fillStyle = 'rgba(96, 165, 250, 0.9)';
    ctx.font = 'bold 11px monospace';
    ctx.textAlign = 'center';
    ctx.fillText(`${bars} Bars ${deckLabel}`, needleX, 14);
    ctx.textAlign = 'left';
  }, [low, mid, high, scrollOffset, deckLabel]);

  return (
    <div ref={containerRef} className="w-full h-full">
      <canvas
        ref={canvasRef}
        className="block w-full h-full"
      />
    </div>
  );
}