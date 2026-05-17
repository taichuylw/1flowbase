import { useQueries } from '@tanstack/react-query';
import { useMemo } from 'react';

import {
  fetchFrontstageBlockCode,
  frontstageBlockCodeQueryKey
} from '../api/block-code';
import type { FrontstagePageRenderPlan } from '../lib/page-canvas/render-plan';
import {
  createFrontstagePageCanvasBlockCodeReadPlan,
  createFrontstagePageCanvasRuntimeSourceState,
  type FrontstagePageCanvasBlockCodeReadPlan,
  type FrontstagePageCanvasBlockCodeReadRequest,
  type FrontstagePageCanvasBlockCodeReadResult,
  type FrontstagePageCanvasRuntimeSourceState
} from '../lib/page-canvas/runtime-source';

export interface UseFrontstagePageCanvasRuntimeSourcesInput {
  workspaceId: string | null | undefined;
  renderPlan: FrontstagePageRenderPlan | null | undefined;
}

export interface UseFrontstagePageCanvasRuntimeSourcesResult {
  readPlan: FrontstagePageCanvasBlockCodeReadPlan | null;
  sourceState: FrontstagePageCanvasRuntimeSourceState | null;
  loading: boolean;
  hasError: boolean;
  errors: Error[];
}

function toError(error: unknown): Error {
  return error instanceof Error
    ? error
    : new Error('frontstage page canvas block code request failed');
}

function isNonEmptyCode(code: unknown): code is string {
  return typeof code === 'string' && code.trim().length > 0;
}

function createCodeResult(
  request: FrontstagePageCanvasBlockCodeReadRequest,
  query: {
    data?: { code?: unknown };
    error: unknown;
    isError: boolean;
  }
): FrontstagePageCanvasBlockCodeReadResult {
  if (query.isError) {
    return {
      codeRef: request.codeRef,
      status: 'failed',
      error: query.error
    };
  }

  if (query.data) {
    if (isNonEmptyCode(query.data.code)) {
      return {
        codeRef: request.codeRef,
        status: 'ready',
        code: query.data.code
      };
    }

    return {
      codeRef: request.codeRef,
      status: 'missing',
      message: `Block code is empty for ${request.codeRef}.`
    };
  }

  return {
    codeRef: request.codeRef,
    status: 'loading'
  };
}

export function useFrontstagePageCanvasRuntimeSources({
  workspaceId,
  renderPlan
}: UseFrontstagePageCanvasRuntimeSourcesInput): UseFrontstagePageCanvasRuntimeSourcesResult {
  const readPlan = useMemo(() => {
    if (!workspaceId || !renderPlan) {
      return null;
    }

    return createFrontstagePageCanvasBlockCodeReadPlan({
      workspaceId,
      renderPlan
    });
  }, [renderPlan, workspaceId]);
  const requests = readPlan?.requests ?? [];

  const blockCodeQueries = useQueries({
    queries: requests.map((request) => ({
      queryKey: frontstageBlockCodeQueryKey(
        request.workspaceId,
        request.pageId,
        request.codeRef
      ),
      queryFn: () =>
        fetchFrontstageBlockCode(
          request.workspaceId,
          request.pageId,
          request.codeRef
        )
    }))
  });

  const codeResults = useMemo(
    () =>
      requests.map((request, index) =>
        createCodeResult(request, blockCodeQueries[index])
      ),
    [blockCodeQueries, requests]
  );

  const sourceState = useMemo(() => {
    if (!renderPlan || !readPlan) {
      return null;
    }

    return createFrontstagePageCanvasRuntimeSourceState({
      renderPlan,
      readPlan,
      codeResults
    });
  }, [codeResults, readPlan, renderPlan]);

  const errors = useMemo(
    () =>
      blockCodeQueries
        .filter((query) => query.isError)
        .map((query) => toError(query.error)),
    [blockCodeQueries]
  );

  return {
    readPlan,
    sourceState,
    loading: blockCodeQueries.some((query) => query.isFetching),
    hasError: errors.length > 0,
    errors
  };
}
