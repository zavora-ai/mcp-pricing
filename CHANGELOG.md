# Changelog

## [2.1.0] - 2026-05-27

### Added — Advanced Rules
- `rules_update` — edit rules with full version history (old versions preserved)
- `rules_history` — get all versions of a rule with timestamps and change reasons
- `rules_schedule` — schedule activation/deactivation at specific times (ISO datetime)
- `rules_conflicts` — detect overlapping rules at same priority with resolution suggestions
- `rules_test` — dry-run a rule (existing or ad-hoc) against test cases with pass/fail results
- Version tracking on all rules (auto-increments on each edit)
- Schedule-aware `get_active_rules` — respects `schedule_from` and `schedule_until`

## [2.0.0] - 2026-05-27

### Added — Enterprise Pricing Engine
- **CEL Rule Engine** — Common Expression Language for type-safe pricing conditions
  - `rules_create` — create rules with CEL conditions and actions
  - `rules_list` — list all rules
  - `rules_activate` / `rules_deactivate` — toggle rules
  - `rules_validate` — validate CEL expressions without saving
- **Price Waterfall** — deterministic 6-step pipeline
  - `price_calculate` — full waterfall with `explain` mode
  - Steps: list → segment → promotions → rules → floor/ceiling → tax
- **Product Catalog** — in-memory SKU store
  - `catalog_upsert` — add/update products
  - `catalog_get` / `catalog_list` — query catalog
- **Customer Segments** — CEL-based classification
  - `segments_create` / `segments_list`
- **Promotions** — coupon, volume tier, BOGO, flash sale, loyalty
  - `promotions_create` / `promotions_list` / `promotions_apply`
- **Quotes/CPQ** — stateful quote lifecycle
  - `quotes_create` — lock prices at calculation time
  - `quotes_get` / `quotes_approve`
- **Market Intelligence**
  - `market_fx_convert` — 170+ currencies (live rates)
  - `market_fx_rates` — batch FX rates
  - `market_tax` — VAT/GST/sales tax for 50+ countries
- **Audit Trail**
  - `audit_log` — immutable log of all mutations
- CEL context variables: item.sku, item.quantity, item.channel, customer.id, customer.segment, customer.country, customer.annual_spend, catalog.list_price, catalog.cost, catalog.category
- Rule actions: set_price, pct_discount, absolute_discount, markup_pct, multiply_price, set_floor, set_ceiling

### Breaking Changes
- Complete rewrite from v1.0.0 (simple calculator) to enterprise pricing engine
- All v1.0.0 tools removed (calculate_fare, calculate_delivery, etc.)
- New tool naming convention: `domain_action` (e.g. `rules_create`, `catalog_upsert`)

## [1.0.0] - 2026-05-27

### Added
- `calculate_fare` — ride/transport fare calculation
- `calculate_delivery` — delivery fee calculation
- `apply_discount` — percentage, fixed, BOGO discounts
- `calculate_subscription` — SaaS subscription pricing
- `convert_currency` — live FX conversion (170+ currencies)
- `calculate_tax` — VAT/GST/sales tax (50+ countries)
- `calculate_tiered` — volume/usage-based pricing
- `split_payment` — split bills between parties
- `calculate_margin` — profit margin/markup calculator
- `calculate_invoice` — invoice generation with line items
- `calculate_surge` — dynamic pricing multiplier
- `calculate_bundle` — bundle discount pricing
