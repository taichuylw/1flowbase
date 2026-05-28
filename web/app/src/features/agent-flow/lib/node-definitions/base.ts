import type { NodeDefinitionField } from './types';
import { i18nText } from '../../../../shared/i18n/text';

export const basicFields: NodeDefinitionField[] = [
  { key: 'alias', label: i18nText("agentFlow", "auto.key_pmgaljeipl"), editor: 'text', required: true },
  { key: 'description', label: i18nText("agentFlow", "auto.key_kbegdcdjie"), editor: 'text' }
];
