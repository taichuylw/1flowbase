import { describe, expect, test } from 'vitest';

import { validateBlockUiSchema } from '../../../page-protocol/src/index';

import {
  Alert,
  Badge,
  Button,
  Caption,
  Checkbox,
  DatePicker,
  Descriptions,
  Divider,
  Empty,
  Form,
  FormItem,
  Grid,
  IconButton,
  Inline,
  Input,
  Modal,
  NumberInput,
  Select,
  Stack,
  Switch,
  Table,
  Text,
  Textarea,
  Title
} from '../index';

describe('AntD-compatible facade', () => {
  test('builds representative protocol nodes that validate', () => {
    const schema = Stack({
      key: 'root',
      props: { role: 'region', className: 'leak' },
      style: {
        spacing: { padding: 'space.4', gap: 'space.2' },
        color: { background: 'surface.default' },
        position: 'absolute'
      },
      children: [
        Text({
          key: 'summary',
          props: {
            children: '24 records',
            type: 'secondary',
            onClick: () => undefined
          }
        }),
        Table({
          key: 'records',
          props: {
            rowKey: 'id',
            columns: [{ key: 'name', title: 'Name', dataIndex: 'name' }],
            dataSource: [{ id: 'record-1', name: 'Ada' }]
          }
        }),
        Form({
          key: 'filters',
          children: [
            FormItem({
              key: 'query-item',
              props: { name: 'query', label: 'Query' },
              children: Input({ key: 'query', props: { placeholder: 'Search' } })
            })
          ]
        }),
        Button({
          key: 'save',
          props: { children: 'Save', type: 'primary' },
          permissions: { actions: ['record.save'] }
        }),
        Modal({
          key: 'details',
          props: { title: 'Record details', open: false },
          children: Descriptions({
            props: {
              items: [{ key: 'name', label: 'Name', children: 'Ada' }]
            }
          })
        })
      ]
    });

    expect(schema).toMatchObject({
      primitive: 'Stack',
      key: 'root',
      props: { role: 'region' },
      style: {
        spacing: { padding: 'space.4', gap: 'space.2' },
        color: { background: 'surface.default' }
      }
    });
    expect(schema.props).not.toHaveProperty('className');
    expect(Object.keys(schema.style ?? {})).not.toContain('position');

    const result = validateBlockUiSchema(schema, {
      allowedActions: ['record.save']
    });

    expect(result).toEqual({ ok: true, schema, errors: [] });
  });

  test('exports all controlled primitive facades without React elements', () => {
    const schema = Stack({
      children: [
        Inline(),
        Grid(),
        Divider(),
        Text({ props: { children: 'text' } }),
        Title({ props: { children: 'title' } }),
        Caption({ props: { children: 'caption' } }),
        Badge({ props: { children: 'badge' } }),
        Table(),
        Descriptions(),
        Empty(),
        Alert(),
        Form(),
        FormItem(),
        Input(),
        Textarea(),
        Select(),
        Checkbox(),
        Switch(),
        DatePicker(),
        NumberInput(),
        Button(),
        IconButton(),
        Modal()
      ]
    });

    expect(validateBlockUiSchema(schema)).toEqual({
      ok: true,
      schema,
      errors: []
    });
    expect(schema.children?.map((child) => child.primitive)).toEqual([
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
    ]);
    expect(schema.children?.[0]).not.toHaveProperty('$$typeof');
  });

  test('cleans unsupported children props style and permission payloads', () => {
    const reactLikeElement = { type: 'span', props: { children: 'bad' } };
    const domLikeNode = { nodeType: 1, nodeName: 'DIV' };
    const cyclic: Record<string, unknown> = { label: 'safe' };
    cyclic.self = cyclic;

    const schema = Stack({
      key: 42,
      props: {
        className: 'raw-css',
        style: { color: 'red' },
        symbol: Symbol('bad'),
        callback: () => undefined,
        nested: {
          keep: true,
          cyclic,
          element: reactLikeElement,
          node: domLikeNode
        }
      },
      style: {
        spacing: {
          gap: 'space.1',
          unsafe: 'nope'
        },
        color: {
          text: 'text.primary',
          background: () => 'bad'
        },
        className: 'raw-css'
      },
      permissions: {
        data: ['query', 'drop'],
        actions: ['save', Symbol('bad')],
        events: ['submitted', '']
      },
      children: [
        Text({ props: { children: 'safe child' } }),
        reactLikeElement,
        domLikeNode,
        () => Text()
      ]
    });

    expect(schema).toEqual({
      primitive: 'Stack',
      props: { nested: { keep: true } },
      style: {
        spacing: { gap: 'space.1' },
        color: { text: 'text.primary' }
      },
      children: [Text({ props: { children: 'safe child' } })],
      permissions: {
        data: ['query'],
        actions: ['save'],
        events: ['submitted']
      }
    });
    expect(
      validateBlockUiSchema(schema, {
        allowedDataPermissions: ['query'],
        allowedActions: ['save'],
        allowedEvents: ['submitted']
      })
    ).toEqual({ ok: true, schema, errors: [] });
  });
});
