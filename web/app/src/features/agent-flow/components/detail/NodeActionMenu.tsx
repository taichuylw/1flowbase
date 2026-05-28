import { MoreOutlined } from '@ant-design/icons';
import { Button, Dropdown } from 'antd';
import { i18nText } from '../../../../shared/i18n/text';

export function NodeActionMenu({
  onLocate,
  onCopy,
  onDelete
}: {
  onLocate: () => void;
  onCopy: () => void;
  onDelete: () => void;
}) {
  return (
    <Dropdown
      trigger={['click']}
      menu={{
        items: [
          {
            key: 'locate',
            label: i18nText("agentFlow", "auto.k_c4c6d554cb"),
            onClick: onLocate
          },
          {
            key: 'copy',
            label: i18nText("agentFlow", "auto.k_26ac0cbe0f"),
            onClick: onCopy
          },
          {
            key: 'delete',
            label: i18nText("agentFlow", "auto.k_ff37dc39f9"),
            danger: true,
            onClick: onDelete
          }
        ]
      }}
    >
      <Button aria-label={i18nText("agentFlow", "auto.k_77836d3a99")} icon={<MoreOutlined />} type="text" />
    </Dropdown>
  );
}
