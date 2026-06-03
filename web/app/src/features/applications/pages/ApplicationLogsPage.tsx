import {
  ReloadOutlined,
  SearchOutlined,
  SortAscendingOutlined,
  SortDescendingOutlined
} from '@ant-design/icons';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Alert, App, Button, Empty, Input, Spin, Tooltip } from 'antd';
import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { AutosizeSelect } from '../../../shared/ui/autosize-select/AutosizeSelect';
import type { AgentFlowDebugMessage } from '../../agent-flow/api/runtime';
import { ConversationLogPanel } from '../../agent-flow/components/debug-console/ConversationLogPanel';
import {
  applicationRunsQueryKey,
  fetchApplicationRuns,
  type FetchApplicationRunsInput,
  fetchRuntimeDebugArtifact,
  type ApplicationRunSortField,
  type ApplicationRunSortOrder,
  type ApplicationRunSummary
} from '../api/runtime';
import { ApplicationRunDetailPanel } from '../components/logs/ApplicationRunDetailPanel';
import { ApplicationLogsFloatingWindow } from '../components/logs/ApplicationLogsFloatingWindow';
import { ApplicationRunResumeTimelinePanel } from '../components/logs/ApplicationRunResumeTimelinePanel';
import {
  clampRect,
  applyStoredWidth,
  DEFAULT_MIN_WIDTH,
  DEFAULT_MIN_HEIGHT,
  type FloatingWindowRect
} from '../components/logs/floating-window-geometry';
import { getApplicationRunsTableColumns } from '../components/logs/application-runs-table-columns';
import {
  ApplicationRunsTable,
  ApplicationRunsTableColumnSettings
} from '../components/logs/ApplicationRunsTable';
import { useApplicationRunsTableConfiguration } from '../components/logs/useApplicationRunsTableConfiguration';
import { isActiveRunStatus } from '../lib/run-status';
import './application-logs-page.css';

const FLOATING_WINDOW_TOP = 112;
const FLOATING_WINDOW_GAP = 16;
const FLOATING_WINDOW_RIGHT = 32;
const FLOATING_WINDOW_MIN_WIDTH = 360;
const FLOATING_WINDOW_MAX_HEIGHT = 720;
const ACTIVE_RUNS_REFETCH_INTERVAL_MS = 2_000;
const DEFAULT_TIME_RANGE = '7';
const PAGE_SIZE = 20;

type ApplicationLogTimeRange = '1' | '7' | '28' | '90' | '365' | 'all';
type ApplicationLogsFloatingWindowKind =
  | 'conversation-log'
  | 'resume-timeline'
  | 'run-detail';

const TIME_RANGE_OPTIONS: Array<{
  labelKey: string;
  value: ApplicationLogTimeRange;
}> = [
  { labelKey: 'auto.today', value: '1' },
  { labelKey: 'auto.past_seven_days', value: '7' },
  { labelKey: 'auto.past_four_weeks', value: '28' },
  { labelKey: 'auto.past_three_months', value: '90' },
  { labelKey: 'auto.past_twelve_months', value: '365' },
  { labelKey: 'auto.all_time', value: 'all' }
];
const RUN_SORT_FIELD_OPTIONS: Array<{
  labelKey: string;
  value: ApplicationRunSortField;
}> = [
  { labelKey: 'auto.start_time', value: 'started_at' },
  { labelKey: 'auto.updated_at', value: 'updated_at' }
];
const DEFAULT_SORT_BY: ApplicationRunSortField = 'started_at';
const DEFAULT_SORT_ORDER: ApplicationRunSortOrder = 'desc';

function getSortOrderToggleLabel(
  sortOrder: ApplicationRunSortOrder,
  t: (key: string) => string
) {
  return sortOrder === 'desc'
    ? t('auto.sort_descending_toggle_to_ascending')
    : t('auto.sort_ascending_toggle_to_descending');
}

function getViewportSize() {
  if (typeof window === 'undefined') {
    return { width: 1280, height: 720 };
  }

  return {
    width: window.innerWidth,
    height: window.innerHeight
  };
}

function getFloatingWindowHeight() {
  const viewport = getViewportSize();

  return Math.max(
    320,
    Math.min(
      FLOATING_WINDOW_MAX_HEIGHT,
      viewport.height - FLOATING_WINDOW_TOP - FLOATING_WINDOW_RIGHT
    )
  );
}

function getRunDetailInitialRect() {
  const viewport = getViewportSize();

  return {
    left: viewport.width - FLOATING_WINDOW_MIN_WIDTH - FLOATING_WINDOW_RIGHT,
    top: FLOATING_WINDOW_TOP,
    width: FLOATING_WINDOW_MIN_WIDTH,
    height: getFloatingWindowHeight()
  };
}

function getConversationLogInitialRect() {
  const runDetailRect = getRunDetailInitialRect();

  return {
    left: runDetailRect.left - FLOATING_WINDOW_MIN_WIDTH - FLOATING_WINDOW_GAP,
    top: FLOATING_WINDOW_TOP,
    width: FLOATING_WINDOW_MIN_WIDTH,
    height: getFloatingWindowHeight()
  };
}

function getResumeTimelineInitialRect() {
  return getConversationLogInitialRect();
}

function resolveCollision(
  rectA: FloatingWindowRect,
  rectB: FloatingWindowRect,
  viewportWidth: number,
  minWidthB: number = DEFAULT_MIN_WIDTH,
  gap: number = FLOATING_WINDOW_GAP,
  margin: number = 8
): { rectA: FloatingWindowRect; rectB: FloatingWindowRect } {
  let nextLeftB = rectA.left - rectB.width - gap;

  if (nextLeftB < margin) {
    nextLeftB = margin;
    const availableWidthB = rectA.left - margin - gap;
    let nextWidthB = rectB.width;
    if (availableWidthB < rectB.width) {
      nextWidthB = Math.max(minWidthB, availableWidthB);
    }

    const overlap = nextLeftB + nextWidthB + gap - rectA.left;
    let nextLeftA = rectA.left;
    if (overlap > 0) {
      nextLeftA = Math.min(
        viewportWidth - rectA.width - margin,
        rectA.left + overlap
      );

      const newAvailableWidthB = nextLeftA - margin - gap;
      nextWidthB = Math.max(
        minWidthB,
        Math.min(rectB.width, newAvailableWidthB)
      );
      nextLeftB = Math.max(margin, nextLeftA - nextWidthB - gap);
    }

    return {
      rectA: { ...rectA, left: nextLeftA },
      rectB: { ...rectB, left: nextLeftB, width: nextWidthB }
    };
  } else {
    return {
      rectA,
      rectB: { ...rectB, left: nextLeftB }
    };
  }
}

export function ApplicationLogsPage({
  applicationId
}: {
  applicationId: string;
}) {
  const { t } = useTranslation('applications');
  const [selectedRunId, setSelectedRunId] = useState<string | null>(null);
  const [openConversationLogMessage, setOpenConversationLogMessage] =
    useState<AgentFlowDebugMessage | null>(null);
  const [openResumeTimelineRunId, setOpenResumeTimelineRunId] = useState<
    string | null
  >(null);
  const [runDetailRect, setRunDetailRect] = useState<FloatingWindowRect | null>(
    null
  );
  const [conversationLogRect, setConversationLogRect] =
    useState<FloatingWindowRect | null>(null);
  const [resumeTimelineRect, setResumeTimelineRect] =
    useState<FloatingWindowRect | null>(null);
  const [timeRange, setTimeRange] =
    useState<ApplicationLogTimeRange>(DEFAULT_TIME_RANGE);

  useEffect(() => {
    function handleViewportResize() {
      if (runDetailRect) {
        setRunDetailRect(
          clampRect(runDetailRect, DEFAULT_MIN_WIDTH, DEFAULT_MIN_HEIGHT)
        );
      }
      if (conversationLogRect) {
        setConversationLogRect(
          clampRect(conversationLogRect, DEFAULT_MIN_WIDTH, DEFAULT_MIN_HEIGHT)
        );
      }
      if (resumeTimelineRect) {
        setResumeTimelineRect(
          clampRect(resumeTimelineRect, DEFAULT_MIN_WIDTH, DEFAULT_MIN_HEIGHT)
        );
      }
    }

    window.addEventListener('resize', handleViewportResize);
    return () => window.removeEventListener('resize', handleViewportResize);
  }, [runDetailRect, conversationLogRect, resumeTimelineRect]);
  const [keywordSearch, setKeywordSearch] = useState('');
  const [page, setPage] = useState(1);
  const [sortBy, setSortBy] =
    useState<ApplicationRunSortField>(DEFAULT_SORT_BY);
  const [sortOrder, setSortOrder] =
    useState<ApplicationRunSortOrder>(DEFAULT_SORT_ORDER);
  const [refreshingRuns, setRefreshingRuns] = useState(false);
  const [activeFloatingWindow, setActiveFloatingWindow] =
    useState<ApplicationLogsFloatingWindowKind>('run-detail');
  const { message } = App.useApp();
  const queryClient = useQueryClient();
  const timeRangeOptions = useMemo(
    () =>
      TIME_RANGE_OPTIONS.map((option) => ({
        label: t(option.labelKey),
        value: option.value
      })),
    [t]
  );
  const runSortFieldOptions = useMemo(
    () =>
      RUN_SORT_FIELD_OPTIONS.map((option) => ({
        label: t(option.labelKey),
        value: option.value
      })),
    [t]
  );
  const runSortFieldMeasureLabels = useMemo(
    () =>
      runSortFieldOptions.map((option) =>
        t('auto.sort_by_value', { value1: option.label })
      ),
    [runSortFieldOptions, t]
  );
  const runsTableColumns = useMemo(
    () => getApplicationRunsTableColumns(t),
    [t]
  );
  const runsTableConfiguration =
    useApplicationRunsTableConfiguration(runsTableColumns);
  const titleIncludes = keywordSearch.trim();
  const runsInput: FetchApplicationRunsInput = {
    page,
    pageSize: PAGE_SIZE,
    timeRangeDays: timeRange === 'all' ? null : Number(timeRange),
    sortBy,
    sortOrder,
    titleIncludes: titleIncludes || undefined
  };
  const runsQuery = useQuery({
    queryKey: applicationRunsQueryKey(applicationId, runsInput),
    queryFn: () => fetchApplicationRuns(applicationId, runsInput)
  });
  const refetchRuns = runsQuery.refetch;
  const runsPage = runsQuery.data;
  const runs = useMemo(() => runsPage?.items ?? [], [runsPage?.items]);
  const total = runsPage?.total ?? 0;

  useEffect(() => {
    setPage(1);
  }, [applicationId, timeRange, sortBy, sortOrder, titleIncludes]);

  useEffect(() => {
    if (!runs.some((run) => isActiveRunStatus(run.status))) {
      return;
    }

    const intervalId = window.setInterval(() => {
      void refetchRuns();
    }, ACTIVE_RUNS_REFETCH_INTERVAL_MS);

    return () => window.clearInterval(intervalId);
  }, [runs, refetchRuns]);

  function selectRun(run: ApplicationRunSummary | null) {
    const nextRunId = run ? (run.flow_run_id ?? run.id) : null;
    setSelectedRunId(nextRunId);
    setOpenConversationLogMessage(null);
    setOpenResumeTimelineRunId(null);
    setActiveFloatingWindow('run-detail');

    if (nextRunId) {
      const initial = clampRect(
        applyStoredWidth(
          getRunDetailInitialRect(),
          'application-logs-floating-run-detail'
        ),
        DEFAULT_MIN_WIDTH,
        DEFAULT_MIN_HEIGHT
      );
      setRunDetailRect(initial);
    } else {
      setRunDetailRect(null);
    }
    setConversationLogRect(null);
    setResumeTimelineRect(null);
  }

  const handleRectChange = (
    type: ApplicationLogsFloatingWindowKind,
    newRect: FloatingWindowRect
  ) => {
    if (type === 'run-detail') {
      setRunDetailRect(newRect);
    } else if (type === 'conversation-log') {
      setConversationLogRect(newRect);
    } else {
      setResumeTimelineRect(newRect);
    }
  };

  function toggleSortOrder() {
    setSortOrder((current) => (current === 'desc' ? 'asc' : 'desc'));
  }

  async function refreshRunsFromDurable() {
    setRefreshingRuns(true);
    try {
      const refreshedRuns = await fetchApplicationRuns(applicationId, {
        ...runsInput,
        cacheMode: 'refresh'
      });
      queryClient.setQueryData(
        applicationRunsQueryKey(applicationId, runsInput),
        refreshedRuns
      );
    } catch {
      message.error(t('auto.refresh_failed'));
    } finally {
      setRefreshingRuns(false);
    }
  }

  function openConversationLog(message: AgentFlowDebugMessage) {
    setOpenConversationLogMessage(message);
    setActiveFloatingWindow('conversation-log');

    const initial = clampRect(
      applyStoredWidth(
        getConversationLogInitialRect(),
        'application-logs-floating-conversation-log'
      ),
      DEFAULT_MIN_WIDTH,
      DEFAULT_MIN_HEIGHT
    );
    const anchorRect = resumeTimelineRect ?? runDetailRect;
    if (anchorRect) {
      const viewport = getViewportSize();
      const resolved = resolveCollision(anchorRect, initial, viewport.width);

      if (resumeTimelineRect) {
        setResumeTimelineRect(resolved.rectA);
      } else {
        setRunDetailRect(resolved.rectA);
      }
      setConversationLogRect(resolved.rectB);
    } else {
      setConversationLogRect(initial);
    }
  }

  function openResumeTimeline() {
    if (!selectedRunId) {
      return;
    }

    setOpenResumeTimelineRunId(selectedRunId);
    setActiveFloatingWindow('resume-timeline');

    const initial = clampRect(
      applyStoredWidth(
        getResumeTimelineInitialRect(),
        'application-logs-floating-resume-timeline'
      ),
      DEFAULT_MIN_WIDTH,
      DEFAULT_MIN_HEIGHT
    );
    const anchorRect = conversationLogRect ?? runDetailRect;
    if (anchorRect) {
      const viewport = getViewportSize();
      const resolved = resolveCollision(anchorRect, initial, viewport.width);

      if (conversationLogRect) {
        setConversationLogRect(resolved.rectA);
      } else {
        setRunDetailRect(resolved.rectA);
      }
      setResumeTimelineRect(resolved.rectB);
    } else {
      setResumeTimelineRect(initial);
    }
  }

  const logsHeader = (
    <div className="application-logs-page__header">
      <div className="application-logs-page__filters" role="search">
        <AutosizeSelect<ApplicationLogTimeRange>
          aria-label={t('auto.time_range')}
          options={timeRangeOptions}
          value={timeRange}
          onChange={setTimeRange}
        />
        <span
          className="application-logs-page__sort-control"
          data-testid="application-logs-sort-control"
        >
          <AutosizeSelect<ApplicationRunSortField>
            aria-label={t('auto.sort_field')}
            autosizeLabels={runSortFieldMeasureLabels}
            className="application-logs-page__sort-select"
            options={runSortFieldOptions}
            prefix={
              <span className="application-logs-page__sort-select-prefix">
                {t('auto.sort_by_prefix')}
              </span>
            }
            value={sortBy}
            onChange={setSortBy}
          />
          <Button
            aria-label={getSortOrderToggleLabel(sortOrder, t)}
            className="application-logs-page__sort-direction-button"
            icon={
              sortOrder === 'desc' ? (
                <SortDescendingOutlined aria-hidden="true" />
              ) : (
                <SortAscendingOutlined aria-hidden="true" />
              )
            }
            onClick={toggleSortOrder}
          />
        </span>
        <Input
          allowClear
          aria-label={t('auto.keyword_search')}
          className="application-logs-page__filter-search"
          placeholder={t('auto.search_title')}
          prefix={<SearchOutlined />}
          value={keywordSearch}
          onChange={(event) => setKeywordSearch(event.target.value)}
        />
        <div className="application-logs-page__filter-actions">
          <Tooltip title={t('auto.refresh_logs')}>
            <Button
              aria-label={t('auto.refresh_logs')}
              icon={<ReloadOutlined aria-hidden="true" />}
              loading={refreshingRuns}
              onClick={() => {
                void refreshRunsFromDurable();
              }}
            />
          </Tooltip>
          <ApplicationRunsTableColumnSettings
            columns={runsTableColumns}
            configuration={runsTableConfiguration}
          />
        </div>
      </div>
    </div>
  );
  const logsList = (
    <section
      className="application-logs-page__list"
      data-testid="application-logs-list"
    >
      {runsQuery.isPending ? (
        <div className="application-logs-page__state" role="status">
          <Spin aria-hidden="true" />
          <span>{t('auto.logs_loading')}</span>
        </div>
      ) : runsQuery.isError ? (
        <Alert
          action={
            <Button
              size="small"
              onClick={() => {
                void runsQuery.refetch();
              }}
            >
              {t('auto.refresh_logs')}
            </Button>
          }
          description={t('auto.logs_load_failed_description')}
          message={t('auto.logs_load_failed')}
          showIcon
          type="error"
        />
      ) : runs.length === 0 ? (
        <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={null} />
      ) : (
        <ApplicationRunsTable
          loading={runsQuery.isFetching}
          page={page}
          pageSize={PAGE_SIZE}
          total={total}
          configuration={runsTableConfiguration}
          columns={runsTableColumns}
          runs={runs}
          selectedRunId={selectedRunId}
          onPageChange={setPage}
          onSelectRun={selectRun}
        />
      )}
    </section>
  );

  return (
    <div className="application-logs-page" data-testid="application-logs-page">
      <div className="application-logs-page__stack">
        {logsHeader}
        {logsList}
      </div>
      {openConversationLogMessage ? (
        <ApplicationLogsFloatingWindow
          active={activeFloatingWindow === 'conversation-log'}
          initialRect={getConversationLogInitialRect}
          rect={conversationLogRect ?? undefined}
          onRectChange={(rect) => handleRectChange('conversation-log', rect)}
          testId="application-logs-floating-conversation-log"
          title={t('auto.conversation_logs')}
          onActivate={() => setActiveFloatingWindow('conversation-log')}
        >
          <div className="application-logs-page__conversation-log-panel">
            <ConversationLogPanel
              message={openConversationLogMessage}
              onClose={() => {
                setOpenConversationLogMessage(null);
                setConversationLogRect(null);
              }}
              onLoadArtifact={(artifactRef) =>
                fetchRuntimeDebugArtifact(applicationId, artifactRef)
              }
            />
          </div>
        </ApplicationLogsFloatingWindow>
      ) : null}
      {openResumeTimelineRunId ? (
        <ApplicationLogsFloatingWindow
          active={activeFloatingWindow === 'resume-timeline'}
          initialRect={getResumeTimelineInitialRect}
          rect={resumeTimelineRect ?? undefined}
          onRectChange={(rect) => handleRectChange('resume-timeline', rect)}
          testId="application-logs-floating-resume-timeline"
          title={t('auto.resume_timeline')}
          onActivate={() => setActiveFloatingWindow('resume-timeline')}
        >
          <div className="application-logs-page__resume-timeline-panel">
            <ApplicationRunResumeTimelinePanel
              applicationId={applicationId}
              runId={openResumeTimelineRunId}
              onClose={() => {
                setOpenResumeTimelineRunId(null);
                setResumeTimelineRect(null);
              }}
            />
          </div>
        </ApplicationLogsFloatingWindow>
      ) : null}
      {selectedRunId ? (
        <ApplicationLogsFloatingWindow
          active={activeFloatingWindow === 'run-detail'}
          initialRect={getRunDetailInitialRect}
          rect={runDetailRect ?? undefined}
          onRectChange={(rect) => handleRectChange('run-detail', rect)}
          testId="application-logs-floating-run-detail"
          title={t('auto.run_details')}
          onActivate={() => setActiveFloatingWindow('run-detail')}
        >
          <ApplicationRunDetailPanel
            applicationId={applicationId}
            onClose={() => selectRun(null)}
            onOpenMessageLog={openConversationLog}
            onOpenResumeTimeline={openResumeTimeline}
            runId={selectedRunId}
          />
        </ApplicationLogsFloatingWindow>
      ) : null}
    </div>
  );
}
