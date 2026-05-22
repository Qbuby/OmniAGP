import { useState } from 'react';

interface ConfigState {
  llm_base_url: string;
  llm_model: string;
  target_engine: string;
  asset_quality: string;
  gpu_enabled: boolean;
}

const PRESETS: Record<string, Partial<ConfigState>> = {
  fast: { llm_model: 'qwen2.5-coder-3b', asset_quality: 'low', gpu_enabled: false },
  standard: { llm_model: 'qwen2.5-coder-7b', asset_quality: 'medium', gpu_enabled: true },
  quality: { llm_model: 'qwen2.5-coder-32b', asset_quality: 'high', gpu_enabled: true },
};

export default function ConfigPage() {
  const [config, setConfig] = useState<ConfigState>({
    llm_base_url: 'http://localhost:11434/v1',
    llm_model: 'qwen2.5-coder-7b',
    target_engine: 'godot4',
    asset_quality: 'medium',
    gpu_enabled: true,
  });

  const [saved, setSaved] = useState(false);

  const applyPreset = (key: string) => {
    const preset = PRESETS[key];
    if (preset) {
      setConfig((prev) => ({ ...prev, ...preset }));
    }
  };

  const handleSave = () => {
    localStorage.setItem('omniagp-config', JSON.stringify(config));
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  const qualityLabels: Record<string, string> = {
    low: '低',
    medium: '中',
    high: '高',
  };

  return (
    <div className="max-w-2xl mx-auto">
      <h2 className="text-2xl font-bold text-gray-900 mb-6">配置编辑器</h2>

      <div className="mb-6">
        <label className="block text-sm font-medium text-gray-700 mb-2">预设模板</label>
        <div className="flex gap-2">
          {Object.entries(PRESETS).map(([key]) => (
            <button
              key={key}
              onClick={() => applyPreset(key)}
              className="px-4 py-2 rounded-lg border border-gray-300 text-sm font-medium hover:bg-gray-50 transition-colors capitalize"
            >
              {key === 'fast' ? '快速' : key === 'standard' ? '标准' : '高质量'}
            </button>
          ))}
        </div>
      </div>

      <div className="bg-white rounded-xl border border-gray-200 p-6 space-y-5">
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">
            LLM API 地址
          </label>
          <input
            type="text"
            value={config.llm_base_url}
            onChange={(e) => setConfig({ ...config, llm_base_url: e.target.value })}
            className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-transparent outline-none"
          />
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">
            LLM 模型
          </label>
          <select
            value={config.llm_model}
            onChange={(e) => setConfig({ ...config, llm_model: e.target.value })}
            className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-transparent outline-none"
          >
            <option value="qwen2.5-coder-3b">Qwen 2.5 Coder 3B (快速)</option>
            <option value="qwen2.5-coder-7b">Qwen 2.5 Coder 7B (推荐)</option>
            <option value="qwen2.5-coder-32b">Qwen 2.5 Coder 32B (高质量)</option>
            <option value="deepseek-coder-v2">DeepSeek Coder V2</option>
          </select>
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">
            目标引擎
          </label>
          <select
            value={config.target_engine}
            onChange={(e) => setConfig({ ...config, target_engine: e.target.value })}
            className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-transparent outline-none"
          >
            <option value="godot4">Godot 4</option>
          </select>
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-2">
            资产质量: {qualityLabels[config.asset_quality]}
          </label>
          <input
            type="range"
            min="0"
            max="2"
            value={config.asset_quality === 'low' ? 0 : config.asset_quality === 'medium' ? 1 : 2}
            onChange={(e) => {
              const v = parseInt(e.target.value);
              setConfig({ ...config, asset_quality: v === 0 ? 'low' : v === 1 ? 'medium' : 'high' });
            }}
            className="w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer accent-primary-600"
          />
          <div className="flex justify-between text-xs text-gray-400 mt-1">
            <span>低</span>
            <span>中</span>
            <span>高</span>
          </div>
        </div>

        <div className="flex items-center gap-3">
          <label className="relative inline-flex items-center cursor-pointer">
            <input
              type="checkbox"
              checked={config.gpu_enabled}
              onChange={(e) => setConfig({ ...config, gpu_enabled: e.target.checked })}
              className="sr-only peer"
            />
            <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-primary-300 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary-600"></div>
          </label>
          <span className="text-sm font-medium text-gray-700">启用 GPU 加速</span>
        </div>
      </div>

      <div className="mt-6 flex items-center gap-3">
        <button
          onClick={handleSave}
          className="bg-primary-600 text-white px-6 py-2 rounded-lg font-medium hover:bg-primary-700 transition-colors"
        >
          保存配置
        </button>
        {saved && <span className="text-green-600 text-sm">已保存</span>}
      </div>
    </div>
  );
}
