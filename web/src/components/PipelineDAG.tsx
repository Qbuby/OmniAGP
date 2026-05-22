import { PipelineEvent } from '../hooks/useWebSocket';

interface Props {
  events: PipelineEvent[];
  currentStatus: string;
}

const PIPELINE_STEPS = [
  { key: 'Game Design Analysis', label: '设计分析' },
  { key: 'Code Generation', label: '代码生成' },
  { key: 'Asset Generation', label: '资产生成' },
  { key: 'Scene Assembly', label: '场景组装' },
  { key: 'complete', label: '完成' },
];

export default function PipelineDAG({ events }: Props) {
  const stepStatuses = new Map<string, string>();
  for (const ev of events) {
    stepStatuses.set(ev.step_name, ev.status);
  }

  const getStepState = (key: string) => {
    const status = stepStatuses.get(key);
    if (!status) return 'pending';
    if (status === 'running') return 'running';
    if (status === 'complete') return 'complete';
    if (status.startsWith('failed')) return 'failed';
    return 'pending';
  };

  const stateStyles = {
    pending: 'bg-gray-100 border-gray-300 text-gray-500',
    running: 'bg-blue-50 border-blue-400 text-blue-700 animate-pulse',
    complete: 'bg-green-50 border-green-400 text-green-700',
    failed: 'bg-red-50 border-red-400 text-red-700',
  };

  const connectorColor = (fromKey: string) => {
    const state = getStepState(fromKey);
    if (state === 'complete') return 'bg-green-400';
    if (state === 'running') return 'bg-blue-400';
    return 'bg-gray-300';
  };

  return (
    <div className="flex items-center gap-0 overflow-x-auto py-2">
      {PIPELINE_STEPS.map((step, i) => (
        <div key={step.key} className="flex items-center">
          <div
            className={`flex flex-col items-center justify-center w-24 h-20 rounded-lg border-2 text-center px-2 ${
              stateStyles[getStepState(step.key)]
            }`}
          >
            <div className="text-lg mb-1">
              {getStepState(step.key) === 'complete' && '✓'}
              {getStepState(step.key) === 'running' && '⟳'}
              {getStepState(step.key) === 'failed' && '✗'}
              {getStepState(step.key) === 'pending' && '○'}
            </div>
            <span className="text-xs font-medium leading-tight">{step.label}</span>
          </div>
          {i < PIPELINE_STEPS.length - 1 && (
            <div className={`w-8 h-0.5 ${connectorColor(step.key)}`} />
          )}
        </div>
      ))}
    </div>
  );
}
