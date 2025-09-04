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
  'python3.11',
  'rust'
] as const;

export type Runtime = typeof AVAILABLE_RUNTIMES[number];
