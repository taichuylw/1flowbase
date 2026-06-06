export interface ParsedHttpRequestUrlParts {
  url: string;
  params: Array<{ name: string; value: string }>;
}

export function parseHttpRequestUrlParts(
  rawUrl: string
): ParsedHttpRequestUrlParts {
  const queryStartIndex = rawUrl.indexOf('?');

  if (queryStartIndex < 0) {
    return { url: rawUrl, params: [] };
  }

  const url = rawUrl.slice(0, queryStartIndex);
  const queryWithFragment = rawUrl.slice(queryStartIndex + 1);
  const fragmentStartIndex = queryWithFragment.indexOf('#');
  const query =
    fragmentStartIndex >= 0
      ? queryWithFragment.slice(0, fragmentStartIndex)
      : queryWithFragment;
  const fragment =
    fragmentStartIndex >= 0 ? queryWithFragment.slice(fragmentStartIndex) : '';
  const searchParams = new URLSearchParams(query);

  return {
    url: `${url}${fragment}`,
    params: [...searchParams.entries()].map(([name, value]) => ({
      name,
      value
    }))
  };
}
