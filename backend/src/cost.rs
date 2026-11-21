use std::collections::HashMap;
use tracing::warn;

use crate::metrics::MetricsCollector;

/// Cost calculator for different LLM models.
///
/// Prices are USD per 1M tokens. Every model `llm-prices` covers is priced from that crate's
/// dated, source-cited table (each row carries the provider pricing URL and the date it was
/// checked), so a price change is a `cargo update` instead of a hand edit here. The crate also
/// resolves dated snapshots, `-latest` ids and provider prefixes (`anthropic/`, `models/`,
/// Bedrock and Vertex ids), so those no longer fall through to the flat default.
///
/// `LEGACY_PRICING` only holds ids the crate does not carry (older models still accepted by
/// the gateway).
pub struct CostCalculator {
    pricing: HashMap<String, ModelPricing>,
}

#[derive(Debug, Clone)]
pub struct ModelPricing {
    pub input_price_per_million: f64,
    pub output_price_per_million: f64,
}

/// Models `llm-prices` 1.8 has no row for: (model, input, output), USD per 1M tokens.
const LEGACY_PRICING: &[(&str, f64, f64)] = &[
    ("claude-3-opus-20240229", 15.0, 75.0),
    ("claude-3-sonnet-20240229", 3.0, 15.0),
    ("claude-3-haiku-20240307", 0.25, 1.25),
    ("claude-3-5-sonnet-20241022", 3.0, 15.0),
    // Preview pricing for prompts up to 200K tokens.
    ("gemini-3-pro-preview", 2.0, 12.0),
    ("gemini-2.0-flash", 0.1, 0.4),
    ("gpt-4", 30.0, 60.0),
];

/// Looks `model` up in the `llm-prices` table. Rows without both an input and an output price
/// (embedding models) are skipped.
fn lookup_listed(model: &str) -> Option<ModelPricing> {
    let price = llm_prices::lookup(model)?;
    Some(ModelPricing {
        input_price_per_million: price.input?,
        output_price_per_million: price.output?,
    })
}

impl CostCalculator {
    pub fn new() -> Self {
        let mut pricing = HashMap::new();
        for &(model, input, output) in LEGACY_PRICING {
            pricing.insert(
                model.to_string(),
                ModelPricing {
                    input_price_per_million: input,
                    output_price_per_million: output,
                },
            );
        }

        // Pre-resolve every model the gateway routes so suggest_cheaper_alternative() can
        // compare across them. Anything else is still resolved per request in lookup().
        let calculator = Self { pricing };
        let mut resolved = HashMap::new();
        for provider in ["anthropic", "gemini", "openai", "azure"] {
            for model in crate::provider_config::get_supported_models(provider) {
                if let Some(p) = calculator.lookup(&model) {
                    resolved.insert(model, p);
                }
            }
        }
        let mut pricing = calculator.pricing;
        pricing.extend(resolved);
        Self { pricing }
    }

    /// Resolves a model id: the table built in `new()`, then `llm-prices` (dated snapshots,
    /// aliases and provider prefixes), then Azure deployment names (`azure-gpt-4o`,
    /// `azure-gpt-35-turbo`) priced as the OpenAI model they deploy.
    fn lookup(&self, model: &str) -> Option<ModelPricing> {
        if let Some(p) = self.pricing.get(model) {
            return Some(p.clone());
        }
        if let Some(p) = lookup_listed(model) {
            return Some(p);
        }
        let deployed = model.strip_prefix("azure-")?.replace("gpt-35", "gpt-3.5");
        self.pricing
            .get(&deployed)
            .cloned()
            .or_else(|| lookup_listed(&deployed))
    }

    pub fn calculate_cost(&self, model: &str, prompt_tokens: i32, completion_tokens: i32) -> f64 {
        let pricing = match self.lookup(model) {
            Some(p) => p,
            None => {
                MetricsCollector::record_cost_unknown_model();
                warn!(
                    model = %model,
                    prompt_tokens,
                    completion_tokens,
                    "No pricing entry for model; falling back to default. Billing will be inaccurate. If this model should be supported, add it to LEGACY_PRICING in cost.rs or update llm-prices; otherwise verify the caller is sending a valid model ID."
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

    pub fn get_model_pricing(&self, model: &str) -> Option<ModelPricing> {
        self.lookup(model)
    }

    pub fn suggest_cheaper_alternative(&self, model: &str) -> Option<String> {
        let current_pricing = self.lookup(model)?;
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

        // Test Gemini 2.5 Flash: $0.30/1M input, $2.50/1M output
        let cost = calculator.calculate_cost("gemini-2.5-flash", 10000, 5000);
        let expected_cost = (10000.0 / 1_000_000.0) * 0.30 + (5000.0 / 1_000_000.0) * 2.50;
        assert!((cost - expected_cost).abs() < 0.00001);
    }

    #[test]
    fn test_new_anthropic_model_pricing() {
        let calculator = CostCalculator::new();

        let opus_47 = calculator
            .get_model_pricing("claude-opus-4-7")
            .expect("claude-opus-4-7 must have pricing");
        assert!((opus_47.input_price_per_million - 5.0).abs() < 1e-9);
        assert!((opus_47.output_price_per_million - 25.0).abs() < 1e-9);

        let sonnet_46 = calculator
            .get_model_pricing("claude-sonnet-4-6")
            .expect("claude-sonnet-4-6 must have pricing");
        assert!((sonnet_46.input_price_per_million - 3.0).abs() < 1e-9);
        assert!((sonnet_46.output_price_per_million - 15.0).abs() < 1e-9);

        // Exercises the full calculate_cost path so a future refactor that
        // normalizes the lookup key is caught here, not in production billing.
        let opus_cost = calculator.calculate_cost("claude-opus-4-7", 1_000_000, 1_000_000);
        assert!((opus_cost - 30.0).abs() < 1e-9);
        let sonnet_cost = calculator.calculate_cost("claude-sonnet-4-6", 1_000_000, 1_000_000);
        assert!((sonnet_cost - 18.0).abs() < 1e-9);

        // Asymmetric input/output to catch a swapped-column regression that
        // symmetric 1M/1M totals would silently tolerate (5+25 == 25+5).
        let opus_input_only = calculator.calculate_cost("claude-opus-4-7", 1_000_000, 0);
        assert!((opus_input_only - 5.0).abs() < 1e-9);
        let opus_output_only = calculator.calculate_cost("claude-opus-4-7", 0, 1_000_000);
        assert!((opus_output_only - 25.0).abs() < 1e-9);
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
            ("gemini", "gemini-3-pro-image-preview"),
            ("gemini", "gemini-2.5-flash-image"),
            ("gemini", "gemini-2.0-flash-lite"),
            ("openai", "gpt-5-chat"),
            ("openai", "gpt-4-turbo-preview"),
            ("openai", "gpt-4-vision-preview"),
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
    fn test_list_prices_from_llm_prices() {
        // These rows were hand-typed before and had drifted from the providers' pricing
        // pages: opus-4-7 was billed at 15/75, haiku-4-5 at 0.8/4, gemini-2.5-pro output
        // at 5, gemini-2.5-flash at 0.075/0.3 and gemini-2.5-flash-lite at 0.075/0.3.
        let calculator = CostCalculator::new();
        let cases: &[(&str, f64, f64)] = &[
            ("claude-opus-4-7", 5.0, 25.0),
            ("claude-opus-4-5-20251101", 5.0, 25.0),
            ("claude-haiku-4-5-20251001", 1.0, 5.0),
            ("gemini-2.5-pro", 1.25, 10.0),
            ("gemini-2.5-flash", 0.30, 2.50),
            ("gemini-2.5-flash-lite", 0.10, 0.40),
            // Previously unpriced, billed at the flat $2/$6 default.
            ("claude-sonnet-4-20250514", 3.0, 15.0),
            ("gpt-5", 1.25, 10.0),
            ("gpt-4o-mini", 0.15, 0.60),
            ("azure-gpt-4o", 2.5, 10.0),
            ("azure-gpt-35-turbo", 0.5, 1.5),
            ("azure-gpt-4", 30.0, 60.0),
        ];
        for &(model, input, output) in cases {
            let p = calculator
                .get_model_pricing(model)
                .unwrap_or_else(|| panic!("{model} must have pricing"));
            assert!(
                (p.input_price_per_million - input).abs() < 1e-9
                    && (p.output_price_per_million - output).abs() < 1e-9,
                "{model}: got {}/{}, want {input}/{output}",
                p.input_price_per_million,
                p.output_price_per_million
            );
        }
    }

    #[test]
    fn test_snapshot_and_prefixed_ids_do_not_hit_default() {
        // Ids that are not exact keys used to fall to the $2/$6 default and bump
        // COST_UNKNOWN_MODEL. They now resolve to the model's list price.
        let calculator = CostCalculator::new();
        let sonnet = 1_000_000.0 * 3.0 / 1_000_000.0 + 1_000_000.0 * 15.0 / 1_000_000.0;
        for id in [
            "claude-sonnet-4-5",
            "anthropic/claude-sonnet-4-6",
            "CLAUDE-SONNET-4-6",
        ] {
            let cost = calculator.calculate_cost(id, 1_000_000, 1_000_000);
            assert!((cost - sonnet).abs() < 1e-9, "{id}: got {cost}");
        }
        let flash = calculator.calculate_cost("models/gemini-2.5-flash", 1_000_000, 1_000_000);
        assert!((flash - 2.80).abs() < 1e-9, "got {flash}");
    }

    #[test]
    fn test_cheaper_alternative() {
        let calculator = CostCalculator::new();

        // Claude Opus is expensive, should suggest cheaper alternatives
        let alternative = calculator.suggest_cheaper_alternative("claude-3-opus-20240229");
        assert!(alternative.is_some());
    }
}
