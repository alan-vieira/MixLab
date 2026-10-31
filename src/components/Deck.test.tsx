import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Deck } from './Deck';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

const mockInvoke = vi.mocked(invoke);
const mockOpen = vi.mocked(open);

describe('Deck Component', () => {
  const defaultProps = {
      side: 'A' as const,
      color: 'blue' as const,
      scrollOffset: 0.5,
      isPlaying: false,
      pitch: 1.0,
      vuLevel: 0,
      onPlay: vi.fn(),
      onPause: vi.fn(),
      onLoad: vi.fn(),
      onSeek: vi.fn(),
      bpm: 120,
    };

  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue(undefined);
    mockOpen.mockResolvedValue(null);
  });

  it('renderiza o cabeçalho do deck com label correto', () => {
    render(<Deck {...defaultProps} />);
    
    expect(screen.getByText('DECK A')).toBeInTheDocument();
  });

  it('mostra BPM quando disponível', () => {
    render(<Deck {...defaultProps} />);
    
    expect(screen.getByText('120.0')).toBeInTheDocument();
    expect(screen.getByText('BPM')).toBeInTheDocument();
  });

  it('mostra --- quando BPM é 0', () => {
    render(<Deck {...defaultProps} bpm={0} />);
    
    expect(screen.getByText('---')).toBeInTheDocument();
  });

  it('botão LOAD existe e tem aria-label correto', () => {
    render(<Deck {...defaultProps} />);
    
    const loadButton = screen.getByRole('button', { name: /carregar faixa no deck a/i });
    expect(loadButton).toBeInTheDocument();
  });

  it('botão PLAY existe quando não está tocando', () => {
    render(<Deck {...defaultProps} />);
    
    const playButton = screen.getByRole('button', { name: /reproduzir deck a/i });
    expect(playButton).toBeInTheDocument();
  });

  it('botão PAUSE existe quando está tocando', () => {
    render(<Deck {...defaultProps} isPlaying={true} />);
    
    const pauseButton = screen.getByRole('button', { name: /pausar deck a/i });
    expect(pauseButton).toBeInTheDocument();
  });

  it('mostra tempo decorrido e duração', () => {
    render(<Deck {...defaultProps} />);
    
    expect(screen.getAllByText(/00:00/)).toHaveLength(2);
  });

  it('pitch fader tem aria-label correto', () => {
    render(<Deck {...defaultProps} />);
    
    const pitchInput = screen.getByLabelText(/pitch do deck a/i);
    expect(pitchInput).toBeInTheDocument();
    expect(pitchInput).toHaveAttribute('min', '0.8');
    expect(pitchInput).toHaveAttribute('max', '1.2');
    expect(pitchInput).toHaveAttribute('step', '0.01');
  });

  it('botão CUE tem label correto quando não marcado', () => {
    render(<Deck {...defaultProps} />);
    
    const cueButton = screen.getByRole('button', { name: /marcar ponto cue no deck a/i });
    expect(cueButton).toBeInTheDocument();
    expect(cueButton).toHaveTextContent('CUE (marcar)');
  });

  it('botão CUE existe', () => {
    render(<Deck {...defaultProps} />);
    
    const cueButton = screen.getByRole('button', { name: /marcar ponto cue no deck a/i });
    expect(cueButton).toBeInTheDocument();
  });
});