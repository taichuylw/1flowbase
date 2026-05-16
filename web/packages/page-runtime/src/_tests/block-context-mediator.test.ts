import { describe, expect, test } from 'vitest';

import {
  createBlockContextMediator,
  createBlockContextMediatorState,
  reduceBlockContextMediator
} from '../index';

describe('BlockContext host mediator', () => {
  test('allows worker event, action, and data requests that match the policy', () => {
    const mediator = createBlockContextMediator({
      allowedEvents: ['record.saved'],
      allowedActions: ['record.save'],
      allowedDataModels: ['records'],
      allowedDataOperations: ['query']
    });

    expect(
      mediator.handle(
        {
          type: 'event',
          requestId: 'request-1',
          name: 'record.saved',
          payload: { id: 'record-1', tags: ['ready'] }
        },
        { tickId: 'tick-1' }
      ).result
    ).toMatchObject({
      ok: true,
      requestId: 'request-1',
      effect: {
        type: 'event',
        name: 'record.saved',
        payload: { id: 'record-1', tags: ['ready'] }
      }
    });

    expect(
      mediator.handle({
        type: 'action',
        requestId: 'request-1',
        actionId: 'record.save',
        payload: { id: 'record-1' }
      }).result
    ).toMatchObject({
      ok: true,
      effect: {
        type: 'action',
        actionId: 'record.save',
        payload: { id: 'record-1' }
      }
    });

    expect(
      mediator.handle({
        type: 'data',
        requestId: 'request-1',
        operation: 'query',
        payload: { model: 'records', where: { id: 'record-1' } }
      }).result
    ).toMatchObject({
      ok: true,
      effect: {
        type: 'data',
        operation: 'query',
        payload: { model: 'records', where: { id: 'record-1' } }
      }
    });
  });

  test.each([
    [
      { type: 'event', requestId: 'request-1', name: 'record.deleted' },
      'event_denied'
    ],
    [
      { type: 'action', requestId: 'request-1', actionId: 'record.delete' },
      'action_denied'
    ],
    [
      {
        type: 'data',
        requestId: 'request-1',
        operation: 'query',
        payload: { model: 'private_records' }
      },
      'query_denied'
    ],
    [
      {
        type: 'data',
        requestId: 'request-1',
        operation: 'create',
        payload: { model: 'records', input: { title: 'Ready' } }
      },
      'create_denied'
    ],
    [
      {
        type: 'data',
        requestId: 'request-1',
        operation: 'update',
        payload: { model: 'records', id: 'record-1', input: { title: 'Ready' } }
      },
      'update_denied'
    ],
    [
      {
        type: 'data',
        requestId: 'request-1',
        operation: 'delete',
        payload: { model: 'records', id: 'record-1' }
      },
      'delete_denied'
    ]
  ] as const)(
    'rejects denied worker request with stable code %s',
    (effect, expectedCode) => {
      const { result } = reduceBlockContextMediator(
        createBlockContextMediatorState(),
        effect,
        {
          allowedEvents: ['record.saved'],
          allowedActions: ['record.save'],
          allowedDataModels: ['records'],
          allowedDataOperations: ['query']
        }
      );

      expect(result).toMatchObject({
        ok: false,
        requestId: 'request-1',
        code: expectedCode
      });
    }
  );

  test('rejects non JSON-compatible payloads as structured results without throwing', () => {
    const cycle: Record<string, unknown> = {};
    cycle.self = cycle;

    const throwingAccessor = {};
    Object.defineProperty(throwingAccessor, 'bad', {
      enumerable: true,
      get() {
        throw new Error('getter failed');
      }
    });

    const mediator = createBlockContextMediator({
      allowedEvents: ['record.saved']
    });

    for (const payload of [
      { fn: () => undefined },
      { symbol: Symbol('private') },
      cycle,
      throwingAccessor
    ]) {
      expect(() =>
        mediator.handle({
          type: 'event',
          requestId: 'request-1',
          name: 'record.saved',
          payload
        })
      ).not.toThrow();

      expect(
        mediator.handle({
          type: 'event',
          requestId: 'request-1',
          name: 'record.saved',
          payload
        }).result
      ).toMatchObject({
        ok: false,
        requestId: 'request-1',
        code: 'payload_invalid'
      });
    }
  });

  test('rejects event chains that exceed the per request tick limit', () => {
    const mediator = createBlockContextMediator({
      allowedEvents: ['record.saved'],
      maxEventChainDepth: 2
    });
    const effect = {
      type: 'event',
      requestId: 'request-1',
      name: 'record.saved'
    } as const;

    expect(mediator.handle(effect, { tickId: 'tick-1' }).result.ok).toBe(true);
    expect(mediator.handle(effect, { tickId: 'tick-1' }).result.ok).toBe(true);

    expect(mediator.handle(effect, { tickId: 'tick-1' }).result).toMatchObject({
      ok: false,
      code: 'event_denied',
      path: 'event.chain'
    });

    expect(mediator.handle(effect, { tickId: 'tick-2' }).result.ok).toBe(true);
  });
});
