import axios, { AxiosError } from "axios";

// Create axios instance with base configuration
const api = axios.create({
  baseURL: import.meta.env.VITE_API_URL || "http://localhost:3000",
  headers: {
    "Content-Type": "application/json",
  },
});

// Request interceptor - automatically attach JWT token
api.interceptors.request.use(
  (config) => {
    const token = localStorage.getItem("jwt_token");
    if (token) {
      config.headers.Authorization = `Bearer ${token}`;
    }
    return config;
  },
  (error) => {
    return Promise.reject(error);
  },
);

// Response interceptor - handle 401 errors globally
api.interceptors.response.use(
  (response) => response,
  (error: AxiosError) => {
    if (error.response?.status === 401) {
      // Clear token and redirect to login
      localStorage.removeItem("jwt_token");
      localStorage.removeItem("user");

      // Only redirect if not already on login/register page
      if (
        !window.location.pathname.includes("/login") &&
        !window.location.pathname.includes("/register") &&
        !window.location.pathname.includes("/auth/oauth/callback")
      ) {
        window.location.href = "/login";
      }
    }
    return Promise.reject(error);
  },
);

export default api;

// ============================================================================
// Type Definitions
// ============================================================================

export interface User {
  id: string;
  email: string;
  username?: string;
  role: string;
}

export interface AuthResponse {
  token: string;
  user: User;
}

export interface RegisterRequest {
  email: string;
  password: string;
  username?: string;
}

export interface LoginRequest {
  email: string;
  password: string;
}

export interface OAuthStartResponse {
  auth_url: string;
  state: string;
}

export interface VirtualKey {
  id: string;
  key_prefix: string;
  name?: string;
  max_budget?: number;
  current_spend: number;
  rate_limit_rpm?: number;
  rate_limit_tpm?: number;
  allowed_models?: string[];
  expires_at?: string;
  blocked: boolean;
  created_at: string;
  last_used_at?: string;
}

export interface VirtualKeyResponse extends VirtualKey {
  key: string; // Full key only returned on creation
}

export interface CreateVirtualKeyRequest {
  name?: string;
  max_budget?: number;
  rate_limit_rpm?: number;
  rate_limit_tpm?: number;
  allowed_models?: string[];
  expires_at?: string;
}

export interface UpdateVirtualKeyRequest {
  key_id: string;
  name?: string;
  max_budget?: number;
  rate_limit_rpm?: number;
  rate_limit_tpm?: number;
  allowed_models?: string[];
  blocked?: boolean;
}

export interface Provider {
  id: string;
  name: string;
  status: "active" | "inactive" | "error";
  models: string[];
  endpoint: string;
  api_key_configured: boolean;
}

export interface ProvidersListResponse {
  object: string;
  data: Provider[];
}

// ============================================================================
// API Functions
// ============================================================================

// Authentication
export const authApi = {
  register: async (data: RegisterRequest): Promise<AuthResponse> => {
    const response = await api.post<AuthResponse>("/auth/register", data);
    return response.data;
  },

  login: async (data: LoginRequest): Promise<AuthResponse> => {
    const response = await api.post<AuthResponse>("/auth/login", data);
    return response.data;
  },

  logout: async (): Promise<void> => {
    await api.post("/auth/logout");
  },

  getCurrentUser: async (): Promise<User> => {
    const response = await api.get<User>("/auth/me");
    return response.data;
  },

  // OAuth
  startGitHubOAuth: async (): Promise<OAuthStartResponse> => {
    const response = await api.get<OAuthStartResponse>("/auth/oauth/github");
    return response.data;
  },

  handleOAuthCallback: async (
    code: string,
    state: string,
  ): Promise<AuthResponse> => {
    const response = await api.get<AuthResponse>(
      `/auth/oauth/callback?code=${code}&state=${state}`,
    );
    return response.data;
  },
};

// Virtual Keys
export const keysApi = {
  getUserKeys: async (): Promise<VirtualKey[]> => {
    const response = await api.get<VirtualKey[]>("/auth/keys");
    return response.data;
  },

  generateKey: async (
    data: CreateVirtualKeyRequest,
  ): Promise<VirtualKeyResponse> => {
    const response = await api.post<VirtualKeyResponse>(
      "/auth/key/generate",
      data,
    );
    return response.data;
  },

  updateKey: async (data: UpdateVirtualKeyRequest): Promise<VirtualKey> => {
    const response = await api.post<VirtualKey>("/auth/key/update", data);
    return response.data;
  },

  deleteKey: async (keyId: string): Promise<void> => {
    await api.post(`/auth/key/delete?key_id=${keyId}`);
  },

  getKeyInfo: async (keyId: string): Promise<VirtualKey> => {
    const response = await api.get<VirtualKey>(
      `/auth/key/info?key_id=${keyId}`,
    );
    return response.data;
  },
};

// Providers
export const providersApi = {
  listProviders: async (): Promise<Provider[]> => {
    const response = await api.get<ProvidersListResponse>("/v1/providers");
    return response.data.data;
  },

  configureProvider: async (
    providerId: string,
    apiKey: string,
    azureResourceName?: string,
  ): Promise<{
    success: boolean;
    message: string;
    provider_id: string;
    models_configured: number;
  }> => {
    const payload: {
      provider_id: string;
      api_key: string;
      azure_resource_name?: string;
    } = {
      provider_id: providerId,
      api_key: apiKey,
    };

    if (azureResourceName) {
      payload.azure_resource_name = azureResourceName;
    }

    const response = await api.post("/v1/providers/configure", payload);
    return response.data;
  },

  deleteProvider: async (
    providerId: string,
  ): Promise<{
    success: boolean;
    message: string;
    provider_id: string;
    models_removed: number;
  }> => {
    const response = await api.post("/v1/providers/delete", {
      provider_id: providerId,
    });
    return response.data;
  },
};
