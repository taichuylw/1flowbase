import type { NodeDefinitionField } from './types';
import { i18nText } from '../../../../shared/i18n/text';

export const basicFields: NodeDefinitionField[] = [
  { key: 'alias', label: i18nText("agentFlow", "auto.node_alias"), editor: 'text', required: true },
  { key: 'description', label: i18nText("agentFlow", "auto.node_introduction"), editor: 'text' }
];
