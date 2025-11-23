import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { keysApi, VirtualKey, VirtualKeyResponse } from "../lib/api";

export default function ApiKeys() {
  const queryClient = useQueryClient();
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [newKeyData, setNewKeyData] = useState<VirtualKeyResponse | null>(null);

  // Fetch user's keys
  const { data: keys, isLoading } = useQuery({
    queryKey: ["api-keys"],
    queryFn: keysApi.getUserKeys,
  });

  // Create key mutation
  const createKeyMutation = useMutation({
    mutationFn: keysApi.generateKey,
    onSuccess: (data) => {
      setNewKeyData(data);
      queryClient.invalidateQueries({ queryKey: ["api-keys"] });
    },
  });

  // Delete key mutation
  const deleteKeyMutation = useMutation({
    mutationFn: keysApi.deleteKey,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["api-keys"] });
    },
  });

  // Update key mutation
  const updateKeyMutation = useMutation({
    mutationFn: keysApi.updateKey,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["api-keys"] });
    },
  });

  const handleCreateKey = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const formData = new FormData(e.currentTarget);

    createKeyMutation.mutate({
      name: (formData.get("name") as string) || undefined,
      max_budget: formData.get("max_budget")
        ? parseFloat(formData.get("max_budget") as string)
        : undefined,
      rate_limit_rpm: formData.get("rate_limit_rpm")
        ? parseInt(formData.get("rate_limit_rpm") as string)
        : undefined,
      rate_limit_tpm: formData.get("rate_limit_tpm")
        ? parseInt(formData.get("rate_limit_tpm") as string)
        : undefined,
    });

    setShowCreateModal(false);
  };

  const handleToggleBlock = (key: VirtualKey) => {
    updateKeyMutation.mutate({
      key_id: key.id,
      blocked: !key.blocked,
    });
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    alert("Copied to clipboard!");
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600"></div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">API Keys</h1>
          <p className="mt-1 text-sm text-gray-600">
            Manage your virtual API keys for programmatic access
          </p>
        </div>
        <button
          onClick={() => setShowCreateModal(true)}
          className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          Create New Key
        </button>
      </div>

      {/* New Key Created Modal */}
      {newKeyData && (
        <div className="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50">
          <div className="relative top-20 mx-auto p-5 border w-11/12 max-w-md shadow-lg rounded-md bg-white">
            <div className="mt-3">
              <div className="mx-auto flex items-center justify-center h-12 w-12 rounded-full bg-green-100">
                <svg
                  className="h-6 w-6 text-green-600"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M5 13l4 4L19 7"
                  />
                </svg>
              </div>
              <h3 className="text-lg leading-6 font-medium text-gray-900 text-center mt-4">
                API Key Created!
              </h3>
              <div className="mt-4 text-sm text-gray-500">
                <p className="mb-2">
                  <strong>Important:</strong> Copy this key now. You won't be
                  able to see it again!
                </p>
                <div className="bg-gray-100 p-3 rounded-md break-all font-mono text-xs">
                  {newKeyData.key}
                </div>
                <button
                  onClick={() => copyToClipboard(newKeyData.key)}
                  className="mt-3 w-full px-4 py-2 bg-gray-200 text-gray-700 rounded-md hover:bg-gray-300"
                >
                  Copy to Clipboard
                </button>
              </div>
              <div className="mt-6">
                <button
                  onClick={() => setNewKeyData(null)}
                  className="w-full px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700"
                >
                  Close
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Create Key Modal */}
      {showCreateModal && (
        <div className="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50">
          <div className="relative top-20 mx-auto p-5 border w-11/12 max-w-md shadow-lg rounded-md bg-white">
            <div className="mt-3">
              <h3 className="text-lg leading-6 font-medium text-gray-900">
                Create New API Key
              </h3>
              <form onSubmit={handleCreateKey} className="mt-4 space-y-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700">
                    Name (optional)
                  </label>
                  <input
                    type="text"
                    name="name"
                    className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                    placeholder="Production Key"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700">
                    Max Budget (USD, optional)
                  </label>
                  <input
                    type="number"
                    name="max_budget"
                    step="0.01"
                    className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                    placeholder="100.00"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700">
                    Rate Limit - Requests/min (optional)
                  </label>
                  <input
                    type="number"
                    name="rate_limit_rpm"
                    className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                    placeholder="60"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700">
                    Rate Limit - Tokens/min (optional)
                  </label>
                  <input
                    type="number"
                    name="rate_limit_tpm"
                    className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                    placeholder="100000"
                  />
                </div>
                <div className="flex space-x-3">
                  <button
                    type="submit"
                    disabled={createKeyMutation.isPending}
                    className="flex-1 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
                  >
                    {createKeyMutation.isPending ? "Creating..." : "Create Key"}
                  </button>
                  <button
                    type="button"
                    onClick={() => setShowCreateModal(false)}
                    className="flex-1 px-4 py-2 bg-gray-200 text-gray-700 rounded-md hover:bg-gray-300"
                  >
                    Cancel
                  </button>
                </div>
              </form>
            </div>
          </div>
        </div>
      )}

      {/* Keys List */}
      <div className="bg-white shadow overflow-hidden sm:rounded-md">
        {!keys || keys.length === 0 ? (
          <div className="text-center py-12">
            <svg
              className="mx-auto h-12 w-12 text-gray-400"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z"
              />
            </svg>
            <h3 className="mt-2 text-sm font-medium text-gray-900">
              No API keys
            </h3>
            <p className="mt-1 text-sm text-gray-500">
              Get started by creating a new API key.
            </p>
          </div>
        ) : (
          <ul className="divide-y divide-gray-200">
            {keys.map((key) => (
              <li key={key.id} className="px-6 py-4">
                <div className="flex items-center justify-between">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center space-x-3">
                      <p className="text-sm font-medium text-gray-900 truncate">
                        {key.name || "Unnamed Key"}
                      </p>
                      {key.blocked && (
                        <span className="px-2 inline-flex text-xs leading-5 font-semibold rounded-full bg-red-100 text-red-800">
                          Blocked
                        </span>
                      )}
                    </div>
                    <p className="mt-1 text-sm text-gray-500 font-mono">
                      {key.key_prefix}...
                    </p>
                    <div className="mt-2 flex flex-wrap items-center text-sm text-gray-500 gap-x-4 gap-y-1">
                      {key.max_budget && (
                        <span>
                          💰 Budget: ${key.current_spend.toFixed(2)} / $
                          {key.max_budget.toFixed(2)}
                        </span>
                      )}
                      {key.rate_limit_rpm && (
                        <span>🔄 {key.rate_limit_rpm} req/min</span>
                      )}
                      {key.rate_limit_tpm && (
                        <span>
                          🎫 {key.rate_limit_tpm.toLocaleString()} tokens/min
                        </span>
                      )}
                      <span>
                        📅 Created:{" "}
                        {new Date(key.created_at).toLocaleDateString()}
                      </span>
                    </div>
                  </div>
                  <div className="flex space-x-2">
                    <button
                      onClick={() => handleToggleBlock(key)}
                      disabled={updateKeyMutation.isPending}
                      className={`px-3 py-1 text-sm rounded-md ${
                        key.blocked
                          ? "bg-green-100 text-green-700 hover:bg-green-200"
                          : "bg-yellow-100 text-yellow-700 hover:bg-yellow-200"
                      } disabled:opacity-50`}
                    >
                      {key.blocked ? "Unblock" : "Block"}
                    </button>
                    <button
                      onClick={() => {
                        if (
                          confirm(
                            "Are you sure you want to delete this key? This action cannot be undone.",
                          )
                        ) {
                          deleteKeyMutation.mutate(key.id);
                        }
                      }}
                      disabled={deleteKeyMutation.isPending}
                      className="px-3 py-1 text-sm bg-red-100 text-red-700 rounded-md hover:bg-red-200 disabled:opacity-50"
                    >
                      Delete
                    </button>
                  </div>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
