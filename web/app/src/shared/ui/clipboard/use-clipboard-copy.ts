import { useCallback, useState } from 'react';

import { copyTextToClipboard } from './copy-text';

export function useClipboardCopy(resetDelayMs = 1600) {
  const [copied, setCopied] = useState(false);

  const copy = useCallback(
    async (text: string) => {
      await copyTextToClipboard(text);
      setCopied(true);
      window.setTimeout(() => setCopied(false), resetDelayMs);
    },
    [resetDelayMs]
  );

  return { copied, copy };
}
