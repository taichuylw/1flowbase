import { describe, expect, test } from 'vitest';

import { parseHttpRequestCurlCommand } from '../lib/http-request/curl';

describe('parseHttpRequestCurlCommand', () => {
  test('extracts method url headers query and json body', () => {
    expect(
      parseHttpRequestCurlCommand(
        "curl -X POST 'https://api.example.com/orders?page=1' -H 'Authorization: Bearer token' -H 'Content-Type: application/json' -d '{\"query\":\"{{node-start.query}}\"}'"
      )
    ).toEqual({
      method: 'POST',
      url: 'https://api.example.com/orders',
      headers: [
        { name: 'Authorization', value: 'Bearer token' },
        { name: 'Content-Type', value: 'application/json' }
      ],
      params: [{ name: 'page', value: '1' }],
      body: '{"query":"{{node-start.query}}"}',
      bodyType: 'json'
    });
  });

  test('defaults to GET and raw body when curl omits explicit content type', () => {
    expect(
      parseHttpRequestCurlCommand(
        "curl 'https://api.example.com/search?q=refund' --data-raw 'plain text'"
      )
    ).toEqual({
      method: 'POST',
      url: 'https://api.example.com/search',
      headers: [],
      params: [{ name: 'q', value: 'refund' }],
      body: 'plain text',
      bodyType: 'raw'
    });
  });
});
