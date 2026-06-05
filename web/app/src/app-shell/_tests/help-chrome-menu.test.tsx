import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, test, vi } from 'vitest';

const { fetchConsoleReleaseStatus } = vi.hoisted(() => ({
  fetchConsoleReleaseStatus: vi.fn()
}));

vi.mock('@1flowbase/api-client', () => ({
  fetchConsoleReleaseStatus
}));

import { AppProviders } from '../../app/AppProviders';
import { appI18n } from '../../shared/i18n/app-i18n';
import { HelpChromeMenu } from '../HelpChromeMenu';

describe('HelpChromeMenu', () => {
  test('shows release status and Docker upgrade commands in the help popup', async () => {
    await appI18n.changeLanguage('zh_Hans');
    fetchConsoleReleaseStatus.mockResolvedValue({
      current_version: '0.1.5',
      latest_version: '0.1.6',
      has_update: true,
      release_info: {
        name: 'v0.1.6',
        body: 'Release notes',
        published_at: '2026-06-05T00:00:00Z',
        html_url: 'https://github.com/taichuy/1flowbase/releases/tag/v0.1.6'
      },
      contributors_url:
        'https://github.com/taichuy/1flowbase/graphs/contributors',
      upgrade_commands: {
        shell:
          'curl -fsSL https://raw.githubusercontent.com/taichuy/1flowbase/main/scripts/shell/docker-deploy.sh | sh',
        powershell:
          'irm https://raw.githubusercontent.com/taichuy/1flowbase/main/scripts/powershell/docker-deploy.ps1 | iex'
      },
      cached: false,
      warning: null
    });

    render(
      <AppProviders>
        <HelpChromeMenu />
      </AppProviders>
    );

    fireEvent.mouseEnter(screen.getByLabelText('帮助'));

    expect(await screen.findByText('版本')).toBeInTheDocument();
    expect(await screen.findByText('0.1.5')).toBeInTheDocument();
    expect(await screen.findByText('0.1.6')).toBeInTheDocument();
    expect(await screen.findByText('查看发布')).toHaveAttribute(
      'href',
      'https://github.com/taichuy/1flowbase/releases/tag/v0.1.6'
    );
    expect(await screen.findByText('贡献者')).toHaveAttribute(
      'href',
      'https://github.com/taichuy/1flowbase/graphs/contributors'
    );
    expect(
      await screen.findByText(/scripts\/shell\/docker-deploy\.sh/u)
    ).toBeInTheDocument();
  });
});
