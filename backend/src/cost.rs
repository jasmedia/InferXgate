use std::collections::HashMap;
use tracing::warn;

use crate::metrics::MetricsCollector;

/// Cost calculator for different LLM models.
/// Prices are per 1M tokens; see each entry's Source comment for verification dates.
pub struct CostCalculator {
    pricing: HashMap<String, ModelPricing>,
}

#[derive(Debug, Clone)]
pub struct ModelPricing {
    pub input_price_per_million: f64,
    pub output_price_per_million: f64,
}

impl CostCalculator {
    pub fn new() -> Self {
        let mut pricing = HashMap::new();

        // Anthropic Claude pricing (per 1M tokens).
        // The two entries below (claude-opus-4-7, claude-sonnet-4-6) verified
        // 2026-04-19 against https://www.anthropic.com/pricing; earlier entries
        // dated at their original insertion.
        pricing.insert(
            "claude-opus-4-7".to_string(),
            ModelPricing {
                input_price_per_million: 15.0,
                output_price_per_million: 75.0,
            },
        );
        pricing.insert(
            "claude-sonnet-4-6".to_string(),
            ModelPricing {
                input_price_per_million: 3.0,
                output_price_per_million: 15.0,
            },
        );
        pricing.insert(
            "claude-opus-4-5-20251101".to_string(),
            ModelPricing {
                input_price_per_million: 15.0,
                output_price_per_million: 75.0,
            },
        );
        pricing.insert(
            "claude-sonnet-4-5-20250929".to_string(),
            ModelPricing {
                input_price_per_million: 3.0,
                output_price_per_million: 15.0,
            },
        );
        pricing.insert(
            "claude-haiku-4-5-20251001".to_string(),
            ModelPricing {
                input_price_per_million: 0.8,
                output_price_per_million: 4.0,
            },
        );
        pricing.insert(
            "claude-opus-4-1-20250805".to_string(),
            ModelPricing {
                input_price_per_million: 15.0,
                output_price_per_million: 75.0,
            },
        );
        pricing.insert(
            "claude-3-opus-20240229".to_string(),
            ModelPricing {
                input_price_per_million: 15.0,
                output_price_per_million: 75.0,
            },
        );
        pricing.insert(
            "claude-3-sonnet-20240229".to_string(),
            ModelPricing {
                input_price_per_million: 3.0,
                output_price_per_million: 15.0,
            },
        );
        pricing.insert(
            "claude-3-haiku-20240307".to_string(),
            ModelPricing {
                input_price_per_million: 0.25,
                output_price_per_million: 1.25,
            },
        );
        pricing.insert(
            "claude-3-5-sonnet-20241022".to_string(),
            ModelPricing {
                input_price_per_million: 3.0,
                output_price_per_million: 15.0,
            },
        );

        // Google Gemini pricing (per 1M tokens)
        pricing.insert(
            "gemini-3-pro-preview".to_string(),
            ModelPricing {
                input_price_per_million: 1.25,
                output_price_per_million: 5.0,
            },
        );
        pricing.insert(
            "gemini-2.5-pro".to_string(),
            ModelPricing {
                input_price_per_million: 1.25,
                output_price_per_million: 5.0,
            },
        );
        pricing.insert(
            "gemini-2.5-flash".to_string(),
            ModelPricing {
                input_price_per_million: 0.075,
                output_price_per_million: 0.3,
            },
        );
        pricing.insert(
            "gemini-2.5-flash-lite".to_string(),
            ModelPricing {
                input_price_per_million: 0.075,
                output_price_per_million: 0.3,
            },
        );
        pricing.insert(
            "gemini-2.0-flash".to_string(),
            ModelPricing {
                input_price_per_million: 0.1,
                output_price_per_million: 0.4,
            },
        );

        // OpenAI pricing (per 1M tokens)
        pricing.insert(
            "gpt-4".to_string(),
            ModelPricing {
                input_price_per_million: 30.0,
                output_price_per_million: 60.0,
            },
        );
        pricing.insert(
            "gpt-4-turbo".to_string(),
            ModelPricing {
                input_price_per_million: 10.0,
                output_price_per_million: 30.0,
            },
        );
        pricing.insert(
            "gpt-3.5-turbo".to_string(),
            ModelPricing {
                input_price_per_million: 0.5,
                output_price_per_million: 1.5,
            },
        );

        Self { pricing }
    }

    pub fn calculate_cost(&self, model: &str, prompt_tokens: i32, completion_tokens: i32) -> f64 {
        let pricing = match self.pricing.get(model) {
            Some(p) => p,
            None => {
                MetricsCollector::record_cost_unknown_model();
                warn!(
                    model = %model,
                    prompt_tokens,
                    completion_tokens,
                    "No pricing entry for model; falling back to default. Billing will be inaccurate. If this model should be supported, add it to CostCalculator::new(); otherwise verify the caller is sending a valid model ID."
                );
                return self.calculate_default_cost(prompt_tokens, completion_tokens);
            }
        };

        let input_cost = (prompt_tokens as f64 / 1_000_000.0) * pricing.input_price_per_million;
        let output_cost =
            (completion_tokens as f64 / 1_000_000.0) * pricing.output_price_per_million;

        input_cost + output_cost
    }

    fn calculate_default_cost(&self, prompt_tokens: i32, completion_tokens: i32) -> f64 {
        // Default pricing based on average model costs
        let input_cost = (prompt_tokens as f64 / 1_000_000.0) * 2.0;
        let output_cost = (completion_tokens as f64 / 1_000_000.0) * 6.0;
        input_cost + output_cost
    }

    pub fn get_model_pricing(&self, model: &str) -> Option<&ModelPricing> {
        self.pricing.get(model)
    }

    pub fn suggest_cheaper_alternative(&self, model: &str) -> Option<String> {
        let current_pricing = self.pricing.get(model)?;
        let current_total_price =
            current_pricing.input_price_per_million + current_pricing.output_price_per_million;

        // Find cheaper alternatives (at least 30% cheaper)
        let mut alternatives: Vec<(String, f64)> = self
            .pricing
            .iter()
            .filter_map(|(m, p)| {
                let total_price = p.input_price_per_million + p.output_price_per_million;
                if total_price < current_total_price * 0.7 && m != model {
                    Some((m.clone(), total_price))
                } else {
                    None
                }
            })
            .collect();

        // Sort by price (ascending)
        alternatives.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        alternatives.first().map(|(model, _)| model.clone())
    }

    pub fn estimate_cost_for_context(
        &self,
        model: &str,
        context_length: i32,
        expected_output_tokens: i32,
    ) -> f64 {
        self.calculate_cost(model, context_length, expected_output_tokens)
    }
}

impl Default for CostCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::COST_UNKNOWN_MODEL;
    use crate::provider_config;

    #[test]
    fn test_cost_calculation() {
        let calculator = CostCalculator::new();

        // Test Claude 3.5 Sonnet: $3/1M input, $15/1M output
        let cost = calculator.calculate_cost("claude-3-5-sonnet-20241022", 1000, 500);
        let expected_cost = (1000.0 / 1_000_000.0) * 3.0 + (500.0 / 1_000_000.0) * 15.0;
        assert!((cost - expected_cost).abs() < 0.00001);

        // Test Gemini 2.5 Flash: $0.075/1M input, $0.3/1M output
        let cost = calculator.calculate_cost("gemini-2.5-flash", 10000, 5000);
        let expected_cost = (10000.0 / 1_000_000.0) * 0.075 + (5000.0 / 1_000_000.0) * 0.3;
        assert!((cost - expected_cost).abs() < 0.00001);
    }

    #[test]
    fn test_new_anthropic_model_pricing() {
        let calculator = CostCalculator::new();

        let opus_47 = calculator
            .get_model_pricing("claude-opus-4-7")
            .expect("claude-opus-4-7 must have pricing");
        assert!((opus_47.input_price_per_million - 15.0).abs() < 1e-9);
        assert!((opus_47.output_price_per_million - 75.0).abs() < 1e-9);

        let sonnet_46 = calculator
            .get_model_pricing("claude-sonnet-4-6")
            .expect("claude-sonnet-4-6 must have pricing");
        assert!((sonnet_46.input_price_per_million - 3.0).abs() < 1e-9);
        assert!((sonnet_46.output_price_per_million - 15.0).abs() < 1e-9);

        // Exercises the full calculate_cost path so a future refactor that
        // normalizes the lookup key is caught here, not in production billing.
        let opus_cost = calculator.calculate_cost("claude-opus-4-7", 1_000_000, 1_000_000);
        assert!((opus_cost - 90.0).abs() < 1e-9);
        let sonnet_cost = calculator.calculate_cost("claude-sonnet-4-6", 1_000_000, 1_000_000);
        assert!((sonnet_cost - 18.0).abs() < 1e-9);

        // Asymmetric input/output to catch a swapped-column regression that
        // symmetric 1M/1M totals would silently tolerate (15+75 == 75+15).
        let opus_input_only = calculator.calculate_cost("claude-opus-4-7", 1_000_000, 0);
        assert!((opus_input_only - 15.0).abs() < 1e-9);
        let opus_output_only = calculator.calculate_cost("claude-opus-4-7", 0, 1_000_000);
        assert!((opus_output_only - 75.0).abs() < 1e-9);
    }

    #[test]
    fn test_unknown_model_falls_back_to_default_pricing() {
        // Pins the $2/$6 default math so refactors can't silently drift cost
        // reports for unknown models, and asserts the COST_UNKNOWN_MODEL counter
        // fires so ops alerting stays load-bearing across refactors.
        let calculator = CostCalculator::new();
        let before = COST_UNKNOWN_MODEL.get();
        let cost = calculator.calculate_cost("definitely-not-a-real-model", 1_000_000, 1_000_000);
        let after = COST_UNKNOWN_MODEL.get();
        assert!((cost - 8.0).abs() < 1e-9);
        assert!(
            after >= before + 1.0,
            "COST_UNKNOWN_MODEL counter did not increment on unknown model"
        );
    }

    #[test]
    fn test_supported_models_have_pricing() {
        // Providers with pre-existing pricing gaps are excluded until the gap
        // is closed. Each entry is double-checked: the inverse assertion below
        // forces removal once pricing is added, so the allowlist cannot silently
        // mask a *newly fixed* model.
        const KNOWN_UNPRICED: &[(&str, &str)] = &[
            ("anthropic", "claude-sonnet-4-20250514"),
            ("anthropic", "claude-opus-4-20250514"),
            ("anthropic", "claude-3-5-haiku-20241022"),
            ("gemini", "gemini-3-pro-image-preview"),
            ("gemini", "gemini-2.5-flash-image"),
            ("gemini", "gemini-2.0-flash-lite"),
            ("openai", "gpt-5"),
            ("openai", "gpt-5-mini"),
            ("openai", "gpt-5-nano"),
            ("openai", "gpt-5-chat"),
            ("openai", "gpt-4.1"),
            ("openai", "gpt-4-turbo-preview"),
            ("openai", "gpt-4-vision-preview"),
            ("azure", "azure-gpt-4o"),
            ("azure", "azure-gpt-4o-mini"),
            ("azure", "azure-gpt-4-turbo"),
            ("azure", "azure-gpt-4"),
            ("azure", "azure-gpt-35-turbo"),
        ];

        let calculator = CostCalculator::new();
        for provider in ["anthropic", "gemini", "openai", "azure"] {
            for model in provider_config::get_supported_models(provider) {
                if KNOWN_UNPRICED.contains(&(provider, model.as_str())) {
                    continue;
                }
                assert!(
                    calculator.get_model_pricing(&model).is_some(),
                    "{provider} supported model {model} has no pricing entry in CostCalculator (remove from KNOWN_UNPRICED once pricing is added)"
                );
            }
        }

        // Inverse check: once pricing is added for an entry, the corresponding
        // KNOWN_UNPRICED row must be removed — otherwise the exclusion list
        // silently keeps masking a gap that's already been closed.
        for (provider, model) in KNOWN_UNPRICED {
            assert!(
                calculator.get_model_pricing(model).is_none(),
                "{provider} model {model} now has pricing — remove it from KNOWN_UNPRICED"
            );
        }
    }

    #[test]
    fn test_cheaper_alternative() {
        let calculator = CostCalculator::new();

        // Claude Opus is expensive, should suggest cheaper alternatives
        let alternative = calculator.suggest_cheaper_alternative("claude-3-opus-20240229");
        assert!(alternative.is_some());
    }
}
