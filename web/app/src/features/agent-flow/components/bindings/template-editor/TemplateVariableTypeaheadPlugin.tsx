import { Empty } from 'antd';
import { createPortal } from 'react-dom';
import type { CSSProperties, KeyboardEvent, RefObject } from 'react';

import type { FlowSelectorOption } from '../../../lib/selector-options';
import { i18nText } from '../../../../../shared/i18n/text';

interface TemplateVariableTypeaheadPluginProps {
  open: boolean;
  options: FlowSelectorOption[];
  query: string;
  activeIndex: number;
  position?: {
    left: number;
    top: number;
    width: number;
  } | null;
  popupRef?: RefObject<HTMLDivElement | null>;
  onQueryChange: (value: string) => void;
  onKeyDown: (event: KeyboardEvent<HTMLDivElement | HTMLInputElement>) => void;
  onSelect: (selector: string[]) => void;
}

export function TemplateVariableTypeaheadPlugin({
  open,
  options,
  query,
  activeIndex,
  position,
  popupRef,
  onQueryChange,
  onKeyDown,
  onSelect
}: TemplateVariableTypeaheadPluginProps) {
  const emptyDescription = query.trim().length > 0 ? i18nText("agentFlow", "auto.matching_variable_found") : i18nText("agentFlow", "auto.no_variables_available");
  const popupStyle: CSSProperties | undefined = position
    ? {
        left: `${position.left}px`,
        top: `${position.top}px`,
        width: `${position.width}px`
      }
    : undefined;

  if (!open) {
    return null;
  }

  const popup = (
    <div
      ref={popupRef}
      className="agent-flow-templated-text-field__typeahead"
      role="listbox"
      aria-label={i18nText("agentFlow", "auto.variable_suggestions")}
      style={popupStyle}
      onKeyDown={onKeyDown}
      onMouseDown={(event) => event.preventDefault()}
    >
      <div className="agent-flow-templated-text-field__typeahead-search">
        <input
          aria-label={i18nText("agentFlow", "auto.search_variables")}
          role="searchbox"
          className="agent-flow-templated-text-field__typeahead-searchbox"
          autoFocus
          value={query}
          placeholder={i18nText("agentFlow", "auto.search_node_field")}
          onChange={(event) => onQueryChange(event.target.value)}
        />
      </div>
      {options.length === 0 ? (
        <div className="agent-flow-templated-text-field__typeahead-empty">
          <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={emptyDescription} />
        </div>
      ) : (
        options.map((option, optionIndex) => (
          <button
            key={option.value.join('.')}
            type="button"
            role="option"
            aria-selected={activeIndex === optionIndex}
            className={
              activeIndex === optionIndex
                ? 'agent-flow-templated-text-field__typeahead-option agent-flow-templated-text-field__typeahead-option--active'
                : 'agent-flow-templated-text-field__typeahead-option'
            }
            onMouseDown={(event) => event.preventDefault()}
            onClick={() => onSelect(option.value)}
          >
            {option.displayLabel}
          </button>
        ))
      )}
    </div>
  );

  if (typeof document === 'undefined') {
    return popup;
  }

  return createPortal(popup, document.body);
}
