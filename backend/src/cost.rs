use std::collections::HashMap;

/// Cost calculator for different LLM models
/// Prices are per 1M tokens (as of 2025)
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

        // Anthropic Claude pricing (per 1M tokens)
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
            "gemini-1.5-pro".to_string(),
            ModelPricing {
                input_price_per_million: 1.25,
                output_price_per_million: 5.0,
            },
        );
        pricing.insert(
            "gemini-1.5-flash".to_string(),
            ModelPricing {
                input_price_per_million: 0.075,
                output_price_per_million: 0.3,
            },
        );
        pricing.insert(
            "gemini-1.0-pro".to_string(),
            ModelPricing {
                input_price_per_million: 0.5,
                output_price_per_million: 1.5,
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
                // Return default pricing if model not found
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

    #[test]
    fn test_cost_calculation() {
        let calculator = CostCalculator::new();

        // Test Claude 3.5 Sonnet: $3/1M input, $15/1M output
        let cost = calculator.calculate_cost("claude-3-5-sonnet-20241022", 1000, 500);
        let expected_cost = (1000.0 / 1_000_000.0) * 3.0 + (500.0 / 1_000_000.0) * 15.0;
        assert!((cost - expected_cost).abs() < 0.00001);

        // Test Gemini 1.5 Flash: $0.075/1M input, $0.3/1M output
        let cost = calculator.calculate_cost("gemini-1.5-flash", 10000, 5000);
        let expected_cost = (10000.0 / 1_000_000.0) * 0.075 + (5000.0 / 1_000_000.0) * 0.3;
        assert!((cost - expected_cost).abs() < 0.00001);
    }

    #[test]
    fn test_cheaper_alternative() {
        let calculator = CostCalculator::new();

        // Claude Opus is expensive, should suggest cheaper alternatives
        let alternative = calculator.suggest_cheaper_alternative("claude-3-opus-20240229");
        assert!(alternative.is_some());

        // Gemini Flash is already cheap, might not have alternatives
        let alternative = calculator.suggest_cheaper_alternative("gemini-1.5-flash");
        // This might be None or a very cheap model
        println!("Cheaper than Gemini Flash: {:?}", alternative);
    }
}
