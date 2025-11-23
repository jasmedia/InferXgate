import {
  BeakerIcon,
  ChartBarIcon,
  CogIcon,
  HomeIcon,
  KeyIcon,
  ServerStackIcon,
} from "@heroicons/react/24/outline";
import { useState } from "react";
import { Link, Outlet, useLocation, useNavigate } from "react-router-dom";
import { useAuth } from "../contexts/AuthContext";
import logo from "../assets/logo.png";

const Layout = () => {
  const location = useLocation();
  const navigate = useNavigate();
  const { user, logout } = useAuth();
  const [showUserMenu, setShowUserMenu] = useState(false);

  const navigation = [
    { name: "Dashboard", href: "/", icon: HomeIcon },
    { name: "Playground", href: "/playground", icon: BeakerIcon },
    { name: "Providers", href: "/providers", icon: ServerStackIcon },
    { name: "Analytics", href: "/analytics", icon: ChartBarIcon },
    { name: "API Keys", href: "/api-keys", icon: KeyIcon },
    { name: "Settings", href: "/settings", icon: CogIcon },
  ];

  const handleLogout = async () => {
    try {
      await logout();
      navigate("/login");
    } catch (error) {
      console.error("Logout failed:", error);
      // Still navigate to login even if API call fails
      navigate("/login");
    }
  };

  return (
    <div className="flex h-screen bg-gray-50">
      {/* Sidebar */}
      <div className="w-64 bg-white shadow-md flex flex-col">
        <div className="flex items-center justify-center h-16 border-b gap-2">
          <img
            src={logo}
            alt="InferXgate Logo"
            className="w-8 h-8 object-contain"
          />
          <h1 className="text-xl font-bold text-gray-800">InferXgate</h1>
        </div>
        <nav className="mt-6 flex-1">
          {navigation.map((item) => {
            const isActive = location.pathname === item.href;
            return (
              <Link
                key={item.name}
                to={item.href}
                className={`flex items-center px-6 py-3 text-sm font-medium transition-colors ${
                  isActive
                    ? "bg-blue-50 text-blue-600 border-r-2 border-blue-600"
                    : "text-gray-600 hover:bg-gray-50 hover:text-gray-900"
                }`}
              >
                <item.icon className="w-5 h-5 mr-3" />
                {item.name}
              </Link>
            );
          })}
        </nav>

        {/* User menu at bottom */}
        <div className="border-t">
          <div className="relative">
            <button
              onClick={() => setShowUserMenu(!showUserMenu)}
              className="w-full flex items-center px-6 py-4 text-sm font-medium text-gray-600 hover:bg-gray-50 transition-colors"
            >
              <div className="flex items-center justify-center w-8 h-8 rounded-full bg-blue-100 text-blue-600 mr-3">
                {user?.username?.[0]?.toUpperCase() ||
                  user?.email?.[0]?.toUpperCase() ||
                  "U"}
              </div>
              <div className="flex-1 text-left truncate">
                <div className="text-sm font-medium text-gray-900 truncate">
                  {user?.username || user?.email?.split("@")[0]}
                </div>
                <div className="text-xs text-gray-500 truncate">
                  {user?.email}
                </div>
              </div>
              <svg
                className={`w-4 h-4 transition-transform ${showUserMenu ? "rotate-180" : ""}`}
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M19 9l-7 7-7-7"
                />
              </svg>
            </button>

            {/* Dropdown menu */}
            {showUserMenu && (
              <div className="absolute bottom-full left-0 right-0 mb-2 mx-4 bg-white rounded-md shadow-lg border border-gray-200">
                <div className="py-1">
                  <div className="px-4 py-2 text-xs text-gray-500 border-b">
                    {user?.role === "admin" ? "Administrator" : "User"}
                  </div>
                  <button
                    onClick={handleLogout}
                    className="w-full text-left px-4 py-2 text-sm text-gray-700 hover:bg-gray-100 transition-colors"
                  >
                    Sign out
                  </button>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Main content */}
      <div className="flex-1 overflow-auto">
        <main className="p-6">
          <Outlet />
        </main>
      </div>
    </div>
  );
};

export default Layout;
