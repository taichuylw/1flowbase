import { DeleteOutlined, PlusOutlined } from '@ant-design/icons';
import { Button, Empty, Input } from 'antd';

import type { FlowStartModelDescriptor } from '@1flowbase/flow-schema';

function normalizeModelList(value: unknown): FlowStartModelDescriptor[] {
  if (!Array.isArray(value)) {
    return [];
  }

  return value
    .map((item) => {
      if (typeof item === 'string') {
        return { id: item };
      }
      if (typeof item !== 'object' || item === null) {
        return null;
      }
      const source = item as Record<string, unknown>;
      const id =
        typeof source.id === 'string'
          ? source.id
          : typeof source.model === 'string'
            ? source.model
            : typeof source.value === 'string'
              ? source.value
              : '';
      const name =
        typeof source.name === 'string'
          ? source.name
          : typeof source.label === 'string'
            ? source.label
            : typeof source.display_name === 'string'
              ? source.display_name
              : undefined;

      return {
        id,
        ...(name ? { name } : {})
      };
    })
    .filter((item): item is FlowStartModelDescriptor => item !== null);
}

function cleanRows(rows: FlowStartModelDescriptor[]) {
  return rows.map((row) => ({
    id: row.id.trim(),
    ...(row.name?.trim() ? { name: row.name.trim() } : {})
  }));
}

export function StartModelListField({
  value,
  onChange
}: {
  value: unknown;
  onChange: (value: FlowStartModelDescriptor[]) => void;
}) {
  const rows = normalizeModelList(value);

  function updateRow(index: number, patch: Partial<FlowStartModelDescriptor>) {
    onChange(
      cleanRows(
        rows.map((row, rowIndex) =>
          rowIndex === index ? { ...row, ...patch } : row
        )
      )
    );
  }

  return (
    <div className="agent-flow-start-model-list">
      <div className="agent-flow-start-input-fields__header">
        <Button
          aria-label="新增模型"
          icon={<PlusOutlined />}
          size="small"
          type="text"
          onClick={() => onChange([...rows, { id: '' }])}
        />
      </div>
      {rows.length > 0 ? (
        <div className="agent-flow-node-detail__list">
          {rows.map((row, index) => (
            <div
              className="agent-flow-node-detail__list-item"
              data-testid={`start-model-row-${index + 1}`}
              key={index}
            >
              <div className="agent-flow-node-detail__list-item-left">
                <Input
                  aria-label={`模型 ID ${index + 1}`}
                  placeholder="model-id"
                  value={row.id}
                  onChange={(event) =>
                    updateRow(index, { id: event.target.value })
                  }
                />
                <Input
                  aria-label={`模型显示名 ${index + 1}`}
                  placeholder="display name"
                  value={row.name ?? ''}
                  onChange={(event) =>
                    updateRow(index, { name: event.target.value })
                  }
                />
              </div>
              <Button
                aria-label={`删除模型 ${index + 1}`}
                danger
                icon={<DeleteOutlined />}
                size="small"
                type="text"
                onClick={() =>
                  onChange(rows.filter((_, rowIndex) => rowIndex !== index))
                }
              />
            </div>
          ))}
        </div>
      ) : (
        <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="暂无模型" />
      )}
    </div>
  );
}
