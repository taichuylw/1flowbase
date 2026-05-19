import { type DataTableConfiguration } from '../../../../shared/ui/data-table/data-table-state';
import { useUserPreferenceDataTableConfiguration } from '../../../../shared/ui/data-table/user-preference-data-table';
import type { ApplicationRunSummary } from '../../api/runtime';
import { APPLICATION_RUNS_TABLE_COLUMNS } from './application-runs-table-columns';

const USER_PREFERENCE_KEY = 'applications.logs.runs';

export type ApplicationRunsTableConfiguration = DataTableConfiguration;

export function useApplicationRunsTableConfiguration(): ApplicationRunsTableConfiguration {
  return useUserPreferenceDataTableConfiguration<ApplicationRunSummary>({
    columns: APPLICATION_RUNS_TABLE_COLUMNS,
    preferenceKey: USER_PREFERENCE_KEY
  });
}
