import { useEffect, useState } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import { useAuth } from "../contexts/AuthContext";
import { authApi } from "../lib/api";

export default function OAuthCallback() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const { setAuthData } = useAuth();
  const [error, setError] = useState("");

  useEffect(() => {
    const handleCallback = async () => {
      // Check if we have token and user data (from backend redirect)
      const token = searchParams.get("token");
      const userEncoded = searchParams.get("user");

      if (token && userEncoded) {
        try {
          // Decode base64-encoded user data
          const userJson = atob(userEncoded);
          const user = JSON.parse(userJson);

          // Clear stored state
          sessionStorage.removeItem("oauth_state");

          // Set auth data
          setAuthData(token, user);

          // Redirect to dashboard
          navigate("/");
          return;
        } catch (err: unknown) {
          console.error("Failed to parse user data:", err);
          setError("Failed to complete authentication");
          setTimeout(() => navigate("/login"), 3000);
          return;
        }
      }

      // Legacy code path (if backend returns code/state - shouldn't happen anymore)
      const code = searchParams.get("code");
      const state = searchParams.get("state");
      const storedState = sessionStorage.getItem("oauth_state");

      // Check for OAuth error
      const oauthError = searchParams.get("error");
      if (oauthError) {
        setError(`OAuth error: ${searchParams.get("error_description") || oauthError}`);
        setTimeout(() => navigate("/login"), 3000);
        return;
      }

      // Validate required parameters
      if (!code || !state) {
        setError("Missing OAuth parameters");
        setTimeout(() => navigate("/login"), 3000);
        return;
      }

      // Verify CSRF state
      if (state !== storedState) {
        setError("Invalid OAuth state - possible CSRF attack");
        sessionStorage.removeItem("oauth_state");
        setTimeout(() => navigate("/login"), 3000);
        return;
      }

      try {
        // Exchange code for token
        const response = await authApi.handleOAuthCallback(code, state);

        // Clear stored state
        sessionStorage.removeItem("oauth_state");

        // Set auth data
        setAuthData(response.token, response.user);

        // Redirect to dashboard
        navigate("/");
      } catch (err: unknown) {
        console.error("OAuth callback error:", err);
        const error = err as {
          response?: { data?: { error?: { message?: string } } };
        };
        setError(
          error.response?.data?.error?.message ||
            "Failed to complete OAuth login. Please try again."
        );
        setTimeout(() => navigate("/login"), 3000);
      }
    };

    handleCallback();
  }, [searchParams, navigate, setAuthData]);

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50">
      <div className="max-w-md w-full space-y-8 text-center">
        {error ? (
          <div>
            <div className="mx-auto flex items-center justify-center h-12 w-12 rounded-full bg-red-100">
              <svg
                className="h-6 w-6 text-red-600"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                aria-hidden="true"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M6 18L18 6M6 6l12 12"
                />
              </svg>
            </div>
            <h2 className="mt-6 text-3xl font-extrabold text-gray-900">Authentication Failed</h2>
            <p className="mt-2 text-sm text-gray-600">{error}</p>
            <p className="mt-4 text-xs text-gray-500">Redirecting to login page...</p>
          </div>
        ) : (
          <div>
            <div className="mx-auto flex items-center justify-center h-12 w-12 rounded-full bg-blue-100">
              <svg
                className="animate-spin h-6 w-6 text-blue-600"
                xmlns="http://www.w3.org/2000/svg"
                fill="none"
                viewBox="0 0 24 24"
                aria-hidden="true"
              >
                <circle
                  className="opacity-25"
                  cx="12"
                  cy="12"
                  r="10"
                  stroke="currentColor"
                  strokeWidth="4"
                ></circle>
                <path
                  className="opacity-75"
                  fill="currentColor"
                  d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                ></path>
              </svg>
            </div>
            <h2 className="mt-6 text-3xl font-extrabold text-gray-900">
              Completing authentication
            </h2>
            <p className="mt-2 text-sm text-gray-600">Please wait while we sign you in...</p>
          </div>
        )}
      </div>
    </div>
  );
}
