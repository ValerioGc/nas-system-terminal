import { t } from '@/i18n';

export function failureMessage(reason: unknown) {
  return reason instanceof Error && reason.message ? reason.message : t('requestFailed');
}
