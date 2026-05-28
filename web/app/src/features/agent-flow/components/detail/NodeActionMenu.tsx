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
            label: i18nText("agentFlow", "auto.key_memgnffeml"),
            onClick: onLocate
          },
          {
            key: 'copy',
            label: i18nText("agentFlow", "auto.key_cgkmamloap"),
            onClick: onCopy
          },
          {
            key: 'delete',
            label: i18nText("agentFlow", "auto.key_ppdhnmdjpj"),
            danger: true,
            onClick: onDelete
          }
        ]
      }}
    >
      <Button aria-label={i18nText("agentFlow", "auto.key_hhidgndkjj")} icon={<MoreOutlined />} type="text" />
    </Dropdown>
  );
}
