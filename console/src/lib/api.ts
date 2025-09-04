import { 
  Function, 
  CreateFunctionRequest, 
  ListFunctionsResponse,
  ErrorShape 
} from '../types/api';

const API_BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:9000';

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
  async listFunctions(): Promise<ListFunctionsResponse> {
    const response = await fetch(`${API_BASE_URL}/2015-03-31/functions`);
    return handleResponse(response);
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
    const startTime = Date.now();
    
    const response = await fetch(`${API_BASE_URL}/2015-03-31/functions/${encodeURIComponent(name)}/invocations`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Amz-Invocation-Type': 'RequestResponse',
        'X-Amz-Log-Type': logType,
      },
      body: JSON.stringify(payload),
    });

    const endTime = Date.now();
    const duration = endTime - startTime;

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
    
    return {
      response: responseData,
      statusCode: response.status,
      executedVersion: response.headers.get('X-Amz-Executed-Version') || undefined,
      functionError: response.headers.get('X-Amz-Function-Error') || undefined,
      logResult: response.headers.get('X-Amz-Log-Result') || undefined,
      duration,
    };
  },

  // Health check
  async healthCheck(): Promise<string> {
    const response = await fetch(`${API_BASE_URL}/healthz`);
    if (!response.ok) {
      throw new ApiError(`Health check failed: ${response.status}`, response.status);
    }
    return response.text();
  },
};

export { ApiError };
