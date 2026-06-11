import type { AgentFlowTemplatePackage } from '../api/applications';

export function buildTemplateFileName(name: string) {
  const normalized = name
    .trim()
    .replace(/[^\p{Letter}\p{Number}._-]+/gu, '-')
    .replace(/^-+|-+$/g, '');

  return `${normalized || 'agent-flow-template'}.1flowbase-template.json`;
}

export function downloadTemplateFile(template: AgentFlowTemplatePackage) {
  const blob = new Blob([JSON.stringify(template, null, 2)], {
    type: 'application/json'
  });
  const url = window.URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = buildTemplateFileName(template.application.name);
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  window.URL.revokeObjectURL(url);
}
