use serde::{Deserialize, Serialize};
use serde_json::Value;

// === Core Models ===

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Product {
    pub sku: String,
    pub name: String,
    pub category: String,
    pub list_price: f64,
    pub cost: f64,
    pub currency: String,
    pub tags: Vec<String>,
    pub attributes: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PricingRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub priority: i32,
    pub condition: String, // CEL expression
    pub actions: Vec<PricingAction>,
    pub segments: Option<Vec<String>>,
    pub channels: Option<Vec<String>>,
    pub tags: Vec<String>,
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PricingAction {
    #[serde(rename = "type")]
    pub action_type: String, // set_price, pct_discount, absolute_discount, markup_pct, set_floor, set_ceiling
    pub value: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Segment {
    pub id: String,
    pub name: String,
    pub condition: String, // CEL expression
    pub priority: i32,
    pub discount_pct: f64,
    pub metadata: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Promotion {
    pub id: String,
    pub name: String,
    pub promo_type: String, // coupon, volume_tier, bogo, flash_sale, loyalty
    pub code: Option<String>,
    pub discount_type: String, // pct, absolute, free_item
    pub discount_value: f64,
    pub condition: String, // CEL
    pub max_uses: Option<u32>,
    pub uses: u32,
    pub valid_from: String,
    pub valid_until: String,
    pub stackable: bool,
    pub active: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Quote {
    pub id: String,
    pub customer_id: String,
    pub status: String, // draft, pending_approval, approved, rejected, converted, expired
    pub lines: Vec<QuoteLine>,
    pub currency: String,
    pub total_list: f64,
    pub total_net: f64,
    pub total_discount: f64,
    pub total_tax: f64,
    pub valid_until: String,
    pub notes: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuoteLine {
    pub sku: String,
    pub quantity: f64,
    pub list_price: f64,
    pub net_price: f64,
    pub discount: f64,
    pub tax: f64,
    pub applied_rules: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WaterfallStep {
    pub step: String,
    pub rule_id: Option<String>,
    pub rule_name: Option<String>,
    pub price_before: f64,
    pub price_after: f64,
    pub delta: f64,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: String,
    pub entity_type: String, // rule, product, quote, promotion, segment
    pub entity_id: String,
    pub action: String, // created, updated, deleted, activated, deactivated
    pub actor: String,
    pub details: Value,
}
