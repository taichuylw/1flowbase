import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { PropsWithChildren } from 'react';
import { useState } from 'react';

import { AppThemeProvider } from '@1flowbase/ui';

import { AppI18nProvider } from './AppI18nProvider';

export function AppProviders({ children }: PropsWithChildren) {
  const [queryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            retry: false
          },
          mutations: {
            retry: false
          }
        }
      })
  );

  return (
    <AppThemeProvider>
      <AppI18nProvider>
        <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
      </AppI18nProvider>
    </AppThemeProvider>
  );
}
