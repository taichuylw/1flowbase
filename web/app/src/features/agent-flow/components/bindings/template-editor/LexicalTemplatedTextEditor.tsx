import type { EditorState, LexicalEditor } from 'lexical';
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
  useCallback,
  useEffect,
  useImperativeHandle,
  useLayoutEffect,
  useMemo,
  useReducer,
  useRef
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
  ref?: Ref<LexicalTemplatedTextEditorHandle>;
  onChange: (value: string) => void;
  onTriggerChange?: (open: boolean) => void;
}

interface EditorApiBridgeProps {
  editorRef: MutableRefObject<LexicalEditor | null>;
  options: FlowSelectorOption[];
  forwardedRef?: Ref<LexicalTemplatedTextEditorHandle>;
  onOpenVariablePicker: () => void;
}

interface TypeaheadState {
  open: boolean;
  query: string;
  activeIndex: number;
  position: TypeaheadPosition;
}

const INITIAL_TYPEAHEAD_STATE: TypeaheadState = {
  open: false,
  query: '',
  activeIndex: 0,
  position: DEFAULT_TYPEAHEAD_POSITION
};

type TypeaheadAction =
  | { type: 'open'; query: string; position: TypeaheadPosition }
  | { type: 'close' }
  | { type: 'set-query'; query: string }
  | {
      type: 'move-active';
      direction: 'next' | 'previous';
      optionCount: number;
    };

function nextTypeaheadIndex({
  currentIndex,
  direction,
  optionCount
}: {
  currentIndex: number;
  direction: 'next' | 'previous';
  optionCount: number;
}) {
  if (optionCount === 0) {
    return currentIndex;
  }

  if (direction === 'next') {
    const nextIndex = currentIndex + 1;

    return nextIndex >= optionCount ? 0 : nextIndex;
  }

  const nextIndex = currentIndex - 1;

  return nextIndex < 0 ? optionCount - 1 : nextIndex;
}

function typeaheadReducer(
  state: TypeaheadState,
  action: TypeaheadAction
): TypeaheadState {
  switch (action.type) {
    case 'open':
      return {
        open: true,
        query: action.query,
        activeIndex: 0,
        position: action.position
      };
    case 'close':
      return INITIAL_TYPEAHEAD_STATE;
    case 'set-query':
      return {
        ...state,
        query: action.query,
        activeIndex: 0
      };
    case 'move-active':
      return {
        ...state,
        activeIndex: nextTypeaheadIndex({
          currentIndex: state.activeIndex,
          direction: action.direction,
          optionCount: action.optionCount
        })
      };
    default:
      return state;
  }
}

function isExternalEditableTarget(node: EventTarget | null) {
  return (
    node instanceof HTMLElement &&
    node.matches(
      'input:not([type="button"]):not([type="submit"]):not([type="reset"]), textarea, select, [contenteditable="true"], [role="textbox"], [role="combobox"]'
    )
  );
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

    const removedTrigger =
      removeTriggerQueryBeforeSelection(TRIGGER_CHARACTERS);

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

function useLexicalTemplatedTextEditorController({
  ariaLabel,
  options,
  value,
  displayMode = 'block',
  onChange,
  onTriggerChange
}: LexicalTemplatedTextEditorProps) {
  const editorRef = useRef<LexicalEditor | null>(null);
  const shellRef = useRef<HTMLDivElement | null>(null);
  const typeaheadRef = useRef<HTMLDivElement | null>(null);
  const blurCloseTimerRef = useRef<number | null>(null);
  const inlineTriggerQueryRef = useRef<string | null>(null);
  const [typeaheadState, dispatchTypeahead] = useReducer(
    typeaheadReducer,
    INITIAL_TYPEAHEAD_STATE
  );
  const typeaheadOpen = typeaheadState.open;
  const query = typeaheadState.query;

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

  const activeIndex =
    typeaheadOpen && filteredOptions.length === 0
      ? -1
      : Math.min(
          typeaheadState.activeIndex,
          Math.max(filteredOptions.length - 1, 0)
        );

  useEffect(
    () => () => {
      if (blurCloseTimerRef.current !== null) {
        window.clearTimeout(blurCloseTimerRef.current);
      }
    },
    []
  );

  const clearBlurCloseTimer = useCallback(() => {
    if (blurCloseTimerRef.current === null) {
      return;
    }

    window.clearTimeout(blurCloseTimerRef.current);
    blurCloseTimerRef.current = null;
  }, []);

  const isInsideEditorOrTypeahead = useCallback((node: EventTarget | null) => {
    return (
      node instanceof Node &&
      (shellRef.current?.contains(node) || typeaheadRef.current?.contains(node))
    );
  }, []);

  const openTypeahead = useCallback(
    (
      nextQuery = '',
      nextPosition: TypeaheadPosition = DEFAULT_TYPEAHEAD_POSITION
    ) => {
      clearBlurCloseTimer();
      dispatchTypeahead({
        type: 'open',
        query: nextQuery,
        position: nextPosition
      });
      onTriggerChange?.(true);
    },
    [clearBlurCloseTimer, onTriggerChange]
  );

  const openTypeaheadAtSelection = useCallback(
    (nextQuery = '') => {
      openTypeahead(
        nextQuery,
        shellRef.current
          ? calculateTypeaheadPosition(shellRef.current)
          : DEFAULT_TYPEAHEAD_POSITION
      );
    },
    [openTypeahead]
  );

  const closeTypeahead = useCallback(() => {
    inlineTriggerQueryRef.current = null;
    dispatchTypeahead({ type: 'close' });
    onTriggerChange?.(false);
  }, [onTriggerChange]);

  const clearEditorSelection = useCallback(() => {
    editorRef.current?.update(
      () => {
        $setSelection(null);
      },
      {
        discrete: true,
        tag: SKIP_DOM_SELECTION_TAG
      }
    );
  }, []);

  const handleBlur = useCallback(
    (event: FocusEvent<HTMLDivElement>) => {
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
    },
    [clearEditorSelection, closeTypeahead, isInsideEditorOrTypeahead]
  );

  const handleOpenVariablePicker = useCallback(() => {
    if (editorRef.current) {
      focusEditor(editorRef.current);
    }
    inlineTriggerQueryRef.current = null;
    openTypeaheadAtSelection();
  }, [openTypeaheadAtSelection]);

  const handleSelect = useCallback(
    (selector: string[]) => {
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
    },
    [closeTypeahead, options]
  );

  const handleTypeaheadKeyDown = useCallback(
    (event: TypeaheadKeyboardEvent) => {
      if (!typeaheadOpen) {
        return;
      }

      if (event.key === 'ArrowDown') {
        event.preventDefault();

        if (filteredOptions.length === 0) {
          return;
        }

        dispatchTypeahead({
          type: 'move-active',
          direction: 'next',
          optionCount: filteredOptions.length
        });
        return;
      }

      if (event.key === 'ArrowUp') {
        event.preventDefault();

        if (filteredOptions.length === 0) {
          return;
        }

        dispatchTypeahead({
          type: 'move-active',
          direction: 'previous',
          optionCount: filteredOptions.length
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
    },
    [activeIndex, closeTypeahead, filteredOptions, handleSelect, typeaheadOpen]
  );

  const handleEditorKeyDown = useCallback(
    (event: ReactKeyboardEvent<HTMLDivElement>) => {
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
        dispatchTypeahead({ type: 'set-query', query: nextInlineQuery });
        window.setTimeout(() => {
          editorRef.current?.getEditorState().read(() => {
            const triggerContext = getTriggerContext(TRIGGER_CHARACTERS);

            if (triggerContext) {
              const nextQuery = triggerContext.query;
              inlineTriggerQueryRef.current = nextQuery;
              dispatchTypeahead({ type: 'set-query', query: nextQuery });
              openTypeaheadAtSelection(nextQuery);
            }
          });
        }, 0);
      }

      handleTypeaheadKeyDown(event);
    },
    [
      closeTypeahead,
      displayMode,
      handleTypeaheadKeyDown,
      openTypeaheadAtSelection,
      typeaheadOpen
    ]
  );

  const contentEditable = useMemo(
    () => (
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
    ),
    [ariaLabel, displayMode, handleEditorKeyDown]
  );

  const handleEditorStateChange = useCallback(
    (editorState: EditorState) => {
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
    },
    [onChange, openTypeaheadAtSelection, typeaheadOpen]
  );

  return {
    activeIndex,
    contentEditable,
    displayMode,
    editorRef,
    filteredOptions,
    handleBlur,
    handleEditorStateChange,
    handleOpenVariablePicker,
    handleSelect,
    handleTypeaheadKeyDown,
    initialConfig,
    query,
    shellRef,
    typeaheadOpen,
    typeaheadPosition: typeaheadState.position,
    typeaheadRef
  };
}

export function LexicalTemplatedTextEditor(
  props: LexicalTemplatedTextEditorProps
) {
  const { options, placeholder, ref, value } = props;
  const controller = useLexicalTemplatedTextEditorController(props);

  return (
    <LexicalComposer initialConfig={controller.initialConfig}>
      <div
        data-testid="templated-text-editor-shell"
        ref={controller.shellRef}
        className={[
          'agent-flow-templated-text-field__editor-shell',
          controller.displayMode === 'input'
            ? 'agent-flow-templated-text-field__editor-shell--input'
            : null
        ]
          .filter(Boolean)
          .join(' ')}
        onBlurCapture={controller.handleBlur}
      >
        <RichTextPlugin
          contentEditable={controller.contentEditable}
          placeholder={
            <div className="agent-flow-templated-text-field__placeholder">
              {placeholder ||
                i18nText('agentFlow', 'auto.enter_template_content')}
            </div>
          }
          ErrorBoundary={LexicalErrorBoundary}
        />
        <TemplateVariableTypeaheadPlugin
          popupRef={controller.typeaheadRef}
          open={controller.typeaheadOpen}
          options={controller.filteredOptions}
          query={controller.query}
          activeIndex={controller.activeIndex}
          position={controller.typeaheadPosition}
          onKeyDown={controller.handleTypeaheadKeyDown}
          onSelect={controller.handleSelect}
        />
      </div>
      <TemplateVariableReplacementPlugin options={options} />
      <TypeaheadKeyboardCommandPlugin
        open={controller.typeaheadOpen}
        onKeyDown={controller.handleTypeaheadKeyDown}
      />
      <ControlledValuePlugin value={value} options={options} />
      <EditorApiBridge
        editorRef={controller.editorRef}
        options={options}
        forwardedRef={ref}
        onOpenVariablePicker={controller.handleOpenVariablePicker}
      />
      <OnChangePlugin
        ignoreSelectionChange
        onChange={controller.handleEditorStateChange}
      />
      <HistoryPlugin />
    </LexicalComposer>
  );
}
