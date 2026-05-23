import { useState, useEffect } from 'react';
import { useNavigate, useSearchParams, Link } from 'react-router-dom';
import { useAuthStore } from '../stores/auth';
import { authApi, AuthProviders } from '../api/client';

export default function LoginPage() {
  const [searchParams] = useSearchParams();
  const setAuth = useAuthStore((s) => s.setAuth);
  const token = useAuthStore((s) => s.token);
  const navigate = useNavigate();

  const [providers, setProviders] = useState<AuthProviders | null>(null);
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  const next = searchParams.get('next') || '/projects';

  useEffect(() => {
    if (token) {
      navigate(next, { replace: true });
      return;
    }

    const tokenParam = searchParams.get('token');
    const name = searchParams.get('name');
    const id = searchParams.get('id');
    const avatar = searchParams.get('avatar_url');

    if (tokenParam && name && id) {
      setAuth(tokenParam, { id, username: name, name, avatar_url: avatar });
      navigate(next, { replace: true });
      return;
    }

    authApi.providers().then(setProviders).catch(() => {
      setProviders({ local: true, github: false });
    });
  }, [searchParams, setAuth, navigate, token, next]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    setLoading(true);
    try {
      const res = await authApi.login({ username, password });
      setAuth(res.token, { id: res.user.id, username: res.user.username });
      navigate(next, { replace: true });
    } catch (err: any) {
      setError(err.response?.data?.message || '登录失败，请重试');
    } finally {
      setLoading(false);
    }
  };

  const handleGithubLogin = () => {
    window.location.href = '/api/v1/auth/github';
  };

  if (!providers) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gradient-to-br from-primary-50 to-blue-100">
        <div className="text-gray-500">加载中...</div>
      </div>
    );
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-gradient-to-br from-primary-50 to-blue-100">
      <div className="bg-white rounded-2xl shadow-xl p-8 w-full max-w-md">
        <div className="text-center mb-8">
          <h1 className="text-3xl font-bold text-gray-900 mb-2">OmniAGP</h1>
          <p className="text-gray-500">AI 游戏自动生成平台</p>
        </div>

        {providers.local && (
          <form onSubmit={handleSubmit} className="space-y-4">
            <div>
              <label htmlFor="username" className="block text-sm font-medium text-gray-700 mb-1">
                用户名
              </label>
              <input
                id="username"
                type="text"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                className="w-full rounded-lg border border-gray-300 px-4 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                placeholder="请输入用户名"
                required
                autoComplete="username"
              />
            </div>
            <div>
              <label htmlFor="password" className="block text-sm font-medium text-gray-700 mb-1">
                密码
              </label>
              <input
                id="password"
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className="w-full rounded-lg border border-gray-300 px-4 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                placeholder="请输入密码"
                required
                autoComplete="current-password"
              />
            </div>

            {error && (
              <p className="text-red-600 text-sm" role="alert">{error}</p>
            )}

            <button
              type="submit"
              disabled={loading}
              className="w-full bg-blue-600 text-white rounded-lg px-4 py-3 font-medium hover:bg-blue-700 transition-colors disabled:opacity-50"
            >
              {loading ? '登录中...' : '登录'}
            </button>

            <p className="text-center text-sm text-gray-500">
              没有账号？
              <Link to="/register" className="text-blue-600 hover:underline ml-1">
                去注册
              </Link>
            </p>
          </form>
        )}

        {providers.github && (
          <>
            {providers.local && (
              <div className="my-4 flex items-center gap-3">
                <div className="flex-1 h-px bg-gray-200" />
                <span className="text-sm text-gray-400">或</span>
                <div className="flex-1 h-px bg-gray-200" />
              </div>
            )}
            <button
              onClick={handleGithubLogin}
              className="w-full flex items-center justify-center gap-3 bg-gray-900 text-white rounded-lg px-4 py-3 font-medium hover:bg-gray-800 transition-colors"
            >
              <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
              </svg>
              使用 GitHub 登录
            </button>
          </>
        )}

        <div className="mt-6 text-center text-sm text-gray-400">
          登录后即可创建和管理游戏生成项目
        </div>
      </div>
    </div>
  );
}
