import type { LexicalEditor } from 'lexical';
import type {
  FocusEvent,
  KeyboardEvent as ReactKeyboardEvent,
  MutableRefObject,
  Ref
} from 'react';
import type { FlowSelectorOption } from '../../../lib/selector-options';

import { LexicalComposer } from '@lexical/react/LexicalComposer';
import { ContentEditable } from '@lexical/react/LexicalContentEditable';
import { LexicalErrorBoundary } from '@lexical/react/LexicalErrorBoundary';
import { HistoryPlugin } from '@lexical/react/LexicalHistoryPlugin';
import { OnChangePlugin } from '@lexical/react/LexicalOnChangePlugin';
import { useLexicalComposerContext } from '@lexical/react/LexicalComposerContext';
import { RichTextPlugin } from '@lexical/react/LexicalRichTextPlugin';
import { mergeRegister } from '@lexical/utils';
import {
  $getRoot,
  $getSelection,
  $insertNodes,
  $isRangeSelection,
  $setSelection,
  COMMAND_PRIORITY_HIGH,
  KEY_ARROW_DOWN_COMMAND,
  KEY_ARROW_UP_COMMAND,
  KEY_ENTER_COMMAND,
  KEY_ESCAPE_COMMAND,
  KEY_TAB_COMMAND,
  SKIP_DOM_SELECTION_TAG
} from 'lexical';
import {
  forwardRef,
  useEffect,
  useImperativeHandle,
  useLayoutEffect,
  useMemo,
  useRef,
  useState
} from 'react';

import { getTemplateSelectorLabel } from '../../../lib/template-binding';
import { TemplateVariableReplacementPlugin } from './TemplateVariableReplacementPlugin';
import {
  $createTemplateVariableNode,
  TemplateVariableNode
} from './TemplateVariableNode';
import { TemplateVariableTypeaheadPlugin } from './TemplateVariableTypeaheadPlugin';
import {
  editorStateToText,
  getTriggerContext,
  removeTriggerQueryAtDocumentEnd,
  removeTriggerQueryBeforeSelection,
  textToEditorState
} from './template-editor-utils';
import { i18nText } from '../../../../../shared/i18n/text';

const TRIGGER_CHARACTERS = new Set(['/', '{']);
const TYPEAHEAD_OFFSET = 8;
const TYPEAHEAD_MIN_LEFT = 8;
const TYPEAHEAD_MIN_TOP = 8;
const TYPEAHEAD_MAX_WIDTH = 320;
const TYPEAHEAD_HORIZONTAL_GUTTER = 16;
const DEFAULT_TYPEAHEAD_POSITION = {
  left: 8,
  top: 36,
  width: 304
};

interface TypeaheadPosition {
  left: number;
  top: number;
  width: number;
}

type InsertSelectorMode = 'current-selection' | 'inline-trigger';
type TypeaheadKeyboardEvent = Pick<
  KeyboardEvent,
  'defaultPrevented' | 'key' | 'preventDefault'
>;

export interface LexicalTemplatedTextEditorHandle {
  focus: () => void;
  insertSelector: (selector: string[]) => void;
  openVariablePicker: () => void;
}

interface LexicalTemplatedTextEditorProps {
  ariaLabel: string;
  placeholder?: string;
  options: FlowSelectorOption[];
  value: string;
  displayMode?: 'block' | 'input';
  onChange: (value: string) => void;
  onTriggerChange?: (open: boolean) => void;
}

interface EditorApiBridgeProps {
  editorRef: MutableRefObject<LexicalEditor | null>;
  options: FlowSelectorOption[];
  forwardedRef: Ref<LexicalTemplatedTextEditorHandle>;
  onOpenVariablePicker: () => void;
}

function ControlledValuePlugin({
  value,
  options
}: {
  value: string;
  options: FlowSelectorOption[];
}) {
  const [editor] = useLexicalComposerContext();

  useLayoutEffect(() => {
    const currentText = editorStateToText(editor.getEditorState());

    if (currentText === value) {
      return;
    }

    const nextState = editor.parseEditorState(textToEditorState(value));
    editor.setEditorState(nextState, {
      tag: SKIP_DOM_SELECTION_TAG
    });
  }, [editor, options, value]);

  return null;
}

function insertSelectorNode(
  editor: LexicalEditor,
  selector: string[],
  options: FlowSelectorOption[],
  mode: InsertSelectorMode = 'current-selection'
) {
  const label = getTemplateSelectorLabel(selector, options);

  focusEditor(editor);
  editor.update(() => {
    if (!$isRangeSelection($getSelection())) {
      $getRoot().selectEnd();
    }

    const removedTrigger = removeTriggerQueryBeforeSelection(TRIGGER_CHARACTERS);

    if (!removedTrigger && mode === 'inline-trigger') {
      const removedTriggerAtEnd =
        removeTriggerQueryAtDocumentEnd(TRIGGER_CHARACTERS);

      if (!removedTriggerAtEnd) {
        $getRoot().selectEnd();
      }
    }

    $insertNodes([$createTemplateVariableNode(selector, label)]);
  });
}

function focusEditor(editor: LexicalEditor) {
  editor.focus();
}

function getRangeRect(range: Range) {
  if (typeof range.getBoundingClientRect !== 'function') {
    return null;
  }

  const rangeRect = range.getBoundingClientRect();

  if (rangeRect.width > 0 || rangeRect.height > 0) {
    return rangeRect;
  }

  if (typeof range.getClientRects !== 'function') {
    return null;
  }

  const clientRects = range.getClientRects();

  return clientRects.length > 0 ? clientRects[0] : null;
}

function getViewportWidth() {
  if (typeof window === 'undefined') {
    return TYPEAHEAD_MAX_WIDTH + TYPEAHEAD_HORIZONTAL_GUTTER * 2;
  }

  return window.innerWidth;
}

function clampTypeaheadLeft(left: number, width: number) {
  const maxLeft = Math.max(
    TYPEAHEAD_MIN_LEFT,
    getViewportWidth() - width - TYPEAHEAD_HORIZONTAL_GUTTER
  );

  return Math.min(Math.max(TYPEAHEAD_MIN_LEFT, left), maxLeft);
}

function calculateTypeaheadPosition(container: HTMLElement) {
  const selection = window.getSelection();
  const containerRect = container.getBoundingClientRect();
  const width = Math.max(
    240,
    Math.min(
      TYPEAHEAD_MAX_WIDTH,
      containerRect.width - TYPEAHEAD_HORIZONTAL_GUTTER
    )
  );

  if (!selection || selection.rangeCount === 0) {
    return {
      left: clampTypeaheadLeft(containerRect.left + TYPEAHEAD_MIN_LEFT, width),
      top: Math.max(TYPEAHEAD_MIN_TOP, containerRect.top + 36),
      width
    };
  }

  const sourceRange = selection.getRangeAt(0);
  const range =
    typeof sourceRange.cloneRange === 'function'
      ? sourceRange.cloneRange()
      : sourceRange;
  const rangeRect = getRangeRect(range);

  if (!rangeRect) {
    return {
      left: clampTypeaheadLeft(containerRect.left + TYPEAHEAD_MIN_LEFT, width),
      top: Math.max(TYPEAHEAD_MIN_TOP, containerRect.top + 36),
      width
    };
  }

  return {
    left: clampTypeaheadLeft(rangeRect.left, width),
    top: Math.max(TYPEAHEAD_MIN_TOP, rangeRect.bottom + TYPEAHEAD_OFFSET),
    width
  };
}

function EditorApiBridge({
  editorRef,
  options,
  forwardedRef,
  onOpenVariablePicker
}: EditorApiBridgeProps) {
  const [editor] = useLexicalComposerContext();

  useEffect(() => {
    editorRef.current = editor;

    return () => {
      editorRef.current = null;
    };
  }, [editor, editorRef]);

  useImperativeHandle(
    forwardedRef,
    () => ({
      focus() {
        focusEditor(editor);
      },
      insertSelector(selector: string[]) {
        insertSelectorNode(editor, selector, options);
      },
      openVariablePicker() {
        onOpenVariablePicker();
      }
    }),
    [editor, onOpenVariablePicker, options]
  );

  return null;
}

function TypeaheadKeyboardCommandPlugin({
  open,
  onKeyDown
}: {
  open: boolean;
  onKeyDown: (event: TypeaheadKeyboardEvent) => void;
}) {
  const [editor] = useLexicalComposerContext();

  useEffect(() => {
    function handleCommand(event: KeyboardEvent | null) {
      if (!open || event === null) {
        return false;
      }

      onKeyDown(event);
      return event.defaultPrevented;
    }

    return mergeRegister(
      editor.registerCommand(
        KEY_ARROW_DOWN_COMMAND,
        handleCommand,
        COMMAND_PRIORITY_HIGH
      ),
      editor.registerCommand(
        KEY_ARROW_UP_COMMAND,
        handleCommand,
        COMMAND_PRIORITY_HIGH
      ),
      editor.registerCommand(
        KEY_ENTER_COMMAND,
        handleCommand,
        COMMAND_PRIORITY_HIGH
      ),
      editor.registerCommand(
        KEY_ESCAPE_COMMAND,
        handleCommand,
        COMMAND_PRIORITY_HIGH
      ),
      editor.registerCommand(
        KEY_TAB_COMMAND,
        handleCommand,
        COMMAND_PRIORITY_HIGH
      )
    );
  }, [editor, onKeyDown, open]);

  return null;
}

export const LexicalTemplatedTextEditor = forwardRef<
  LexicalTemplatedTextEditorHandle,
  LexicalTemplatedTextEditorProps
>(function LexicalTemplatedTextEditor(
  {
    ariaLabel,
    options,
    value,
    displayMode = 'block',
    onChange,
    onTriggerChange,
    placeholder
  },
  ref
) {
  const editorRef = useRef<LexicalEditor | null>(null);
  const shellRef = useRef<HTMLDivElement | null>(null);
  const typeaheadRef = useRef<HTMLDivElement | null>(null);
  const blurCloseTimerRef = useRef<number | null>(null);
  const inlineTriggerQueryRef = useRef<string | null>(null);
  const [typeaheadOpen, setTypeaheadOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [activeIndex, setActiveIndex] = useState(0);
  const [typeaheadPosition, setTypeaheadPosition] = useState<TypeaheadPosition>(
    DEFAULT_TYPEAHEAD_POSITION
  );

  const initialConfig = useMemo(
    () => ({
      namespace: 'agent-flow-templated-text-editor',
      nodes: [TemplateVariableNode],
      editorState: textToEditorState(value),
      onError(error: Error) {
        throw error;
      }
    }),
    [value]
  );

  const filteredOptions = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();

    if (!normalizedQuery) {
      return options;
    }

    return options.filter((option) =>
      [
        option.displayLabel,
        option.nodeLabel,
        option.outputLabel,
        option.outputKey,
        option.value.join('.')
      ].some((candidate) => candidate.toLowerCase().includes(normalizedQuery))
    );
  }, [options, query]);

  useEffect(() => {
    if (!typeaheadOpen) {
      return;
    }

    setActiveIndex(filteredOptions.length > 0 ? 0 : -1);
  }, [filteredOptions.length, query, typeaheadOpen]);

  useEffect(
    () => () => {
      if (blurCloseTimerRef.current !== null) {
        window.clearTimeout(blurCloseTimerRef.current);
      }
    },
    []
  );

  function clearBlurCloseTimer() {
    if (blurCloseTimerRef.current === null) {
      return;
    }

    window.clearTimeout(blurCloseTimerRef.current);
    blurCloseTimerRef.current = null;
  }

  function isInsideEditorOrTypeahead(node: EventTarget | null) {
    return (
      node instanceof Node &&
      (shellRef.current?.contains(node) || typeaheadRef.current?.contains(node))
    );
  }

  function isExternalEditableTarget(node: EventTarget | null) {
    return (
      node instanceof HTMLElement &&
      node.matches(
        'input:not([type="button"]):not([type="submit"]):not([type="reset"]), textarea, select, [contenteditable="true"], [role="textbox"], [role="combobox"]'
      )
    );
  }

  function openTypeahead(
    nextQuery = '',
    nextPosition: TypeaheadPosition = DEFAULT_TYPEAHEAD_POSITION
  ) {
    clearBlurCloseTimer();
    setQuery(nextQuery);
    setActiveIndex(0);
    setTypeaheadPosition(nextPosition);
    setTypeaheadOpen(true);
    onTriggerChange?.(true);
  }

  function openTypeaheadAtSelection(nextQuery = '') {
    openTypeahead(
      nextQuery,
      shellRef.current
        ? calculateTypeaheadPosition(shellRef.current)
        : DEFAULT_TYPEAHEAD_POSITION
    );
  }

  function closeTypeahead() {
    inlineTriggerQueryRef.current = null;
    setQuery('');
    setActiveIndex(0);
    setTypeaheadPosition(DEFAULT_TYPEAHEAD_POSITION);
    setTypeaheadOpen(false);
    onTriggerChange?.(false);
  }

  function clearEditorSelection() {
    editorRef.current?.update(
      () => {
        $setSelection(null);
      },
      {
        discrete: true,
        tag: SKIP_DOM_SELECTION_TAG
      }
    );
  }

  function handleBlur(event: FocusEvent<HTMLDivElement>) {
    const nextFocusedNode = event.relatedTarget;

    if (isInsideEditorOrTypeahead(nextFocusedNode)) {
      return;
    }

    if (isExternalEditableTarget(nextFocusedNode)) {
      clearEditorSelection();
    }

    if (blurCloseTimerRef.current !== null) {
      window.clearTimeout(blurCloseTimerRef.current);
    }

    blurCloseTimerRef.current = window.setTimeout(() => {
      blurCloseTimerRef.current = null;

      if (isInsideEditorOrTypeahead(document.activeElement)) {
        return;
      }

      clearEditorSelection();
      closeTypeahead();
    }, 120);
  }

  function handleOpenVariablePicker() {
    if (editorRef.current) {
      focusEditor(editorRef.current);
    }
    inlineTriggerQueryRef.current = null;
    openTypeaheadAtSelection();
  }

  function handleEditorKeyDown(event: ReactKeyboardEvent<HTMLDivElement>) {
    if (event.defaultPrevented) {
      return;
    }

    if (!typeaheadOpen && displayMode === 'input' && event.key === 'Enter') {
      event.preventDefault();
      return;
    }

    if (
      !typeaheadOpen &&
      event.key.length === 1 &&
      TRIGGER_CHARACTERS.has(event.key)
    ) {
      inlineTriggerQueryRef.current = '';
      openTypeaheadAtSelection();
      return;
    }

    if (typeaheadOpen && event.key.length === 1) {
      if (inlineTriggerQueryRef.current === null) {
        closeTypeahead();
        return;
      }

      if (/\s/.test(event.key)) {
        closeTypeahead();
        return;
      }

      const nextInlineQuery = `${inlineTriggerQueryRef.current}${event.key}`;
      inlineTriggerQueryRef.current = nextInlineQuery;
      setQuery(nextInlineQuery);
      window.setTimeout(() => {
        editorRef.current?.getEditorState().read(() => {
          const triggerContext = getTriggerContext(TRIGGER_CHARACTERS);

          if (triggerContext) {
            const nextQuery = triggerContext.query;
            inlineTriggerQueryRef.current = nextQuery;
            setQuery(nextQuery);
            openTypeaheadAtSelection(nextQuery);
          }
        });
      }, 0);
    }

    handleTypeaheadKeyDown(event);
  }

  function handleTypeaheadKeyDown(event: TypeaheadKeyboardEvent) {
    if (!typeaheadOpen) {
      return;
    }

    if (event.key === 'ArrowDown') {
      event.preventDefault();

      if (filteredOptions.length === 0) {
        return;
      }

      setActiveIndex((currentIndex) => {
        const nextIndex = currentIndex + 1;

        return nextIndex >= filteredOptions.length ? 0 : nextIndex;
      });
      return;
    }

    if (event.key === 'ArrowUp') {
      event.preventDefault();

      if (filteredOptions.length === 0) {
        return;
      }

      setActiveIndex((currentIndex) => {
        const nextIndex = currentIndex - 1;

        return nextIndex < 0 ? filteredOptions.length - 1 : nextIndex;
      });
      return;
    }

    if (event.key === 'Enter' || event.key === 'Tab') {
      const activeOption = filteredOptions[activeIndex];

      if (!activeOption) {
        return;
      }

      event.preventDefault();
      handleSelect(activeOption.value);
      return;
    }

    if (event.key === 'Escape') {
      event.preventDefault();
      closeTypeahead();
      if (editorRef.current) {
        focusEditor(editorRef.current);
      }
    }
  }

  function handleSelect(selector: string[]) {
    const editor = editorRef.current;

    if (editor) {
      insertSelectorNode(
        editor,
        selector,
        options,
        inlineTriggerQueryRef.current === null
          ? 'current-selection'
          : 'inline-trigger'
      );
    }
    closeTypeahead();
  }

  return (
    <LexicalComposer initialConfig={initialConfig}>
      <div
        data-testid="templated-text-editor-shell"
        ref={shellRef}
        className={[
          'agent-flow-templated-text-field__editor-shell',
          displayMode === 'input'
            ? 'agent-flow-templated-text-field__editor-shell--input'
            : null
        ]
          .filter(Boolean)
          .join(' ')}
        onBlurCapture={handleBlur}
      >
        <RichTextPlugin
          contentEditable={
            <ContentEditable
              aria-label={ariaLabel}
              role="textbox"
              aria-multiline={displayMode === 'input' ? 'false' : 'true'}
              className={[
                'agent-flow-templated-text-field__editor',
                displayMode === 'input'
                  ? 'agent-flow-templated-text-field__editor--input'
                  : null
              ]
                .filter(Boolean)
                .join(' ')}
              onKeyDown={handleEditorKeyDown}
            />
          }
          placeholder={
            <div className="agent-flow-templated-text-field__placeholder">
              {placeholder || i18nText("agentFlow", "auto.enter_template_content")}
            </div>
          }
          ErrorBoundary={LexicalErrorBoundary}
        />
        <TemplateVariableTypeaheadPlugin
          popupRef={typeaheadRef}
          open={typeaheadOpen}
          options={filteredOptions}
          query={query}
          activeIndex={activeIndex}
          position={typeaheadPosition}
          onKeyDown={handleTypeaheadKeyDown}
          onSelect={handleSelect}
        />
      </div>
      <TemplateVariableReplacementPlugin options={options} />
      <TypeaheadKeyboardCommandPlugin
        open={typeaheadOpen}
        onKeyDown={handleTypeaheadKeyDown}
      />
      <ControlledValuePlugin value={value} options={options} />
      <EditorApiBridge
        editorRef={editorRef}
        options={options}
        forwardedRef={ref}
        onOpenVariablePicker={handleOpenVariablePicker}
      />
      <OnChangePlugin
        ignoreSelectionChange
        onChange={(editorState) => {
          onChange(editorStateToText(editorState));

          if (!typeaheadOpen) {
            return;
          }

          let nextQuery: string | null = null;

          editorState.read(() => {
            const triggerContext = getTriggerContext(TRIGGER_CHARACTERS);

            if (triggerContext) {
              nextQuery = triggerContext.query;
            }
          });

          if (nextQuery === null) {
            return;
          }

          inlineTriggerQueryRef.current = nextQuery;
          openTypeaheadAtSelection(nextQuery);
        }}
      />
      <HistoryPlugin />
    </LexicalComposer>
  );
});
