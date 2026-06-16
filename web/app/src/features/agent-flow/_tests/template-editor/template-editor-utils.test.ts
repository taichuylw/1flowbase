import { createEditor } from 'lexical';
import { describe, expect, test } from 'vitest';

import {
  editorStateToText,
  textToEditorState
} from '../../components/bindings/template-editor/template-editor-utils';

function roundTripTemplateText(text: string) {
  const editor = createEditor({
    namespace: 'template-editor-utils-test',
    onError(error: Error) {
      throw error;
    }
  });

  editor.setEditorState(editor.parseEditorState(textToEditorState(text)));

  return editorStateToText(editor.getEditorState());
}

describe('template editor utils', () => {
  test('round trips multiline template text without expanding line breaks', () => {
    expect(roundTripTemplateText('第一行\n第二行')).toBe('第一行\n第二行');
    expect(roundTripTemplateText('第一段\n\n第二段')).toBe('第一段\n\n第二段');
  });
});
