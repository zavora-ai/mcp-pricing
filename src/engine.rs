use cel_interpreter::{Context, Program};
use std::collections::HashMap;
use crate::types::*;

/// Evaluate a CEL expression against a pricing context. Returns true/false.
pub fn eval_cel(expr: &str, vars: &CelVars) -> bool {
    let program = match Program::compile(expr) {
        Ok(p) => p,
        Err(_) => return false,
    };
    let mut ctx = Context::default();
    ctx.add_variable_from_value("item", HashMap::from([
        ("sku", cel_interpreter::Value::from(vars.sku.clone())),
        ("quantity", cel_interpreter::Value::from(vars.quantity as i64)),
        ("channel", cel_interpreter::Value::from(vars.channel.clone())),
    ]));
    ctx.add_variable_from_value("customer", HashMap::from([
        ("id", cel_interpreter::Value::from(vars.customer_id.clone())),
        ("segment", cel_interpreter::Value::from(vars.segment.clone())),
        ("country", cel_interpreter::Value::from(vars.country.clone())),
        ("annual_spend", cel_interpreter::Value::from(vars.annual_spend as i64)),
    ]));
    ctx.add_variable_from_value("catalog", HashMap::from([
        ("list_price", cel_interpreter::Value::from(vars.list_price as i64)),
        ("cost", cel_interpreter::Value::from(vars.cost as i64)),
        ("category", cel_interpreter::Value::from(vars.category.clone())),
    ]));
    match program.execute(&ctx) {
        Ok(val) => val == cel_interpreter::Value::from(true),
        Err(_) => false,
    }
}

/// Validate a CEL expression (parse only, no execution)
pub fn validate_cel(expr: &str) -> Result<(), String> {
    Program::compile(expr).map(|_| ()).map_err(|e| format!("{:?}", e))
}

/// Context variables for CEL evaluation
pub struct CelVars {
    pub sku: String,
    pub quantity: f64,
    pub channel: String,
    pub customer_id: String,
    pub segment: String,
    pub country: String,
    pub annual_spend: f64,
    pub list_price: f64,
    pub cost: f64,
    pub category: String,
}

/// Run the price waterfall for a single item
pub fn run_waterfall(
    list_price: f64,
    rules: &[PricingRule],
    promotions: &[Promotion],
    segment_discount: f64,
    tax_rate: f64,
    vars: &CelVars,
    explain: bool,
) -> (f64, f64, Vec<WaterfallStep>) {
    let mut price = list_price;
    let mut steps = Vec::new();

    // Step 1: List price
    if explain {
        steps.push(WaterfallStep {
            step: "list".into(), rule_id: None, rule_name: None,
            price_before: price, price_after: price, delta: 0.0,
            reason: "Base list price".into(),
        });
    }

    // Step 2: Segment discount
    if segment_discount > 0.0 {
        let before = price;
        price *= 1.0 - (segment_discount / 100.0);
        if explain {
            steps.push(WaterfallStep {
                step: "segment".into(), rule_id: None, rule_name: None,
                price_before: before, price_after: round2(price), delta: round2(price - before),
                reason: format!("Segment discount {}%", segment_discount),
            });
        }
    }

    // Step 3: Promotions
    for promo in promotions {
        if !promo.active { continue; }
        if promo.condition.is_empty() || eval_cel(&promo.condition, vars) {
            let before = price;
            match promo.discount_type.as_str() {
                "pct" => price *= 1.0 - (promo.discount_value / 100.0),
                "absolute" => price -= promo.discount_value,
                _ => {}
            }
            price = price.max(0.0);
            if explain {
                steps.push(WaterfallStep {
                    step: "promotion".into(), rule_id: Some(promo.id.clone()), rule_name: Some(promo.name.clone()),
                    price_before: round2(before), price_after: round2(price), delta: round2(price - before),
                    reason: format!("{} {} {}", promo.discount_type, promo.discount_value, promo.name),
                });
            }
            if !promo.stackable { break; }
        }
    }

    // Step 4: Pricing rules (CEL-based)
    let mut floor: Option<f64> = None;
    let mut ceiling: Option<f64> = None;

    for rule in rules {
        if rule.condition.is_empty() || eval_cel(&rule.condition, vars) {
            let before = price;
            for action in &rule.actions {
                match action.action_type.as_str() {
                    "pct_discount" => price *= 1.0 - (action.value / 100.0),
                    "absolute_discount" => price -= action.value,
                    "markup_pct" => price *= 1.0 + (action.value / 100.0),
                    "set_price" => price = action.value,
                    "multiply_price" => price *= action.value,
                    "set_floor" => floor = Some(action.value),
                    "set_ceiling" => ceiling = Some(action.value),
                    _ => {}
                }
            }
            price = price.max(0.0);
            if explain && (price - before).abs() > 0.001 {
                steps.push(WaterfallStep {
                    step: "rule".into(), rule_id: Some(rule.id.clone()), rule_name: Some(rule.name.clone()),
                    price_before: round2(before), price_after: round2(price), delta: round2(price - before),
                    reason: format!("Rule: {}", rule.name),
                });
            }
        }
    }

    // Step 5: Floor/ceiling guards
    if let Some(f) = floor {
        if price < f {
            let before = price;
            price = f;
            if explain {
                steps.push(WaterfallStep {
                    step: "floor".into(), rule_id: None, rule_name: None,
                    price_before: round2(before), price_after: round2(price), delta: round2(price - before),
                    reason: format!("Floor guard: {}", f),
                });
            }
        }
    }
    if let Some(c) = ceiling {
        if price > c {
            let before = price;
            price = c;
            if explain {
                steps.push(WaterfallStep {
                    step: "ceiling".into(), rule_id: None, rule_name: None,
                    price_before: round2(before), price_after: round2(price), delta: round2(price - before),
                    reason: format!("Ceiling guard: {}", c),
                });
            }
        }
    }

    // Step 6: Tax
    let tax = price * (tax_rate / 100.0);
    if explain && tax > 0.0 {
        steps.push(WaterfallStep {
            step: "tax".into(), rule_id: None, rule_name: None,
            price_before: round2(price), price_after: round2(price + tax), delta: round2(tax),
            reason: format!("Tax {}%", tax_rate),
        });
    }

    (round2(price), round2(tax), steps)
}

fn round2(v: f64) -> f64 { (v * 100.0).round() / 100.0 }
