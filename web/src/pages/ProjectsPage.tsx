import { useQuery } from '@tanstack/react-query';
import { Link } from 'react-router-dom';
import { projectsApi, Project } from '../api/client';
import { useState } from 'react';

export default function ProjectsPage() {
  const [search, setSearch] = useState('');

  const { data, isLoading } = useQuery({
    queryKey: ['projects', search],
    queryFn: () => projectsApi.list({ search: search || undefined }),
  });

  const statusColor = (status: string) => {
    if (status.includes('Complete')) return 'bg-green-100 text-green-700';
    if (status.includes('Failed')) return 'bg-red-100 text-red-700';
    if (status.includes('Created')) return 'bg-gray-100 text-gray-700';
    return 'bg-blue-100 text-blue-700';
  };

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-2xl font-bold text-gray-900">项目列表</h2>
        <Link
          to="/projects/new"
          className="bg-primary-600 text-white px-4 py-2 rounded-lg text-sm font-medium hover:bg-primary-700 transition-colors"
        >
          创建项目
        </Link>
      </div>

      <div className="mb-4">
        <input
          type="text"
          placeholder="搜索项目..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="w-full max-w-md px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-transparent outline-none"
        />
      </div>

      {isLoading ? (
        <div className="text-center py-12 text-gray-500">加载中...</div>
      ) : !data?.projects.length ? (
        <div className="text-center py-12">
          <p className="text-gray-500 mb-4">还没有项目</p>
          <Link
            to="/projects/new"
            className="text-primary-600 hover:text-primary-700 font-medium"
          >
            创建第一个项目
          </Link>
        </div>
      ) : (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {data.projects.map((project: Project) => (
            <Link
              key={project.id}
              to={`/projects/${project.id}`}
              className="block bg-white rounded-xl border border-gray-200 p-5 hover:shadow-md transition-shadow"
            >
              <div className="flex items-start justify-between mb-2">
                <h3 className="font-semibold text-gray-900 truncate">
                  {project.name}
                </h3>
                <span
                  className={`text-xs px-2 py-0.5 rounded-full whitespace-nowrap ${statusColor(project.status)}`}
                >
                  {project.status}
                </span>
              </div>
              <p className="text-sm text-gray-500 line-clamp-2 mb-3">
                {project.description}
              </p>
              <p className="text-xs text-gray-400">
                {new Date(project.created_at).toLocaleDateString('zh-CN')}
              </p>
            </Link>
          ))}
        </div>
      )}
    </div>
  );
}
