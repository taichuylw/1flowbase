import { apiFetch } from './transport';

export interface ConsoleSystemRuntimeLocaleMeta {
  requested_locale: string | null;
  resolved_locale: string;
  source: string;
  fallback_locale: string;
  supported_locales: string[];
}

export interface ConsoleSystemRuntimeTopology {
  relationship: string;
}

export interface ConsoleSystemRuntimeService {
  reachable: boolean;
  service: string;
  status: string | null;
  version: string | null;
  host_fingerprint: string | null;
}

export interface ConsoleSystemRuntimeServices {
  api_server: ConsoleSystemRuntimeService;
  plugin_runner: ConsoleSystemRuntimeService;
}

export interface ConsoleSystemRuntimePlatform {
  os: string;
  arch: string;
  libc: string | null;
  rust_target_triple: string;
}

export interface ConsoleSystemRuntimeCpu {
  logical_count: number;
}

export interface ConsoleSystemRuntimeMemory {
  total_bytes: number;
  total_gb: number;
  available_bytes: number;
  available_gb: number;
  process_bytes: number;
  process_gb: number;
}

export interface ConsoleSystemRuntimeHost {
  host_fingerprint: string;
  platform: ConsoleSystemRuntimePlatform;
  cpu: ConsoleSystemRuntimeCpu;
  memory: ConsoleSystemRuntimeMemory;
  services: string[];
}

export interface ConsoleNativeResumeWorkerRuntime {
  worker_id: string | null;
  status: string;
  started_at: string | null;
  last_heartbeat_at: string | null;
  last_poll_at: string | null;
  last_claimed_at: string | null;
  last_success_at: string | null;
  last_error_at: string | null;
  last_error: string | null;
  current_request_id: string | null;
  current_flow_run_id: string | null;
  processed_count: number;
  succeeded_count: number;
  failed_count: number;
  last_duration_ms: number | null;
}

export interface ConsoleNativeResumeWorkerQueue {
  pending_count: number;
  claimed_count: number;
  succeeded_count: number;
  failed_count: number;
  cancelled_count: number;
  expired_claim_count: number;
  oldest_pending_created_at: string | null;
  oldest_pending_age_seconds: number | null;
}

export interface ConsoleNativeResumeWorkerObservation {
  runtime: ConsoleNativeResumeWorkerRuntime;
  queue: ConsoleNativeResumeWorkerQueue;
}

export interface ConsoleSystemRuntimeProfile {
  provider_install_root: string;
  host_extension_dropin_root: string;
  locale_meta: ConsoleSystemRuntimeLocaleMeta;
  topology: ConsoleSystemRuntimeTopology;
  services: ConsoleSystemRuntimeServices;
  hosts: ConsoleSystemRuntimeHost[];
  native_resume_worker: ConsoleNativeResumeWorkerObservation;
}

export function fetchConsoleSystemRuntimeProfile(baseUrl?: string) {
  return apiFetch<ConsoleSystemRuntimeProfile>({
    path: '/api/console/system/runtime-profile',
    baseUrl
  });
}
