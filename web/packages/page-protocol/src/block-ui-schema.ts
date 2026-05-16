import type {
  BlockProtocolError,
  BlockRuntimeErrorCode
} from './block-runtime-error';

export const BLOCK_UI_PRIMITIVES = [
  'Stack',
  'Inline',
  'Grid',
  'Divider',
  'Text',
  'Title',
  'Caption',
  'Badge',
  'Table',
  'Descriptions',
  'Empty',
  'Alert',
  'Form',
  'FormItem',
  'Input',
  'Textarea',
  'Select',
  'Checkbox',
  'Switch',
  'DatePicker',
  'NumberInput',
  'Button',
  'IconButton',
  'Modal'
] as const;

export const BLOCK_STYLE_CATEGORIES = [
  'spacing',
  'color',
  'typography',
  'border',
  'radius',
  'layout'
] as const;

export const BLOCK_DATA_PERMISSIONS = [
  'query',
  'create',
  'update',
  'delete'
] as const;

export type BlockUiPrimitive = (typeof BLOCK_UI_PRIMITIVES)[number];
export type BlockStyleCategory = (typeof BLOCK_STYLE_CATEGORIES)[number];
export type BlockDataPermission = (typeof BLOCK_DATA_PERMISSIONS)[number];
export type BlockStyleTokenValue = string | number | boolean | null;
export type BlockUiProps = Record<string, unknown>;

export type BlockUiStyle = Partial<{
  spacing: Partial<Record<BlockSpacingStyleKey, BlockStyleTokenValue>>;
  color: Partial<Record<BlockColorStyleKey, BlockStyleTokenValue>>;
  typography: Partial<Record<BlockTypographyStyleKey, BlockStyleTokenValue>>;
  border: Partial<Record<BlockBorderStyleKey, BlockStyleTokenValue>>;
  radius: Partial<Record<BlockRadiusStyleKey, BlockStyleTokenValue>>;
  layout: Partial<Record<BlockLayoutStyleKey, BlockStyleTokenValue>>;
}>;

export interface BlockUiPermissionMarkers {
  data?: BlockDataPermission[];
  actions?: string[];
  events?: string[];
}

export interface BlockUiSchemaNode {
  primitive: BlockUiPrimitive;
  key?: string;
  props?: BlockUiProps;
  style?: BlockUiStyle;
  children?: BlockUiSchemaNode[];
  permissions?: BlockUiPermissionMarkers;
}

export type BlockUiSchema = BlockUiSchemaNode;

export interface BlockUiSchemaValidationOptions {
  maxDepth?: number;
  maxNodes?: number;
  allowedDataPermissions?: readonly BlockDataPermission[];
  allowedActions?: readonly string[];
  allowedEvents?: readonly string[];
}

export type BlockUiSchemaValidationResult =
  | {
      ok: true;
      schema: BlockUiSchema;
      errors: [];
    }
  | {
      ok: false;
      errors: BlockProtocolError[];
    };

type BlockStyleKeyByCategory = {
  spacing: BlockSpacingStyleKey;
  color: BlockColorStyleKey;
  typography: BlockTypographyStyleKey;
  border: BlockBorderStyleKey;
  radius: BlockRadiusStyleKey;
  layout: BlockLayoutStyleKey;
};

type BlockSpacingStyleKey =
  | 'margin'
  | 'marginX'
  | 'marginY'
  | 'marginTop'
  | 'marginRight'
  | 'marginBottom'
  | 'marginLeft'
  | 'padding'
  | 'paddingX'
  | 'paddingY'
  | 'paddingTop'
  | 'paddingRight'
  | 'paddingBottom'
  | 'paddingLeft'
  | 'gap'
  | 'rowGap'
  | 'columnGap';

type BlockColorStyleKey =
  | 'text'
  | 'background'
  | 'border'
  | 'accent'
  | 'status';

type BlockTypographyStyleKey =
  | 'fontSize'
  | 'fontWeight'
  | 'lineHeight'
  | 'align'
  | 'truncate';

type BlockBorderStyleKey = 'width' | 'style' | 'color';
type BlockRadiusStyleKey = 'all' | 'top' | 'right' | 'bottom' | 'left';

type BlockLayoutStyleKey =
  | 'display'
  | 'width'
  | 'minWidth'
  | 'maxWidth'
  | 'height'
  | 'minHeight'
  | 'maxHeight'
  | 'overflow'
  | 'align'
  | 'justify'
  | 'direction'
  | 'wrap'
  | 'columns'
  | 'span';

const DEFAULT_MAX_DEPTH = 8;
const DEFAULT_MAX_NODES = 250;

const primitiveSet = new Set<string>(BLOCK_UI_PRIMITIVES);
const styleCategorySet = new Set<string>(BLOCK_STYLE_CATEGORIES);
const dataPermissionSet = new Set<string>(BLOCK_DATA_PERMISSIONS);
const nodeKeySet = new Set([
  'primitive',
  'key',
  'props',
  'style',
  'children',
  'permissions'
]);

const styleKeys: {
  [Category in BlockStyleCategory]: readonly BlockStyleKeyByCategory[Category][];
} = {
  spacing: [
    'margin',
    'marginX',
    'marginY',
    'marginTop',
    'marginRight',
    'marginBottom',
    'marginLeft',
    'padding',
    'paddingX',
    'paddingY',
    'paddingTop',
    'paddingRight',
    'paddingBottom',
    'paddingLeft',
    'gap',
    'rowGap',
    'columnGap'
  ],
  color: ['text', 'background', 'border', 'accent', 'status'],
  typography: ['fontSize', 'fontWeight', 'lineHeight', 'align', 'truncate'],
  border: ['width', 'style', 'color'],
  radius: ['all', 'top', 'right', 'bottom', 'left'],
  layout: [
    'display',
    'width',
    'minWidth',
    'maxWidth',
    'height',
    'minHeight',
    'maxHeight',
    'overflow',
    'align',
    'justify',
    'direction',
    'wrap',
    'columns',
    'span'
  ]
};

const styleKeySets: Record<BlockStyleCategory, ReadonlySet<string>> = {
  spacing: new Set(styleKeys.spacing),
  color: new Set(styleKeys.color),
  typography: new Set(styleKeys.typography),
  border: new Set(styleKeys.border),
  radius: new Set(styleKeys.radius),
  layout: new Set(styleKeys.layout)
};

const dataPermissionErrorCodes: Record<
  BlockDataPermission,
  BlockRuntimeErrorCode
> = {
  query: 'query_denied',
  create: 'create_denied',
  update: 'update_denied',
  delete: 'delete_denied'
};

export function validateBlockUiSchema(
  schema: unknown,
  options: BlockUiSchemaValidationOptions = {}
): BlockUiSchemaValidationResult {
  const errors: BlockProtocolError[] = [];
  const state = {
    maxDepth: options.maxDepth ?? DEFAULT_MAX_DEPTH,
    maxNodes: options.maxNodes ?? DEFAULT_MAX_NODES,
    nodeCount: 0,
    allowedDataPermissions: new Set(options.allowedDataPermissions ?? []),
    allowedActions: new Set(options.allowedActions ?? []),
    allowedEvents: new Set(options.allowedEvents ?? []),
    seen: new WeakSet<object>()
  };

  try {
    validateNode(schema, 'root', 1, state, errors);
  } catch {
    addError(
      errors,
      'schema_invalid',
      'root',
      'Schema validation failed.'
    );
  }

  if (errors.length > 0) {
    return { ok: false, errors };
  }

  return {
    ok: true,
    schema: schema as BlockUiSchema,
    errors: []
  };
}

interface ValidationState {
  maxDepth: number;
  maxNodes: number;
  nodeCount: number;
  allowedDataPermissions: ReadonlySet<BlockDataPermission>;
  allowedActions: ReadonlySet<string>;
  allowedEvents: ReadonlySet<string>;
  seen: WeakSet<object>;
}

function validateNode(
  value: unknown,
  path: string,
  depth: number,
  state: ValidationState,
  errors: BlockProtocolError[]
): void {
  if (errors.length > 0) {
    return;
  }

  if (depth > state.maxDepth) {
    addError(errors, 'schema_invalid', path, 'Schema exceeds maximum depth.');
    return;
  }

  if (!isRecord(value)) {
    addError(errors, 'schema_invalid', path, 'Schema node must be an object.');
    return;
  }

  if (state.seen.has(value)) {
    addError(errors, 'schema_invalid', path, 'Schema contains a cycle.');
    return;
  }
  state.seen.add(value);

  state.nodeCount += 1;
  if (state.nodeCount > state.maxNodes) {
    addError(errors, 'schema_invalid', path, 'Schema exceeds maximum nodes.');
    return;
  }

  const keys = getObjectKeys(value, path, errors);
  if (keys === undefined) {
    return;
  }

  for (const key of keys) {
    if (!nodeKeySet.has(key)) {
      addError(errors, 'schema_invalid', `${path}.${key}`, 'Unknown schema key.');
      return;
    }
  }

  const primitive = readProperty(value, 'primitive', `${path}.primitive`, errors);
  if (!primitive.ok) {
    return;
  }

  if (typeof primitive.value !== 'string' || !primitiveSet.has(primitive.value)) {
    addError(
      errors,
      'schema_invalid',
      `${path}.primitive`,
      'Unknown block UI primitive.'
    );
    return;
  }

  const key = readProperty(value, 'key', `${path}.key`, errors);
  if (!key.ok) {
    return;
  }

  if (key.value !== undefined && typeof key.value !== 'string') {
    addError(errors, 'schema_invalid', `${path}.key`, 'Key must be a string.');
    return;
  }

  const props = readProperty(value, 'props', `${path}.props`, errors);
  if (!props.ok) {
    return;
  }

  if (
    props.value !== undefined &&
    !validateJsonCompatible(props.value, `${path}.props`, errors)
  ) {
    return;
  }

  const style = readProperty(value, 'style', `${path}.style`, errors);
  if (!style.ok) {
    return;
  }

  if (style.value !== undefined) {
    validateStyle(style.value, `${path}.style`, errors);
    if (errors.length > 0) {
      return;
    }
  }

  const permissions = readProperty(
    value,
    'permissions',
    `${path}.permissions`,
    errors
  );
  if (!permissions.ok) {
    return;
  }

  if (permissions.value !== undefined) {
    validatePermissions(permissions.value, `${path}.permissions`, state, errors);
    if (errors.length > 0) {
      return;
    }
  }

  const children = readProperty(value, 'children', `${path}.children`, errors);
  if (!children.ok) {
    return;
  }

  if (children.value === undefined) {
    return;
  }

  if (!Array.isArray(children.value)) {
    addError(
      errors,
      'schema_invalid',
      `${path}.children`,
      'Children must be an array.'
    );
    return;
  }

  for (let index = 0; index < children.value.length; index += 1) {
    const child = readArrayItem(
      children.value,
      index,
      `${path}.children[${index}]`,
      errors
    );
    if (!child.ok) {
      return;
    }
    validateNode(
      child.value,
      `${path}.children[${index}]`,
      depth + 1,
      state,
      errors
    );
  }
}

function validateStyle(
  value: unknown,
  path: string,
  errors: BlockProtocolError[]
): void {
  if (!isRecord(value)) {
    addError(errors, 'schema_invalid', path, 'Style must be an object.');
    return;
  }

  const categories = getObjectKeys(value, path, errors);
  if (categories === undefined) {
    return;
  }

  for (const category of categories) {
    if (!styleCategorySet.has(category)) {
      addError(
        errors,
        'schema_invalid',
        `${path}.${category}`,
        'Style category is not allowed.'
      );
      return;
    }

    const categoryValue = readProperty(
      value,
      category,
      `${path}.${category}`,
      errors
    );
    if (!categoryValue.ok) {
      return;
    }

    if (!isRecord(categoryValue.value)) {
      addError(
        errors,
        'schema_invalid',
        `${path}.${category}`,
        'Style category must be a token object.'
      );
      return;
    }

    const allowedKeys = styleKeySets[category as BlockStyleCategory];
    const styleTokenKeys = getObjectKeys(categoryValue.value, `${path}.${category}`, errors);
    if (styleTokenKeys === undefined) {
      return;
    }

    for (const styleKey of styleTokenKeys) {
      if (!allowedKeys.has(styleKey)) {
        addError(
          errors,
          'schema_invalid',
          `${path}.${category}.${styleKey}`,
          'Style token key is not allowed.'
        );
        return;
      }

      const tokenValue = readProperty(
        categoryValue.value,
        styleKey,
        `${path}.${category}.${styleKey}`,
        errors
      );
      if (!tokenValue.ok) {
        return;
      }

      if (!isTokenValue(tokenValue.value)) {
        addError(
          errors,
          'schema_invalid',
          `${path}.${category}.${styleKey}`,
          'Style token value must be scalar.'
        );
        return;
      }
    }
  }
}

function validatePermissions(
  value: unknown,
  path: string,
  state: ValidationState,
  errors: BlockProtocolError[]
): void {
  if (!isRecord(value)) {
    addError(errors, 'schema_invalid', path, 'Permissions must be an object.');
    return;
  }

  const keys = getObjectKeys(value, path, errors);
  if (keys === undefined) {
    return;
  }

  for (const key of keys) {
    if (key !== 'data' && key !== 'actions' && key !== 'events') {
      addError(
        errors,
        'schema_invalid',
        `${path}.${key}`,
        'Unknown permission marker group.'
      );
      return;
    }
  }

  const data = readProperty(value, 'data', `${path}.data`, errors);
  if (!data.ok) {
    return;
  }
  validateDataPermissions(data.value, `${path}.data`, state, errors);
  if (errors.length > 0) {
    return;
  }
  const actions = readProperty(value, 'actions', `${path}.actions`, errors);
  if (!actions.ok) {
    return;
  }
  validateStringPermissionList(
    actions.value,
    `${path}.actions`,
    state.allowedActions,
    'action_denied',
    errors
  );
  if (errors.length > 0) {
    return;
  }
  const events = readProperty(value, 'events', `${path}.events`, errors);
  if (!events.ok) {
    return;
  }
  validateStringPermissionList(
    events.value,
    `${path}.events`,
    state.allowedEvents,
    'event_denied',
    errors
  );
}

function validateDataPermissions(
  value: unknown,
  path: string,
  state: ValidationState,
  errors: BlockProtocolError[]
): void {
  if (value === undefined) {
    return;
  }

  if (!Array.isArray(value)) {
    addError(errors, 'schema_invalid', path, 'Data permissions must be an array.');
    return;
  }

  for (let index = 0; index < value.length; index += 1) {
    const itemPath = `${path}[${index}]`;
    const permission = readArrayItem(value, index, itemPath, errors);
    if (!permission.ok) {
      return;
    }

    if (
      typeof permission.value !== 'string' ||
      !dataPermissionSet.has(permission.value)
    ) {
      addError(
        errors,
        'schema_invalid',
        itemPath,
        'Unknown data permission marker.'
      );
      return;
    }

    const dataPermission = permission.value as BlockDataPermission;
    if (!state.allowedDataPermissions.has(dataPermission)) {
      addError(
        errors,
        dataPermissionErrorCodes[dataPermission],
        itemPath,
        'Data permission marker is not allowed.'
      );
    }
  }
}

function validateStringPermissionList(
  value: unknown,
  path: string,
  allowed: ReadonlySet<string>,
  deniedCode: BlockRuntimeErrorCode,
  errors: BlockProtocolError[]
): void {
  if (value === undefined) {
    return;
  }

  if (!Array.isArray(value)) {
    addError(errors, 'schema_invalid', path, 'Permission markers must be an array.');
    return;
  }

  for (let index = 0; index < value.length; index += 1) {
    const itemPath = `${path}[${index}]`;
    const permission = readArrayItem(value, index, itemPath, errors);
    if (!permission.ok) {
      return;
    }

    if (typeof permission.value !== 'string' || permission.value.length === 0) {
      addError(
        errors,
        'schema_invalid',
        itemPath,
        'Permission marker must be a non-empty string.'
      );
      return;
    }

    if (!allowed.has(permission.value)) {
      addError(
        errors,
        deniedCode,
        itemPath,
        'Permission marker is not allowed.'
      );
    }
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    return false;
  }

  try {
    return Object.getPrototypeOf(value) === Object.prototype;
  } catch {
    return false;
  }
}

function isTokenValue(value: unknown): value is BlockStyleTokenValue {
  return (
    value === null ||
    typeof value === 'string' ||
    typeof value === 'number' ||
    typeof value === 'boolean'
  );
}

type SafeReadResult =
  | {
      ok: true;
      value: unknown;
    }
  | {
      ok: false;
    };

function getObjectKeys(
  value: Record<string, unknown>,
  path: string,
  errors: BlockProtocolError[]
): string[] | undefined {
  try {
    return Object.keys(value);
  } catch {
    addError(errors, 'schema_invalid', path, 'Schema object keys are unreadable.');
    return undefined;
  }
}

function readProperty(
  value: Record<string, unknown>,
  key: string,
  path: string,
  errors: BlockProtocolError[]
): SafeReadResult {
  try {
    return { ok: true, value: value[key] };
  } catch {
    addError(errors, 'schema_invalid', path, 'Schema field is unreadable.');
    return { ok: false };
  }
}

function readArrayItem(
  value: unknown[],
  index: number,
  path: string,
  errors: BlockProtocolError[]
): SafeReadResult {
  try {
    return { ok: true, value: value[index] };
  } catch {
    addError(errors, 'schema_invalid', path, 'Schema array item is unreadable.');
    return { ok: false };
  }
}

function validateJsonCompatible(
  value: unknown,
  path: string,
  errors: BlockProtocolError[],
  seen: WeakSet<object> = new WeakSet()
): boolean {
  if (isTokenValue(value)) {
    return true;
  }

  if (Array.isArray(value)) {
    if (seen.has(value)) {
      addError(
        errors,
        'schema_invalid',
        path,
        'Props must not contain cycles.'
      );
      return false;
    }

    seen.add(value);

    for (let index = 0; index < value.length; index += 1) {
      const itemPath = `${path}[${index}]`;
      const item = readArrayItem(value, index, itemPath, errors);
      if (!item.ok || !validateJsonCompatible(item.value, itemPath, errors, seen)) {
        seen.delete(value);
        return false;
      }
    }

    seen.delete(value);
    return true;
  }

  if (isRecord(value)) {
    if (seen.has(value)) {
      addError(
        errors,
        'schema_invalid',
        path,
        'Props must not contain cycles.'
      );
      return false;
    }

    seen.add(value);

    const keys = getObjectKeys(value, path, errors);
    if (keys === undefined) {
      seen.delete(value);
      return false;
    }

    for (const key of keys) {
      const itemPath = `${path}.${key}`;
      const item = readProperty(value, key, itemPath, errors);
      if (!item.ok || !validateJsonCompatible(item.value, itemPath, errors, seen)) {
        seen.delete(value);
        return false;
      }
    }

    seen.delete(value);
    return true;
  }

  addError(errors, 'schema_invalid', path, 'Props must be JSON-compatible data.');
  return false;
}

function addError(
  errors: BlockProtocolError[],
  code: BlockRuntimeErrorCode,
  path: string,
  message: string
): void {
  errors.push({ code, path, message });
}
