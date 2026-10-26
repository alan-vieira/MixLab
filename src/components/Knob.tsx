import { useRef, useState, useCallback, useEffect } from 'react';

interface KnobProps {
  value: number;          // Valor atual (-12 a 12 para EQ, 0 a 100 para crossfader)
  min: number;            // Valor mínimo
  max: number;            // Valor máximo
  defaultValue?: number;  // Valor padrão (0 para EQ, 50 para crossfader)
  onChange: (value: number) => void;
  label?: string;
  color?: string;         // Cor do indicador
  size?: number;          // Tamanho em pixels
}

export function Knob({ 
  value, 
  min, 
  max, 
  defaultValue = 0, 
  onChange, 
  label, 
  color = '#ffffff',
  size = 48 
}: KnobProps) {
  const knobRef = useRef<HTMLDivElement>(null);
  const [isDragging, setIsDragging] = useState(false);
  const startY = useRef(0);
  const startValue = useRef(0);

  // Converte valor para ângulo (-135° a +135°)
  const range = max - min;
  const normalizedValue = (value - min) / range; // 0 a 1
  const angle = -135 + normalizedValue * 270; // -135° a +135°

  // Handler de drag vertical
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsDragging(true);
    startY.current = e.clientY;
    startValue.current = value;
  }, [value]);

  useEffect(() => {
    if (!isDragging) return;

    const handleMouseMove = (e: MouseEvent) => {
      const deltaY = startY.current - e.clientY; // Invertido: subir = aumentar
      const sensitivity = range / 150; // Ajuste a sensibilidade
      const newValue = startValue.current + deltaY * sensitivity;
      const clampedValue = Math.max(min, Math.min(max, newValue));
      onChange(clampedValue);
    };

    const handleMouseUp = () => {
      setIsDragging(false);
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);

    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isDragging, min, max, range, onChange]);

  // Double click para resetar ao valor padrão
  const handleDoubleClick = () => {
    onChange(defaultValue);
  };

  // Calcula a posição do indicador (ponto no círculo)
  const radius = size * 0.35;
  const centerX = size / 2;
  const centerY = size / 2;
  const indicatorAngle = (angle - 90) * (Math.PI / 180); // Converte para radianos
  const indicatorX = centerX + radius * Math.cos(indicatorAngle);
  const indicatorY = centerY + radius * Math.sin(indicatorAngle);

  return (
    <div className="flex flex-col items-center gap-1">
      {label && (
        <span 
          className="text-[10px] font-bold tracking-wider"
          style={{ color }}
        >
          {label}
        </span>
      )}
      
      <div
        ref={knobRef}
        onMouseDown={handleMouseDown}
        onDoubleClick={handleDoubleClick}
        className={`relative cursor-pointer select-none ${isDragging ? 'cursor-grabbing' : 'cursor-grab'}`}
        style={{ width: size, height: size }}
        title={`${value.toFixed(1)} (duplo clique para resetar)`}
      >
        <svg width={size} height={size} className="transform">
          {/* Fundo do knob (círculo escuro) */}
          <circle
            cx={centerX}
            cy={centerY}
            r={radius + 4}
            fill="#1a1a1a"
            stroke="#333"
            strokeWidth="2"
          />
          
          {/* Anel de progresso (arco colorido) */}
          <circle
            cx={centerX}
            cy={centerY}
            r={radius + 4}
            fill="none"
            stroke={color}
            strokeWidth="3"
            strokeDasharray={`${(normalizedValue * 270 / 360) * 2 * Math.PI * (radius + 4)} ${2 * Math.PI * (radius + 4)}`}
            strokeDashoffset={-(135 / 360) * 2 * Math.PI * (radius + 4)}
            transform={`rotate(-90 ${centerX} ${centerY})`}
            opacity="0.6"
          />
          
          {/* Indicador (linha/ponto que gira) */}
          <line
            x1={centerX}
            y1={centerY}
            x2={indicatorX}
            y2={indicatorY}
            stroke={color}
            strokeWidth="2.5"
            strokeLinecap="round"
          />
          
          {/* Centro do knob */}
          <circle
            cx={centerX}
            cy={centerY}
            r={4}
            fill={color}
          />
        </svg>
      </div>
      
      {/* Valor numérico abaixo */}
      <span className="text-[9px] text-gray-400 font-mono">
        {value > 0 ? '+' : ''}{value.toFixed(0)}
      </span>
    </div>
  );
}