import { afterEach, expect, it, vi } from 'vitest';
import { request } from '@/services/api';
import { t, setLocale, messages } from '@/i18n';
import { failureMessage } from '@/services/failureMessage';

afterEach(() => vi.unstubAllGlobals());

it('keeps requests inside the application and accepts JSON or an empty response', async () => {
  const fetcher = vi
    .fn()
    .mockResolvedValueOnce(new Response(JSON.stringify({ fontSize: 14 })))
    .mockResolvedValueOnce(new Response(null, { status: 204 }));
  vi.stubGlobal('fetch', fetcher);
  expect(await request('/settings/terminal')).toEqual({ fontSize: 14 });
  expect(fetcher).toHaveBeenCalledWith(
    'api/settings/terminal',
    expect.objectContaining({ headers: { 'Content-Type': 'application/json' } }),
  );

  const signal = new AbortController().signal;
  await expect(
    request('/settings/terminal', { method: 'PUT', body: '{"fontSize":14}', signal }),
  ).resolves.toBeUndefined();
  expect(fetcher).toHaveBeenLastCalledWith(
    'api/settings/terminal',
    expect.objectContaining({ method: 'PUT', signal }),
  );
});

it.each([
  [JSON.stringify({ error: 'no session' }), 'no session'],
  [JSON.stringify({ error: 1 }), 'Unable to complete the request.'],
  ['null', 'Unable to complete the request.'],
  ['not JSON', 'Unable to complete the request.'],
])('reports launcher failures without redirecting the application: %s', async (body, expected) => {
  setLocale('en');
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(body, { status: 503 })));
  await expect(request('/settings/terminal')).rejects.toThrow(expected);
});

it('keeps translations, substitution and failure fallbacks in both languages', () => {
  setLocale('en');
  expect(t('terminalConsole')).toBe('Console');
  expect(t('unknown')).toBe('unknown');
  expect(failureMessage(new Error('offline'))).toBe('offline');
  expect(failureMessage(new Error())).toBe('Unable to complete the request.');
  expect(failureMessage(null)).toBe('Unable to complete the request.');
  setLocale('it');
  expect(failureMessage(null)).toBe(messages.it.requestFailed);
});
