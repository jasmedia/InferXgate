import {
  ChartBarIcon,
  CheckCircleIcon,
  ClockIcon,
  CurrencyDollarIcon,
  ServerIcon,
} from "@heroicons/react/24/outline";
import { useQuery } from "@tanstack/react-query";
import type React from "react";
import api from "../lib/api";

const Dashboard: React.FC = () => {
  // Fetch models
  const { data: models, isLoading: modelsLoading } = useQuery({
    queryKey: ["models"],
    queryFn: async () => {
      const response = await api.post("/v1/models");
      return response.data.data;
    },
  });

  // Fetch health status
  const { data: health } = useQuery({
    queryKey: ["health"],
    queryFn: async () => {
      const response = await api.post("/health");
      return response.data;
    },
    refetchInterval: 5000, // Refresh every 5 seconds
  });

  // Fetch usage statistics from the new /stats endpoint
  const { data: statsData } = useQuery({
    queryKey: ["stats"],
    queryFn: async () => {
      const response = await api.get("/stats");
      return response.data;
    },
    refetchInterval: 10000, // Refresh every 10 seconds
  });

  const usageStats = statsData?.usage_stats;
  const recentRequests = statsData?.recent_requests || [];

  const stats = [
    {
      name: "Total Models",
      value: models?.length || 0,
      icon: ServerIcon,
      color: "bg-blue-500",
    },
    {
      name: "Requests (7d)",
      value: usageStats?.total_requests?.toLocaleString() || "0",
      icon: ChartBarIcon,
      color: "bg-purple-500",
    },
    {
      name: "Avg Latency",
      value: usageStats
        ? `${Math.round(usageStats.average_latency_ms)}ms`
        : "N/A",
      icon: ClockIcon,
      color: "bg-yellow-500",
    },
    {
      name: "Cache Hit Rate",
      value: usageStats ? `${usageStats.cache_hit_rate.toFixed(1)}%` : "N/A",
      icon: CheckCircleIcon,
      color: "bg-green-500",
    },
    {
      name: "Total Cost (7d)",
      value: usageStats ? `$${usageStats.total_cost.toFixed(2)}` : "N/A",
      icon: CurrencyDollarIcon,
      color: "bg-red-500",
    },
  ];

  return (
    <div>
      <div className="mb-8">
        <h2 className="text-2xl font-bold text-gray-900">Dashboard</h2>
        <p className="mt-1 text-sm text-gray-500">
          Overview of your LLM Gateway performance and status
        </p>
      </div>

      {/* Stats Grid */}
      <div className="grid grid-cols-1 gap-6 mb-8 md:grid-cols-2 lg:grid-cols-5">
        {stats.map((stat) => (
          <div
            key={stat.name}
            className="bg-white rounded-lg shadow p-6 hover:shadow-lg transition-shadow"
          >
            <div className="flex items-center">
              <div className={`p-3 rounded-lg ${stat.color} bg-opacity-10`}>
                <stat.icon
                  className={`w-6 h-6 text-white ${stat.color.replace("bg-", "text-")}`}
                />
              </div>
              <div className="ml-4">
                <p className="text-sm font-medium text-gray-500">{stat.name}</p>
                <p className="text-2xl font-semibold text-gray-900">
                  {stat.value}
                </p>
              </div>
            </div>
          </div>
        ))}
      </div>

      {/* Status Section */}
      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
        {/* System Status */}
        <div className="bg-white rounded-lg shadow p-6">
          <h3 className="text-lg font-semibold text-gray-900 mb-4">
            System Status
          </h3>
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-sm text-gray-500">Gateway Status</span>
              <span className="flex items-center text-sm text-green-600">
                <div className="w-2 h-2 bg-green-600 rounded-full mr-2 animate-pulse"></div>
                {health?.status || "Checking..."}
              </span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-sm text-gray-500">Cache Enabled</span>
              <span className="text-sm text-gray-900">
                {statsData?.cache_enabled ? "✓ Yes" : "✗ No"}
              </span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-sm text-gray-500">Database Enabled</span>
              <span className="text-sm text-gray-900">
                {statsData?.database_enabled ? "✓ Yes" : "✗ No"}
              </span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-sm text-gray-500">Total Tokens (7d)</span>
              <span className="text-sm text-gray-900">
                {usageStats?.total_tokens?.toLocaleString() || "0"}
              </span>
            </div>
          </div>
        </div>

        {/* Available Models */}
        <div className="bg-white rounded-lg shadow p-6">
          <h3 className="text-lg font-semibold text-gray-900 mb-4">
            Available Models
          </h3>
          {modelsLoading ? (
            <div className="text-sm text-gray-500">Loading models...</div>
          ) : (
            <div className="space-y-2 max-h-40 overflow-y-auto">
              {models?.map((model: any) => (
                <div
                  key={model.id}
                  className="flex items-center justify-between py-2 px-3 hover:bg-gray-50 rounded"
                >
                  <span className="text-sm text-gray-900">{model.id}</span>
                  <span className="text-xs text-gray-500 bg-gray-100 px-2 py-1 rounded">
                    {model.owned_by}
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Recent Activity */}
      <div className="mt-8 bg-white rounded-lg shadow p-6">
        <h3 className="text-lg font-semibold text-gray-900 mb-4">
          Recent Activity
        </h3>
        {recentRequests.length === 0 ? (
          <p className="text-sm text-gray-500">
            No recent requests. Make some API calls to see them here!
          </p>
        ) : (
          <div className="space-y-3">
            {recentRequests.slice(0, 10).map((request: any, idx: number) => {
              const timeAgo = request.created_at
                ? new Date(request.created_at).toLocaleString()
                : "Unknown time";
              const isError = request.error !== null;
              const isCached = request.cached === true;
              const statusColor = isError
                ? "bg-red-500"
                : isCached
                  ? "bg-blue-500"
                  : "bg-green-500";
              const statusText = isError
                ? "Error"
                : isCached
                  ? "Cached"
                  : "Success";
              const statusTextColor = isError
                ? "text-red-600"
                : isCached
                  ? "text-blue-600"
                  : "text-green-600";

              return (
                <div
                  key={idx}
                  className="flex items-center justify-between py-2 border-b"
                >
                  <div className="flex items-center flex-1">
                    <div
                      className={`w-2 h-2 ${statusColor} rounded-full mr-3`}
                    ></div>
                    <div className="flex-1">
                      <p className="text-sm text-gray-900">{request.model}</p>
                      <p className="text-xs text-gray-500">
                        {timeAgo} • {request.total_tokens} tokens • $
                        {request.cost_usd?.toFixed(4)}
                      </p>
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className={`text-xs ${statusTextColor}`}>
                      {statusText}
                    </span>
                    <span className="text-xs text-gray-400">
                      {request.latency_ms}ms
                    </span>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
};

export default Dashboard;
