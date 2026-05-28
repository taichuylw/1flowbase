import { Alert, Button, Modal, Typography, Upload } from 'antd';
import type { UploadFile } from 'antd/es/upload/interface';
import { i18nText } from '../../../../shared/i18n/text';

export function PluginUploadInstallModal({
  open,
  submitting,
  resultSummary,
  errorMessage,
  fileList,
  onClose,
  onChange,
  onSubmit
}: {
  open: boolean;
  submitting: boolean;
  resultSummary: {
    displayName: string;
    version: string;
    trustLabel: string;
    availabilityLabel: string;
  } | null;
  errorMessage: string | null;
  fileList: UploadFile[];
  onClose: () => void;
  onChange: (nextFiles: UploadFile[]) => void;
  onSubmit: () => void;
}) {
  return (
    <Modal
      open={open}
      title={i18nText("settings", "auto.key_dbpeahngoi")}
      onCancel={onClose}
      footer={null}
      destroyOnHidden
    >
      <div className="model-provider-panel__upload-modal">
        <Typography.Paragraph type="secondary">
          {i18nText("settings", "auto.key_jchkfchgfb")}</Typography.Paragraph>
        <Upload.Dragger
          beforeUpload={() => false}
          maxCount={1}
          fileList={fileList}
          onChange={({ fileList: nextFiles }) => onChange(nextFiles)}
        >
          {i18nText("settings", "auto.key_eepfkgbpgb")}</Upload.Dragger>
        {resultSummary ? (
          <Alert
            type="success"
            showIcon
            message={`${resultSummary.displayName} ${resultSummary.version}`}
            description={i18nText("settings", "auto.key_agpfeolilc", { value1: resultSummary.trustLabel, value2: resultSummary.availabilityLabel })}
          />
        ) : null}
        {errorMessage ? <Alert type="error" showIcon message={errorMessage} /> : null}
        <Button type="primary" block loading={submitting} onClick={onSubmit}>
          {i18nText("settings", "auto.key_dloakpbanm")}</Button>
      </div>
    </Modal>
  );
}
