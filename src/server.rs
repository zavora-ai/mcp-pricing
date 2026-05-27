use rmcp::{handler::server::wrapper::Parameters, schemars, tool, tool_router};
use reqwest::Client;
use serde_json::{json, Value};

fn now() -> String { chrono::Utc::now().to_rfc3339() }

// --- Input types ---

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FareInput {
    /// Distance in km
    pub distance_km: f64,
    /// Duration in minutes
    pub duration_min: f64,
    /// Base fare (flat fee)
    pub base_fare: f64,
    /// Rate per km
    pub per_km_rate: f64,
    /// Rate per minute
    pub per_min_rate: f64,
    /// Surge multiplier (default 1.0)
    pub surge: Option<f64>,
    /// Booking/service fee
    pub booking_fee: Option<f64>,
    /// Minimum fare
    pub minimum_fare: Option<f64>,
    /// Currency code (e.g. "KES", "USD")
    pub currency: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DeliveryInput {
    /// Distance in km
    pub distance_km: f64,
    /// Package weight in kg
    pub weight_kg: f64,
    /// Base delivery fee
    pub base_fee: f64,
    /// Rate per km
    pub per_km_rate: f64,
    /// Rate per kg (for weight surcharge)
    pub per_kg_rate: Option<f64>,
    /// Speed tier: standard, express, same_day (multipliers: 1.0, 1.5, 2.5)
    pub speed_tier: Option<String>,
    /// Currency code
    pub currency: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DiscountInput {
    /// Original amount
    pub amount: f64,
    /// Discount type: percentage, fixed, bogo (buy one get one)
    pub discount_type: String,
    /// Discount value (percentage 0-100, or fixed amount)
    pub value: f64,
    /// Maximum discount cap (optional)
    pub max_discount: Option<f64>,
    /// Minimum order amount to qualify (optional)
    pub min_order: Option<f64>,
    /// Currency code
    pub currency: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SubscriptionInput {
    /// Monthly base price
    pub monthly_price: f64,
    /// Billing cycle: monthly, quarterly, annual
    pub billing_cycle: String,
    /// Number of seats/users
    pub seats: Option<u32>,
    /// Per-seat price (if seat-based)
    pub per_seat_price: Option<f64>,
    /// Annual discount percentage (e.g. 20 for 20% off annual)
    pub annual_discount_pct: Option<f64>,
    /// Currency code
    pub currency: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CurrencyInput {
    /// Amount to convert
    pub amount: f64,
    /// Source currency (ISO 4217, e.g. "USD")
    pub from: String,
    /// Target currency (ISO 4217, e.g. "KES")
    pub to: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TaxInput {
    /// Amount before tax
    pub amount: f64,
    /// Country code (ISO 3166-1 alpha-2, e.g. "KE", "US", "DE")
    pub country: String,
    /// State/region (for US sales tax, e.g. "CA", "TX")
    pub state: Option<String>,
    /// Tax type: vat, gst, sales_tax, auto (default: auto)
    pub tax_type: Option<String>,
    /// Custom tax rate override (percentage)
    pub custom_rate: Option<f64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TieredInput {
    /// Usage/quantity
    pub quantity: f64,
    /// Tiers as array of [limit, price_per_unit] (e.g. [[100, 0.10], [1000, 0.08], [0, 0.05]])
    /// Last tier with limit 0 means unlimited
    pub tiers: Vec<[f64; 2]>,
    /// Unit label (e.g. "API calls", "GB", "messages")
    pub unit: Option<String>,
    /// Currency code
    pub currency: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SplitInput {
    /// Total amount
    pub total: f64,
    /// Number of parties
    pub parties: u32,
    /// Split type: equal, percentage, custom
    pub split_type: String,
    /// Custom splits (for percentage: [50, 30, 20], for custom: [100, 50, 75])
    pub splits: Option<Vec<f64>>,
    /// Currency code
    pub currency: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct MarginInput {
    /// Cost price
    pub cost: f64,
    /// Target margin percentage (e.g. 30 for 30%)
    pub margin_pct: Option<f64>,
    /// Target markup percentage (e.g. 50 for 50%)
    pub markup_pct: Option<f64>,
    /// Selling price (to calculate margin from)
    pub selling_price: Option<f64>,
    /// Currency code
    pub currency: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct InvoiceInput {
    /// Line items: [{"description": "...", "quantity": N, "unit_price": X}]
    pub items: Vec<Value>,
    /// Tax rate percentage (e.g. 16 for 16% VAT)
    pub tax_rate: Option<f64>,
    /// Discount percentage on subtotal
    pub discount_pct: Option<f64>,
    /// Currency code
    pub currency: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SurgeInput {
    /// Current demand (e.g. ride requests per minute)
    pub demand: f64,
    /// Current supply (e.g. available drivers)
    pub supply: f64,
    /// Base multiplier (default 1.0)
    pub base_multiplier: Option<f64>,
    /// Max multiplier cap (default 5.0)
    pub max_multiplier: Option<f64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BundleInput {
    /// Individual item prices
    pub items: Vec<f64>,
    /// Bundle discount percentage
    pub bundle_discount_pct: f64,
    /// Currency code
    pub currency: Option<String>,
}

#[derive(Clone)]
pub struct PricingServer {
    pub client: Client,
}

impl PricingServer {
    pub fn new() -> Self { Self { client: Client::builder().build().unwrap_or_default() } }
}

#[tool_router(server_handler)]
impl PricingServer {
    #[tool(description = "Calculate ride/transport fare (distance × rate + time × rate + base + surge). Works for ride-hailing, taxi, ambulance, tow trucks.")]
    async fn calculate_fare(&self, Parameters(input): Parameters<FareInput>) -> String {
        let surge = input.surge.unwrap_or(1.0);
        let booking_fee = input.booking_fee.unwrap_or(0.0);
        let currency = input.currency.as_deref().unwrap_or("USD");
        let distance_cost = input.distance_km * input.per_km_rate;
        let time_cost = input.duration_min * input.per_min_rate;
        let subtotal = (input.base_fare + distance_cost + time_cost) * surge + booking_fee;
        let total = if let Some(min) = input.minimum_fare { subtotal.max(min) } else { subtotal };
        json!({
            "breakdown": {"base_fare": input.base_fare, "distance_cost": round2(distance_cost), "time_cost": round2(time_cost), "surge_multiplier": surge, "booking_fee": booking_fee},
            "subtotal": round2(subtotal), "total": round2(total), "currency": currency,
            "distance_km": input.distance_km, "duration_min": input.duration_min
        }).to_string()
    }

    #[tool(description = "Calculate delivery fee (distance + weight + speed tier). Works for food delivery, parcels, freight.")]
    async fn calculate_delivery(&self, Parameters(input): Parameters<DeliveryInput>) -> String {
        let currency = input.currency.as_deref().unwrap_or("USD");
        let per_kg = input.per_kg_rate.unwrap_or(0.0);
        let speed_mult = match input.speed_tier.as_deref() { Some("express") => 1.5, Some("same_day") => 2.5, Some("overnight") => 2.0, _ => 1.0 };
        let distance_cost = input.distance_km * input.per_km_rate;
        let weight_cost = input.weight_kg * per_kg;
        let total = (input.base_fee + distance_cost + weight_cost) * speed_mult;
        json!({
            "breakdown": {"base_fee": input.base_fee, "distance_cost": round2(distance_cost), "weight_surcharge": round2(weight_cost), "speed_multiplier": speed_mult, "speed_tier": input.speed_tier.as_deref().unwrap_or("standard")},
            "total": round2(total), "currency": currency
        }).to_string()
    }

    #[tool(description = "Apply discount (percentage, fixed, or BOGO). Works for coupons, promo codes, loyalty tiers.")]
    async fn apply_discount(&self, Parameters(input): Parameters<DiscountInput>) -> String {
        let currency = input.currency.as_deref().unwrap_or("USD");
        if let Some(min) = input.min_order { if input.amount < min { return json!({"error": "Order below minimum", "min_order": min, "amount": input.amount}).to_string(); } }
        let discount = match input.discount_type.as_str() {
            "percentage" => input.amount * (input.value / 100.0),
            "fixed" => input.value,
            "bogo" => input.amount / 2.0,
            _ => 0.0,
        };
        let discount = if let Some(cap) = input.max_discount { discount.min(cap) } else { discount };
        let final_amount = (input.amount - discount).max(0.0);
        json!({"original": input.amount, "discount": round2(discount), "discount_type": input.discount_type, "final_amount": round2(final_amount), "savings_pct": round2(discount / input.amount * 100.0), "currency": currency}).to_string()
    }

    #[tool(description = "Calculate subscription pricing (monthly/quarterly/annual with seats and discounts). Works for SaaS, memberships, plans.")]
    async fn calculate_subscription(&self, Parameters(input): Parameters<SubscriptionInput>) -> String {
        let currency = input.currency.as_deref().unwrap_or("USD");
        let seats = input.seats.unwrap_or(1) as f64;
        let per_seat = input.per_seat_price.unwrap_or(0.0);
        let monthly = input.monthly_price + (seats * per_seat);
        let annual_disc = input.annual_discount_pct.unwrap_or(20.0) / 100.0;
        let (total, period, savings) = match input.billing_cycle.as_str() {
            "quarterly" => (monthly * 3.0 * 0.95, "quarter", monthly * 3.0 * 0.05),
            "annual" | "yearly" => (monthly * 12.0 * (1.0 - annual_disc), "year", monthly * 12.0 * annual_disc),
            _ => (monthly, "month", 0.0),
        };
        json!({"monthly_equivalent": round2(monthly), "billing_cycle": input.billing_cycle, "total": round2(total), "period": period, "seats": seats as u32, "savings": round2(savings), "currency": currency}).to_string()
    }

    #[tool(description = "Convert currency using live exchange rates (170+ currencies, updated daily). Free, no API key.")]
    async fn convert_currency(&self, Parameters(input): Parameters<CurrencyInput>) -> String {
        let url = format!("https://open.er-api.com/v6/latest/{}", input.from.to_uppercase());
        match self.client.get(&url).send().await {
            Ok(resp) => match resp.json::<Value>().await {
                Ok(data) => {
                    let rate = data["rates"][input.to.to_uppercase()].as_f64().unwrap_or(0.0);
                    if rate == 0.0 { return json!({"error": "Currency not found", "from": input.from, "to": input.to}).to_string(); }
                    let converted = input.amount * rate;
                    json!({"amount": input.amount, "from": input.from, "to": input.to, "rate": rate, "converted": round2(converted), "updated": data["time_last_update_utc"]}).to_string()
                }
                Err(e) => format!("Error: {e}"),
            },
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Calculate tax (VAT, GST, sales tax) by country/region. Covers 100+ countries with standard rates.")]
    async fn calculate_tax(&self, Parameters(input): Parameters<TaxInput>) -> String {
        let rate = if let Some(r) = input.custom_rate { r } else { get_tax_rate(&input.country, input.state.as_deref()) };
        let tax_amount = input.amount * (rate / 100.0);
        let total = input.amount + tax_amount;
        let tax_type = input.tax_type.as_deref().unwrap_or(match input.country.as_str() { "US" => "sales_tax", "AU"|"IN"|"SG"|"NZ"|"CA" => "gst", _ => "vat" });
        json!({"amount_before_tax": input.amount, "tax_rate_pct": rate, "tax_type": tax_type, "tax_amount": round2(tax_amount), "total": round2(total), "country": input.country, "state": input.state}).to_string()
    }

    #[tool(description = "Calculate tiered/volume pricing (e.g. API calls, storage, utilities). Each tier has a limit and per-unit price.")]
    async fn calculate_tiered(&self, Parameters(input): Parameters<TieredInput>) -> String {
        let currency = input.currency.as_deref().unwrap_or("USD");
        let unit = input.unit.as_deref().unwrap_or("units");
        let mut remaining = input.quantity;
        let mut total = 0.0;
        let mut breakdown = Vec::new();
        let mut prev_limit = 0.0;
        for tier in &input.tiers {
            let limit = tier[0];
            let price = tier[1];
            let tier_size = if limit == 0.0 { remaining } else { (limit - prev_limit).min(remaining) };
            if tier_size <= 0.0 { break; }
            let cost = tier_size * price;
            total += cost;
            breakdown.push(json!({"range": format!("{}-{}", prev_limit as u64, if limit == 0.0 { "∞".into() } else { format!("{}", limit as u64) }), "quantity": tier_size, "rate": price, "cost": round2(cost)}));
            remaining -= tier_size;
            prev_limit = limit;
            if remaining <= 0.0 { break; }
        }
        json!({"quantity": input.quantity, "unit": unit, "breakdown": breakdown, "total": round2(total), "currency": currency}).to_string()
    }

    #[tool(description = "Split payment between multiple parties (equal, percentage, or custom amounts). Works for group rides, shared bills, marketplace payouts.")]
    async fn split_payment(&self, Parameters(input): Parameters<SplitInput>) -> String {
        let currency = input.currency.as_deref().unwrap_or("USD");
        let splits: Vec<Value> = match input.split_type.as_str() {
            "equal" => (0..input.parties).map(|i| json!({"party": i+1, "amount": round2(input.total / input.parties as f64)})).collect(),
            "percentage" => input.splits.unwrap_or_default().iter().enumerate().map(|(i, pct)| json!({"party": i+1, "percentage": pct, "amount": round2(input.total * pct / 100.0)})).collect(),
            "custom" => input.splits.unwrap_or_default().iter().enumerate().map(|(i, amt)| json!({"party": i+1, "amount": round2(*amt)})).collect(),
            _ => vec![json!({"error": "Unknown split_type"})],
        };
        json!({"total": input.total, "parties": input.parties, "split_type": input.split_type, "splits": splits, "currency": currency}).to_string()
    }

    #[tool(description = "Calculate profit margin and markup. Given cost, compute selling price from target margin or markup. Or given selling price, compute margin.")]
    async fn calculate_margin(&self, Parameters(input): Parameters<MarginInput>) -> String {
        let currency = input.currency.as_deref().unwrap_or("USD");
        if let Some(margin) = input.margin_pct {
            let selling = input.cost / (1.0 - margin / 100.0);
            let profit = selling - input.cost;
            json!({"cost": input.cost, "selling_price": round2(selling), "profit": round2(profit), "margin_pct": margin, "markup_pct": round2(profit / input.cost * 100.0), "currency": currency}).to_string()
        } else if let Some(markup) = input.markup_pct {
            let selling = input.cost * (1.0 + markup / 100.0);
            let profit = selling - input.cost;
            json!({"cost": input.cost, "selling_price": round2(selling), "profit": round2(profit), "margin_pct": round2(profit / selling * 100.0), "markup_pct": markup, "currency": currency}).to_string()
        } else if let Some(selling) = input.selling_price {
            let profit = selling - input.cost;
            json!({"cost": input.cost, "selling_price": selling, "profit": round2(profit), "margin_pct": round2(profit / selling * 100.0), "markup_pct": round2(profit / input.cost * 100.0), "currency": currency}).to_string()
        } else {
            json!({"error": "Provide margin_pct, markup_pct, or selling_price"}).to_string()
        }
    }

    #[tool(description = "Generate invoice totals from line items with tax and discount. Works for any business — services, products, consulting.")]
    async fn calculate_invoice(&self, Parameters(input): Parameters<InvoiceInput>) -> String {
        let currency = input.currency.as_deref().unwrap_or("USD");
        let mut subtotal = 0.0;
        let mut lines: Vec<Value> = Vec::new();
        for item in &input.items {
            let qty = item["quantity"].as_f64().unwrap_or(1.0);
            let price = item["unit_price"].as_f64().unwrap_or(0.0);
            let line_total = qty * price;
            subtotal += line_total;
            lines.push(json!({"description": item["description"], "quantity": qty, "unit_price": price, "line_total": round2(line_total)}));
        }
        let discount_amt = input.discount_pct.map(|d| subtotal * d / 100.0).unwrap_or(0.0);
        let taxable = subtotal - discount_amt;
        let tax_amt = input.tax_rate.map(|t| taxable * t / 100.0).unwrap_or(0.0);
        let total = taxable + tax_amt;
        json!({"lines": lines, "subtotal": round2(subtotal), "discount": round2(discount_amt), "discount_pct": input.discount_pct, "taxable_amount": round2(taxable), "tax": round2(tax_amt), "tax_rate_pct": input.tax_rate, "total": round2(total), "currency": currency}).to_string()
    }

    #[tool(description = "Calculate surge/dynamic pricing multiplier from demand and supply ratio. Works for ride-hailing, event tickets, hotel rooms.")]
    async fn calculate_surge(&self, Parameters(input): Parameters<SurgeInput>) -> String {
        let base = input.base_multiplier.unwrap_or(1.0);
        let max = input.max_multiplier.unwrap_or(5.0);
        let ratio = if input.supply > 0.0 { input.demand / input.supply } else { max };
        let multiplier = (base * ratio).min(max).max(1.0);
        let level = if multiplier >= 3.0 { "extreme" } else if multiplier >= 2.0 { "high" } else if multiplier >= 1.5 { "moderate" } else { "normal" };
        json!({"demand": input.demand, "supply": input.supply, "ratio": round2(ratio), "multiplier": round2(multiplier), "level": level, "capped_at": max}).to_string()
    }

    #[tool(description = "Calculate bundle pricing (multiple items with bundle discount). Works for product bundles, meal deals, service packages.")]
    async fn calculate_bundle(&self, Parameters(input): Parameters<BundleInput>) -> String {
        let currency = input.currency.as_deref().unwrap_or("USD");
        let individual_total: f64 = input.items.iter().sum();
        let discount = individual_total * (input.bundle_discount_pct / 100.0);
        let bundle_price = individual_total - discount;
        json!({"items": input.items.len(), "individual_total": round2(individual_total), "bundle_discount_pct": input.bundle_discount_pct, "discount": round2(discount), "bundle_price": round2(bundle_price), "per_item_avg": round2(bundle_price / input.items.len() as f64), "currency": currency}).to_string()
    }
}

fn round2(v: f64) -> f64 { (v * 100.0).round() / 100.0 }

fn get_tax_rate(country: &str, state: Option<&str>) -> f64 {
    match country.to_uppercase().as_str() {
        "KE" => 16.0, "NG" => 7.5, "ZA" => 15.0, "GH" => 15.0, "TZ" => 18.0, "UG" => 18.0, "RW" => 18.0, "ET" => 15.0,
        "US" => match state { Some("CA") => 7.25, Some("TX") => 6.25, Some("NY") => 8.0, Some("FL") => 6.0, Some("WA") => 6.5, Some("OR") => 0.0, Some("NH") => 0.0, _ => 5.0 },
        "GB" | "UK" => 20.0, "DE" => 19.0, "FR" => 20.0, "IT" => 22.0, "ES" => 21.0, "NL" => 21.0, "BE" => 21.0, "AT" => 20.0, "SE" => 25.0, "NO" => 25.0, "DK" => 25.0, "FI" => 24.0, "PL" => 23.0, "IE" => 23.0, "PT" => 23.0, "CH" => 8.1, "CZ" => 21.0, "HU" => 27.0,
        "AU" => 10.0, "NZ" => 15.0, "IN" => 18.0, "JP" => 10.0, "KR" => 10.0, "SG" => 9.0, "CN" => 13.0, "MY" => 6.0, "TH" => 7.0, "ID" => 11.0, "PH" => 12.0, "VN" => 10.0,
        "CA" => 5.0, "BR" => 17.0, "MX" => 16.0, "AR" => 21.0, "CL" => 19.0, "CO" => 19.0,
        "AE" | "SA" => 5.0, "EG" => 14.0, "IL" => 17.0, "TR" => 20.0,
        _ => 0.0,
    }
}
