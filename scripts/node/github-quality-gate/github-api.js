const https = require('node:https');

function createIssueWithGitHubApi({ token, repository, title, body, labels }) {
  if (!repository) {
    throw new Error('GITHUB_REPOSITORY is required to create a quality gate issue');
  }

  const requestBody = JSON.stringify({ title, body, labels });

  return new Promise((resolve, reject) => {
    const request = https.request(
      {
        hostname: 'api.github.com',
        method: 'POST',
        path: `/repos/${repository}/issues`,
        headers: {
          Accept: 'application/vnd.github+json',
          Authorization: `Bearer ${token}`,
          'Content-Type': 'application/json',
          'Content-Length': Buffer.byteLength(requestBody),
          'User-Agent': '1flowbase-quality-gate',
          'X-GitHub-Api-Version': '2022-11-28',
        },
      },
      (response) => {
        let responseBody = '';
        response.setEncoding('utf8');
        response.on('data', (chunk) => {
          responseBody += chunk;
        });
        response.on('end', () => {
          if (response.statusCode >= 200 && response.statusCode < 300) {
            resolve(JSON.parse(responseBody));
            return;
          }

          reject(Object.assign(
            new Error(`GitHub Issue creation failed with HTTP ${response.statusCode}: ${responseBody}`),
            { statusCode: response.statusCode }
          ));
        });
      }
    );

    request.on('error', reject);
    request.write(requestBody);
    request.end();
  });
}

function requestGitHubJson({ token, repository, method, path: requestPath, body }) {
  if (!repository) {
    throw new Error('GITHUB_REPOSITORY is required for quality gate issue maintenance');
  }

  const requestBody = body === undefined ? '' : JSON.stringify(body);

  return new Promise((resolve, reject) => {
    const request = https.request(
      {
        hostname: 'api.github.com',
        method,
        path: `/repos/${repository}${requestPath}`,
        headers: {
          Accept: 'application/vnd.github+json',
          Authorization: `Bearer ${token}`,
          'Content-Type': 'application/json',
          'Content-Length': Buffer.byteLength(requestBody),
          'User-Agent': '1flowbase-quality-gate',
          'X-GitHub-Api-Version': '2022-11-28',
        },
      },
      (response) => {
        let responseBody = '';
        response.setEncoding('utf8');
        response.on('data', (chunk) => {
          responseBody += chunk;
        });
        response.on('end', () => {
          if (response.statusCode >= 200 && response.statusCode < 300) {
            resolve(responseBody ? JSON.parse(responseBody) : {});
            return;
          }

          reject(Object.assign(
            new Error(`GitHub request failed with HTTP ${response.statusCode}: ${responseBody}`),
            { statusCode: response.statusCode }
          ));
        });
      }
    );

    request.on('error', reject);
    if (requestBody) {
      request.write(requestBody);
    }
    request.end();
  });
}

function listOpenQualityGateIssuesWithGitHubApi({ token, repository }) {
  return requestGitHubJson({
    token,
    repository,
    method: 'GET',
    path: '/issues?state=open&labels=quality-gate&per_page=100',
  });
}

function closeIssueWithGitHubApi({ token, repository, number }) {
  return requestGitHubJson({
    token,
    repository,
    method: 'PATCH',
    path: `/issues/${number}`,
    body: { state: 'closed', state_reason: 'completed' },
  });
}

async function createIssueWithLabelFallback({ createIssueImpl, issue }) {
  try {
    return await createIssueImpl(issue);
  } catch (error) {
    if (error.statusCode !== 422 || issue.labels.length === 0) {
      throw error;
    }

    return createIssueImpl({
      ...issue,
      labels: [],
    });
  }
}

function issueNumberFromIssue(issue) {
  if (Number.isInteger(issue.number)) {
    return issue.number;
  }

  const match = String(issue.html_url || '').match(/\/issues\/(\d+)$/u);
  return match ? Number.parseInt(match[1], 10) : null;
}

function isPullRequestIssue(issue) {
  return issue && typeof issue === 'object' && issue.pull_request !== undefined;
}

function qualityGateIssueScopeFromTitle(title) {
  const match = String(title || '').match(
    /^\[Quality Gate\]\[([^\]]+)\]\s+\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}\s+(\S+)\s+\S+\s+(?:passed|failed)$/u
  );

  return match
    ? {
      reportType: match[1],
      target: match[2],
    }
    : null;
}

function isSameQualityGateScope(issue, latestScope) {
  if (!latestScope) {
    return false;
  }

  const issueScope = qualityGateIssueScopeFromTitle(issue.title);

  return Boolean(
    issueScope
    && issueScope.reportType === latestScope.reportType
    && issueScope.target === latestScope.target
  );
}

async function closeStaleOpenQualityGateIssues({
  token,
  repository,
  latestIssue,
  listOpenQualityGateIssuesImpl,
  closeIssueImpl,
}) {
  const latestIssueNumber = issueNumberFromIssue(latestIssue);
  const latestScope = qualityGateIssueScopeFromTitle(latestIssue.title);

  if (!latestIssueNumber || !latestScope) {
    return;
  }

  const openIssues = await listOpenQualityGateIssuesImpl({ token, repository });

  for (const issue of openIssues) {
    if (isPullRequestIssue(issue)) {
      continue;
    }

    const issueNumber = issueNumberFromIssue(issue);

    if (!issueNumber || issueNumber === latestIssueNumber) {
      continue;
    }

    if (!isSameQualityGateScope(issue, latestScope)) {
      continue;
    }

    await closeIssueImpl({
      token,
      repository,
      number: issueNumber,
    });
  }
}

module.exports = {
  closeIssueWithGitHubApi,
  closeStaleOpenQualityGateIssues,
  createIssueWithGitHubApi,
  createIssueWithLabelFallback,
  listOpenQualityGateIssuesWithGitHubApi,
};
