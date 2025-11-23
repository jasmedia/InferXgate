import type React from "react";
import { useState } from "react";
import toast from "react-hot-toast";

const Settings: React.FC = () => {
  const [settings, setSettings] = useState({
    general: {
      gatewayName: "InferXgate",
      defaultModel: "claude-sonnet-4-5-20250929",
      timeout: 30,
      retryAttempts: 3,
    },
    rateLimiting: {
      enabled: true,
      requestsPerMinute: 60,
      requestsPerHour: 1000,
      burstSize: 10,
    },
    caching: {
      enabled: true,
      ttl: 3600,
      maxSize: "100MB",
      semanticCaching: false,
    },
    logging: {
      level: "info",
      logRequests: true,
      logResponses: false,
      logErrors: true,
    },
    security: {
      requireAuth: true,
      apiKeyRotation: 90,
      ipWhitelist: false,
      ipAddresses: "",
    },
  });

  const handleSave = (section: string) => {
    toast.success(`${section} settings saved successfully`);
  };

  return (
    <div>
      <div className="mb-8">
        <h2 className="text-2xl font-bold text-gray-900">Settings</h2>
        <p className="mt-1 text-sm text-gray-500">
          Configure your InferXgate settings and preferences
        </p>
      </div>

      <div className="space-y-6">
        {/* General Settings */}
        <div className="bg-white rounded-lg shadow p-6">
          <h3 className="text-lg font-semibold text-gray-900 mb-4">
            General Settings
          </h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label
                htmlFor="gateway-name"
                className="block text-sm font-medium text-gray-700 mb-1"
              >
                Gateway Name
              </label>
              <input
                type="text"
                id="gateway-name"
                value={settings.general.gatewayName}
                onChange={(e) =>
                  setSettings({
                    ...settings,
                    general: {
                      ...settings.general,
                      gatewayName: e.target.value,
                    },
                  })
                }
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
            <div>
              <label
                htmlFor="default-model"
                className="block text-sm font-medium text-gray-700 mb-1"
              >
                Default Model
              </label>
              <select
                id="default-model"
                value={settings.general.defaultModel}
                onChange={(e) =>
                  setSettings({
                    ...settings,
                    general: {
                      ...settings.general,
                      defaultModel: e.target.value,
                    },
                  })
                }
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                <option value="claude-sonnet-4-5-20250929">
                  Claude Sonnet 4.5
                </option>
                <option value="claude-haiku-4-5-20251001">
                  Claude Haiku 4.5
                </option>
                <option value="claude-opus-4-1-20250805">
                  Claude Opus 4.1
                </option>
                <option value="gemini-2.5-pro">Gemini 2.5 Pro</option>
                <option value="gemini-2.5-flash">Gemini 2.5 Flash</option>
                <option value="gpt-5">GPT-5</option>
                <option value="gpt-4-turbo">GPT-4 Turbo</option>
              </select>
            </div>
            <div>
              <label
                htmlFor="request-timeout"
                className="block text-sm font-medium text-gray-700 mb-1"
              >
                Request Timeout (seconds)
              </label>
              <input
                type="number"
                id="request-timeout"
                value={settings.general.timeout}
                onChange={(e) =>
                  setSettings({
                    ...settings,
                    general: {
                      ...settings.general,
                      timeout: parseInt(e.target.value, 10),
                    },
                  })
                }
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
            <div>
              <label
                htmlFor="retry-attempts"
                className="block text-sm font-medium text-gray-700 mb-1"
              >
                Retry Attempts
              </label>
              <input
                type="number"
                id="retry-attempts"
                value={settings.general.retryAttempts}
                onChange={(e) =>
                  setSettings({
                    ...settings,
                    general: {
                      ...settings.general,
                      retryAttempts: parseInt(e.target.value, 10),
                    },
                  })
                }
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
          </div>
          <button
            onClick={() => handleSave("General")}
            className="mt-4 px-4 py-2 bg-blue-500 text-white rounded-md hover:bg-blue-600 transition-colors"
          >
            Save General Settings
          </button>
        </div>

        {/* Rate Limiting */}
        <div className="bg-white rounded-lg shadow p-6">
          <h3 className="text-lg font-semibold text-gray-900 mb-4">
            Rate Limiting
          </h3>
          <div className="space-y-4">
            <div className="flex items-center">
              <input
                type="checkbox"
                id="rateLimitEnabled"
                checked={settings.rateLimiting.enabled}
                onChange={(e) =>
                  setSettings({
                    ...settings,
                    rateLimiting: {
                      ...settings.rateLimiting,
                      enabled: e.target.checked,
                    },
                  })
                }
                className="mr-2"
              />
              <label
                htmlFor="rateLimitEnabled"
                className="text-sm text-gray-700"
              >
                Enable Rate Limiting
              </label>
            </div>
            {settings.rateLimiting.enabled && (
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                <div>
                  <label
                    htmlFor="requests-per-minute"
                    className="block text-sm font-medium text-gray-700 mb-1"
                  >
                    Requests per Minute
                  </label>
                  <input
                    type="number"
                    id="requests-per-minute"
                    value={settings.rateLimiting.requestsPerMinute}
                    onChange={(e) =>
                      setSettings({
                        ...settings,
                        rateLimiting: {
                          ...settings.rateLimiting,
                          requestsPerMinute: parseInt(e.target.value, 10),
                        },
                      })
                    }
                    className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                  />
                </div>
                <div>
                  <label
                    htmlFor="requests-per-hour"
                    className="block text-sm font-medium text-gray-700 mb-1"
                  >
                    Requests per Hour
                  </label>
                  <input
                    type="number"
                    id="requests-per-hour"
                    value={settings.rateLimiting.requestsPerHour}
                    onChange={(e) =>
                      setSettings({
                        ...settings,
                        rateLimiting: {
                          ...settings.rateLimiting,
                          requestsPerHour: parseInt(e.target.value, 10),
                        },
                      })
                    }
                    className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                  />
                </div>
                <div>
                  <label
                    htmlFor="burst-size"
                    className="block text-sm font-medium text-gray-700 mb-1"
                  >
                    Burst Size
                  </label>
                  <input
                    type="number"
                    id="burst-size"
                    value={settings.rateLimiting.burstSize}
                    onChange={(e) =>
                      setSettings({
                        ...settings,
                        rateLimiting: {
                          ...settings.rateLimiting,
                          burstSize: parseInt(e.target.value, 10),
                        },
                      })
                    }
                    className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                  />
                </div>
              </div>
            )}
          </div>
          <button
            onClick={() => handleSave("Rate Limiting")}
            className="mt-4 px-4 py-2 bg-blue-500 text-white rounded-md hover:bg-blue-600 transition-colors"
          >
            Save Rate Limiting Settings
          </button>
        </div>

        {/* Caching */}
        <div className="bg-white rounded-lg shadow p-6">
          <h3 className="text-lg font-semibold text-gray-900 mb-4">Caching</h3>
          <div className="space-y-4">
            <div className="flex items-center">
              <input
                type="checkbox"
                id="cachingEnabled"
                checked={settings.caching.enabled}
                onChange={(e) =>
                  setSettings({
                    ...settings,
                    caching: { ...settings.caching, enabled: e.target.checked },
                  })
                }
                className="mr-2"
              />
              <label htmlFor="cachingEnabled" className="text-sm text-gray-700">
                Enable Response Caching
              </label>
            </div>
            {settings.caching.enabled && (
              <>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                  <div>
                    <label
                      htmlFor="cache-ttl"
                      className="block text-sm font-medium text-gray-700 mb-1"
                    >
                      Cache TTL (seconds)
                    </label>
                    <input
                      type="number"
                      id="cache-ttl"
                      value={settings.caching.ttl}
                      onChange={(e) =>
                        setSettings({
                          ...settings,
                          caching: {
                            ...settings.caching,
                            ttl: parseInt(e.target.value, 10),
                          },
                        })
                      }
                      className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                    />
                  </div>
                  <div>
                    <label
                      htmlFor="max-cache-size"
                      className="block text-sm font-medium text-gray-700 mb-1"
                    >
                      Max Cache Size
                    </label>
                    <input
                      type="text"
                      id="max-cache-size"
                      value={settings.caching.maxSize}
                      onChange={(e) =>
                        setSettings({
                          ...settings,
                          caching: {
                            ...settings.caching,
                            maxSize: e.target.value,
                          },
                        })
                      }
                      className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                    />
                  </div>
                </div>
                <div className="flex items-center">
                  <input
                    type="checkbox"
                    id="semanticCaching"
                    checked={settings.caching.semanticCaching}
                    onChange={(e) =>
                      setSettings({
                        ...settings,
                        caching: {
                          ...settings.caching,
                          semanticCaching: e.target.checked,
                        },
                      })
                    }
                    className="mr-2"
                  />
                  <label
                    htmlFor="semanticCaching"
                    className="text-sm text-gray-700"
                  >
                    Enable Semantic Caching (Experimental)
                  </label>
                </div>
              </>
            )}
          </div>
          <button
            onClick={() => handleSave("Caching")}
            className="mt-4 px-4 py-2 bg-blue-500 text-white rounded-md hover:bg-blue-600 transition-colors"
          >
            Save Caching Settings
          </button>
        </div>

        {/* Logging */}
        <div className="bg-white rounded-lg shadow p-6">
          <h3 className="text-lg font-semibold text-gray-900 mb-4">Logging</h3>
          <div className="space-y-4">
            <div>
              <label
                htmlFor="log-level"
                className="block text-sm font-medium text-gray-700 mb-1"
              >
                Log Level
              </label>
              <select
                id="log-level"
                value={settings.logging.level}
                onChange={(e) =>
                  setSettings({
                    ...settings,
                    logging: { ...settings.logging, level: e.target.value },
                  })
                }
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                <option value="debug">Debug</option>
                <option value="info">Info</option>
                <option value="warn">Warning</option>
                <option value="error">Error</option>
              </select>
            </div>
            <div className="space-y-2">
              <div className="flex items-center">
                <input
                  type="checkbox"
                  id="logRequests"
                  checked={settings.logging.logRequests}
                  onChange={(e) =>
                    setSettings({
                      ...settings,
                      logging: {
                        ...settings.logging,
                        logRequests: e.target.checked,
                      },
                    })
                  }
                  className="mr-2"
                />
                <label htmlFor="logRequests" className="text-sm text-gray-700">
                  Log Requests
                </label>
              </div>
              <div className="flex items-center">
                <input
                  type="checkbox"
                  id="logResponses"
                  checked={settings.logging.logResponses}
                  onChange={(e) =>
                    setSettings({
                      ...settings,
                      logging: {
                        ...settings.logging,
                        logResponses: e.target.checked,
                      },
                    })
                  }
                  className="mr-2"
                />
                <label htmlFor="logResponses" className="text-sm text-gray-700">
                  Log Responses
                </label>
              </div>
              <div className="flex items-center">
                <input
                  type="checkbox"
                  id="logErrors"
                  checked={settings.logging.logErrors}
                  onChange={(e) =>
                    setSettings({
                      ...settings,
                      logging: {
                        ...settings.logging,
                        logErrors: e.target.checked,
                      },
                    })
                  }
                  className="mr-2"
                />
                <label htmlFor="logErrors" className="text-sm text-gray-700">
                  Log Errors
                </label>
              </div>
            </div>
          </div>
          <button
            onClick={() => handleSave("Logging")}
            className="mt-4 px-4 py-2 bg-blue-500 text-white rounded-md hover:bg-blue-600 transition-colors"
          >
            Save Logging Settings
          </button>
        </div>

        {/* Security */}
        <div className="bg-white rounded-lg shadow p-6">
          <h3 className="text-lg font-semibold text-gray-900 mb-4">Security</h3>
          <div className="space-y-4">
            <div className="flex items-center">
              <input
                type="checkbox"
                id="requireAuth"
                checked={settings.security.requireAuth}
                onChange={(e) =>
                  setSettings({
                    ...settings,
                    security: {
                      ...settings.security,
                      requireAuth: e.target.checked,
                    },
                  })
                }
                className="mr-2"
              />
              <label htmlFor="requireAuth" className="text-sm text-gray-700">
                Require Authentication
              </label>
            </div>
            <div>
              <label
                htmlFor="api-key-rotation"
                className="block text-sm font-medium text-gray-700 mb-1"
              >
                API Key Rotation (days)
              </label>
              <input
                type="number"
                id="api-key-rotation"
                value={settings.security.apiKeyRotation}
                onChange={(e) =>
                  setSettings({
                    ...settings,
                    security: {
                      ...settings.security,
                      apiKeyRotation: parseInt(e.target.value, 10),
                    },
                  })
                }
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
            <div className="space-y-2">
              <div className="flex items-center">
                <input
                  type="checkbox"
                  id="ipWhitelist"
                  checked={settings.security.ipWhitelist}
                  onChange={(e) =>
                    setSettings({
                      ...settings,
                      security: {
                        ...settings.security,
                        ipWhitelist: e.target.checked,
                      },
                    })
                  }
                  className="mr-2"
                />
                <label htmlFor="ipWhitelist" className="text-sm text-gray-700">
                  Enable IP Whitelist
                </label>
              </div>
              {settings.security.ipWhitelist && (
                <div>
                  <label
                    htmlFor="ip-addresses"
                    className="block text-sm font-medium text-gray-700 mb-1"
                  >
                    Whitelisted IP Addresses (one per line)
                  </label>
                  <textarea
                    id="ip-addresses"
                    value={settings.security.ipAddresses}
                    onChange={(e) =>
                      setSettings({
                        ...settings,
                        security: {
                          ...settings.security,
                          ipAddresses: e.target.value,
                        },
                      })
                    }
                    rows={4}
                    className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                    placeholder="192.168.1.1&#10;10.0.0.0/24"
                  />
                </div>
              )}
            </div>
          </div>
          <button
            onClick={() => handleSave("Security")}
            className="mt-4 px-4 py-2 bg-blue-500 text-white rounded-md hover:bg-blue-600 transition-colors"
          >
            Save Security Settings
          </button>
        </div>
      </div>
    </div>
  );
};

export default Settings;
