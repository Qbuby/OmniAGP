import axios from 'axios';
import { useAuthStore } from '../stores/auth';

const api = axios.create({
  baseURL: '/api/v1',
});

api.interceptors.request.use((config) => {
  const token = useAuthStore.getState().token;
  if (token) {
    config.headers.Authorization = `Bearer ${token}`;
  }
  return config;
});

api.interceptors.response.use(
  (res) => res,
  (err) => {
    if (err.response?.status === 401) {
      const url = err.config?.url || '';
      if (!url.startsWith('/auth/')) {
        useAuthStore.getState().logout();
        window.location.href = '/login';
      }
    }
    return Promise.reject(err);
  },
);

export interface Project {
  id: string;
  name: string;
  description: string;
  status: string;
  created_at: string;
}

export interface ProjectList {
  projects: Project[];
  total: number;
}

export const projectsApi = {
  list: (params?: { search?: string; limit?: number; offset?: number }) =>
    api.get<ProjectList>('/projects', { params }).then((r) => r.data),

  get: (id: string) => api.get<Project>(`/projects/${id}`).then((r) => r.data),

  create: (data: { name: string; description: string; quality?: string }) =>
    api.post<Project>('/projects', data).then((r) => r.data),

  update: (id: string, data: { name?: string; description?: string }) =>
    api.put<Project>(`/projects/${id}`, data).then((r) => r.data),

  delete: (id: string) => api.delete(`/projects/${id}`),

  runPipeline: (id: string) =>
    api.post<{ status: string; project_id: string }>(`/projects/${id}/run`).then((r) => r.data),
};

export interface AuthProviders {
  local: boolean;
  github: boolean;
}

export interface AuthResponse {
  token: string;
  user: { id: string; username: string };
}

export const authApi = {
  providers: () => api.get<AuthProviders>('/auth/providers').then((r) => r.data),
  login: (data: { username: string; password: string }) =>
    api.post<AuthResponse>('/auth/login', data).then((r) => r.data),
  register: (data: { username: string; password: string }) =>
    api.post<AuthResponse>('/auth/register', data).then((r) => r.data),
  me: () => api.get('/auth/me').then((r) => r.data),
};

export default api;
