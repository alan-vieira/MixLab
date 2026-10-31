import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Mixer } from './Mixer';
import { invoke } from '@tauri-apps/api/core';

const mockInvoke = vi.mocked(invoke);

describe('Mixer Component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue([0, 0]); // VU levels
  });

  it('renderiza título MIXER', () => {
    render(<Mixer />);
    
    expect(screen.getByText('MIXER')).toBeInTheDocument();
  });

  it('renderiza 2 canvas de VU meter', () => {
    render(<Mixer />);
    
    // VUMeter usa canvas, verificamos que existem 2 canvas
    const canvasElements = document.querySelectorAll('canvas');
    expect(canvasElements.length).toBeGreaterThanOrEqual(2);
  });

  it('renderiza 3 knobs de EQ por deck (HI, MID, LO)', () => {
    render(<Mixer />);
    
    const hiLabels = screen.getAllByText('HI');
    const midLabels = screen.getAllByText('MID');
    const loLabels = screen.getAllByText('LO');
    expect(hiLabels).toHaveLength(2);
    expect(midLabels).toHaveLength(2);
    expect(loLabels).toHaveLength(2);
  });

  it('renderiza knob de FILTER por deck', () => {
    render(<Mixer />);
    
    const filterLabels = screen.getAllByText('FILTER');
    expect(filterLabels).toHaveLength(2);
  });

  it('renderiza fader de volume por deck + crossfader', () => {
    render(<Mixer />);
    
    const volumeInputs = screen.getAllByRole('slider');
    // 2 volume faders + 1 crossfader = 3 sliders total
    expect(volumeInputs.length).toBeGreaterThanOrEqual(3);
  });

  it('renderiza crossfader', () => {
    render(<Mixer />);
    
    expect(screen.getByText('CROSSFADER')).toBeInTheDocument();
    const sliders = screen.getAllByRole('slider');
    expect(sliders.length).toBeGreaterThanOrEqual(3);
  });

  it('labels A e B no crossfader', () => {
    render(<Mixer />);
    
    expect(screen.getByText('A')).toBeInTheDocument();
    expect(screen.getByText('B')).toBeInTheDocument();
  });

  it('monta sem erros', () => {
    render(<Mixer />);
    expect(screen.getByText('MIXER')).toBeInTheDocument();
  });
});