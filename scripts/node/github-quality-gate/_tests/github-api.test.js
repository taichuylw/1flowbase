const test = require('node:test');
const assert = require('node:assert/strict');

const { upsertIssueCommentWithMarker } = require('../github-api.js');

test('upsertIssueCommentWithMarker updates the existing marker comment', async () => {
  const calls = [];

  const result = await upsertIssueCommentWithMarker({
    token: 'token',
    repository: 'taichuy/1flowbase',
    number: 658,
    marker: '<!-- marker -->',
    body: '<!-- marker -->\nupdated',
    listCommentsImpl() {
      return [
        { id: 10, body: 'other comment' },
        { id: 11, body: '<!-- marker -->\nold report' },
      ];
    },
    createCommentImpl(comment) {
      calls.push({ kind: 'create', comment });
      return { html_url: 'created' };
    },
    updateCommentImpl(comment) {
      calls.push({ kind: 'update', comment });
      return { html_url: 'updated' };
    },
  });

  assert.deepEqual(result, { html_url: 'updated' });
  assert.deepEqual(calls, [{
    kind: 'update',
    comment: {
      token: 'token',
      repository: 'taichuy/1flowbase',
      commentId: 11,
      body: '<!-- marker -->\nupdated',
    },
  }]);
});

test('upsertIssueCommentWithMarker creates a marker comment when none exists', async () => {
  const calls = [];

  const result = await upsertIssueCommentWithMarker({
    token: 'token',
    repository: 'taichuy/1flowbase',
    number: 658,
    marker: '<!-- marker -->',
    body: '<!-- marker -->\nnew report',
    listCommentsImpl() {
      return [{ id: 10, body: 'other comment' }];
    },
    createCommentImpl(comment) {
      calls.push({ kind: 'create', comment });
      return { html_url: 'created' };
    },
    updateCommentImpl(comment) {
      calls.push({ kind: 'update', comment });
      return { html_url: 'updated' };
    },
  });

  assert.deepEqual(result, { html_url: 'created' });
  assert.deepEqual(calls, [{
    kind: 'create',
    comment: {
      token: 'token',
      repository: 'taichuy/1flowbase',
      number: 658,
      body: '<!-- marker -->\nnew report',
    },
  }]);
});
