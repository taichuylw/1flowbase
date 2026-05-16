import { Component, type ComponentType, type ErrorInfo, type ReactNode } from 'react';
import { App as AntdApp, ConfigProvider } from 'antd';
import { createRoot as defaultCreateRoot } from 'react-dom/client';

import type { BlockProtocolError } from '@1flowbase/page-protocol';
import {
  createNativeTrustedBlockPortalContainment,
  isNativeTrustedBlockRuntimeError,
  type NativeTrustedBlockHostAdapter,
  type NativeTrustedBlockPortalContainment,
  type NativeTrustedBlockPreparePlan
} from '@1flowbase/page-runtime';

export interface FrontstageNativeTrustedBlockReactComponentProps {
  plan: NativeTrustedBlockPreparePlan;
  props: NativeTrustedBlockPreparePlan['props'];
  portalContainment: NativeTrustedBlockPortalContainment;
}

export type FrontstageNativeTrustedBlockReactComponent =
  ComponentType<FrontstageNativeTrustedBlockReactComponentProps>;

export type FrontstageNativeTrustedBlockResolveComponent = (
  plan: NativeTrustedBlockPreparePlan
) => FrontstageNativeTrustedBlockReactComponent;

export interface FrontstageNativeTrustedBlockReactRoot {
  render(children: ReactNode): void;
  unmount(): void;
}

export type FrontstageNativeTrustedBlockCreateRoot = (
  root: Element
) => FrontstageNativeTrustedBlockReactRoot;

export interface FrontstageNativeTrustedBlockProviderContext {
  plan: NativeTrustedBlockPreparePlan;
  root: Element;
  portalContainment: NativeTrustedBlockPortalContainment;
}

export type FrontstageNativeTrustedBlockProviderWrapper = (
  children: ReactNode,
  context: FrontstageNativeTrustedBlockProviderContext
) => ReactNode;

export interface FrontstageNativeTrustedBlockRuntimeErrorContext
  extends FrontstageNativeTrustedBlockProviderContext {
  blockId: string;
  componentStack?: string;
}

export type FrontstageNativeTrustedBlockRuntimeErrorHandler = (
  error: BlockProtocolError,
  context: FrontstageNativeTrustedBlockRuntimeErrorContext
) => void;

export interface FrontstageNativeTrustedBlockReactAdapterOptions {
  resolveComponent: FrontstageNativeTrustedBlockResolveComponent;
  createRoot?: FrontstageNativeTrustedBlockCreateRoot;
  providerWrapper?: FrontstageNativeTrustedBlockProviderWrapper;
  onRuntimeError?: FrontstageNativeTrustedBlockRuntimeErrorHandler;
}

export function createFrontstageNativeTrustedBlockReactAdapter(
  options: FrontstageNativeTrustedBlockReactAdapterOptions
): NativeTrustedBlockHostAdapter {
  return {
    async mount(input) {
      const rootElement = validateRootElement(input.root);
      const Component = options.resolveComponent(input.plan);
      const portalContainment = createPortalContainment(rootElement);
      const reactRoot = (options.createRoot ?? defaultCreateRoot)(rootElement);
      const providerContext = {
        plan: input.plan,
        root: rootElement,
        portalContainment
      };
      let didUnmount = false;

      reactRoot.render(
        wrapWithHostProviders(
          <FrontstageNativeTrustedBlockErrorBoundary
            context={{
              ...providerContext,
              blockId: input.plan.blockId
            }}
            onRuntimeError={options.onRuntimeError}
          >
            <Component
              plan={input.plan}
              props={input.plan.props}
              portalContainment={portalContainment}
            />
          </FrontstageNativeTrustedBlockErrorBoundary>,
          providerContext,
          options.providerWrapper
        )
      );

      return {
        dispose() {
          if (didUnmount) {
            return;
          }

          didUnmount = true;
          reactRoot.unmount();
        }
      };
    }
  };
}

interface FrontstageNativeTrustedBlockErrorBoundaryProps {
  children: ReactNode;
  context: FrontstageNativeTrustedBlockRuntimeErrorContext;
  onRuntimeError?: FrontstageNativeTrustedBlockRuntimeErrorHandler;
}

interface FrontstageNativeTrustedBlockErrorBoundaryState {
  didCatch: boolean;
}

class FrontstageNativeTrustedBlockErrorBoundary extends Component<
  FrontstageNativeTrustedBlockErrorBoundaryProps,
  FrontstageNativeTrustedBlockErrorBoundaryState
> {
  state: FrontstageNativeTrustedBlockErrorBoundaryState = { didCatch: false };

  static getDerivedStateFromError(): FrontstageNativeTrustedBlockErrorBoundaryState {
    return { didCatch: true };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
    this.props.onRuntimeError?.(
      createRuntimeRenderError(error),
      createRuntimeErrorContext(this.props.context, errorInfo)
    );
  }

  render(): ReactNode {
    if (this.state.didCatch) {
      return null;
    }

    return this.props.children;
  }
}

function validateRootElement(root: unknown): Element {
  if (typeof Element === 'undefined' || !(root instanceof Element)) {
    throw new Error(
      'Native trusted block React adapter root must be a DOM Element.'
    );
  }

  return root;
}

function createPortalContainment(
  root: Element
): NativeTrustedBlockPortalContainment {
  const result = createNativeTrustedBlockPortalContainment({ root });
  if (!result.ok) {
    throw new Error(
      result.errors.map((error) => error.message).join(' ') ||
        'Native trusted block portal containment creation failed.'
    );
  }

  return result.containment;
}

function createRuntimeRenderError(error: unknown): BlockProtocolError {
  if (isNativeTrustedBlockRuntimeError(error) && error.errors.length > 0) {
    return error.errors[0];
  }

  return {
    code: 'runtime_error',
    path: 'runtime.render',
    message: getErrorMessage(error)
  };
}

function createRuntimeErrorContext(
  context: FrontstageNativeTrustedBlockRuntimeErrorContext,
  errorInfo: ErrorInfo
): FrontstageNativeTrustedBlockRuntimeErrorContext {
  const componentStack = errorInfo.componentStack?.trim();

  if (!componentStack) {
    return context;
  }

  return {
    ...context,
    componentStack
  };
}

function getErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message) {
    return error.message;
  }

  if (typeof error === 'string' && error.trim() !== '') {
    return error;
  }

  return 'unknown error';
}

function wrapWithHostProviders(
  children: ReactNode,
  context: FrontstageNativeTrustedBlockProviderContext,
  providerWrapper?: FrontstageNativeTrustedBlockProviderWrapper
): ReactNode {
  if (providerWrapper) {
    return providerWrapper(children, context);
  }

  const getPopupContainer = () => context.root as HTMLElement;

  return (
    <ConfigProvider getPopupContainer={getPopupContainer}>
      <AntdApp>{children}</AntdApp>
    </ConfigProvider>
  );
}
