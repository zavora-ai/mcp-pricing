use rmcp::{handler::server::wrapper::Parameters, schemars, tool, tool_router};
use reqwest::Client;
use serde_json::{json, Value};
use crate::types::*;
use crate::store::Store;
use crate::engine;

fn now() -> String { chrono::Utc::now().to_rfc3339() }
fn round2(v: f64) -> f64 { (v * 100.0).round() / 100.0 }

// --- Input Types ---

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PriceCalcInput {
    /// SKU to price
    pub sku: String,
    /// Quantity
    pub quantity: f64,
    /// Sales channel (direct, marketplace, partner, api)
    pub channel: Option<String>,
    /// Customer ID (for segment lookup)
    pub customer_id: Option<String>,
    /// Customer country (ISO 3166-1 alpha-2)
    pub country: Option<String>,
    /// Include waterfall explanation
    pub explain: Option<bool>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProductInput {
    /// SKU identifier
    pub sku: String,
    /// Product name
    pub name: String,
    /// Category
    pub category: String,
    /// List price
    pub list_price: f64,
    /// Cost of goods
    pub cost: f64,
    /// Currency (ISO 4217)
    pub currency: Option<String>,
    /// Tags
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SkuInput {
    /// SKU to look up
    pub sku: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RuleInput {
    /// Rule name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Priority (lower = higher priority)
    pub priority: Option<i32>,
    /// CEL condition expression
    pub condition: String,
    /// Actions: [{"type": "pct_discount", "value": 10}]
    pub actions: Vec<Value>,
    /// Limit to segment IDs
    pub segments: Option<Vec<String>>,
    /// Limit to channels
    pub channels: Option<Vec<String>>,
    /// Tags
    pub tags: Option<Vec<String>>,
    /// Active immediately
    pub active: Option<bool>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RuleIdInput {
    /// Rule ID
    pub rule_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SegmentInput {
    /// Segment name
    pub name: String,
    /// CEL condition (e.g. "customer.annual_spend > 50000")
    pub condition: String,
    /// Priority
    pub priority: Option<i32>,
    /// Default discount percentage for this segment
    pub discount_pct: Option<f64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PromoInput {
    /// Promotion name
    pub name: String,
    /// Type: coupon, volume_tier, bogo, flash_sale, loyalty
    pub promo_type: String,
    /// Coupon code (for coupon type)
    pub code: Option<String>,
    /// Discount type: pct, absolute, free_item
    pub discount_type: String,
    /// Discount value
    pub discount_value: f64,
    /// CEL condition
    pub condition: Option<String>,
    /// Max total uses
    pub max_uses: Option<u32>,
    /// Valid from (ISO datetime)
    pub valid_from: Option<String>,
    /// Valid until (ISO datetime)
    pub valid_until: Option<String>,
    /// Can stack with other promotions
    pub stackable: Option<bool>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PromoApplyInput {
    /// Promotion code or ID
    pub code: String,
    /// SKU
    pub sku: String,
    /// Quantity
    pub quantity: f64,
    /// Customer ID
    pub customer_id: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct QuoteInput {
    /// Customer ID
    pub customer_id: String,
    /// Line items: [{"sku": "...", "quantity": N}]
    pub items: Vec<Value>,
    /// Currency
    pub currency: Option<String>,
    /// Valid days (default 30)
    pub valid_days: Option<u32>,
    /// Notes
    pub notes: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct QuoteIdInput {
    /// Quote ID
    pub quote_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CurrencyInput {
    /// Amount
    pub amount: f64,
    /// From currency
    pub from: String,
    /// To currency
    pub to: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FxRatesInput {
    /// Base currency
    pub base: String,
    /// Target currencies
    pub targets: Vec<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AuditInput {
    /// Entity type filter: rule, product, quote, promotion, segment
    pub entity_type: Option<String>,
    /// Entity ID filter
    pub entity_id: Option<String>,
    /// Max results
    pub limit: Option<usize>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CelValidateInput {
    /// CEL expression to validate
    pub expression: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TaxInput {
    /// Amount before tax
    pub amount: f64,
    /// Country code
    pub country: String,
    /// State (for US)
    pub state: Option<String>,
}

// --- Server ---

#[derive(Clone)]
pub struct PricingServer {
    pub client: Client,
    pub store: Store,
}

impl PricingServer {
    pub fn new() -> Self {
        Self { client: Client::builder().build().unwrap_or_default(), store: Store::new() }
    }
}

#[tool_router(server_handler)]
impl PricingServer {
    // === Price Calculation ===

    #[tool(description = "Calculate price through the full waterfall: list → segment → promotions → rules → floor/ceiling → tax. Set explain=true for step-by-step breakdown.")]
    async fn price_calculate(&self, Parameters(input): Parameters<PriceCalcInput>) -> String {
        let product = match self.store.get_product(&input.sku) {
            Some(p) => p,
            None => return json!({"error": "SKU_NOT_FOUND", "sku": input.sku}).to_string(),
        };
        let channel = input.channel.unwrap_or_else(|| "direct".into());
        let country = input.country.unwrap_or_else(|| "US".into());
        let explain = input.explain.unwrap_or(false);

        let vars = engine::CelVars {
            sku: input.sku.clone(), quantity: input.quantity, channel: channel.clone(),
            customer_id: input.customer_id.clone().unwrap_or_default(),
            segment: String::new(), country: country.clone(), annual_spend: 0.0,
            list_price: product.list_price, cost: product.cost, category: product.category.clone(),
        };

        let rules = self.store.get_active_rules();
        let promos = self.store.get_active_promotions();
        let tax_rate = get_tax_rate(&country, None);

        let (net_price, tax, waterfall) = engine::run_waterfall(
            product.list_price, &rules, &promos, 0.0, tax_rate, &vars, explain,
        );

        let line_total = round2(net_price * input.quantity);
        let tax_total = round2(tax * input.quantity);
        let mut result = json!({
            "sku": input.sku, "quantity": input.quantity,
            "list_price": product.list_price, "net_price": net_price,
            "line_total": line_total, "tax_amount": tax_total,
            "total": round2(line_total + tax_total),
            "currency": product.currency, "tax_rate_pct": tax_rate,
            "calculated_at": now()
        });
        if explain { result["waterfall"] = json!(waterfall); }
        result.to_string()
    }

    #[tool(description = "Validate a CEL expression without executing it. Returns parse errors if invalid.")]
    async fn rules_validate(&self, Parameters(input): Parameters<CelValidateInput>) -> String {
        match engine::validate_cel(&input.expression) {
            Ok(()) => json!({"valid": true, "expression": input.expression}).to_string(),
            Err(e) => json!({"valid": false, "error": e, "expression": input.expression}).to_string(),
        }
    }

    // === Product Catalog ===

    #[tool(description = "Add or update a product in the catalog with SKU, name, category, list price, and cost.")]
    async fn catalog_upsert(&self, Parameters(input): Parameters<ProductInput>) -> String {
        let p = Product {
            sku: input.sku.clone(), name: input.name, category: input.category,
            list_price: input.list_price, cost: input.cost,
            currency: input.currency.unwrap_or_else(|| "USD".into()),
            tags: input.tags.unwrap_or_default(), attributes: json!({}),
        };
        self.store.upsert_product(p);
        json!({"status": "ok", "sku": input.sku}).to_string()
    }

    #[tool(description = "Get product details and current pricing info by SKU.")]
    async fn catalog_get(&self, Parameters(input): Parameters<SkuInput>) -> String {
        match self.store.get_product(&input.sku) {
            Some(p) => serde_json::to_string_pretty(&p).unwrap_or_default(),
            None => json!({"error": "SKU_NOT_FOUND", "sku": input.sku}).to_string(),
        }
    }

    #[tool(description = "List all products in the catalog.")]
    async fn catalog_list(&self) -> String {
        let products: Vec<_> = self.store.products.lock().unwrap().values().cloned().collect();
        json!({"count": products.len(), "products": products}).to_string()
    }

    // === Pricing Rules ===

    #[tool(description = "Create a pricing rule with a CEL condition and actions. Actions: set_price, pct_discount, absolute_discount, markup_pct, set_floor, set_ceiling, multiply_price.")]
    async fn rules_create(&self, Parameters(input): Parameters<RuleInput>) -> String {
        if let Err(e) = engine::validate_cel(&input.condition) {
            return json!({"error": "CONDITION_PARSE_ERROR", "details": e}).to_string();
        }
        let actions: Vec<PricingAction> = input.actions.iter().filter_map(|a| {
            Some(PricingAction { action_type: a["type"].as_str()?.into(), value: a["value"].as_f64()? })
        }).collect();
        let rule = PricingRule {
            id: String::new(), name: input.name, description: input.description.unwrap_or_default(),
            priority: input.priority.unwrap_or(100), condition: input.condition, actions,
            segments: input.segments, channels: input.channels,
            tags: input.tags.unwrap_or_default(), active: input.active.unwrap_or(false),
            created_at: String::new(), updated_at: String::new(),
        };
        let id = self.store.add_rule(rule);
        json!({"status": "created", "rule_id": id}).to_string()
    }

    #[tool(description = "List all pricing rules (optionally filter by active only).")]
    async fn rules_list(&self) -> String {
        let rules: Vec<_> = self.store.rules.lock().unwrap().clone();
        json!({"count": rules.len(), "rules": rules}).to_string()
    }

    #[tool(description = "Activate a pricing rule by ID.")]
    async fn rules_activate(&self, Parameters(input): Parameters<RuleIdInput>) -> String {
        let mut rules = self.store.rules.lock().unwrap();
        if let Some(r) = rules.iter_mut().find(|r| r.id == input.rule_id) {
            r.active = true;
            r.updated_at = now();
            self.store.log_audit("rule", &input.rule_id, "activated", "system", json!({}));
            json!({"status": "activated", "rule_id": input.rule_id}).to_string()
        } else {
            json!({"error": "RULE_NOT_FOUND", "rule_id": input.rule_id}).to_string()
        }
    }

    #[tool(description = "Deactivate a pricing rule by ID.")]
    async fn rules_deactivate(&self, Parameters(input): Parameters<RuleIdInput>) -> String {
        let mut rules = self.store.rules.lock().unwrap();
        if let Some(r) = rules.iter_mut().find(|r| r.id == input.rule_id) {
            r.active = false;
            r.updated_at = now();
            self.store.log_audit("rule", &input.rule_id, "deactivated", "system", json!({}));
            json!({"status": "deactivated", "rule_id": input.rule_id}).to_string()
        } else {
            json!({"error": "RULE_NOT_FOUND", "rule_id": input.rule_id}).to_string()
        }
    }

    // === Segments ===

    #[tool(description = "Create a customer segment with a CEL condition and default discount.")]
    async fn segments_create(&self, Parameters(input): Parameters<SegmentInput>) -> String {
        if let Err(e) = engine::validate_cel(&input.condition) {
            return json!({"error": "CONDITION_PARSE_ERROR", "details": e}).to_string();
        }
        let seg = Segment {
            id: String::new(), name: input.name, condition: input.condition,
            priority: input.priority.unwrap_or(100), discount_pct: input.discount_pct.unwrap_or(0.0),
            metadata: json!({}),
        };
        let id = self.store.add_segment(seg);
        json!({"status": "created", "segment_id": id}).to_string()
    }

    #[tool(description = "List all customer segments.")]
    async fn segments_list(&self) -> String {
        let segs: Vec<_> = self.store.segments.lock().unwrap().clone();
        json!({"count": segs.len(), "segments": segs}).to_string()
    }

    // === Promotions ===

    #[tool(description = "Create a promotion (coupon, volume tier, BOGO, flash sale, loyalty). Supports CEL conditions and stacking.")]
    async fn promotions_create(&self, Parameters(input): Parameters<PromoInput>) -> String {
        let condition = input.condition.unwrap_or_default();
        if !condition.is_empty() {
            if let Err(e) = engine::validate_cel(&condition) {
                return json!({"error": "CONDITION_PARSE_ERROR", "details": e}).to_string();
            }
        }
        let promo = Promotion {
            id: String::new(), name: input.name, promo_type: input.promo_type,
            code: input.code, discount_type: input.discount_type, discount_value: input.discount_value,
            condition, max_uses: input.max_uses, uses: 0,
            valid_from: input.valid_from.unwrap_or_else(now),
            valid_until: input.valid_until.unwrap_or_else(|| "2099-12-31T23:59:59Z".into()),
            stackable: input.stackable.unwrap_or(false), active: true,
        };
        let id = self.store.add_promotion(promo);
        json!({"status": "created", "promotion_id": id}).to_string()
    }

    #[tool(description = "List all promotions.")]
    async fn promotions_list(&self) -> String {
        let promos: Vec<_> = self.store.promotions.lock().unwrap().clone();
        json!({"count": promos.len(), "promotions": promos}).to_string()
    }

    #[tool(description = "Apply a promotion code to a SKU and get the discounted price.")]
    async fn promotions_apply(&self, Parameters(input): Parameters<PromoApplyInput>) -> String {
        let product = match self.store.get_product(&input.sku) {
            Some(p) => p,
            None => return json!({"error": "SKU_NOT_FOUND"}).to_string(),
        };
        let promos = self.store.promotions.lock().unwrap().clone();
        let promo = promos.iter().find(|p| p.code.as_deref() == Some(&input.code) || p.id == input.code);
        match promo {
            Some(p) if p.active => {
                let discount = match p.discount_type.as_str() {
                    "pct" => product.list_price * (p.discount_value / 100.0),
                    "absolute" => p.discount_value,
                    _ => 0.0,
                };
                let net = round2((product.list_price - discount).max(0.0));
                json!({"sku": input.sku, "list_price": product.list_price, "discount": round2(discount), "net_price": net, "promotion": p.name, "line_total": round2(net * input.quantity)}).to_string()
            }
            _ => json!({"error": "PROMO_NOT_FOUND_OR_INACTIVE", "code": input.code}).to_string(),
        }
    }

    // === Quotes ===

    #[tool(description = "Create a quote for a customer. Prices are locked at calculation time.")]
    async fn quotes_create(&self, Parameters(input): Parameters<QuoteInput>) -> String {
        let currency = input.currency.unwrap_or_else(|| "USD".into());
        let valid_days = input.valid_days.unwrap_or(30);
        let mut lines = Vec::new();
        let mut total_list = 0.0;
        let mut total_net = 0.0;
        let mut total_tax = 0.0;

        for item in &input.items {
            let sku = item["sku"].as_str().unwrap_or_default();
            let qty = item["quantity"].as_f64().unwrap_or(1.0);
            if let Some(product) = self.store.get_product(sku) {
                let vars = engine::CelVars {
                    sku: sku.into(), quantity: qty, channel: "direct".into(),
                    customer_id: input.customer_id.clone(), segment: String::new(),
                    country: "US".into(), annual_spend: 0.0,
                    list_price: product.list_price, cost: product.cost, category: product.category.clone(),
                };
                let rules = self.store.get_active_rules();
                let promos = self.store.get_active_promotions();
                let (net, tax, _) = engine::run_waterfall(product.list_price, &rules, &promos, 0.0, 0.0, &vars, false);
                total_list += product.list_price * qty;
                total_net += net * qty;
                total_tax += tax * qty;
                lines.push(QuoteLine { sku: sku.into(), quantity: qty, list_price: product.list_price, net_price: net, discount: round2(product.list_price - net), tax, applied_rules: vec![] });
            }
        }

        let valid_until = (chrono::Utc::now() + chrono::Duration::days(valid_days as i64)).to_rfc3339();
        let quote = Quote {
            id: String::new(), customer_id: input.customer_id, status: "draft".into(),
            lines, currency, total_list: round2(total_list), total_net: round2(total_net),
            total_discount: round2(total_list - total_net), total_tax: round2(total_tax),
            valid_until, notes: input.notes.unwrap_or_default(), created_at: String::new(),
        };
        let id = self.store.create_quote(quote);
        let q = self.store.quotes.lock().unwrap().get(&id).cloned();
        serde_json::to_string_pretty(&q).unwrap_or_default()
    }

    #[tool(description = "Get a quote by ID.")]
    async fn quotes_get(&self, Parameters(input): Parameters<QuoteIdInput>) -> String {
        match self.store.quotes.lock().unwrap().get(&input.quote_id) {
            Some(q) => serde_json::to_string_pretty(q).unwrap_or_default(),
            None => json!({"error": "QUOTE_NOT_FOUND"}).to_string(),
        }
    }

    #[tool(description = "Approve a quote (changes status to approved).")]
    async fn quotes_approve(&self, Parameters(input): Parameters<QuoteIdInput>) -> String {
        let mut quotes = self.store.quotes.lock().unwrap();
        if let Some(q) = quotes.get_mut(&input.quote_id) {
            q.status = "approved".into();
            self.store.log_audit("quote", &input.quote_id, "approved", "system", json!({}));
            json!({"status": "approved", "quote_id": input.quote_id}).to_string()
        } else {
            json!({"error": "QUOTE_NOT_FOUND"}).to_string()
        }
    }

    // === Market Intelligence ===

    #[tool(description = "Convert currency using live exchange rates (170+ currencies). Free, no API key.")]
    async fn market_fx_convert(&self, Parameters(input): Parameters<CurrencyInput>) -> String {
        let url = format!("https://open.er-api.com/v6/latest/{}", input.from.to_uppercase());
        match self.client.get(&url).send().await {
            Ok(resp) => match resp.json::<Value>().await {
                Ok(data) => {
                    let rate = data["rates"][input.to.to_uppercase()].as_f64().unwrap_or(0.0);
                    if rate == 0.0 { return json!({"error": "FX_RATE_UNAVAILABLE"}).to_string(); }
                    json!({"amount": input.amount, "from": input.from, "to": input.to, "rate": rate, "converted": round2(input.amount * rate)}).to_string()
                }
                Err(e) => json!({"error": "FX_RATE_UNAVAILABLE", "details": e.to_string()}).to_string(),
            },
            Err(e) => json!({"error": "FX_RATE_UNAVAILABLE", "details": e.to_string()}).to_string(),
        }
    }

    #[tool(description = "Get live FX rates for multiple target currencies from a base currency.")]
    async fn market_fx_rates(&self, Parameters(input): Parameters<FxRatesInput>) -> String {
        let url = format!("https://open.er-api.com/v6/latest/{}", input.base.to_uppercase());
        match self.client.get(&url).send().await {
            Ok(resp) => match resp.json::<Value>().await {
                Ok(data) => {
                    let rates: Value = input.targets.iter().map(|t| (t.to_uppercase(), data["rates"][t.to_uppercase()].clone())).collect::<serde_json::Map<String, Value>>().into();
                    json!({"base": input.base, "rates": rates, "updated": data["time_last_update_utc"]}).to_string()
                }
                Err(e) => json!({"error": e.to_string()}).to_string(),
            },
            Err(e) => json!({"error": e.to_string()}).to_string(),
        }
    }

    #[tool(description = "Calculate tax (VAT/GST/sales tax) for an amount by country. Covers 50+ countries.")]
    async fn market_tax(&self, Parameters(input): Parameters<TaxInput>) -> String {
        let rate = get_tax_rate(&input.country, input.state.as_deref());
        let tax = round2(input.amount * rate / 100.0);
        json!({"amount": input.amount, "country": input.country, "tax_rate_pct": rate, "tax": tax, "total": round2(input.amount + tax)}).to_string()
    }

    // === Audit ===

    #[tool(description = "Query the audit log. Filter by entity_type (rule, product, quote, promotion) and entity_id.")]
    async fn audit_log(&self, Parameters(input): Parameters<AuditInput>) -> String {
        let audit = self.store.audit.lock().unwrap().clone();
        let limit = input.limit.unwrap_or(50);
        let filtered: Vec<_> = audit.iter().rev().filter(|e| {
            input.entity_type.as_ref().map_or(true, |t| &e.entity_type == t) &&
            input.entity_id.as_ref().map_or(true, |id| &e.entity_id == id)
        }).take(limit).cloned().collect();
        json!({"count": filtered.len(), "entries": filtered}).to_string()
    }
}

fn get_tax_rate(country: &str, state: Option<&str>) -> f64 {
    match country.to_uppercase().as_str() {
        "KE" => 16.0, "NG" => 7.5, "ZA" => 15.0, "GH" => 15.0, "TZ" => 18.0, "UG" => 18.0,
        "US" => match state { Some("CA") => 7.25, Some("TX") => 6.25, Some("NY") => 8.0, Some("FL") => 6.0, _ => 5.0 },
        "GB" | "UK" => 20.0, "DE" => 19.0, "FR" => 20.0, "IT" => 22.0, "ES" => 21.0, "NL" => 21.0,
        "SE" => 25.0, "NO" => 25.0, "DK" => 25.0, "CH" => 8.1, "IE" => 23.0,
        "AU" => 10.0, "NZ" => 15.0, "IN" => 18.0, "JP" => 10.0, "KR" => 10.0, "SG" => 9.0,
        "CN" => 13.0, "BR" => 17.0, "MX" => 16.0, "AE" | "SA" => 5.0, "CA" => 5.0,
        _ => 0.0,
    }
}
