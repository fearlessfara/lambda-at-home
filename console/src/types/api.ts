// TypeScript interfaces matching the Rust API models

export interface Function {
  function_id: string;
  function_name: string;
  runtime: string;
  role?: string;
  handler: string;
  code_sha256: string;
  description?: string;
  timeout: number;
  memory_size: number;
  environment: Record<string, string>;
  last_modified: string; // ISO 8601 string
  code_size: number;
  version: string;
  state: FunctionState;
  state_reason?: string;
  state_reason_code?: string;
}

export type FunctionState = 'Pending' | 'Active' | 'Inactive' | 'Failed';

export interface CreateFunctionRequest {
  function_name: string;
  runtime: string;
  role?: string;
  handler: string;
  code: FunctionCode;
  description?: string;
  timeout?: number;
  memory_size?: number;
  environment?: Record<string, string>;
  publish?: boolean;
}

export interface FunctionCode {
  zip_file?: string; // base64 encoded
  s3_bucket?: string;
  s3_key?: string;
  s3_object_version?: string;
}

export interface ListFunctionsResponse {
  functions: Function[];
  next_marker?: string;
  total_count?: number;
}

export interface InvokeRequest {
  function_name: string;
  invocation_type: InvocationType;
  log_type?: LogType;
  client_context?: string; // base64 encoded
  payload?: any;
  qualifier?: string;
}

export type InvocationType = 'RequestResponse' | 'Event' | 'DryRun';
export type LogType = 'None' | 'Tail';

export interface InvokeResponse {
  status_code: number;
  payload?: any;
  executed_version?: string;
  function_error?: FunctionError;
  log_result?: string; // base64 encoded log tail
  headers: Record<string, string>;
}

export type FunctionError = 'Handled' | 'Unhandled';

export interface ErrorShape {
  error_message: string;
  error_type: string;
  stack_trace?: string[];
}

// Available runtimes
export const AVAILABLE_RUNTIMES = [
  'nodejs18.x',
  'nodejs22.x',
  'python3.11',
  'rust'
] as const;

export type Runtime = typeof AVAILABLE_RUNTIMES[number];

// Docker Statistics Types
export interface DockerStats {
  system_info: DockerSystemInfo;
  disk_usage: DockerDiskUsage;
  version: DockerVersion;
  cache_stats: CacheStats;
}

export interface DockerSystemInfo {
  containers: number;
  containers_running: number;
  containers_paused: number;
  containers_stopped: number;
  images: number;
  driver: string;
  driver_status: Record<string, string>;
  memory_total: number;
  memory_available: number;
  cpu_count: number;
  operating_system: string;
  os_type: string;
  architecture: string;
  kernel_version: string;
  docker_root_dir: string;
  storage_driver: string;
  logging_driver: string;
  cgroup_driver: string;
  cgroup_version: string;
  experimental_build: boolean;
  server_version: string;
  cluster_store: string;
  cluster_advertise: string;
  default_runtime: string;
  live_restore_enabled: boolean;
  isolation: string;
  init_binary: string;
  product_license: string;
  default_address_pools: Array<{ base: string; size: number }>;
  http_proxy: string;
  https_proxy: string;
  no_proxy: string;
  name: string;
  labels: string[];
  security_options: string[];
  runtimes: Record<string, { path: string; runtime_args: string[] }>;
  default_ulimits: Record<string, { name: string; soft: number; hard: number }>;
  init_commit: { id: string; expected: string };
  containerd_commit: { id: string; expected: string };
  runc_commit: { id: string; expected: string };
  warnings: string[];
}

export interface DockerDiskUsage {
  layers_size: number;
  images: DockerImageUsage[];
  containers: DockerContainerUsage[];
  volumes: DockerVolumeUsage[];
  build_cache: DockerBuildCacheUsage[];
}

export interface DockerImageUsage {
  containers: number;
  created: number;
  id: string;
  labels: Record<string, string>;
  parent_id: string;
  repo_digests: string[];
  repo_tags: string[];
  shared_size: number;
  size: number;
  virtual_size: number;
}

export interface DockerContainerUsage {
  id: string;
  names: string[];
  image: string;
  image_id: string;
  command: string;
  created: number;
  ports: Array<{ ip: string; private_port: number; public_port: number; type: string }>;
  size_rw: number;
  size_root_fs: number;
  labels: Record<string, string>;
  state: string;
  status: string;
  host_config: Record<string, any>;
  network_settings: Record<string, any>;
  mounts: Array<{ name: string; source: string; destination: string; driver: string; mode: string; rw: boolean; propagation: string }>;
}

export interface DockerVolumeUsage {
  created_at: string;
  driver: string;
  labels: Record<string, string>;
  mountpoint: string;
  name: string;
  options: Record<string, string>;
  scope: string;
  size: number;
  usage_data: Record<string, any>;
}

export interface DockerBuildCacheUsage {
  id: string;
  parent: string;
  type: string;
  description: string;
  in_use: boolean;
  shared: boolean;
  size: number;
  created_at: string;
  last_used_at: string;
  usage_count: number;
}

export interface DockerVersion {
  platform: { name: string };
  components: Array<{ name: string; version: string; details: Record<string, any> }>;
  version: string;
  api_version: string;
  min_api_version: string;
  git_commit: string;
  go_version: string;
  os: string;
  arch: string;
  kernel_version: string;
  experimental: boolean;
  build_time: string;
}

export interface CacheStats {
  functions: CacheTypeStats;
  concurrency: CacheTypeStats;
  env_vars: CacheTypeStats;
  secrets: CacheTypeStats;
}

export interface CacheTypeStats {
  hits: number;
  misses: number;
  evictions: number;
  invalidations: number;
  size: number;
  hit_rate: number;
}

// Lambda Service Statistics Types
export interface LambdaServiceStats {
  total_functions: number;
  active_functions: number;
  stopped_functions: number;
  failed_functions: number;
  total_memory_mb: number;
  total_cpu_cores: number;
  warm_containers: number;
  active_containers: number;
  idle_containers: number;
  total_invocations_24h: number;
  successful_invocations_24h: number;
  failed_invocations_24h: number;
  avg_duration_ms: number;
  max_duration_ms: number;
  min_duration_ms: number;
}
