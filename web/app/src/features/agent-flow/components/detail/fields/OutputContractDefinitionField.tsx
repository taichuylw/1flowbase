import { DeleteOutlined, PlusOutlined } from '@ant-design/icons';
import { Button, Empty, Input, Select, Typography } from 'antd';

import type { FlowNodeDocument } from '@1flowbase/flow-schema';
import { i18nText } from '../../../../../shared/i18n/text';

const valueTypeOptions = [
  { value: 'string', label: 'String' },
  { value: 'number', label: 'Number' },
  { value: 'boolean', label: 'Boolean' },
  { value: 'array', label: 'Array' },
  { value: 'json', label: 'JSON' },
  { value: 'unknown', label: 'Unknown' }
] satisfies Array<{
  value: FlowNodeDocument['outputs'][number]['valueType'];
  label: string;
}>;

function createNextOutput(index: number): FlowNodeDocument['outputs'][number] {
  const key = `output_${index + 1}`;

  return {
    key,
    title: key,
    valueType: 'string'
  };
}

export function OutputContractDefinitionField({
  value,
  onChange
}: {
  value: FlowNodeDocument['outputs'];
  onChange: (value: FlowNodeDocument['outputs']) => void;
}) {
  return (
    <div className="agent-flow-output-contract-editor">
      <div className="agent-flow-output-contract-editor__header">
        <Typography.Text className="agent-flow-node-detail__section-subtitle">
          {i18nText("agentFlow", "auto.key_dpbdpcpjln")}</Typography.Text>
        <Button
          aria-label={i18nText("agentFlow", "auto.key_igfnnomnkj")}
          icon={<PlusOutlined />}
          size="small"
          type="text"
          onClick={() => onChange([...value, createNextOutput(value.length)])}
        />
      </div>
      {value.length > 0 ? (
        <div className="agent-flow-output-contract-editor__list">
          {value.map((output, index) => (
            <div
              key={`${output.key}-${index}`}
              className="agent-flow-output-contract-editor__row"
            >
              <label className="agent-flow-output-contract-editor__cell">
                <span>{i18nText("agentFlow", "auto.key_gdnfjhhnog")}</span>
                <Input
                  aria-label={i18nText("agentFlow", "auto.key_facokndckb", { value1: index + 1 })}
                  value={output.key}
                  onChange={(event) =>
                    onChange(
                      value.map((candidate, candidateIndex) =>
                        candidateIndex === index
                          ? { ...candidate, key: event.target.value }
                          : candidate
                      )
                    )
                  }
                />
              </label>
              <label className="agent-flow-output-contract-editor__cell">
                <span>{i18nText("agentFlow", "auto.key_mballpfnnn")}</span>
                <Input
                  aria-label={i18nText("agentFlow", "auto.key_aimodjbajj", { value1: index + 1 })}
                  value={output.title}
                  onChange={(event) =>
                    onChange(
                      value.map((candidate, candidateIndex) =>
                        candidateIndex === index
                          ? { ...candidate, title: event.target.value }
                          : candidate
                      )
                    )
                  }
                />
              </label>
              <label className="agent-flow-output-contract-editor__cell">
                <span>{i18nText("agentFlow", "auto.key_oeoegmhcdf")}</span>
                <Select
                  aria-label={i18nText("agentFlow", "auto.key_pojgcmimep", { value1: index + 1 })}
                  options={valueTypeOptions}
                  value={output.valueType}
                  onChange={(valueType) =>
                    onChange(
                      value.map((candidate, candidateIndex) =>
                        candidateIndex === index
                          ? { ...candidate, valueType }
                          : candidate
                      )
                    )
                  }
                />
              </label>
              <Button
                aria-label={i18nText("agentFlow", "auto.key_koocgghegk", { value1: output.key || index + 1 })}
                className="agent-flow-output-contract-editor__delete"
                danger
                icon={<DeleteOutlined />}
                size="small"
                type="text"
                onClick={() =>
                  onChange(
                    value.filter((_, outputIndex) => outputIndex !== index)
                  )
                }
              />
            </div>
          ))}
        </div>
      ) : (
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={i18nText("agentFlow", "auto.key_bndolnolcc")}
        />
      )}
    </div>
  );
}
