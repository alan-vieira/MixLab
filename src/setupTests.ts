import { vi } from 'vitest';
import '@testing-library/jest-dom';

// Mock HTMLCanvasElement.getContext for jsdom
const mockGetContext = vi.fn();
Object.defineProperty(HTMLCanvasElement.prototype, 'getContext', {
  value: mockGetContext,
  writable: true,
});

// Mock TAURI internals for @tauri-apps/api v2
const mockInvoke = vi.fn();
Object.defineProperty(window, '__TAURI_INTERNALS__', {
  value: {
    invoke: mockInvoke,
  },
  writable: true,
});

// Create spy functions for each resolver
export const mockInvokeResolvers = {
  getDeckStatus: vi.fn((_deck: 'A' | 'B', overrides = {}) => ({
    position: 0,
    vu_level: 0,
    is_playing: false,
    pitch: 1.0,
    ...overrides,
  })),
  loadTrack: vi.fn((overrides = {}) => ({
    duration_secs: 180,
    ...overrides,
  })),
  getWaveformPeaks: vi.fn(() => new Array(1000).fill(0).map((_, i) => Math.sin(i * 0.1))),
  getWaveformLayers: vi.fn(() => [
    new Array(5000).fill(0),
    new Array(5000).fill(0),
    new Array(5000).fill(0),
  ]),
  playDeck: vi.fn(() => Promise.resolve()),
  pauseDeck: vi.fn(() => Promise.resolve()),
  setDeckPitch: vi.fn(() => Promise.resolve()),
  jogScrub: vi.fn(() => Promise.resolve()),
  jumpToCue: vi.fn(() => Promise.resolve()),
  setCuePoint: vi.fn(() => Promise.resolve()),
  setDeckEq: vi.fn(() => Promise.resolve()),
  setDeckGain: vi.fn(() => Promise.resolve()),
  setDeckFilter: vi.fn(() => Promise.resolve()),
  setCrossfader: vi.fn(() => Promise.resolve()),
  getVuLevels: vi.fn(() => Promise.resolve([0, 0])),
};

// Setup default mock behavior - calls the CURRENT resolver each time
mockInvoke.mockImplementation((command: string, args: any) => {
  switch (command) {
    case 'get_deck_status':
      return Promise.resolve(mockInvokeResolvers.getDeckStatus(args.deck));
    case 'load_track':
      return Promise.resolve(mockInvokeResolvers.loadTrack());
    case 'get_waveform_peaks':
      return Promise.resolve(mockInvokeResolvers.getWaveformPeaks());
    case 'get_waveform_layers':
      return Promise.resolve(mockInvokeResolvers.getWaveformLayers());
    case 'play_deck':
      return mockInvokeResolvers.playDeck();
    case 'pause_deck':
      return mockInvokeResolvers.pauseDeck();
    case 'set_deck_pitch':
      return mockInvokeResolvers.setDeckPitch();
    case 'jog_scrub':
      return mockInvokeResolvers.jogScrub();
    case 'jump_to_cue':
      return mockInvokeResolvers.jumpToCue();
    case 'set_cue_point':
      return mockInvokeResolvers.setCuePoint();
    case 'set_deck_eq':
      return mockInvokeResolvers.setDeckEq();
    case 'set_deck_gain':
      return mockInvokeResolvers.setDeckGain();
    case 'set_deck_filter':
      return mockInvokeResolvers.setDeckFilter();
    case 'set_crossfader':
      return mockInvokeResolvers.setCrossfader();
    case 'get_vu_levels':
      return mockInvokeResolvers.getVuLevels();
    default:
      return Promise.resolve();
  }
});

// Helper to reset mocks between tests
export const resetTauriMocks = () => {
  mockInvoke.mockClear();
  mockGetContext.mockClear();
  // Reset all resolver spies
  Object.values(mockInvokeResolvers).forEach(resolver => {
    if (vi.isMockFunction(resolver)) {
      resolver.mockClear();
    }
  });
};

// Mock canvas context for waveform rendering
mockGetContext.mockReturnValue({
  fillRect: vi.fn(),
  clearRect: vi.fn(),
  beginPath: vi.fn(),
  moveTo: vi.fn(),
  lineTo: vi.fn(),
  stroke: vi.fn(),
  fill: vi.fn(),
  arc: vi.fn(),
  closePath: vi.fn(),
  createLinearGradient: vi.fn(() => ({
    addColorStop: vi.fn(),
  })),
  save: vi.fn(),
  restore: vi.fn(),
  translate: vi.fn(),
  scale: vi.fn(),
  rotate: vi.fn(),
  drawImage: vi.fn(),
  getImageData: vi.fn(() => ({ data: new Uint8ClampedArray(4) })),
  putImageData: vi.fn(),
});