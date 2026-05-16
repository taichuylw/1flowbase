export interface PageDefinition {
  route: string;
  title: string;
}

export function renderPageTitle(definition: PageDefinition): string {
  return `${definition.title} (${definition.route})`;
}

export * from './js-block-source-policy';
