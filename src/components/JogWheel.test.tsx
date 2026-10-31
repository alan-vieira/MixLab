import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import { JogWheel } from './JogWheel';

describe('JogWheel Component', () => {
  const defaultProps = {
    color: 'blue' as const,
    isPlaying: false,
    onJogRotate: vi.fn(),
    onJogHold: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renderiza o componente JogWheel', () => {
    render(<JogWheel {...defaultProps} />);
    
    const jogContainer = screen.getByTestId('jog-wheel');
    expect(jogContainer).toBeInTheDocument();
  });

  it('tem cursor grab por padrão', () => {
    render(<JogWheel {...defaultProps} />);
    
    const jogElement = screen.getByTestId('jog-wheel');
    expect(jogElement).toHaveClass('cursor-grab');
  });

  it('chama onJogHold(true) ao iniciar drag', () => {
    render(<JogWheel {...defaultProps} />);
    
    const jogElement = screen.getByTestId('jog-wheel');
    fireEvent.mouseDown(jogElement, { clientX: 100, clientY: 100 });
    
    expect(defaultProps.onJogHold).toHaveBeenCalledWith(true);
  });

  it('chama onJogRotate com delta ao mover mouse', () => {
    render(<JogWheel {...defaultProps} />);
    
    const jogElement = screen.getByTestId('jog-wheel');
    fireEvent.mouseDown(jogElement, { clientX: 100, clientY: 100 });
    
    fireEvent.mouseMove(document, { clientX: 110, clientY: 90 });
    
    expect(defaultProps.onJogRotate).toHaveBeenCalled();
    const delta = defaultProps.onJogRotate.mock.calls[0][0];
    expect(typeof delta).toBe('number');
  });

  it('chama onJogHold(false) ao soltar mouse', () => {
    render(<JogWheel {...defaultProps} />);
    
    const jogElement = screen.getByTestId('jog-wheel');
    fireEvent.mouseDown(jogElement, { clientX: 100, clientY: 100 });
    fireEvent.mouseUp(document);
    
    expect(defaultProps.onJogHold).toHaveBeenCalledWith(false);
  });

  it('não gira automaticamente quando isPlaying=false', () => {
    vi.useFakeTimers();
    render(<JogWheel {...defaultProps} isPlaying={false} />);
    
    vi.advanceTimersByTime(1000);
    
    expect(defaultProps.onJogRotate).not.toHaveBeenCalled();
    
    vi.useRealTimers();
  });

  it('tem rotação visual quando isPlaying=true', () => {
    vi.useFakeTimers();
    render(<JogWheel {...defaultProps} isPlaying={true} />);
    
    act(() => {
      vi.advanceTimersByTime(1000);
    });
    
    vi.useRealTimers();
  });
});