//! Centralized provider configuration
//!
//! This module contains all provider-specific constants like API URLs and model lists
//! to ensure consistency across the application.

/// Anthropic provider configuration
pub mod anthropic {
    /// Base API URL for Anthropic
    pub const API_URL: &str = "https://api.anthropic.com/v1/messages";

    /// Display endpoint (without path)
    pub const ENDPOINT: &str = "https://api.anthropic.com";

    /// Primary models used for routing (subset of all supported)
    pub const PRIMARY_MODELS: &[&str] = &[
        "claude-sonnet-4-5-20250929",
        "claude-haiku-4-5-20251001",
        "claude-opus-4-1-20250805",
        "claude-3-haiku-20240307",
    ];

    /// All supported Anthropic models
    pub const SUPPORTED_MODELS: &[&str] = &[
        // Latest models (Claude 4.x)
        "claude-sonnet-4-5-20250929", // Latest Sonnet - Best balance
        "claude-haiku-4-5-20251001",  // Latest Haiku - Fastest
        // Claude 4.1 models
        "claude-opus-4-1-20250805", // Latest Opus - Most capable
        // Claude 4.0 models (First generation Claude 4)
        "claude-sonnet-4-20250514", // Original Claude 4 Sonnet
        "claude-opus-4-20250514",   // Original Claude 4 Opus
        // Claude 3.5 models (Active)
        "claude-3-5-haiku-20241022", // Claude 3.5 Haiku
        // Claude 3 models (Legacy - only Haiku 3 still fully supported)
        "claude-3-haiku-20240307", // Original Claude 3 Haiku
    ];
}

/// Google Gemini provider configuration
pub mod gemini {
    /// Base API URL for Gemini
    pub const API_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

    /// Display endpoint (without path)
    pub const ENDPOINT: &str = "https://generativelanguage.googleapis.com";

    /// Primary models used for routing (subset of all supported)
    pub const PRIMARY_MODELS: &[&str] = &[
        "gemini-2.5-pro",
        "gemini-2.5-flash",
        "gemini-2.5-flash-lite",
        "gemini-2.5-flash-image",
        "gemini-2.0-flash",
        "gemini-2.0-flash-lite",
    ];

    /// All supported Gemini models
    pub const SUPPORTED_MODELS: &[&str] = &[
        // Gemini 3 Family (Latest - Released November 2025)
        "gemini-3-pro-preview",       // Best multimodal understanding
        "gemini-3-pro-image-preview", // Image generation and understanding
        // Gemini 2.5 Family (Production - Released 2025)
        "gemini-2.5-pro",         // Advanced reasoning model (1M context)
        "gemini-2.5-flash",       // Best price-performance (1M context)
        "gemini-2.5-flash-lite",  // Fastest, most cost-efficient (1M context)
        "gemini-2.5-flash-image", // Image generation support (65K context)
        // Gemini 2.0 Family (Previous generation - Still Active)
        "gemini-2.0-flash",      // Reliable 2.0 model (1M context)
        "gemini-2.0-flash-lite", // Cost-optimized 2.0 (1M context)
    ];
}

/// OpenAI provider configuration
pub mod openai {
    /// Base API URL for OpenAI
    pub const API_URL: &str = "https://api.openai.com/v1/chat/completions";

    /// Display endpoint (without path)
    pub const ENDPOINT: &str = "https://api.openai.com";

    /// Primary models used for routing (subset of all supported)
    pub const PRIMARY_MODELS: &[&str] = &[
        "gpt-5",
        "gpt-5-mini",
        "gpt-5-nano",
        "gpt-5-chat",
        "gpt-4.1",
        "gpt-4-turbo",
        "gpt-4",
        "gpt-4-turbo-preview",
        "gpt-4-vision-preview",
    ];

    /// All supported OpenAI models
    pub const SUPPORTED_MODELS: &[&str] = &[
        // GPT-5 Family (Latest - Released August 7, 2025)
        "gpt-5",      // Flagship reasoning engine (400K context)
        "gpt-5-mini", // Faster, lower-cost option (400K context)
        "gpt-5-nano", // Ultra-fast for real-time apps (400K context)
        "gpt-5-chat", // Chat-optimized variant
        // GPT-4 Family (Active)
        "gpt-4.1",             // Smartest non-reasoning multimodal LLM
        "gpt-4-turbo",         // Enhanced GPT-4 with improved performance
        "gpt-4",               // Original GPT-4 (legacy support)
        "gpt-4-turbo-preview", // Latest turbo preview
        // GPT-4 Vision
        "gpt-4-vision-preview", // Multimodal vision support
    ];
}

/// Azure OpenAI provider configuration
pub mod azure {
    /// API version for Azure OpenAI
    pub const API_VERSION: &str = "2024-10-21";

    /// Display endpoint (placeholder - actual endpoint uses resource name)
    pub const ENDPOINT: &str = "https://{resource}.openai.azure.com";

    /// Primary models used for routing (subset of all supported)
    /// Note: These use azure- prefix to avoid conflicts with OpenAI models
    pub const PRIMARY_MODELS: &[&str] = &[
        "azure-gpt-4o",
        "azure-gpt-4o-mini",
        "azure-gpt-4-turbo",
        "azure-gpt-4",
        "azure-gpt-35-turbo",
    ];

    /// All supported Azure OpenAI models
    pub const SUPPORTED_MODELS: &[&str] = &[
        // GPT-4o Family (Latest)
        "azure-gpt-4o",      // Most capable multimodal model
        "azure-gpt-4o-mini", // Fast, cost-effective
        // GPT-4 Family
        "azure-gpt-4-turbo", // Enhanced GPT-4
        "azure-gpt-4",       // Original GPT-4
        // GPT-3.5 Family
        "azure-gpt-35-turbo", // Fast and cost-effective (Azure uses 35 not 3.5)
    ];

    /// Map azure model names to deployment names (without azure- prefix)
    pub fn get_deployment_name(model: &str) -> &str {
        match model {
            "azure-gpt-4o" => "gpt-4o",
            "azure-gpt-4o-mini" => "gpt-4o-mini",
            "azure-gpt-4-turbo" => "gpt-4-turbo",
            "azure-gpt-4" => "gpt-4",
            "azure-gpt-35-turbo" => "gpt-35-turbo",
            _ => model.strip_prefix("azure-").unwrap_or(model),
        }
    }
}

/// Get primary models for a provider by name
pub fn get_primary_models(provider: &str) -> &'static [&'static str] {
    match provider {
        "anthropic" => anthropic::PRIMARY_MODELS,
        "gemini" => gemini::PRIMARY_MODELS,
        "openai" => openai::PRIMARY_MODELS,
        "azure" => azure::PRIMARY_MODELS,
        _ => &[],
    }
}

/// Get all supported models for a provider by name
pub fn get_supported_models(provider: &str) -> Vec<String> {
    match provider {
        "anthropic" => anthropic::SUPPORTED_MODELS
            .iter()
            .map(|s| s.to_string())
            .collect(),
        "gemini" => gemini::SUPPORTED_MODELS
            .iter()
            .map(|s| s.to_string())
            .collect(),
        "openai" => openai::SUPPORTED_MODELS
            .iter()
            .map(|s| s.to_string())
            .collect(),
        "azure" => azure::SUPPORTED_MODELS
            .iter()
            .map(|s| s.to_string())
            .collect(),
        _ => vec![],
    }
}

/// Get endpoint for a provider by name
pub fn get_endpoint(provider: &str) -> &'static str {
    match provider {
        "anthropic" => anthropic::ENDPOINT,
        "gemini" => gemini::ENDPOINT,
        "openai" => openai::ENDPOINT,
        "azure" => azure::ENDPOINT,
        _ => "",
    }
}
