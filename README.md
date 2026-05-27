# Pricing Engine MCP Server

[![Crates.io](https://img.shields.io/crates/v/mcp-pricing.svg)](https://crates.io/crates/mcp-pricing)
[![Docs.rs](https://docs.rs/mcp-pricing/badge.svg)](https://docs.rs/mcp-pricing)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![ADK-Rust Enterprise](https://img.shields.io/badge/ADK--Rust-Enterprise-purple.svg)](https://enterprise.adk-rust.com)
[![Registry Ready](https://img.shields.io/badge/ADK_Registry-Ready-green.svg)](https://enterprise.adk-rust.com)

Enterprise pricing engine for [ADK-Rust Enterprise](https://enterprise.adk-rust.com) agents. Provides 21 MCP tools covering the full pricing lifecycle — CEL rule engine, price waterfall, product catalog, customer segments, promotions, quotes/CPQ, market intelligence, and audit trail. **Vertical-agnostic**: works for ride-hailing, SaaS, e-commerce, logistics, marketplaces, and professional services.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                     MCP Transport (stdio)                            │
├─────────────────────────────────────────────────────────────────────┤
│                    Tool Router (21 tools)                            │
├──────────┬──────────┬──────────┬──────────────┬─────────────────────┤
│  Price   │   Rule   │ Catalog  │  Promotions  │   Market Intel      │
│  Calc    │  Engine  │  & SKUs  │  & Quotes    │   & Audit           │
├──────────┴──────────┴──────────┴──────────────┴─────────────────────┤
│                     CEL Evaluation Layer                             │
│         Common Expression Language (cel-interpreter)                 │
├─────────────────────────────────────────────────────────────────────┤
│                     In-Memory Data Store                             │
│   Products · Rules · Segments · Promotions · Quotes · Audit Log     │
└─────────────────────────────────────────────────────────────────────┘
```

## Key Principles

- **CEL-powered rules** — pricing rules use Common Expression Language for type-safe, auditable conditions.
- **Price waterfall** — deterministic 6-step pipeline: list → segment → promotions → rules → floor/ceiling → tax.
- **Explain mode** — every price calculation can return the full waterfall breakdown showing which rule caused each price movement.
- **Vertical-agnostic** — same engine prices rides, SaaS subscriptions, physical goods, API calls, and professional services.
- **Audit everything** — every rule change, product update, and quote approval is logged immutably.
- **Zero configuration** — starts with no external dependencies. Market intel (FX rates) works immediately via free APIs.

## Tools (21)

### Price Calculation

| Tool | Description | Annotations |
|------|-------------|:-----------:|
| `price_calculate` | Calculate price through full waterfall with CEL rules. Set `explain=true` for step-by-step breakdown. | read-only, idempotent |

### Rule Engine

| Tool | Description | Annotations |
|------|-------------|:-----------:|
| `rules_create` | Create a pricing rule with CEL condition and actions | write |
| `rules_list` | List all pricing rules | read-only |
| `rules_activate` | Activate a rule (starts affecting prices) | write |
| `rules_deactivate` | Deactivate a rule (stops affecting prices) | write |
| `rules_validate` | Validate a CEL expression without saving | read-only |

### Product Catalog

| Tool | Description | Annotations |
|------|-------------|:-----------:|
| `catalog_upsert` | Add or update a product (SKU, name, price, cost) | write |
| `catalog_get` | Get product details by SKU | read-only |
| `catalog_list` | List all products in catalog | read-only |

### Customer Segments

| Tool | Description | Annotations |
|------|-------------|:-----------:|
| `segments_create` | Create a segment with CEL condition and default discount | write |
| `segments_list` | List all customer segments | read-only |

### Promotions

| Tool | Description | Annotations |
|------|-------------|:-----------:|
| `promotions_create` | Create a promotion (coupon, volume tier, BOGO, flash sale) | write |
| `promotions_list` | List all promotions | read-only |
| `promotions_apply` | Apply a promo code to a SKU and get discounted price | read-only |

### Quotes / CPQ

| Tool | Description | Annotations |
|------|-------------|:-----------:|
| `quotes_create` | Create a quote with locked prices for a customer | write |
| `quotes_get` | Get quote by ID | read-only |
| `quotes_approve` | Approve a quote (requires approval gate) | write |

### Market Intelligence

| Tool | Description | Annotations |
|------|-------------|:-----------:|
| `market_fx_convert` | Convert between 170+ currencies (live rates) | read-only |
| `market_fx_rates` | Get FX rates for multiple currencies | read-only |
| `market_tax` | Calculate VAT/GST/sales tax by country (50+ countries) | read-only |

### Audit

| Tool | Description | Annotations |
|------|-------------|:-----------:|
| `audit_log` | Query the immutable audit trail | read-only |

## Installation

### From crates.io

```bash
cargo install mcp-pricing
```

### Build from source

```bash
git clone https://github.com/zavora-ai/mcp-pricing
cd mcp-pricing
cargo build --release
```

### Claude Desktop

```json
{
  "mcpServers": {
    "pricing": { "command": "mcp-pricing" }
  }
}
```

### Kiro

Add to `.kiro/settings/mcp.json`:

```json
{
  "mcpServers": {
    "pricing": { "command": "mcp-pricing" }
  }
}
```

### Cursor

Add to `.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "pricing": { "command": "mcp-pricing" }
  }
}
```

### Windsurf / Codex / Open Code

```json
{
  "mcpServers": {
    "pricing": { "command": "mcp-pricing" }
  }
}
```

## Price Waterfall

Every call to `price_calculate` traverses a deterministic waterfall:

```
1. List price (from catalog)
2. Segment discount (customer segment override)
3. Promotions (active promos matching CEL conditions)
4. Pricing rules (CEL-based, sorted by priority)
5. Floor/ceiling guards (prevent below-floor or above-ceiling)
6. Tax calculation (VAT/GST by country)
─────────────────────────────────────────────────────────────
→ Net price returned
```

When `explain: true`, the full waterfall is returned:

```json
{
  "waterfall": [
    { "step": "list", "price_before": 500.0, "price_after": 500.0, "reason": "Base list price" },
    { "step": "rule", "price_before": 500.0, "price_after": 425.0, "reason": "Rule: Volume discount 10+", "rule_id": "rule_bf5d0b54" },
    { "step": "tax", "price_before": 425.0, "price_after": 493.0, "reason": "Tax 16%" }
  ]
}
```

## CEL Expression Language

Rules and segments use [Common Expression Language (CEL)](https://github.com/google/cel-spec) — the same language used by Kubernetes, Google Cloud IAM, and Firebase Security Rules.

### Available Variables

| Variable | Type | Description |
|----------|------|-------------|
| `item.sku` | string | Product SKU being priced |
| `item.quantity` | int | Quantity in the line item |
| `item.channel` | string | Sales channel (direct, marketplace, partner) |
| `customer.id` | string | Customer identifier |
| `customer.segment` | string | Resolved segment name |
| `customer.country` | string | ISO 3166-1 alpha-2 |
| `customer.annual_spend` | int | Lifetime spend |
| `catalog.list_price` | int | Current list price |
| `catalog.cost` | int | Cost of goods |
| `catalog.category` | string | Product category |

### Example Conditions

```python
# Volume discount: 15% off for orders of 10+
item.quantity >= 10

# Enterprise segment pricing
customer.segment == "enterprise" && item.channel == "direct"

# Regional pricing for East Africa
customer.country in ["KE", "TZ", "UG", "RW"]

# Category-specific rule
catalog.category == "SaaS" && customer.annual_spend > 100000

# Channel-specific markup
item.channel == "marketplace"
```

### Rule Actions

| Action Type | Effect | Example |
|-------------|--------|---------|
| `pct_discount` | Reduce price by N% | `{"type": "pct_discount", "value": 15}` |
| `absolute_discount` | Reduce price by fixed amount | `{"type": "absolute_discount", "value": 50}` |
| `markup_pct` | Increase price by N% | `{"type": "markup_pct", "value": 20}` |
| `set_price` | Override to fixed price | `{"type": "set_price", "value": 999}` |
| `multiply_price` | Multiply price (surge) | `{"type": "multiply_price", "value": 1.5}` |
| `set_floor` | Set minimum price guard | `{"type": "set_floor", "value": 100}` |
| `set_ceiling` | Set maximum price guard | `{"type": "set_ceiling", "value": 5000}` |

## Quick Start

### 1. Add a product

```json
{
  "name": "catalog_upsert",
  "arguments": {
    "sku": "RIDE-STD",
    "name": "Standard Ride",
    "category": "transport",
    "list_price": 500,
    "cost": 200,
    "currency": "KES"
  }
}
```

### 2. Create a pricing rule

```json
{
  "name": "rules_create",
  "arguments": {
    "name": "Volume discount 10+",
    "condition": "item.quantity >= 10",
    "actions": [{"type": "pct_discount", "value": 15}],
    "active": true
  }
}
```

### 3. Calculate price with waterfall

```json
{
  "name": "price_calculate",
  "arguments": {
    "sku": "RIDE-STD",
    "quantity": 12,
    "country": "KE",
    "explain": true
  }
}
```

**Response:**

```json
{
  "sku": "RIDE-STD",
  "quantity": 12,
  "list_price": 500.0,
  "net_price": 425.0,
  "line_total": 5100.0,
  "tax_amount": 816.0,
  "total": 5916.0,
  "currency": "KES",
  "tax_rate_pct": 16.0,
  "waterfall": [
    { "step": "list", "price_before": 500.0, "price_after": 500.0, "reason": "Base list price" },
    { "step": "rule", "price_before": 500.0, "price_after": 425.0, "reason": "Rule: Volume discount 10+" },
    { "step": "tax", "price_before": 425.0, "price_after": 493.0, "reason": "Tax 16%" }
  ]
}
```

### 4. Create a promotion

```json
{
  "name": "promotions_create",
  "arguments": {
    "name": "Launch 20% Off",
    "promo_type": "coupon",
    "code": "LAUNCH20",
    "discount_type": "pct",
    "discount_value": 20,
    "condition": "item.channel == \"direct\"",
    "max_uses": 1000,
    "stackable": false
  }
}
```

### 5. Generate a quote

```json
{
  "name": "quotes_create",
  "arguments": {
    "customer_id": "cust_james",
    "items": [
      {"sku": "RIDE-STD", "quantity": 5},
      {"sku": "RIDE-PREMIUM", "quantity": 2}
    ],
    "currency": "KES",
    "valid_days": 14,
    "notes": "Corporate account pricing"
  }
}
```

### 6. Convert currency

```json
{
  "name": "market_fx_convert",
  "arguments": { "amount": 5916, "from": "KES", "to": "USD" }
}
```

## Tax Coverage (50+ Countries)

| Region | Countries | Rate Range |
|--------|-----------|-----------|
| East Africa | KE (16%), TZ (18%), UG (18%), RW (18%), GH (15%) | 15-18% |
| Southern Africa | ZA (15%), NG (7.5%) | 7.5-15% |
| Europe | UK (20%), DE (19%), FR (20%), SE/NO/DK (25%), CH (8.1%) | 8-27% |
| Americas | US (0-8% by state), CA (5%), BR (17%), MX (16%) | 0-17% |
| Asia-Pacific | AU (10%), JP (10%), IN (18%), SG (9%), CN (13%) | 9-18% |
| Middle East | AE/SA (5%) | 5% |

## Use Cases

### Ride-Hailing / Mobility
```
catalog_upsert (RIDE-STD, RIDE-PREMIUM, RIDE-XL)
rules_create ("Surge 2x", "item.channel == 'peak'", multiply_price: 2.0)
rules_create ("Loyalty discount", "customer.annual_spend > 50000", pct_discount: 10)
price_calculate (sku, qty=1, channel="peak", country="KE")
```

### SaaS Subscription
```
catalog_upsert (PLAN-STARTER: $29, PLAN-PRO: $99, PLAN-ENTERPRISE: $299)
segments_create ("Enterprise", "customer.annual_spend > 100000", discount: 25%)
rules_create ("Annual billing", "item.channel == 'annual'", pct_discount: 20)
quotes_create (customer, items=[{sku: "PLAN-PRO", qty: 50}])
```

### E-Commerce
```
catalog_upsert (SKU-WIDGET-A: $49.99, cost: $15)
promotions_create ("SUMMER25", pct, 25%, condition: "catalog.category == 'outdoor'")
rules_create ("Wholesale", "item.quantity >= 100", pct_discount: 40)
price_calculate (sku, qty=150, explain=true)
```

### Marketplace / Multi-Vendor
```
rules_create ("Marketplace fee", "item.channel == 'marketplace'", markup_pct: 15)
rules_create ("Floor guard", condition: "true", set_floor: cost * 1.1)
price_calculate (sku, channel="marketplace")
```

### Professional Services
```
catalog_upsert (CONSULT-HR: $200/hr, CONSULT-LEGAL: $350/hr)
segments_create ("Non-profit", "customer.segment == 'ngo'", discount: 30%)
quotes_create (customer, items=[{sku: "CONSULT-HR", qty: 40}], valid_days: 30)
quotes_approve (quote_id)
```

## Configuration

### Environment Variables

| Variable | Required | Purpose |
|----------|:--------:|---------|
| `RUST_LOG` | No | Log level (default: `info`) |

No API keys needed. FX rates use the free open.er-api.com service. All other functionality is pure computation.

### MCP Server Manifest

```toml
server_id = "mcp_pricing"
display_name = "Pricing Engine"
version = "2.0.0"
domain = "pricing"
risk_level = "medium"
writes_allowed = "gated"
governance_gates = ["audit_all_changes"]
```

## Error Codes

| Code | Meaning |
|------|---------|
| `SKU_NOT_FOUND` | Product SKU not in catalog |
| `RULE_NOT_FOUND` | Rule ID doesn't exist |
| `CONDITION_PARSE_ERROR` | CEL expression failed to parse |
| `QUOTE_NOT_FOUND` | Quote ID doesn't exist |
| `PROMO_NOT_FOUND_OR_INACTIVE` | Promotion code invalid or expired |
| `FX_RATE_UNAVAILABLE` | Currency conversion failed |

## Roadmap

### v2.1 — Advanced Rules
- Rule versioning (edit without losing history)
- Conflict detection (overlapping rules)
- Rule testing (dry-run against test cases)
- Scheduled activation/deactivation

### v2.2 — Analytics
- Revenue by segment report
- Discount leakage detection
- Price change impact analysis
- Margin reports by SKU/channel

### v2.3 — Experiments
- A/B pricing experiments
- Statistical significance calculation
- Automatic winner promotion

### v3.0 — Persistence & Scale
- PostgreSQL backend (event-sourced)
- Redis caching for hot paths
- Bulk calculation (50K items)
- Wasm custom action functions

## Documentation

| Document | Description |
|----------|-------------|
| [mcp-server.toml](mcp-server.toml) | ADK-Rust Enterprise registry manifest |
| [CHANGELOG.md](CHANGELOG.md) | Version history |
| [Rust Docs](https://docs.rs/mcp-pricing) | Generated API documentation |

## Contributing

Contributions welcome. Priority areas:
- Additional CEL context variables
- Competitor price monitoring integration
- Subscription/metered billing support
- Multi-currency price lists
- Approval workflow chains

## Contributors

<!-- ALL-CONTRIBUTORS-LIST:START -->
| [<img src="https://github.com/jkmaina.png" width="80px;" alt=""/><br /><sub><b>James Karanja Maina</b></sub>](https://github.com/jkmaina) |
|:---:|
<!-- ALL-CONTRIBUTORS-LIST:END -->

## License

Apache-2.0 — see [LICENSE](LICENSE) for details.

---

Part of the [ADK-Rust Enterprise](https://enterprise.adk-rust.com) MCP server ecosystem.

Built with ❤️ by [Zavora AI](https://zavora.ai)

## Registry Compliance

This server implements the [ADK MCP SDK](https://crates.io/crates/adk-mcp-sdk) contract:

- **HealthCheck** — async health probe for registry monitoring
- **mcp-server.toml** — manifest declaring tools, risk classes, and credentials
- **Structured tracing** — `RUST_LOG` env-filter for observability
- **Audit trail** — every mutation logged with actor, timestamp, and details
