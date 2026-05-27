import { Alert, Button, Form, Input, Space, Typography, theme } from 'antd';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

import { useNavigate } from '@tanstack/react-router';

import { useAuthStore } from '../../../state/auth-store';
import { fetchCurrentMe, signInWithPassword } from '../api/session';
import { HeroAnimation } from '../components/HeroAnimation';

export function SignInPage() {
  const navigate = useNavigate();
  const { t } = useTranslation('auth');
  const { token } = theme.useToken();
  const setAuthenticated = useAuthStore((state) => state.setAuthenticated);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const handleFinish = async (values: { identifier: string; password: string }) => {
    setSubmitting(true);
    setErrorMessage(null);

    try {
      const session = await signInWithPassword(values);
      const me = await fetchCurrentMe();

      setAuthenticated({
        csrfToken: session.csrf_token,
        actor: {
          id: me.id,
          account: me.account,
          effective_display_role: session.effective_display_role,
          current_workspace_id: session.current_workspace_id
        },
        me
      });
      await navigate({ to: '/' });
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : t('signIn.errorFallback'));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div style={{ display: 'flex', minHeight: '100vh', width: '100vw' }}>
      <HeroAnimation />
      <div 
        style={{ 
          flex: '0 0 480px', 
          display: 'flex', 
          flexDirection: 'column', 
          justifyContent: 'center',
          padding: '0 64px',
          background: `linear-gradient(145deg, ${token.colorBgContainer} 60%, ${token.colorBgLayout} 100%)`,
          boxShadow: '-10px 0 32px rgba(0, 0, 0, 0.05)',
          borderLeft: `1px solid ${token.colorBorderSecondary}`,
          position: 'relative',
          zIndex: 10
        }}
      >
        <Space direction="vertical" size="large" style={{ width: '100%' }}>
          <div style={{ textAlign: 'center', marginBottom: 16 }}>
            <Typography.Title level={2} style={{ margin: 0 }}>
              {t('signIn.title')}
            </Typography.Title>
          </div>
          {errorMessage ? <Alert type="error" message={errorMessage} showIcon /> : null}
          <Form layout="vertical" onFinish={handleFinish} autoComplete="off">
            <Form.Item
              label={t('signIn.identifier.label')}
              name="identifier"
              rules={[{ required: true, message: t('signIn.identifier.required') }]}
            >
              <Input placeholder={t('signIn.identifier.placeholder')} size="large" />
            </Form.Item>
            <Form.Item
              label={t('signIn.password.label')}
              name="password"
              rules={[{ required: true, message: t('signIn.password.required') }]}
            >
              <Input.Password placeholder={t('signIn.password.placeholder')} size="large" />
            </Form.Item>
            <Button type="primary" htmlType="submit" loading={submitting} block size="large">
              {t('signIn.submit')}
            </Button>
          </Form>
        </Space>
        
        <div style={{ textAlign: 'center', marginTop: 48 }}>
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            <a 
              href="https://www.taichuy.com" 
              target="_blank" 
              rel="noreferrer" 
              style={{ color: token.colorTextDescription, textDecoration: 'none' }}
            >
              {t('signIn.footer')}
            </a>
          </Typography.Text>
        </div>
      </div>
    </div>
  );
}
