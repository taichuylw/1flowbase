import { useQuery } from '@tanstack/react-query';

import { AgentFlowDockPanel } from '../../../agent-flow/components/editor/AgentFlowDockPanel';
import {
  applicationRunResumeTimelineQueryKey,
  fetchApplicationRunResumeTimeline
} from '../../api/runtime';
import { i18nText } from '../../../../shared/i18n/text';
import { ApplicationRunResumeTimeline } from './ApplicationRunResumeTimeline';

export function ApplicationRunResumeTimelinePanel({
  applicationId,
  onClose,
  runId
}: {
  applicationId: string;
  onClose: () => void;
  runId: string;
}) {
  const detailQuery = useQuery({
    queryKey: applicationRunResumeTimelineQueryKey(applicationId, runId),
    queryFn: () => fetchApplicationRunResumeTimeline(applicationId, runId),
    refetchInterval: 1000
  });

  return (
    <AgentFlowDockPanel
      bodyClassName="application-run-resume-timeline-panel__body"
      className="application-run-resume-timeline-panel"
      closeLabel={i18nText('applications', 'auto.close_resume_timeline')}
      title={i18nText('applications', 'auto.resume_timeline')}
      onClose={onClose}
    >
      <ApplicationRunResumeTimeline detail={detailQuery.data ?? null} />
    </AgentFlowDockPanel>
  );
}
