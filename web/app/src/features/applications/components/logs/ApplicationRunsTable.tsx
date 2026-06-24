import { FileZipOutlined } from '@ant-design/icons';
import { Button, Space, Tooltip } from 'antd';
import { useMemo } from 'react';
import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';

import {
  DataTable,
  DataTableColumnSettings,
  type DataTableColumn,
  type DataTableRowSelection
} from '../../../../shared/ui/data-table/DataTable';
import type { ApplicationRunSummary } from '../../api/runtime';
import type { ApplicationRunsTableConfiguration } from './useApplicationRunsTableConfiguration';

export function ApplicationRunsTableColumnSettings({
  columns,
  configuration
}: {
  columns: Array<DataTableColumn<ApplicationRunSummary>>;
  configuration: ApplicationRunsTableConfiguration;
}) {
  return (
    <DataTableColumnSettings<ApplicationRunSummary>
      className="application-runs-table__column-selector"
      columns={columns}
      configuration={configuration}
    />
  );
}

export function ApplicationRunsTable({
  loading = false,
  page,
  pageSize,
  total,
  runs,
  columns,
  rowSelection,
  selectedRunId,
  configuration,
  exportingArchiveRunId,
  onPageChange,
  onExportRunArchive,
  onSelectRun
}: {
  loading?: boolean;
  page: number;
  pageSize: number;
  total: number;
  runs: ApplicationRunSummary[];
  columns: Array<DataTableColumn<ApplicationRunSummary>>;
  rowSelection?: DataTableRowSelection<ApplicationRunSummary>;
  selectedRunId?: string | null;
  configuration: ApplicationRunsTableConfiguration;
  exportingArchiveRunId?: string | null;
  onPageChange: (page: number) => void;
  onExportRunArchive?: (run: ApplicationRunSummary) => void;
  onSelectRun: (run: ApplicationRunSummary) => void;
}) {
  const { t } = useTranslation('applications');
  const tableColumns = useMemo<Array<DataTableColumn<ApplicationRunSummary>>>(
    () =>
      columns.map((column) => {
        if (column.key !== 'action') {
          return column;
        }

        return {
          ...column,
          render: (_value: unknown, run: ApplicationRunSummary): ReactNode => (
            <Space size={4}>
              <Button type="link" onClick={() => onSelectRun(run)}>
                {t('auto.view_run_details')}
              </Button>
              {onExportRunArchive ? (
                <Tooltip
                  title={t('auto.export_run_archive_named', {
                    value1: run.title || run.id
                  })}
                >
                  <Button
                    aria-label={t('auto.export_run_archive_named', {
                      value1: run.title || run.id
                    })}
                    icon={<FileZipOutlined aria-hidden="true" />}
                    loading={exportingArchiveRunId === run.id}
                    onClick={() => onExportRunArchive(run)}
                    type="text"
                  />
                </Tooltip>
              ) : null}
            </Space>
          )
        };
      }),
    [columns, exportingArchiveRunId, onExportRunArchive, onSelectRun, t]
  );

  return (
    <DataTable<ApplicationRunSummary>
      className="application-runs-table"
      columns={tableColumns}
      configuration={configuration}
      dataSource={runs}
      loading={loading}
      page={page}
      pageSize={pageSize}
      rowClassName={(record) =>
        record.id === selectedRunId ? 'application-runs-table__row--active' : ''
      }
      rowKey={(record) => record.id}
      rowSelection={rowSelection}
      total={total}
      onPageChange={onPageChange}
    />
  );
}
