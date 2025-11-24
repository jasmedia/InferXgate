import { CheckCircleIcon, KeyIcon, PlusIcon, XCircleIcon } from "@heroicons/react/24/outline";
import type React from "react";
import { useEffect, useState } from "react";
import { type Provider, providersApi } from "../lib/api";

const Providers: React.FC = () => {
  const [providers, setProviders] = useState<Provider[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [showApiKeyModal, setShowApiKeyModal] = useState(false);
  const [showAddProviderModal, setShowAddProviderModal] = useState(false);
  const [showDeleteConfirmModal, setShowDeleteConfirmModal] = useState(false);
  const [selectedProvider, setSelectedProvider] = useState<Provider | null>(null);
  const [apiKey, setApiKey] = useState("");
  const [azureResourceName, setAzureResourceName] = useState("");
  const [deletingProviderId, setDeletingProviderId] = useState<string | null>(null);

  // Fetch providers from backend on mount
  useEffect(() => {
    const fetchProviders = async () => {
      try {
        setLoading(true);
        setError(null);
        const data = await providersApi.listProviders();
        setProviders(data);
      } catch (err) {
        console.error("Failed to fetch providers:", err);
        setError("Failed to load providers. Please try again later.");
      } finally {
        setLoading(false);
      }
    };

    fetchProviders();
  }, []);

  const handleConfigureProvider = (provider: Provider) => {
    setSelectedProvider(provider);
    setShowApiKeyModal(true);
    setApiKey("");
    setAzureResourceName("");
  };

  const handleAddProvider = (provider: Provider) => {
    setSelectedProvider(provider);
    setShowAddProviderModal(false);
    setShowApiKeyModal(true);
    setApiKey("");
    setAzureResourceName("");
  };

  const handleDeleteProvider = (provider: Provider) => {
    setSelectedProvider(provider);
    setShowDeleteConfirmModal(true);
  };

  const confirmDeleteProvider = async () => {
    if (!selectedProvider) return;

    try {
      setDeletingProviderId(selectedProvider.id);
      const result = await providersApi.deleteProvider(selectedProvider.id);

      // Update local state - mark provider as unconfigured
      setProviders(
        providers.map((p) =>
          p.id === selectedProvider.id
            ? { ...p, api_key_configured: false, status: "inactive" as const }
            : p
        )
      );

      setShowDeleteConfirmModal(false);
      setSelectedProvider(null);
      console.log(`✅ ${result.message} (${result.models_removed} models removed)`);
    } catch (err) {
      console.error("Failed to delete provider:", err);
      setError("Failed to delete provider. Please try again.");
    } finally {
      setDeletingProviderId(null);
    }
  };

  // Get unconfigured providers for the "Add Provider" modal
  const unconfiguredProviders = providers.filter((p) => !p.api_key_configured);

  // Get configured providers to display as cards
  const configuredProviders = providers.filter((p) => p.api_key_configured);

  const handleSaveApiKey = async () => {
    if (selectedProvider && apiKey) {
      // Validate Azure-specific fields
      if (selectedProvider.id === "azure" && !azureResourceName.trim()) {
        setError("Azure Resource Name is required");
        return;
      }

      try {
        // Call backend API to configure provider
        const result = await providersApi.configureProvider(
          selectedProvider.id,
          apiKey,
          selectedProvider.id === "azure" ? azureResourceName : undefined
        );

        // Update local state on success
        setProviders(
          providers.map((p) =>
            p.id === selectedProvider.id
              ? { ...p, api_key_configured: true, status: "active" as const }
              : p
          )
        );

        setShowApiKeyModal(false);
        setApiKey("");
        setAzureResourceName("");
        setSelectedProvider(null);

        // Show success message (optional - you could add toast notifications)
        console.log(`✅ ${result.message} (${result.models_configured} models)`);
      } catch (err) {
        console.error("Failed to configure provider:", err);
        setError("Failed to save API key. Please try again.");
      }
    }
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case "active":
        return <CheckCircleIcon className="w-5 h-5 text-green-500" />;
      case "error":
        return <XCircleIcon className="w-5 h-5 text-red-500" />;
      default:
        return <XCircleIcon className="w-5 h-5 text-gray-400" />;
    }
  };

  const getStatusBadge = (status: string) => {
    switch (status) {
      case "active":
        return "bg-green-100 text-green-800";
      case "error":
        return "bg-red-100 text-red-800";
      default:
        return "bg-gray-100 text-gray-800";
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-gray-500">Loading providers...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-red-500">{error}</div>
      </div>
    );
  }

  return (
    <div>
      <div className="mb-8 flex items-start justify-between">
        <div>
          <h2 className="text-2xl font-bold text-gray-900">Providers</h2>
          <p className="mt-1 text-sm text-gray-500">
            Manage and configure your LLM provider connections
          </p>
        </div>
        {unconfiguredProviders.length > 0 && (
          <button
            type="button"
            onClick={() => setShowAddProviderModal(true)}
            className="flex items-center gap-2 px-4 py-2 text-sm font-medium text-white bg-blue-500 rounded-md hover:bg-blue-600 transition-colors"
          >
            <PlusIcon className="w-4 h-4" />
            Add Provider
          </button>
        )}
      </div>

      {/* Configured Providers Grid */}
      {configuredProviders.length === 0 ? (
        <div className="text-center py-12 bg-gray-50 rounded-lg">
          <p className="text-gray-500 mb-4">No providers configured yet</p>
          <button
            type="button"
            onClick={() => setShowAddProviderModal(true)}
            className="inline-flex items-center gap-2 px-4 py-2 text-sm font-medium text-white bg-blue-500 rounded-md hover:bg-blue-600 transition-colors"
          >
            <PlusIcon className="w-4 h-4" />
            Add Your First Provider
          </button>
        </div>
      ) : (
        <div className="grid grid-cols-1 gap-6 lg:grid-cols-2 xl:grid-cols-3">
          {configuredProviders.map((provider) => (
            <div
              key={provider.id}
              className="bg-white rounded-lg shadow hover:shadow-lg transition-shadow p-6"
            >
              <div className="flex items-start justify-between mb-4">
                <div className="flex items-center">
                  {getStatusIcon(provider.status)}
                  <h3 className="ml-2 text-lg font-semibold text-gray-900">{provider.name}</h3>
                </div>
                <span
                  className={`px-2 py-1 text-xs font-medium rounded-full ${getStatusBadge(provider.status)}`}
                >
                  {provider.status}
                </span>
              </div>

              <div className="space-y-3">
                <div>
                  <p className="text-sm text-gray-500">Endpoint</p>
                  <p className="text-sm font-medium text-gray-900 truncate">{provider.endpoint}</p>
                </div>

                <div>
                  <p className="text-sm text-gray-500">Available Models</p>
                  <div className="mt-1 flex flex-wrap gap-1">
                    {provider.models.slice(0, 3).map((model) => (
                      <span
                        key={model}
                        className="px-2 py-1 text-xs bg-gray-100 text-gray-700 rounded"
                      >
                        {model}
                      </span>
                    ))}
                    {provider.models.length > 3 && (
                      <span className="px-2 py-1 text-xs bg-gray-100 text-gray-700 rounded">
                        +{provider.models.length - 3} more
                      </span>
                    )}
                  </div>
                </div>

                <div>
                  <p className="text-sm text-gray-500 mb-2">API Key</p>
                  {provider.api_key_configured ? (
                    <div className="flex items-center text-sm text-green-600">
                      <KeyIcon className="w-4 h-4 mr-1" />
                      Configured
                    </div>
                  ) : (
                    <div className="flex items-center text-sm text-gray-400">
                      <KeyIcon className="w-4 h-4 mr-1" />
                      Not configured
                    </div>
                  )}
                </div>

                <div className="flex gap-2 mt-4">
                  <button
                    type="button"
                    onClick={() => handleConfigureProvider(provider)}
                    className="flex-1 px-4 py-2 text-sm font-medium text-white bg-blue-500 rounded-md hover:bg-blue-600 transition-colors"
                  >
                    Reconfigure
                  </button>
                  <button
                    type="button"
                    onClick={() => handleDeleteProvider(provider)}
                    disabled={deletingProviderId === provider.id}
                    className="px-4 py-2 text-sm font-medium text-red-600 bg-red-50 rounded-md hover:bg-red-100 disabled:opacity-50 transition-colors"
                  >
                    Delete
                  </button>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* API Key Modal */}
      {showApiKeyModal && selectedProvider && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 w-full max-w-md">
            <h3 className="text-lg font-semibold text-gray-900 mb-4">
              Configure {selectedProvider.name}
            </h3>

            <div className="space-y-4">
              {/* Azure-specific: Resource Name field */}
              {selectedProvider.id === "azure" && (
                <div>
                  <label
                    htmlFor="azure-resource"
                    className="block text-sm font-medium text-gray-700 mb-1"
                  >
                    Resource Name
                  </label>
                  <input
                    type="text"
                    id="azure-resource"
                    value={azureResourceName}
                    onChange={(e) => setAzureResourceName(e.target.value)}
                    placeholder="e.g., my-company-openai"
                    className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                  />
                  <p className="mt-1 text-xs text-gray-500">
                    Your Azure OpenAI resource name (from Azure portal)
                  </p>
                </div>
              )}

              <div>
                <label htmlFor="api-key" className="block text-sm font-medium text-gray-700 mb-1">
                  API Key
                </label>
                <input
                  type="password"
                  id="api-key"
                  value={apiKey}
                  onChange={(e) => setApiKey(e.target.value)}
                  placeholder="Enter your API key"
                  className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
                <p className="mt-1 text-xs text-gray-500">
                  Your API key will be securely stored and encrypted
                </p>
              </div>
            </div>

            <div className="flex justify-end gap-3 mt-6">
              <button
                type="button"
                onClick={() => setShowApiKeyModal(false)}
                className="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-md hover:bg-gray-200 transition-colors"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={handleSaveApiKey}
                disabled={!apiKey || (selectedProvider.id === "azure" && !azureResourceName)}
                className="px-4 py-2 text-sm font-medium text-white bg-blue-500 rounded-md hover:bg-blue-600 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors"
              >
                Save Configuration
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Add Provider Modal */}
      {showAddProviderModal && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 w-full max-w-md max-h-[80vh] overflow-y-auto">
            <h3 className="text-lg font-semibold text-gray-900 mb-4">Add Provider</h3>
            <p className="text-sm text-gray-500 mb-4">Select a provider to configure</p>

            <div className="space-y-3">
              {unconfiguredProviders.map((provider) => (
                <button
                  type="button"
                  key={provider.id}
                  onClick={() => handleAddProvider(provider)}
                  className="w-full p-4 text-left border border-gray-200 rounded-lg hover:border-blue-500 hover:bg-blue-50 transition-colors"
                >
                  <div className="font-medium text-gray-900">{provider.name}</div>
                  <div className="text-sm text-gray-500 truncate">{provider.endpoint}</div>
                  <div className="text-xs text-gray-400 mt-1">
                    {provider.models.length} models available
                  </div>
                </button>
              ))}
            </div>

            <div className="flex justify-end mt-6">
              <button
                type="button"
                onClick={() => setShowAddProviderModal(false)}
                className="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-md hover:bg-gray-200 transition-colors"
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Delete Confirmation Modal */}
      {showDeleteConfirmModal && selectedProvider && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 w-full max-w-md">
            <h3 className="text-lg font-semibold text-gray-900 mb-4">Delete Provider</h3>
            <p className="text-sm text-gray-600 mb-4">
              Are you sure you want to delete{" "}
              <span className="font-semibold">{selectedProvider.name}</span>? This will remove the
              API key and all associated model routes. This action cannot be undone.
            </p>

            <div className="flex justify-end gap-3">
              <button
                type="button"
                onClick={() => {
                  setShowDeleteConfirmModal(false);
                  setSelectedProvider(null);
                }}
                className="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-md hover:bg-gray-200 transition-colors"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={confirmDeleteProvider}
                disabled={deletingProviderId === selectedProvider.id}
                className="px-4 py-2 text-sm font-medium text-white bg-red-500 rounded-md hover:bg-red-600 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors"
              >
                {deletingProviderId === selectedProvider.id ? "Deleting..." : "Delete"}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default Providers;
