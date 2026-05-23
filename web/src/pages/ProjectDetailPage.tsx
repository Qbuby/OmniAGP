import { useParams, useNavigate } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { projectsApi } from '../api/client';
import { useWebSocket, PipelineEvent } from '../hooks/useWebSocket';
import PipelineDAG from '../components/PipelineDAG';

export default function ProjectDetailPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const { data: project, isLoading } = useQuery({
    queryKey: ['project', id],
    queryFn: () => projectsApi.get(id!),
    enabled: !!id,
  });

  const { events, connected } = useWebSocket(id);

  const runMutation = useMutation({
    mutationFn: () => projectsApi.runPipeline(id!),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['project', id] });
    },
  });

  const downloadMutation = useMutation({
    mutationFn: () => projectsApi.downloadArtifact(id!),
    onSuccess: (response) => {
      const disposition = response.headers['content-disposition'] || '';
      const match = disposition.match(/filename="?([^";\n]+)"?/);
      const filename = match?.[1] || `${project?.name || 'artifact'}-${id!.slice(0, 8)}.zip`;
      const url = URL.createObjectURL(response.data);
      const a = document.createElement('a');
      a.href = url;
      a.download = filename;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    },
    onError: (err: any) => {
      if (err.response?.status === 409) {
        alert('管线尚未完成，无法下载');
      } else {
        alert(err.response?.data?.message || err.message || '下载失败');
      }
    },
  });

  const deleteMutation = useMutation({
    mutationFn: () => projectsApi.delete(id!),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['projects'] });
      navigate('/projects');
    },
  });

  if (isLoading) {
    return <div className="text-center py-12 text-gray-500">加载中...</div>;
  }

  if (!project) {
    return <div className="text-center py-12 text-gray-500">项目不存在</div>;
  }

  const latestEvent = events[events.length - 1];
  const isRunning = project.status.includes('Analyzing') ||
    project.status.includes('Generating') ||
    project.status.includes('Assembling');

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <h2 className="text-2xl font-bold text-gray-900">{project.name}</h2>
          <p className="text-gray-500 mt-1">{project.description}</p>
        </div>
        <div className="flex items-center gap-2">
          <span className={`text-xs px-2 py-1 rounded-full ${
            connected ? 'bg-green-100 text-green-700' : 'bg-gray-100 text-gray-500'
          }`}>
            {connected ? 'WS 已连接' : 'WS 断开'}
          </span>
        </div>
      </div>

      <div className="grid gap-6 lg:grid-cols-3">
        <div className="lg:col-span-2 space-y-6">
          <div className="bg-white rounded-xl border border-gray-200 p-6">
            <h3 className="font-semibold text-gray-900 mb-4">管线进度</h3>
            <PipelineDAG events={events} currentStatus={project.status} />
            {latestEvent && (
              <div className="mt-4">
                <div className="flex items-center justify-between text-sm mb-1">
                  <span className="text-gray-600">{latestEvent.step_name}</span>
                  <span className="text-gray-500">
                    {Math.round(latestEvent.progress * 100)}%
                  </span>
                </div>
                <div className="w-full bg-gray-200 rounded-full h-2">
                  <div
                    className="bg-primary-600 h-2 rounded-full transition-all duration-500"
                    style={{ width: `${latestEvent.progress * 100}%` }}
                  />
                </div>
              </div>
            )}
          </div>

          <div className="bg-white rounded-xl border border-gray-200 p-6">
            <h3 className="font-semibold text-gray-900 mb-4">事件日志</h3>
            <div className="space-y-2 max-h-64 overflow-y-auto">
              {events.length === 0 ? (
                <p className="text-sm text-gray-400">暂无事件，启动管线后将显示实时进度</p>
              ) : (
                events.map((ev: PipelineEvent, i: number) => (
                  <div key={i} className="flex items-center gap-2 text-sm">
                    <span className="text-gray-400 text-xs whitespace-nowrap">
                      {new Date(ev.timestamp).toLocaleTimeString('zh-CN')}
                    </span>
                    <span className={`px-1.5 py-0.5 rounded text-xs ${
                      ev.status === 'complete' ? 'bg-green-100 text-green-700' :
                      ev.status.startsWith('failed') ? 'bg-red-100 text-red-700' :
                      'bg-blue-100 text-blue-700'
                    }`}>
                      {ev.status}
                    </span>
                    <span className="text-gray-700">{ev.step_name}</span>
                  </div>
                ))
              )}
            </div>
          </div>
        </div>

        <div className="space-y-4">
          <div className="bg-white rounded-xl border border-gray-200 p-6">
            <h3 className="font-semibold text-gray-900 mb-4">操作</h3>
            <div className="space-y-3">
              <button
                onClick={() => runMutation.mutate()}
                disabled={runMutation.isPending || isRunning}
                className="w-full bg-primary-600 text-white px-4 py-2 rounded-lg text-sm font-medium hover:bg-primary-700 transition-colors disabled:opacity-50"
              >
                {isRunning ? '运行中...' : '启动管线'}
              </button>
              {project.status.includes('Complete') && (
                <button
                  onClick={() => downloadMutation.mutate()}
                  disabled={downloadMutation.isPending}
                  className="w-full bg-green-600 text-white px-4 py-2 rounded-lg text-sm font-medium hover:bg-green-700 transition-colors disabled:opacity-50"
                >
                  {downloadMutation.isPending ? '下载中...' : '下载构建产物'}
                </button>
              )}
              <button
                onClick={() => deleteMutation.mutate()}
                disabled={deleteMutation.isPending}
                className="w-full border border-red-300 text-red-600 px-4 py-2 rounded-lg text-sm font-medium hover:bg-red-50 transition-colors"
              >
                删除项目
              </button>
            </div>
          </div>

          <div className="bg-white rounded-xl border border-gray-200 p-6">
            <h3 className="font-semibold text-gray-900 mb-3">项目信息</h3>
            <dl className="space-y-2 text-sm">
              <div>
                <dt className="text-gray-500">状态</dt>
                <dd className="text-gray-900 font-medium">{project.status}</dd>
              </div>
              <div>
                <dt className="text-gray-500">创建时间</dt>
                <dd className="text-gray-900">
                  {new Date(project.created_at).toLocaleString('zh-CN')}
                </dd>
              </div>
              <div>
                <dt className="text-gray-500">ID</dt>
                <dd className="text-gray-900 font-mono text-xs break-all">{project.id}</dd>
              </div>
            </dl>
          </div>
        </div>
      </div>
    </div>
  );
}
