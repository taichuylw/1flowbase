import { useEffect, useMemo, useRef, useState } from 'react';

import { useAuthStore } from '../../../state/auth-store';
import { patchUserPreferences } from '../../user-preferences/user-preferences';
import {
  getDefaultColumnWidths,
  getDefaultVisibleKeys,
  normalizeDataTableState,
  type DataTableColumn,
  type DataTableConfiguration,
  type DataTableState
} from './data-table-state';

const DATA_TABLES_META_PATH = ['ui', 'data_tables'] as const;
const SAVE_DELAY_MS = 250;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function readDataTableStateFromMeta<T extends object>(
  meta: Record<string, unknown> | undefined,
  preferenceKey: string,
  columns: Array<DataTableColumn<T>>
): DataTableState {
  const uiValue = meta?.[DATA_TABLES_META_PATH[0]];
  const ui = isRecord(uiValue) ? uiValue : undefined;
  const dataTablesValue = ui?.[DATA_TABLES_META_PATH[1]];
  const dataTables = isRecord(dataTablesValue) ? dataTablesValue : undefined;
  const tableState = isRecord(dataTables?.[preferenceKey])
    ? (dataTables[preferenceKey] as Partial<DataTableState>)
    : undefined;

  return normalizeDataTableState(columns, tableState);
}

function buildDataTableMetaPatch(
  preferenceKey: string,
  state: DataTableState
): Record<string, unknown> {
  return {
    ui: {
      data_tables: {
        [preferenceKey]: state
      }
    }
  };
}

export function useUserPreferenceDataTableConfiguration<T extends object>({
  columns,
  preferenceKey
}: {
  columns: Array<DataTableColumn<T>>;
  preferenceKey: string;
}): DataTableConfiguration {
  const csrfToken = useAuthStore((state) => state.csrfToken);
  const me = useAuthStore((state) => state.me);
  const setMe = useAuthStore((state) => state.setMe);
  const initialState = useMemo(
    () => readDataTableStateFromMeta(me?.meta, preferenceKey, columns),
    [columns, me?.meta, preferenceKey]
  );
  const [visibleColumnKeys, setVisibleColumnKeys] = useState<string[]>(
    () => initialState.visibleColumnKeys
  );
  const [columnWidths, setColumnWidths] = useState<Record<string, number>>(
    () => initialState.columnWidths
  );
  const hydratingRef = useRef(true);

  useEffect(() => {
    const state = readDataTableStateFromMeta(me?.meta, preferenceKey, columns);
    hydratingRef.current = true;
    setVisibleColumnKeys(state.visibleColumnKeys);
    setColumnWidths(state.columnWidths);
  }, [columns, me?.meta, preferenceKey]);

  useEffect(() => {
    if (hydratingRef.current) {
      hydratingRef.current = false;
      return;
    }

    if (!csrfToken) {
      return;
    }

    const state = {
      visibleColumnKeys:
        visibleColumnKeys.length > 0
          ? visibleColumnKeys
          : getDefaultVisibleKeys(columns),
      columnWidths:
        Object.keys(columnWidths).length > 0
          ? columnWidths
          : getDefaultColumnWidths(columns)
    };
    const timeout = window.setTimeout(() => {
      void patchUserPreferences(
        buildDataTableMetaPatch(preferenceKey, state),
        csrfToken
      )
        .then((updatedMe) => {
          setMe(updatedMe);
        })
        .catch(() => {
          // Preferences are non-critical UI state; keep the in-memory choice.
        });
    }, SAVE_DELAY_MS);

    return () => {
      window.clearTimeout(timeout);
    };
  }, [
    columnWidths,
    columns,
    csrfToken,
    preferenceKey,
    setMe,
    visibleColumnKeys
  ]);

  return {
    visibleColumnKeys,
    columnWidths,
    setVisibleColumnKeys,
    setColumnWidths
  };
}
