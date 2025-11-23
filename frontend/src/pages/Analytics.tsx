import { useQuery } from "@tanstack/react-query";
import type React from "react";
import {
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  Legend,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import api from "../lib/api";

const Analytics: React.FC = () => {
  // Fetch usage statistics from the /stats endpoint
  const { data: statsData, isLoading } = useQuery({
    queryKey: ["stats"],
    queryFn: async () => {
      const response = await api.get("/stats");
      return response.data;
    },
    refetchInterval: 10000, // Refresh every 10 seconds
  });

  const usageStats = statsData?.usage_stats;
  const recentRequests = statsData?.recent_requests || [];

  // Transform model usage data for pie chart
  const modelUsageData =
    usageStats?.requests_by_model?.map((model: any, idx: number) => ({
      name: model.model,
      value: model.count,
      cost: model.total_cost,
      color: ["#3B82F6", "#8B5CF6", "#10B981", "#F59E0B", "#EF4444", "#6B7280"][
        idx % 6
      ],
    })) || [];

  // Transform provider data for bar chart
  const providerData =
    usageStats?.requests_by_provider?.map((provider: any) => ({
      provider: provider.provider,
      requests: provider.count,
      cost: provider.total_cost,
      tokens: provider.total_tokens,
    })) || [];

  // Get error requests
  const errorRequests = recentRequests.filter((req: any) => req.error !== null);

  const metrics = [
    {
      label: "Total Requests (7d)",
      value: usageStats?.total_requests?.toLocaleString() || "0",
    },
    {
      label: "Avg Response Time",
      value: usageStats
        ? `${Math.round(usageStats.average_latency_ms)}ms`
        : "N/A",
    },
    {
      label: "Cache Hit Rate",
      value: usageStats ? `${usageStats.cache_hit_rate.toFixed(1)}%` : "N/A",
    },
    {
      label: "Total Cost (7d)",
      value: usageStats ? `$${usageStats.total_cost.toFixed(2)}` : "N/A",
    },
  ];

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-gray-500">Loading analytics...</div>
      </div>
    );
  }

  return (
    <div>
      <div className="mb-8">
        <h2 className="text-2xl font-bold text-gray-900">Analytics</h2>
        <p className="mt-1 text-sm text-gray-500">
          Monitor performance and usage metrics for your InferXgate
        </p>
      </div>

      {/* Key Metrics */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
        {metrics.map((metric) => (
          <div key={metric.label} className="bg-white rounded-lg shadow p-4">
            <p className="text-sm text-gray-500">{metric.label}</p>
            <p className="text-2xl font-bold text-gray-900 mt-1">
              {metric.value}
            </p>
          </div>
        ))}
      </div>

      {/* Charts Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Model Usage Distribution */}
        <div className="bg-white rounded-lg shadow p-6">
          <h3 className="text-lg font-semibold text-gray-900 mb-4">
            Requests by Model
          </h3>
          {modelUsageData.length === 0 ? (
            <p className="text-sm text-gray-500">No data available</p>
          ) : (
            <ResponsiveContainer width="100%" height={300}>
              <PieChart>
                <Pie
                  data={modelUsageData}
                  cx="50%"
                  cy="50%"
                  labelLine={false}
                  label={({ name, value }) =>
                    `${name.substring(0, 20)}: ${value}`
                  }
                  outerRadius={80}
                  fill="#8884d8"
                  dataKey="value"
                >
                  {modelUsageData.map((entry: any, index: number) => (
                    <Cell key={`cell-${index}`} fill={entry.color} />
                  ))}
                </Pie>
                <Tooltip />
              </PieChart>
            </ResponsiveContainer>
          )}
        </div>

        {/* Provider Comparison */}
        <div className="bg-white rounded-lg shadow p-6">
          <h3 className="text-lg font-semibold text-gray-900 mb-4">
            Requests by Provider
          </h3>
          {providerData.length === 0 ? (
            <p className="text-sm text-gray-500">No data available</p>
          ) : (
            <ResponsiveContainer width="100%" height={300}>
              <BarChart data={providerData}>
                <CartesianGrid strokeDasharray="3 3" stroke="#E5E7EB" />
                <XAxis dataKey="provider" stroke="#6B7280" />
                <YAxis stroke="#6B7280" />
                <Tooltip />
                <Legend />
                <Bar dataKey="requests" fill="#3B82F6" name="Requests" />
              </BarChart>
            </ResponsiveContainer>
          )}
        </div>

        {/* Cost by Model */}
        <div className="bg-white rounded-lg shadow p-6">
          <h3 className="text-lg font-semibold text-gray-900 mb-4">
            Cost by Model
          </h3>
          {modelUsageData.length === 0 ? (
            <p className="text-sm text-gray-500">No data available</p>
          ) : (
            <ResponsiveContainer width="100%" height={300}>
              <BarChart data={modelUsageData}>
                <CartesianGrid strokeDasharray="3 3" stroke="#E5E7EB" />
                <XAxis
                  dataKey="name"
                  stroke="#6B7280"
                  angle={-45}
                  textAnchor="end"
                  height={100}
                />
                <YAxis stroke="#6B7280" />
                <Tooltip formatter={(value: any) => `$${value.toFixed(4)}`} />
                <Bar dataKey="cost" fill="#10B981" name="Cost (USD)" />
              </BarChart>
            </ResponsiveContainer>
          )}
        </div>

        {/* Tokens by Provider */}
        <div className="bg-white rounded-lg shadow p-6">
          <h3 className="text-lg font-semibold text-gray-900 mb-4">
            Token Usage by Provider
          </h3>
          {providerData.length === 0 ? (
            <p className="text-sm text-gray-500">No data available</p>
          ) : (
            <ResponsiveContainer width="100%" height={300}>
              <BarChart data={providerData}>
                <CartesianGrid strokeDasharray="3 3" stroke="#E5E7EB" />
                <XAxis dataKey="provider" stroke="#6B7280" />
                <YAxis stroke="#6B7280" />
                <Tooltip formatter={(value: any) => value.toLocaleString()} />
                <Bar dataKey="tokens" fill="#8B5CF6" name="Total Tokens" />
              </BarChart>
            </ResponsiveContainer>
          )}
        </div>
      </div>

      {/* Recent Errors Table */}
      <div className="mt-8 bg-white rounded-lg shadow p-6">
        <h3 className="text-lg font-semibold text-gray-900 mb-4">
          Recent Errors
        </h3>
        {errorRequests.length === 0 ? (
          <p className="text-sm text-gray-500">
            No errors in recent requests 🎉
          </p>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b">
                  <th className="text-left py-2 px-4 font-medium text-gray-700">
                    Time
                  </th>
                  <th className="text-left py-2 px-4 font-medium text-gray-700">
                    Model
                  </th>
                  <th className="text-left py-2 px-4 font-medium text-gray-700">
                    Provider
                  </th>
                  <th className="text-left py-2 px-4 font-medium text-gray-700">
                    Error
                  </th>
                </tr>
              </thead>
              <tbody>
                {errorRequests.slice(0, 10).map((request: any, idx: number) => (
                  <tr key={idx} className="border-b hover:bg-gray-50">
                    <td className="py-2 px-4 text-gray-600">
                      {new Date(request.created_at).toLocaleString()}
                    </td>
                    <td className="py-2 px-4">{request.model}</td>
                    <td className="py-2 px-4">{request.provider}</td>
                    <td className="py-2 px-4 text-gray-600 max-w-md truncate">
                      {request.error || "Unknown error"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      {/* Summary Statistics */}
      <div className="mt-8 grid grid-cols-1 md:grid-cols-3 gap-6">
        <div className="bg-white rounded-lg shadow p-6">
          <h3 className="text-sm font-medium text-gray-500 mb-2">
            Total Tokens Processed
          </h3>
          <p className="text-3xl font-bold text-gray-900">
            {usageStats?.total_tokens?.toLocaleString() || "0"}
          </p>
          <p className="text-xs text-gray-500 mt-2">Last 7 days</p>
        </div>

        <div className="bg-white rounded-lg shadow p-6">
          <h3 className="text-sm font-medium text-gray-500 mb-2">
            Average Cost per Request
          </h3>
          <p className="text-3xl font-bold text-gray-900">
            {usageStats && usageStats.total_requests > 0
              ? `$${(usageStats.total_cost / usageStats.total_requests).toFixed(4)}`
              : "$0"}
          </p>
          <p className="text-xs text-gray-500 mt-2">Last 7 days</p>
        </div>

        <div className="bg-white rounded-lg shadow p-6">
          <h3 className="text-sm font-medium text-gray-500 mb-2">Error Rate</h3>
          <p className="text-3xl font-bold text-gray-900">
            {recentRequests.length > 0
              ? `${((errorRequests.length / recentRequests.length) * 100).toFixed(2)}%`
              : "0%"}
          </p>
          <p className="text-xs text-gray-500 mt-2">Recent requests</p>
        </div>
      </div>
    </div>
  );
};

export default Analytics;
