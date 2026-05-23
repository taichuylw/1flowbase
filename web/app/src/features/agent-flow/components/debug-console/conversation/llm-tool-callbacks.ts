export interface LlmToolCallback {
  key: string;
  id: string;
  name: string;
  callbackStatus: 'returned' | 'waiting_callback' | 'cancelled';
  executionStatus:
    | 'succeeded'
    | 'failed'
    | 'timed_out'
    | 'cancelled'
    | 'unknown';
  requestPayload: Record<string, unknown>;
  callbackPayload: Record<string, unknown> | null;
  parsedResult: Record<string, unknown> | null;
  requestRoundIndex: number | null;
  resultRoundIndex: number | null;
  call_input_tokens: number | null;
  call_cached_input_tokens: number | null;
  call_output_tokens: number | null;
  result_input_tokens: number | null;
  result_context_input_tokens: number | null;
  result_context_cached_input_tokens: number | null;
  token_count_method: 'estimated' | null;
  detailArtifactRef?: string | null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === 'object' && !Array.isArray(value));
}

function isRuntimeDebugArtifactPreview(value: unknown): value is {
  __runtime_debug_artifact: true;
  artifact_ref: string;
  tool_callbacks?: unknown;
} {
  return (
    isRecord(value) &&
    value.__runtime_debug_artifact === true &&
    typeof value.artifact_ref === 'string'
  );
}

function firstRecordField(
  record: Record<string, unknown>,
  keys: string[]
): Record<string, unknown> {
  return optionalRecordField(record, keys) ?? {};
}

function optionalRecordField(
  record: Record<string, unknown>,
  keys: string[]
): Record<string, unknown> | null {
  for (const key of keys) {
    const value = record[key];

    if (isRecord(value)) {
      return value;
    }
  }

  return null;
}

function firstStringField(
  record: Record<string, unknown>,
  keys: string[]
): string | null {
  for (const key of keys) {
    const value = record[key];

    if (typeof value === 'string' && value.trim().length > 0) {
      return value;
    }
  }

  return null;
}

function recordArray(value: unknown): Record<string, unknown>[] {
  return Array.isArray(value) ? value.filter(isRecord) : [];
}

function roundIndex(round: Record<string, unknown>, fallbackIndex: number) {
  const value = round.round_index;

  if (typeof value === 'number' && Number.isFinite(value)) {
    return value;
  }

  return fallbackIndex;
}

export function readLlmRoundsFromDebugPayload(
  debugPayload: unknown
): Record<string, unknown>[] {
  if (!isRecord(debugPayload)) {
    return [];
  }

  return recordArray(debugPayload.llm_rounds);
}

export function readLlmRoundsArtifactRef(debugPayload: unknown): string | null {
  if (!isRecord(debugPayload)) {
    return null;
  }

  const llmRounds = debugPayload.llm_rounds;
  return isRuntimeDebugArtifactPreview(llmRounds)
    ? llmRounds.artifact_ref
    : null;
}

function callbackStatusValue(
  value: unknown
): LlmToolCallback['callbackStatus'] {
  return value === 'returned' || value === 'cancelled'
    ? value
    : 'waiting_callback';
}

function executionStatusValue(
  value: unknown
): LlmToolCallback['executionStatus'] {
  return value === 'succeeded' ||
    value === 'failed' ||
    value === 'timed_out' ||
    value === 'cancelled' ||
    value === 'unknown'
    ? value
    : 'unknown';
}

function nullableRoundIndex(value: unknown): number | null {
  return typeof value === 'number' && Number.isFinite(value) ? value : null;
}

function nullableTokenCount(value: unknown): number | null {
  return typeof value === 'number' && Number.isFinite(value) ? value : null;
}

function tokenCountMethod(value: unknown): 'estimated' | null {
  return value === 'estimated' ? value : null;
}

function readIndexedToolCallbacks(debugPayload: unknown): LlmToolCallback[] {
  if (
    !isRecord(debugPayload) ||
    !isRuntimeDebugArtifactPreview(debugPayload.llm_rounds)
  ) {
    return [];
  }

  return recordArray(debugPayload.llm_rounds.tool_callbacks).map(
    (toolCallback, index) => {
      const id =
        firstStringField(toolCallback, ['id', 'tool_call_id', 'call_id']) ??
        `tool-callback-${index + 1}`;

      return {
        key: `${id}-${index}`,
        id,
        name: firstStringField(toolCallback, ['name']) ?? 'Tool',
        callbackStatus: callbackStatusValue(toolCallback.callback_status),
        executionStatus: executionStatusValue(toolCallback.execution_status),
        requestPayload: {},
        callbackPayload: null,
        parsedResult: null,
        requestRoundIndex: nullableRoundIndex(toolCallback.request_round_index),
        resultRoundIndex: nullableRoundIndex(toolCallback.result_round_index),
        call_input_tokens: nullableTokenCount(toolCallback.call_input_tokens),
        call_cached_input_tokens: nullableTokenCount(
          toolCallback.call_cached_input_tokens
        ),
        call_output_tokens: nullableTokenCount(toolCallback.call_output_tokens),
        result_input_tokens: nullableTokenCount(
          toolCallback.result_input_tokens
        ),
        result_context_input_tokens: nullableTokenCount(
          toolCallback.result_context_input_tokens
        ),
        result_context_cached_input_tokens: nullableTokenCount(
          toolCallback.result_context_cached_input_tokens
        ),
        token_count_method: tokenCountMethod(toolCallback.token_count_method),
        detailArtifactRef: firstStringField(toolCallback, [
          'artifact_ref',
          'detail_artifact_ref'
        ])
      };
    }
  );
}

export function readLlmToolCallbackDetail(
  loadedPayload: unknown
): Omit<LlmToolCallback, 'key'> | null {
  if (!isRecord(loadedPayload)) {
    return null;
  }

  const id = firstStringField(loadedPayload, ['id', 'tool_call_id', 'call_id']);

  if (!id) {
    return null;
  }

  return {
    id,
    name: firstStringField(loadedPayload, ['name']) ?? 'Tool',
    callbackStatus: callbackStatusValue(loadedPayload.callback_status),
    executionStatus: executionStatusValue(loadedPayload.execution_status),
    requestPayload:
      optionalRecordField(loadedPayload, [
        'request_payload',
        'requestPayload'
      ]) ?? {},
    callbackPayload: optionalRecordField(loadedPayload, [
      'callback_payload',
      'callbackPayload'
    ]),
    parsedResult: optionalRecordField(loadedPayload, [
      'parsed_result',
      'parsedResult'
    ]),
    requestRoundIndex: nullableRoundIndex(loadedPayload.request_round_index),
    resultRoundIndex: nullableRoundIndex(loadedPayload.result_round_index),
    call_input_tokens: nullableTokenCount(loadedPayload.call_input_tokens),
    call_cached_input_tokens: nullableTokenCount(
      loadedPayload.call_cached_input_tokens
    ),
    call_output_tokens: nullableTokenCount(loadedPayload.call_output_tokens),
    result_input_tokens: nullableTokenCount(loadedPayload.result_input_tokens),
    result_context_input_tokens: nullableTokenCount(
      loadedPayload.result_context_input_tokens
    ),
    result_context_cached_input_tokens: nullableTokenCount(
      loadedPayload.result_context_cached_input_tokens
    ),
    token_count_method: tokenCountMethod(loadedPayload.token_count_method),
    detailArtifactRef:
      firstStringField(loadedPayload, [
        'artifact_ref',
        'detail_artifact_ref'
      ]) ?? null
  };
}

export function debugPayloadWithLoadedLlmRounds(
  debugPayload: unknown,
  loadedPayload: unknown
): Record<string, unknown> {
  const loadedRounds =
    isRecord(loadedPayload) && Array.isArray(loadedPayload.llm_rounds)
      ? loadedPayload.llm_rounds
      : loadedPayload;

  return {
    ...(isRecord(debugPayload) ? debugPayload : {}),
    llm_rounds: loadedRounds
  };
}

export function stripLlmRoundsFromDebugPayload(debugPayload: unknown) {
  if (
    !isRecord(debugPayload) ||
    !Object.prototype.hasOwnProperty.call(debugPayload, 'llm_rounds')
  ) {
    return debugPayload;
  }

  return Object.fromEntries(
    Object.entries(debugPayload).filter(([key]) => key !== 'llm_rounds')
  );
}

function readRoundToolCalls(round: Record<string, unknown>) {
  const assistant = firstRecordField(round, ['assistant', 'assistant_message']);
  const assistantToolCalls = recordArray(assistant.tool_calls);

  if (assistantToolCalls.length > 0) {
    return assistantToolCalls;
  }

  return recordArray(round.tool_calls);
}

function readRoundToolResults(round: Record<string, unknown>) {
  return recordArray(round.tool_results);
}

function toolCallId(
  toolCall: Record<string, unknown>,
  roundNumber: number,
  toolCallIndex: number
) {
  return (
    firstStringField(toolCall, ['id', 'tool_call_id', 'call_id']) ??
    `tool-${roundNumber + 1}-${toolCallIndex + 1}`
  );
}

function toolResultId(
  toolResult: Record<string, unknown>,
  roundNumber: number,
  toolResultIndex: number
) {
  return (
    firstStringField(toolResult, ['tool_call_id', 'id', 'call_id']) ??
    `tool-result-${roundNumber + 1}-${toolResultIndex + 1}`
  );
}

function callbackStatus(
  callbackPayload: Record<string, unknown> | null
): LlmToolCallback['callbackStatus'] {
  return callbackPayload ? 'returned' : 'waiting_callback';
}

function normalizedExecutionStatus(
  status: unknown
): LlmToolCallback['executionStatus'] | null {
  if (typeof status !== 'string') {
    return null;
  }

  if (status === 'canceled') {
    return 'cancelled';
  }

  return status === 'succeeded' ||
    status === 'failed' ||
    status === 'timed_out' ||
    status === 'cancelled' ||
    status === 'unknown'
    ? status
    : null;
}

function executionStatusFromCallbackPayload(
  callbackPayload: Record<string, unknown> | null
): LlmToolCallback['executionStatus'] {
  if (!callbackPayload) {
    return 'unknown';
  }

  const execution = isRecord(callbackPayload.execution)
    ? callbackPayload.execution
    : null;
  const executionStatus =
    normalizedExecutionStatus(execution?.status) ??
    normalizedExecutionStatus(callbackPayload.execution_status);

  if (executionStatus) {
    return executionStatus;
  }
  if (callbackPayload.timed_out === true) {
    return 'timed_out';
  }
  if (callbackPayload.cancelled === true) {
    return 'cancelled';
  }
  if (typeof callbackPayload.exit_code === 'number') {
    return callbackPayload.exit_code === 0 ? 'succeeded' : 'failed';
  }
  if (typeof callbackPayload.http_status === 'number') {
    return callbackPayload.http_status >= 200 &&
      callbackPayload.http_status < 300
      ? 'succeeded'
      : 'failed';
  }
  if (
    callbackPayload.is_error === true ||
    (Object.prototype.hasOwnProperty.call(callbackPayload, 'error') &&
      callbackPayload.error !== null &&
      callbackPayload.error !== undefined)
  ) {
    return 'failed';
  }

  return 'unknown';
}

function parsedResultFromCallbackPayload(
  callbackPayload: Record<string, unknown> | null
): Record<string, unknown> | null {
  if (!callbackPayload) {
    return null;
  }

  const parsedEntries = [
    'tool_call_id',
    'id',
    'call_id',
    'name',
    'content',
    'stdout',
    'stderr',
    'error',
    'exit_code',
    'http_status',
    'is_error',
    'timed_out',
    'cancelled',
    'execution',
    'execution_status'
  ]
    .filter((key) => Object.prototype.hasOwnProperty.call(callbackPayload, key))
    .map((key) => [key, callbackPayload[key]] as const);

  return Object.fromEntries(parsedEntries);
}

export function collectLlmToolCallbacks(
  debugPayload: unknown
): LlmToolCallback[] {
  return collectLlmToolCallbacksFromDebugPayloads([debugPayload]);
}

export function collectLlmToolCallbacksFromDebugPayloads(
  debugPayloads: unknown[]
): LlmToolCallback[] {
  return mergeLlmToolCallbacks([
    ...debugPayloads.flatMap(readIndexedToolCallbacks),
    ...collectLlmToolCallbacksFromRounds(
      debugPayloads.flatMap(readLlmRoundsFromDebugPayload)
    )
  ]);
}

function mergeLlmToolCallbacks(callbacks: LlmToolCallback[]) {
  const merged: LlmToolCallback[] = [];
  const callbackIndexById = new Map<string, number>();

  for (const callback of callbacks) {
    const callbackIndex = callbackIndexById.get(callback.id);

    if (callbackIndex === undefined) {
      callbackIndexById.set(callback.id, merged.length);
      merged.push(callback);
      continue;
    }

    const currentCallback = merged[callbackIndex];
    const requestPayload =
      Object.keys(callback.requestPayload).length > 0
        ? callback.requestPayload
        : currentCallback.requestPayload;
    const callbackPayload =
      callback.callbackPayload ?? currentCallback.callbackPayload;
    const parsedResult =
      callback.parsedResult ??
      currentCallback.parsedResult ??
      parsedResultFromCallbackPayload(callbackPayload);

    merged[callbackIndex] = {
      ...currentCallback,
      name:
        currentCallback.name === 'Tool' && callback.name !== 'Tool'
          ? callback.name
          : currentCallback.name,
      callbackStatus:
        callback.callbackStatus !== 'waiting_callback'
          ? callback.callbackStatus
          : currentCallback.callbackStatus !== 'waiting_callback'
            ? currentCallback.callbackStatus
            : callbackStatus(callbackPayload),
      executionStatus:
        callback.executionStatus !== 'unknown'
          ? callback.executionStatus
          : currentCallback.executionStatus !== 'unknown'
            ? currentCallback.executionStatus
            : executionStatusFromCallbackPayload(callbackPayload),
      requestPayload,
      callbackPayload,
      parsedResult,
      requestRoundIndex:
        callback.requestRoundIndex ?? currentCallback.requestRoundIndex,
      resultRoundIndex:
        callback.resultRoundIndex ?? currentCallback.resultRoundIndex,
      detailArtifactRef:
        callback.detailArtifactRef ?? currentCallback.detailArtifactRef,
      call_input_tokens:
        callback.call_input_tokens ?? currentCallback.call_input_tokens,
      call_cached_input_tokens:
        callback.call_cached_input_tokens ??
        currentCallback.call_cached_input_tokens,
      call_output_tokens:
        callback.call_output_tokens ?? currentCallback.call_output_tokens,
      result_input_tokens:
        callback.result_input_tokens ?? currentCallback.result_input_tokens,
      result_context_input_tokens:
        callback.result_context_input_tokens ??
        currentCallback.result_context_input_tokens,
      result_context_cached_input_tokens:
        callback.result_context_cached_input_tokens ??
        currentCallback.result_context_cached_input_tokens,
      token_count_method:
        callback.token_count_method ?? currentCallback.token_count_method
    };
  }

  return merged;
}

function collectLlmToolCallbacksFromRounds(
  rounds: Record<string, unknown>[]
): LlmToolCallback[] {
  const callbacks: LlmToolCallback[] = [];
  const callbackIndexById = new Map<string, number>();

  const upsertCallback = (
    id: string,
    nextCallback: Omit<
      LlmToolCallback,
      'key' | 'callbackStatus' | 'executionStatus' | 'parsedResult'
    >
  ) => {
    const callbackIndex = callbackIndexById.get(id);

    if (callbackIndex === undefined) {
      callbackIndexById.set(id, callbacks.length);
      callbacks.push({
        ...nextCallback,
        key: `${id}-${callbacks.length}`,
        callbackStatus: callbackStatus(nextCallback.callbackPayload),
        executionStatus: executionStatusFromCallbackPayload(
          nextCallback.callbackPayload
        ),
        parsedResult: parsedResultFromCallbackPayload(
          nextCallback.callbackPayload
        )
      });
      return;
    }

    const currentCallback = callbacks[callbackIndex];
    const requestPayload =
      Object.keys(nextCallback.requestPayload).length > 0
        ? nextCallback.requestPayload
        : currentCallback.requestPayload;
    const callbackPayload =
      nextCallback.callbackPayload ?? currentCallback.callbackPayload;
    const parsedResult =
      currentCallback.parsedResult ??
      parsedResultFromCallbackPayload(callbackPayload);

    callbacks[callbackIndex] = {
      ...currentCallback,
      name:
        currentCallback.name === 'Tool' && nextCallback.name !== 'Tool'
          ? nextCallback.name
          : currentCallback.name,
      requestPayload,
      callbackPayload,
      parsedResult,
      requestRoundIndex:
        nextCallback.requestRoundIndex ?? currentCallback.requestRoundIndex,
      resultRoundIndex:
        nextCallback.resultRoundIndex ?? currentCallback.resultRoundIndex,
      callbackStatus: callbackStatus(callbackPayload),
      executionStatus: executionStatusFromCallbackPayload(callbackPayload),
      call_input_tokens:
        nextCallback.call_input_tokens ?? currentCallback.call_input_tokens,
      call_cached_input_tokens:
        nextCallback.call_cached_input_tokens ??
        currentCallback.call_cached_input_tokens,
      call_output_tokens:
        nextCallback.call_output_tokens ?? currentCallback.call_output_tokens,
      result_input_tokens:
        nextCallback.result_input_tokens ?? currentCallback.result_input_tokens,
      result_context_input_tokens:
        nextCallback.result_context_input_tokens ??
        currentCallback.result_context_input_tokens,
      result_context_cached_input_tokens:
        nextCallback.result_context_cached_input_tokens ??
        currentCallback.result_context_cached_input_tokens,
      token_count_method:
        nextCallback.token_count_method ?? currentCallback.token_count_method
    };
  };

  rounds.forEach((round, fallbackRoundIndex) => {
    const currentRoundIndex = roundIndex(round, fallbackRoundIndex);

    readRoundToolCalls(round).forEach((toolCall, toolCallIndex) => {
      const id = toolCallId(toolCall, currentRoundIndex, toolCallIndex);

      upsertCallback(id, {
        id,
        name: firstStringField(toolCall, ['name']) ?? 'Tool',
        requestPayload: toolCall,
        callbackPayload: null,
        requestRoundIndex: currentRoundIndex,
        resultRoundIndex: null,
        call_input_tokens: nullableTokenCount(toolCall.call_input_tokens),
        call_cached_input_tokens: nullableTokenCount(
          toolCall.call_cached_input_tokens
        ),
        call_output_tokens: nullableTokenCount(toolCall.call_output_tokens),
        result_input_tokens: nullableTokenCount(toolCall.result_input_tokens),
        result_context_input_tokens: nullableTokenCount(
          toolCall.result_context_input_tokens
        ),
        result_context_cached_input_tokens: nullableTokenCount(
          toolCall.result_context_cached_input_tokens
        ),
        token_count_method: tokenCountMethod(toolCall.token_count_method)
      });
    });

    readRoundToolResults(round).forEach((toolResult, toolResultIndex) => {
      const id = toolResultId(toolResult, currentRoundIndex, toolResultIndex);

      upsertCallback(id, {
        id,
        name: firstStringField(toolResult, ['name']) ?? 'Tool',
        requestPayload: {},
        callbackPayload: toolResult,
        requestRoundIndex: null,
        resultRoundIndex: currentRoundIndex,
        call_input_tokens: nullableTokenCount(toolResult.call_input_tokens),
        call_cached_input_tokens: nullableTokenCount(
          toolResult.call_cached_input_tokens
        ),
        call_output_tokens: nullableTokenCount(toolResult.call_output_tokens),
        result_input_tokens: nullableTokenCount(toolResult.result_input_tokens),
        result_context_input_tokens: nullableTokenCount(
          toolResult.result_context_input_tokens
        ),
        result_context_cached_input_tokens: nullableTokenCount(
          toolResult.result_context_cached_input_tokens
        ),
        token_count_method: tokenCountMethod(toolResult.token_count_method)
      });
    });
  });

  return callbacks;
}
