import { useRef, useState, useCallback, useEffect } from 'react';

interface JogWheelProps {
  color: 'blue' | 'red';
  isPlaying: boolean;
  onJogRotate: (delta: number) => void;
  onJogHold: (isHolding: boolean) => void;
}

export function JogWheel({ color, isPlaying, onJogRotate, onJogHold }: JogWheelProps) {
  const jogRef = useRef<HTMLDivElement>(null);
  const [rotation, setRotation] = useState(0);
  const [isDragging, setIsDragging] = useState(false);
  const lastAngle = useRef(0);
  const rotationRef = useRef(0);
  const animationRef = useRef<ReturnType<typeof requestAnimationFrame> | null>(null);
  const lastTimeRef = useRef<number>(0);

  const ringColor = color === 'blue' ? '#3b82f6' : '#ef4444';
  const accentColor = color === 'blue' ? 'rgba(59, 130, 246, 0.3)' : 'rgba(239, 68, 68, 0.3)';

  // Velocidade base: 360° em 4 segundos = 90°/segundo
  const ROTATION_SPEED = 90; // graus por segundo

  const getAngle = useCallback((e: MouseEvent) => {
    if (!jogRef.current) return 0;
    const rect = jogRef.current.getBoundingClientRect();
    const centerX = rect.left + rect.width / 2;
    const centerY = rect.top + rect.height / 2;
    return Math.atan2(e.clientY - centerY, e.clientX - centerX) * (180 / Math.PI);
  }, []);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsDragging(true);
    onJogHold(true);
    const angle = getAngle(e.nativeEvent);
    lastAngle.current = angle;
  }, [getAngle, onJogHold]);

  useEffect(() => {
    if (!isDragging) return;

    const handleMouseMove = (e: MouseEvent) => {
      const currentAngle = getAngle(e);
      let delta = currentAngle - lastAngle.current;
      if (delta > 180) delta -= 360;
      if (delta < -180) delta += 360;
      rotationRef.current += delta;
      setRotation(rotationRef.current);
      onJogRotate(delta);
      lastAngle.current = currentAngle;
    };

    const handleMouseUp = () => {
      setIsDragging(false);
      onJogHold(false);
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);

    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isDragging, getAngle, onJogRotate, onJogHold]);

  // ⚡ Rotação automática baseada em DELTA TIME (consistente em qualquer framerate)
  useEffect(() => {
    if (isDragging) {
      if (animationRef.current) {
        cancelAnimationFrame(animationRef.current);
        animationRef.current = null;
      }
      return;
    }

    if (!isPlaying) {
      if (animationRef.current) {
        cancelAnimationFrame(animationRef.current);
        animationRef.current = null;
      }
      return;
    }

    lastTimeRef.current = performance.now();

    const animate = (currentTime: number) => {
      // ⚡ Calcula delta time em segundos
      const deltaTime = (currentTime - lastTimeRef.current) / 1000;
      lastTimeRef.current = currentTime;

      // Limita delta para evitar "pulos" quando a aba volta do background
      const clampedDelta = Math.min(deltaTime, 0.1);

      rotationRef.current += ROTATION_SPEED * clampedDelta;
      setRotation(rotationRef.current);
      animationRef.current = requestAnimationFrame(animate);
    };

    animationRef.current = requestAnimationFrame(animate);

    return () => {
      if (animationRef.current) {
        cancelAnimationFrame(animationRef.current);
        animationRef.current = null;
      }
    };
  }, [isPlaying, isDragging]);

  return (
      <div className="flex flex-col items-center gap-2">
        <div
          ref={jogRef}
          onMouseDown={handleMouseDown}
          data-testid="jog-wheel"
          className={`relative cursor-pointer select-none rounded-full ${isDragging ? 'cursor-grabbing' : 'cursor-grab'}`}
          style={{ width: '160px', height: '160px' }}
        >
          <svg width="160" height="160" className="transform">
            <defs>
              <radialGradient id={`jogGrad-${color}`} cx="50%" cy="50%" r="50%">
                <stop offset="0%" stopColor="#1a1a2e" />
                <stop offset="70%" stopColor="#0f0f1a" />
                <stop offset="100%" stopColor="#2a2a3e" />
              </radialGradient>
              <radialGradient id={`jogInner-${color}`} cx="50%" cy="50%" r="50%">
                <stop offset="0%" stopColor="#2a2a3e" />
                <stop offset="100%" stopColor="#0f0f1a" />
              </radialGradient>
            </defs>

            {/* Base do jog */}
            <circle cx="80" cy="80" r="76" fill={`url(#jogGrad-${color})`} stroke="#333" strokeWidth="1" />

            {/* Anel de LEDs */}
            <circle
              cx="80"
              cy="80"
              r="72"
              fill="none"
              stroke={ringColor}
              strokeWidth="2"
              opacity="0.4"
            />

            {/* Disco interno giratório */}
            <g transform={`rotate(${rotation} 80 80)`}>
              <circle cx="80" cy="80" r="62" fill={`url(#jogInner-${color})`} stroke="#222" strokeWidth="1" />

              {/* Linhas radiais */}
              {Array.from({ length: 8 }).map((_, i) => {
                const angle = (i * 45) * (Math.PI / 180);
                const x1 = 80 + 25 * Math.cos(angle);
                const y1 = 80 + 25 * Math.sin(angle);
                const x2 = 80 + 55 * Math.cos(angle);
                const y2 = 80 + 55 * Math.sin(angle);
                return (
                  <line
                    key={i}
                    x1={x1}
                    y1={y1}
                    x2={x2}
                    y2={y2}
                    stroke="#444"
                    strokeWidth="1"
                    opacity="0.6"
                  />
                );
              })}

              {/* Marcador de posição */}
              <circle cx="80" cy="22" r="3" fill="white" opacity="0.8" />
            </g>

            {/* Centro do jog (fixo) */}
            <circle cx="80" cy="80" r="18" fill="#1a1a2e" stroke={ringColor} strokeWidth="2" opacity="0.6" />
            <circle cx="80" cy="80" r="8" fill={accentColor} />
          </svg>
        </div>
      </div>
    );
}