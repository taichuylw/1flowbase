export type JsonSchemaRootType = 'object' | 'array';

export function createDefaultJsonSchema(
  rootType: JsonSchemaRootType = 'object'
): Record<string, unknown> {
  if (rootType === 'array') {
    return {
      type: 'array',
      items: {
        type: 'object',
        required: [],
        properties: {}
      }
    };
  }

  return {
    type: 'object',
    required: [],
    properties: {}
  };
}

export function jsonSchemaRootType(
  schema: Record<string, unknown>,
  fallback: JsonSchemaRootType
): JsonSchemaRootType {
  if (schema.type === 'array' || schema.type === 'object') {
    return schema.type;
  }

  return fallback;
}
