import { PaperAirplaneIcon } from "@heroicons/react/24/solid";
import { useMutation, useQuery } from "@tanstack/react-query";
import type React from "react";
import { useEffect, useState } from "react";
import toast from "react-hot-toast";
import api from "../lib/api";

interface Message {
  role: "user" | "assistant";
  content: string;
}

interface Model {
  id: string;
  owned_by: string;
}

interface ChatPayload {
  model: string;
  messages: Message[];
  temperature: number;
  max_tokens: number;
  stream: boolean;
}

const Playground: React.FC = () => {
  const [model, setModel] = useState("");
  const [message, setMessage] = useState("");
  const [temperature, setTemperature] = useState(0.7);
  const [maxTokens, setMaxTokens] = useState(1024);
  const [stream, setStream] = useState(false);
  const [conversation, setConversation] = useState<Message[]>([]);

  // Fetch available models
  const { data: models } = useQuery({
    queryKey: ["models"],
    queryFn: async () => {
      const response = await api.post("/v1/models");
      return response.data.data;
    },
  });

  // Set default model when models are loaded
  useEffect(() => {
    if (models && models.length > 0 && !model) {
      setModel(models[0].id);
    }
  }, [models, model]);

  // Chat completion mutation
  const chatMutation = useMutation({
    mutationFn: async (payload: ChatPayload) => {
      if (stream) {
        // For streaming, we'd need to implement EventSource
        const token = localStorage.getItem("jwt_token");
        const response = await fetch("/v1/chat/completions", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            ...(token && { Authorization: `Bearer ${token}` }),
          },
          body: JSON.stringify(payload),
        });

        if (!response.ok) {
          throw new Error("Request failed");
        }

        const reader = response.body?.getReader();
        const decoder = new TextDecoder();
        let assistantMessage = "";

        if (reader) {
          while (true) {
            const { done, value } = await reader.read();
            if (done) break;

            const chunk = decoder.decode(value);
            const lines = chunk.split("\n");

            for (const line of lines) {
              if (line.startsWith("data: ")) {
                const data = line.slice(6);
                if (data === "[DONE]") continue;

                try {
                  const parsed = JSON.parse(data);
                  if (parsed.choices?.[0]?.delta?.content) {
                    assistantMessage += parsed.choices[0].delta.content;
                  }
                } catch (e) {
                  console.error("Failed to parse SSE data:", e);
                }
              }
            }
          }
        }

        return {
          choices: [
            {
              message: {
                role: "assistant",
                content: assistantMessage,
              },
            },
          ],
        };
      } else {
        const response = await api.post("/v1/chat/completions", payload);
        return response.data;
      }
    },
    onSuccess: (data) => {
      const assistantMessage = data.choices[0].message;
      setConversation((prev) => [...prev, assistantMessage]);
      setMessage("");
    },
    onError: (error) => {
      toast.error("Failed to send message");
      console.error(error);
    },
  });

  const handleSend = () => {
    if (!message.trim()) return;

    const userMessage: Message = {
      role: "user",
      content: message,
    };

    setConversation((prev) => [...prev, userMessage]);

    const messages = [...conversation, userMessage];

    chatMutation.mutate({
      model,
      messages,
      temperature,
      max_tokens: maxTokens,
      stream,
    });
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const handleClear = () => {
    setConversation([]);
    setMessage("");
  };

  return (
    <div className="h-full flex flex-col">
      <div className="mb-6">
        <h2 className="text-2xl font-bold text-gray-900">Playground</h2>
        <p className="mt-1 text-sm text-gray-500">
          Test your InferXgate with different models and parameters
        </p>
      </div>

      <div className="flex gap-6 flex-1 min-h-0">
        {/* Settings Panel */}
        <div className="w-80 bg-white rounded-lg shadow p-6 h-fit">
          <h3 className="text-lg font-semibold text-gray-900 mb-4">Settings</h3>

          <div className="space-y-4">
            {/* Model Selection */}
            <div>
              <label
                htmlFor="model-select"
                className="block text-sm font-medium text-gray-700 mb-1"
              >
                Model
              </label>
              <select
                id="model-select"
                value={model}
                onChange={(e) => setModel(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                {models?.map((m: Model) => (
                  <option key={m.id} value={m.id}>
                    {m.id}
                  </option>
                ))}
              </select>
            </div>

            {/* Temperature */}
            <div>
              <label
                htmlFor="temperature"
                className="block text-sm font-medium text-gray-700 mb-1"
              >
                Temperature: {temperature}
              </label>
              <input
                type="range"
                id="temperature"
                min="0"
                max="2"
                step="0.1"
                value={temperature}
                onChange={(e) => setTemperature(parseFloat(e.target.value))}
                className="w-full"
              />
            </div>

            {/* Max Tokens */}
            <div>
              <label
                htmlFor="max-tokens"
                className="block text-sm font-medium text-gray-700 mb-1"
              >
                Max Tokens
              </label>
              <input
                type="number"
                id="max-tokens"
                value={maxTokens}
                onChange={(e) => setMaxTokens(parseInt(e.target.value, 10))}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>

            {/* Streaming */}
            <div className="flex items-center">
              <input
                type="checkbox"
                id="stream"
                checked={stream}
                onChange={(e) => setStream(e.target.checked)}
                className="mr-2"
              />
              <label htmlFor="stream" className="text-sm text-gray-700">
                Enable Streaming
              </label>
            </div>

            {/* Clear Button */}
            <button
              type="button"
              onClick={handleClear}
              className="w-full px-4 py-2 text-sm text-gray-700 bg-gray-100 rounded-md hover:bg-gray-200 transition-colors"
            >
              Clear Conversation
            </button>
          </div>
        </div>

        {/* Chat Interface */}
        <div className="flex-1 bg-white rounded-lg shadow flex flex-col">
          {/* Messages */}
          <div className="flex-1 p-6 overflow-y-auto">
            {conversation.length === 0 ? (
              <div className="text-center text-gray-500 mt-8">
                Start a conversation by sending a message
              </div>
            ) : (
              <div className="space-y-4">
                {conversation.map((msg, idx) => (
                  <div
                    key={`msg-${idx}-${msg.role}`}
                    className={`flex ${msg.role === "user" ? "justify-end" : "justify-start"}`}
                  >
                    <div
                      className={`max-w-[70%] px-4 py-2 rounded-lg ${
                        msg.role === "user"
                          ? "bg-blue-500 text-white"
                          : "bg-gray-100 text-gray-900"
                      }`}
                    >
                      <div className="text-xs font-medium mb-1 opacity-70">
                        {msg.role === "user" ? "You" : "Assistant"}
                      </div>
                      <div className="whitespace-pre-wrap">{msg.content}</div>
                    </div>
                  </div>
                ))}
                {chatMutation.isPending && (
                  <div className="flex justify-start">
                    <div className="bg-gray-100 text-gray-900 px-4 py-2 rounded-lg">
                      <div className="flex space-x-1">
                        <div className="w-2 h-2 bg-gray-400 rounded-full animate-bounce"></div>
                        <div className="w-2 h-2 bg-gray-400 rounded-full animate-bounce delay-100"></div>
                        <div className="w-2 h-2 bg-gray-400 rounded-full animate-bounce delay-200"></div>
                      </div>
                    </div>
                  </div>
                )}
              </div>
            )}
          </div>

          {/* Input */}
          <div className="border-t p-4">
            <div className="flex gap-2">
              <textarea
                value={message}
                onChange={(e) => setMessage(e.target.value)}
                onKeyPress={handleKeyPress}
                placeholder="Type your message..."
                className="flex-1 px-4 py-2 border border-gray-300 rounded-md resize-none focus:outline-none focus:ring-2 focus:ring-blue-500"
                rows={3}
                disabled={chatMutation.isPending}
              />
              <button
                type="button"
                onClick={handleSend}
                disabled={!message.trim() || chatMutation.isPending}
                className="px-4 py-2 bg-blue-500 text-white rounded-md hover:bg-blue-600 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors"
              >
                <PaperAirplaneIcon className="w-5 h-5" />
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default Playground;
