import { test, expect } from '@playwright/test';

test.use({ storageState: { cookies: [], origins: [] } });

test.beforeEach(async ({ page }) => {
  // Injeta o mock Tauri ANTES de qualquer script da aplicação carregar,
  // permitindo testar a UI no Vite dev server sem o backend Rust.
  await page.addInitScript({
    path: 'tests/e2e/mock-tauri.js',
  });
  await page.goto('/');
  await expect(page.getByText('MIXLAB')).toBeVisible();
});

test.describe('Renderização inicial', () => {
  test('aplica carrega com título e status do engine', async ({ page }) => {
    await expect(page.getByText('MIXLAB')).toBeVisible();
    await expect(page.getByText('ENGINE: RUST + CPAL')).toBeVisible();
    await expect(page.getByText('STATUS: ONLINE')).toBeVisible();
  });

  test('Deck A e Deck B estão presentes e vazios inicialmente', async ({ page }) => {
    await expect(page.getByText(/DECK A/).first()).toBeVisible();
    await expect(page.getByText(/DECK B/).first()).toBeVisible();
    await expect(page.getByText(/Deck A - Carregue uma faixa/i)).toBeVisible();
    await expect(page.getByText(/Deck B - Carregue uma faixa/i)).toBeVisible();
  });

  test('Mixer está presente', async ({ page }) => {
    await expect(page.getByText('MIXER')).toBeVisible();
    await expect(page.getByText('CROSSFADER')).toBeVisible();
  });

  test('VU meters renderizam canvas', async ({ page }) => {
    const canvases = page.locator('canvas');
    const count = await canvases.count();
    expect(count).toBeGreaterThanOrEqual(2);
  });

  test('knobs de EQ estão presentes (HI, MID, LO, FILTER por deck)', async ({ page }) => {
    // Cada tipo de knob aparece 2 vezes (uma por deck)
    await expect(page.getByText('HI', { exact: false })).toHaveCount(2);
    await expect(page.getByText('MID', { exact: false })).toHaveCount(2);
    // "LO" pode aparecer em outros contextos, então verificamos presença
    await expect(page.getByText('LO', { exact: false }).first()).toBeVisible();
    await expect(page.getByText('FILTER', { exact: false })).toHaveCount(2);
  });
});

test.describe('Acessibilidade - nomes acessíveis dos controles', () => {
  test('Deck A - controles principais estão presentes', async ({ page }) => {
    // Botão LOAD (nome varia conforme estado de loading, mas sempre presente)
    await expect(
      page.getByRole('button', { name: /carregar faixa no Deck A/i })
    ).toBeVisible();
    // Botão PLAY/PAUSE (nome varia conforme isPlaying)
    await expect(
      page.getByRole('button', {
        name: /(reproduzir|pausar) deck a/i,
      })
    ).toBeVisible();
    // Botão CUE (nome varia conforme cueSet)
    await expect(
      page.getByRole('button', {
        name: /(marcar ponto cue|retornar ao ponto cue) no deck a/i,
      })
    ).toBeVisible();
    // Botão KEY LOCK (nome varia conforme estado)
    await expect(
      page.getByRole('button', {
        name: /(ativar|desativar) key lock deck a/i,
      })
    ).toBeVisible();
    // Slider de pitch
    await expect(page.getByRole('slider', { name: /pitch/i }).first()).toBeVisible();
  });

  test('Deck B - controles principais estão presentes', async ({ page }) => {
    await expect(
      page.getByRole('button', {
        name: /(reproduzir|pausar) deck b/i,
      })
    ).toBeVisible();
    await expect(
      page.getByRole('button', {
        name: /(marcar ponto cue|retornar ao ponto cue) no deck b/i,
      })
    ).toBeVisible();
    await expect(
      page.getByRole('button', {
        name: /(ativar|desativar) key lock deck b/i,
      })
    ).toBeVisible();
    await expect(page.getByRole('slider', { name: /pitch/i }).first()).toBeVisible();
  });

  test('crossfader tem labels A e B', async ({ page }) => {
    await expect(page.getByText(/^A$/)).toBeVisible();
    await expect(page.getByText(/^B$/)).toBeVisible();
  });
});

test.describe('BigWaveform', () => {
  test('Deck A mostra estado vazio sem faixa', async ({ page }) => {
    await expect(page.getByText('Deck A - Carregue uma faixa')).toBeVisible();
  });

  test('Deck B mostra estado vazio sem faixa', async ({ page }) => {
    await expect(page.getByText('Deck B - Carregue uma faixa')).toBeVisible();
  });
});
