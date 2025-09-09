import { 
  Function, 
  CreateFunctionRequest, 
  ListFunctionsResponse,
  ErrorShape,
  DockerStats,
  LambdaServiceStats
} from '../types/api';

// Default to relative /api when served behind the same origin; override via VITE_API_URL in dev
const API_BASE_URL = import.meta.env.VITE_API_URL || '/api';

class ApiError extends Error {
  constructor(
    message: string,
    public status: number,
    public errorShape?: ErrorShape
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

async function handleResponse<T>(response: Response): Promise<T> {
  if (!response.ok) {
    let errorShape: ErrorShape | undefined;
    try {
      errorShape = await response.json();
    } catch {
      // Ignore JSON parse errors
    }
    
    throw new ApiError(
      errorShape?.error_message || `HTTP ${response.status}: ${response.statusText}`,
      response.status,
      errorShape
    );
  }
  
  return response.json();
}

export const api = {
  // Function management
  async listFunctions(params?: { marker?: string; maxItems?: number }): Promise<ListFunctionsResponse> {
    const url = new URL(`${API_BASE_URL}/2015-03-31/functions`);
    if (params?.marker) {
      url.searchParams.set('Marker', params.marker);
    }
    if (params?.maxItems) {
      url.searchParams.set('MaxItems', params.maxItems.toString());
    }
    const response = await fetch(url.toString());
    return handleResponse(response);
  },
  // Secrets admin
  async listSecrets(): Promise<{ secrets: { name: string; created_at: string }[] }> {
    const res = await fetch(`${API_BASE_URL}/admin/secrets`);
    return handleResponse(res);
  },
  async createSecret(data: { name: string; value: string }): Promise<void> {
    const res = await fetch(`${API_BASE_URL}/admin/secrets`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(data) });
    if (!res.ok) throw new ApiError(`HTTP ${res.status}`, res.status);
  },
  async deleteSecret(name: string): Promise<void> {
    const res = await fetch(`${API_BASE_URL}/admin/secrets/${encodeURIComponent(name)}`, { method: 'DELETE' });
    if (!res.ok) throw new ApiError(`HTTP ${res.status}`, res.status);
  },

  // API Gateway-style proxy invocation
  async invokeViaProxy(
    name: string,
    opts: { method: string; pathSuffix?: string; headers?: Record<string,string>; query?: Record<string,string>; body?: string }
  ): Promise<{ status: number; headers: Record<string,string>; bodyText: string }> {
    const path = `/${encodeURIComponent(name)}${opts.pathSuffix || ''}`;
    const qs = opts.query && Object.keys(opts.query).length
      ? `?${new URLSearchParams(opts.query).toString()}`
      : '';
    const url = `${API_BASE_URL}${path}${qs}`;

    const init: RequestInit = {
      method: opts.method || 'GET',
      headers: opts.headers || {},
    };
    if (opts.body && init.method !== 'GET') {
      init.body = opts.body;
    }

    const res = await fetch(url, init);
    const text = await res.text();
    const headers: Record<string,string> = {};
    res.headers.forEach((v, k) => { headers[k] = v; });
    return { status: res.status, headers, bodyText: text };
  },

  // Raw path request against API server (for API Gateway testing)
  async requestPath(
    path: string,
    opts: { method: string; headers?: Record<string,string>; query?: Record<string,string>; body?: string }
  ): Promise<{ status: number; headers: Record<string,string>; bodyText: string }> {
    const qs = opts.query && Object.keys(opts.query).length
      ? `?${new URLSearchParams(opts.query).toString()}`
      : '';
    const url = `${API_BASE_URL}${path}${qs}`;
    const init: RequestInit = { method: opts.method || 'GET', headers: opts.headers || {} };
    if (opts.body && init.method !== 'GET') init.body = opts.body;
    const res = await fetch(url, init);
    const text = await res.text();
    const headers: Record<string,string> = {}; res.headers.forEach((v,k)=> headers[k]=v);
    return { status: res.status, headers, bodyText: text };
  },

  // API Gateway routes admin
  async listApiRoutes(): Promise<{ routes: { route_id: string; path: string; method?: string; function_name: string; created_at: string }[] }> {
    const res = await fetch(`${API_BASE_URL}/admin/api-gateway/routes`);
    return handleResponse(res);
  },
  async createApiRoute(data: { path: string; method?: string; function_name: string }): Promise<{ route_id: string; path: string; method?: string; function_name: string; created_at: string }> {
    const res = await fetch(`${API_BASE_URL}/admin/api-gateway/routes`, {
      method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(data)
    });
    return handleResponse(res);
  },
  async deleteApiRoute(id: string): Promise<void> {
    const res = await fetch(`${API_BASE_URL}/admin/api-gateway/routes/${encodeURIComponent(id)}`, { method: 'DELETE' });
    if (!res.ok) throw new ApiError(`HTTP ${res.status}`, res.status);
  },

  async getFunction(name: string): Promise<Function> {
    const response = await fetch(`${API_BASE_URL}/2015-03-31/functions/${encodeURIComponent(name)}`);
    return handleResponse(response);
  },

  async createFunction(data: CreateFunctionRequest): Promise<Function> {
    const response = await fetch(`${API_BASE_URL}/2015-03-31/functions`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(data),
    });
    return handleResponse(response);
  },

  async updateFunctionConfiguration(name: string, data: Partial<Pick<Function,
    'handler' | 'timeout' | 'memory_size' | 'environment' | 'description'>>): Promise<Function> {
    // Map field names to API expectations
    const payload: any = {};
    if (data.handler !== undefined) payload.handler = data.handler;
    if (data.timeout !== undefined) payload.timeout = data.timeout;
    if (data.memory_size !== undefined) payload.memory_size = data.memory_size;
    if (data.environment !== undefined) payload.environment = data.environment as Record<string,string>;
    if (data.description !== undefined) payload.description = data.description;
    const response = await fetch(`${API_BASE_URL}/2015-03-31/functions/${encodeURIComponent(name)}/configuration`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload),
    });
    return handleResponse(response);
  },

  async deleteFunction(name: string): Promise<void> {
    const response = await fetch(`${API_BASE_URL}/2015-03-31/functions/${encodeURIComponent(name)}`, {
      method: 'DELETE',
    });
    
    if (!response.ok) {
      let errorShape: ErrorShape | undefined;
      try {
        errorShape = await response.json();
      } catch {
        // Ignore JSON parse errors
      }
      
      throw new ApiError(
        errorShape?.error_message || `HTTP ${response.status}: ${response.statusText}`,
        response.status,
        errorShape
      );
    }
  },

  // Function invocation
  async invokeFunction(name: string, payload: any, logType: 'None' | 'Tail' = 'Tail'): Promise<{
    response: any;
    statusCode: number;
    executedVersion?: string;
    functionError?: string;
    logResult?: string;
    duration?: number;
  }> {
    const response = await fetch(`${API_BASE_URL}/2015-03-31/functions/${encodeURIComponent(name)}/invocations`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Amz-Invocation-Type': 'RequestResponse',
        'X-Amz-Log-Type': logType,
      },
      body: JSON.stringify(payload),
    });

    if (!response.ok) {
      let errorShape: ErrorShape | undefined;
      try {
        errorShape = await response.json();
      } catch {
        // Ignore JSON parse errors
      }
      
      throw new ApiError(
        errorShape?.error_message || `HTTP ${response.status}: ${response.statusText}`,
        response.status,
        errorShape
      );
    }

    const responseData = await response.json();
    
    // Get duration from server-provided header
    const durationHeader = response.headers.get('X-Amz-Duration');
    const duration = durationHeader ? parseInt(durationHeader, 10) : undefined;
    
    return {
      response: responseData,
      statusCode: response.status,
      executedVersion: response.headers.get('X-Amz-Executed-Version') || undefined,
      functionError: response.headers.get('X-Amz-Function-Error') || undefined,
      logResult: response.headers.get('X-Amz-Log-Result') || undefined,
      duration,
    };

  },

  // Warm pool summary
  async warmPoolSummary(name: string): Promise<{
    total: number;
    warm_idle: number;
    active: number;
    stopped: number;
    entries: { container_id: string; state: string; idle_for_ms: number }[];
  }> {
    const response = await fetch(`${API_BASE_URL}/admin/warm-pool/${encodeURIComponent(name)}`);
    return handleResponse(response);
  },
  // Health check
  async healthCheck(): Promise<string> {
    const response = await fetch(`${API_BASE_URL}/healthz`);
    if (!response.ok) {
      throw new ApiError(`Health check failed: ${response.status}`, response.status);
    }
    return response.text();
  },

  // Docker statistics
  async getDockerStats(): Promise<DockerStats> {
    const response = await fetch(`${API_BASE_URL}/docker-stats`);
    return handleResponse(response);
  },

  // Lambda service statistics
  async getLambdaServiceStats(): Promise<LambdaServiceStats> {
    const response = await fetch(`${API_BASE_URL}/lambda-service-stats`);
    return handleResponse(response);
  },
};

export { ApiError };
