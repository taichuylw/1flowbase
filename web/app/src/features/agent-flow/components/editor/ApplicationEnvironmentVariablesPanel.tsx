import {
  CloseOutlined,
  CodeOutlined,
  DeleteOutlined,
  EditOutlined,
  PlusOutlined
} from '@ant-design/icons';
import {
  Button,
  Empty,
  Form,
  Input,
  Modal,
  Popconfirm,
  Select,
  Space,
  Tooltip,
  Typography
} from 'antd';
import { useEffect, useMemo, useState } from 'react';

import {
  formatEnvironmentVariableTitle,
  type AgentFlowEnvironmentVariable
} from '../../lib/application-environment-variables';

const valueTypeOptions = [
  'string',
  'number',
  'boolean',
  'object',
  'array[string]',
  'array[number]',
  'array[boolean]',
  'array[object]'
].map((value) => ({ label: value, value }));

interface EnvironmentVariableFormValues {
  name: string;
  value_type: string;
  value: string;
  description?: string;
}

interface ApplicationEnvironmentVariablesPanelProps {
  variables: AgentFlowEnvironmentVariable[];
  loading?: boolean;
  onClose: () => void;
  onSave: (variables: AgentFlowEnvironmentVariable[]) => void;
}

function formatVariableValue(value: unknown) {
  if (typeof value === 'string') {
    return value;
  }

  return JSON.stringify(value, null, 2);
}

function parseVariableValue(valueType: string, rawValue: string) {
  if (valueType === 'string') {
    return rawValue;
  }

  if (valueType === 'number') {
    const parsed = Number(rawValue);
    if (!Number.isFinite(parsed)) {
      throw new Error('value');
    }
    return parsed;
  }

  if (valueType === 'boolean') {
    if (rawValue === 'true') {
      return true;
    }
    if (rawValue === 'false') {
      return false;
    }
    throw new Error('value');
  }

  return JSON.parse(rawValue);
}

function validateParsedValue(valueType: string, value: unknown) {
  const valid =
    (valueType === 'object' &&
      Boolean(value && typeof value === 'object' && !Array.isArray(value))) ||
    (valueType === 'array[string]' &&
      Array.isArray(value) &&
      value.every((item) => typeof item === 'string')) ||
    (valueType === 'array[number]' &&
      Array.isArray(value) &&
      value.every((item) => typeof item === 'number')) ||
    (valueType === 'array[boolean]' &&
      Array.isArray(value) &&
      value.every((item) => typeof item === 'boolean')) ||
    (valueType === 'array[object]' &&
      Array.isArray(value) &&
      value.every(
        (item) =>
          Boolean(item && typeof item === 'object') && !Array.isArray(item)
      ));

  if (['string', 'number', 'boolean'].includes(valueType) || valid) {
    return;
  }

  throw new Error('value');
}

export function ApplicationEnvironmentVariablesPanel({
  variables,
  loading = false,
  onClose,
  onSave
}: ApplicationEnvironmentVariablesPanelProps) {
  const [draftVariables, setDraftVariables] = useState(variables);
  const [editingIndex, setEditingIndex] = useState<number | null>(null);
  const [modalOpen, setModalOpen] = useState(false);
  const [valueError, setValueError] = useState<string | null>(null);
  const [form] = Form.useForm<EnvironmentVariableFormValues>();

  useEffect(() => {
    setDraftVariables(variables);
  }, [variables]);

  const editingVariable =
    editingIndex === null ? null : draftVariables[editingIndex];
  const modalTitle = editingVariable ? '编辑环境变量' : '添加环境变量';

  const existingNames = useMemo(
    () =>
      new Set(
        draftVariables
          .filter((_, index) => index !== editingIndex)
          .map((variable) => variable.name)
      ),
    [draftVariables, editingIndex]
  );

  function openCreateModal() {
    setEditingIndex(null);
    setValueError(null);
    form.setFieldsValue({
      name: '',
      value_type: 'string',
      value: '',
      description: ''
    });
    setModalOpen(true);
  }

  function openEditModal(index: number) {
    const variable = draftVariables[index];
    setEditingIndex(index);
    setValueError(null);
    form.setFieldsValue({
      name: variable.name,
      value_type: variable.value_type,
      value: formatVariableValue(variable.value),
      description: variable.description
    });
    setModalOpen(true);
  }

  async function submitModal() {
    const values = await form.validateFields();
    let parsedValue: unknown;

    try {
      parsedValue = parseVariableValue(values.value_type, values.value);
      validateParsedValue(values.value_type, parsedValue);
    } catch {
      setValueError('变量值与类型不匹配');
      return;
    }

    const nextVariable: AgentFlowEnvironmentVariable = {
      name: values.name,
      value_type: values.value_type,
      value: parsedValue,
      description: values.description?.trim() ?? ''
    };
    const nextVariables =
      editingIndex === null
        ? [...draftVariables, nextVariable]
        : draftVariables.map((variable, index) =>
            index === editingIndex ? nextVariable : variable
          );

    setDraftVariables(nextVariables);
    onSave(nextVariables);
    setModalOpen(false);
  }

  function deleteVariable(index: number) {
    const nextVariables = draftVariables.filter(
      (_, candidate) => candidate !== index
    );
    setDraftVariables(nextVariables);
    onSave(nextVariables);
  }

  return (
    <section
      aria-label="环境变量"
      className="agent-flow-editor__environment-variables-panel"
    >
      <header className="agent-flow-editor__system-variables-header">
        <div className="agent-flow-editor__system-variables-heading">
          <Typography.Title level={3}>环境变量</Typography.Title>
          <Typography.Text type="secondary">
            环境变量属于当前应用，可被画布内任意节点通过 env.xxx 引用。
          </Typography.Text>
        </div>
        <Button
          aria-label="关闭环境变量"
          icon={<CloseOutlined />}
          type="text"
          onClick={onClose}
        />
      </header>
      <div className="agent-flow-editor__environment-variables-body">
        <div className="agent-flow-editor__environment-variables-toolbar">
          <Button
            icon={<PlusOutlined />}
            type="primary"
            onClick={openCreateModal}
          >
            添加环境变量
          </Button>
        </div>
        <div
          aria-busy={loading}
          className="agent-flow-editor__environment-variable-list"
        >
          {draftVariables.length === 0 ? (
            <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="No data" />
          ) : (
            draftVariables.map((variable, index) => (
              <div
                className="agent-flow-editor__environment-variable-row"
                key={variable.name}
              >
                <CodeOutlined className="agent-flow-editor__environment-variable-icon" />
                <div className="agent-flow-editor__environment-variable-content">
                  <div className="agent-flow-editor__environment-variable-title">
                    <Typography.Text strong>
                      {formatEnvironmentVariableTitle(variable.name)}
                    </Typography.Text>
                    <Typography.Text type="secondary">
                      {variable.value_type}
                    </Typography.Text>
                  </div>
                  <Typography.Text
                    className="agent-flow-editor__environment-variable-value"
                    type="secondary"
                  >
                    {formatVariableValue(variable.value)}
                  </Typography.Text>
                  {variable.description ? (
                    <Typography.Text
                      className="agent-flow-editor__environment-variable-description"
                      type="secondary"
                    >
                      {variable.description}
                    </Typography.Text>
                  ) : null}
                </div>
                <Space size={2}>
                  <Tooltip title="编辑">
                    <Button
                      aria-label={`编辑 ${variable.name}`}
                      icon={<EditOutlined />}
                      size="small"
                      type="text"
                      onClick={() => openEditModal(index)}
                    />
                  </Tooltip>
                  <Popconfirm
                    title="删除环境变量"
                    okText="删除"
                    cancelText="取消"
                    onConfirm={() => deleteVariable(index)}
                  >
                    <Tooltip title="删除">
                      <Button
                        aria-label={`删除 ${variable.name}`}
                        danger
                        icon={<DeleteOutlined />}
                        size="small"
                        type="text"
                      />
                    </Tooltip>
                  </Popconfirm>
                </Space>
              </div>
            ))
          )}
        </div>
      </div>
      <Modal
        title={modalTitle}
        open={modalOpen}
        confirmLoading={loading}
        okText="保存"
        cancelText="取消"
        width={420}
        onCancel={() => setModalOpen(false)}
        onOk={() => {
          void submitModal();
        }}
      >
        <Form form={form} layout="vertical">
          <Form.Item
            name="name"
            label="名称"
            rules={[
              { required: true, message: '请输入变量名' },
              {
                pattern: /^[A-Za-z][A-Za-z0-9]*$/,
                message: '仅支持字母开头，包含大小写字母和数字'
              },
              {
                validator(_, value) {
                  if (value && existingNames.has(value)) {
                    return Promise.reject(new Error('变量名已存在'));
                  }
                  return Promise.resolve();
                }
              }
            ]}
          >
            <Input placeholder="ApiBaseUrl" />
          </Form.Item>
          <Form.Item
            name="value_type"
            label="类型"
            rules={[{ required: true, message: '请选择类型' }]}
          >
            <Select options={valueTypeOptions} />
          </Form.Item>
          <Form.Item
            name="value"
            label="值"
            validateStatus={valueError ? 'error' : undefined}
            help={valueError ?? undefined}
            rules={[{ required: true, message: '请输入变量值' }]}
          >
            <Input.TextArea
              autoSize={{ minRows: 3, maxRows: 8 }}
              onChange={() => setValueError(null)}
            />
          </Form.Item>
          <Form.Item name="description" label="描述">
            <Input.TextArea autoSize={{ minRows: 2, maxRows: 4 }} />
          </Form.Item>
        </Form>
      </Modal>
    </section>
  );
}
