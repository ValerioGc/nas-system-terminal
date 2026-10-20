import { t } from '@/i18n';

export async function request<T>(path: string, options: RequestInit = {}): Promise<T> {
  const response = await fetch(`api${path}`, {
    ...options,
    headers: { 'Content-Type': 'application/json', ...options.headers },
    signal: options.signal ?? AbortSignal.timeout(10000),
  });

  if (!response.ok) {
    const body: unknown = await response.json().catch(() => null);
    const reason =
      typeof body === 'object' && body !== null && 'error' in body && typeof body.error === 'string'
        ? body.error
        : '';

    throw new Error(reason || t('requestFailed'));
  }

  const result = response.status === 204 ? undefined : await response.json();

  return result as T;
}
