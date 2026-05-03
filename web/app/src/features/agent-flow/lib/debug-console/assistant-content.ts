const THINK_OPEN_TAG = '<think>';
const THINK_CLOSE_TAG = '</think>';

export interface ParsedAssistantContent {
  reasoningText: string;
  answerText: string;
}

function hasOpenThinkBlock(content: string) {
  return (
    content.lastIndexOf(THINK_OPEN_TAG) >
    content.lastIndexOf(THINK_CLOSE_TAG)
  );
}

export function closeOpenThinkBlock(content: string) {
  return hasOpenThinkBlock(content) ? `${content}${THINK_CLOSE_TAG}` : content;
}

export function appendReasoningDeltaToAssistantContent(
  content: string,
  text: string
) {
  if (!text) {
    return content;
  }

  return hasOpenThinkBlock(content)
    ? `${content}${text}`
    : `${content}${THINK_OPEN_TAG}${text}`;
}

export function appendTextDeltaToAssistantContent(
  content: string,
  text: string
) {
  if (!text) {
    return closeOpenThinkBlock(content);
  }

  return `${closeOpenThinkBlock(content)}${text}`;
}

function isPartialThinkTag(value: string) {
  return (
    THINK_OPEN_TAG.startsWith(value) || THINK_CLOSE_TAG.startsWith(value)
  );
}

export function parseAssistantContent(
  content: string
): ParsedAssistantContent {
  let mode: 'answer' | 'reasoning' = 'answer';
  let reasoningText = '';
  let answerText = '';
  let index = 0;

  while (index < content.length) {
    if (content.startsWith(THINK_OPEN_TAG, index)) {
      mode = 'reasoning';
      index += THINK_OPEN_TAG.length;
      continue;
    }

    if (content.startsWith(THINK_CLOSE_TAG, index)) {
      mode = 'answer';
      index += THINK_CLOSE_TAG.length;
      continue;
    }

    if (content[index] === '<' && isPartialThinkTag(content.slice(index))) {
      break;
    }

    if (mode === 'reasoning') {
      reasoningText += content[index];
    } else {
      answerText += content[index];
    }
    index += 1;
  }

  return { reasoningText, answerText };
}
