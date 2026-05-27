use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::types::*;
use serde_json::json;

fn now() -> String { chrono::Utc::now().to_rfc3339() }
fn uid() -> String { uuid::Uuid::new_v4().to_string()[..8].to_string() }

#[derive(Clone)]
pub struct Store {
    pub products: Arc<Mutex<HashMap<String, Product>>>,
    pub rules: Arc<Mutex<Vec<PricingRule>>>,
    pub rule_versions: Arc<Mutex<Vec<RuleVersion>>>,
    pub segments: Arc<Mutex<Vec<Segment>>>,
    pub promotions: Arc<Mutex<Vec<Promotion>>>,
    pub quotes: Arc<Mutex<HashMap<String, Quote>>>,
    pub audit: Arc<Mutex<Vec<AuditEntry>>>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            products: Arc::new(Mutex::new(HashMap::new())),
            rules: Arc::new(Mutex::new(Vec::new())),
            rule_versions: Arc::new(Mutex::new(Vec::new())),
            segments: Arc::new(Mutex::new(Vec::new())),
            promotions: Arc::new(Mutex::new(Vec::new())),
            quotes: Arc::new(Mutex::new(HashMap::new())),
            audit: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn log_audit(&self, entity_type: &str, entity_id: &str, action: &str, actor: &str, details: serde_json::Value) {
        self.audit.lock().unwrap().push(AuditEntry {
            id: format!("aud_{}", uid()),
            timestamp: now(),
            entity_type: entity_type.into(),
            entity_id: entity_id.into(),
            action: action.into(),
            actor: actor.into(),
            details,
        });
    }

    // Product CRUD
    pub fn upsert_product(&self, p: Product) {
        let sku = p.sku.clone();
        self.log_audit("product", &sku, "upserted", "system", json!({"name": p.name}));
        self.products.lock().unwrap().insert(sku, p);
    }

    pub fn get_product(&self, sku: &str) -> Option<Product> {
        self.products.lock().unwrap().get(sku).cloned()
    }

    // Rule CRUD
    pub fn add_rule(&self, mut r: PricingRule) -> String {
        r.id = format!("rule_{}", uid());
        r.version = 1;
        r.created_at = now();
        r.updated_at = now();
        let id = r.id.clone();
        self.log_audit("rule", &id, "created", "system", json!({"name": r.name}));
        self.rule_versions.lock().unwrap().push(RuleVersion {
            rule_id: id.clone(), version: 1, snapshot: r.clone(), changed_at: now(), reason: "Initial creation".into(),
        });
        self.rules.lock().unwrap().push(r);
        id
    }

    pub fn update_rule(&self, rule_id: &str, updater: impl FnOnce(&mut PricingRule), reason: &str) -> bool {
        let mut rules = self.rules.lock().unwrap();
        if let Some(r) = rules.iter_mut().find(|r| r.id == rule_id) {
            updater(r);
            r.version += 1;
            r.updated_at = now();
            let ver = r.version;
            let snapshot = r.clone();
            drop(rules);
            self.rule_versions.lock().unwrap().push(RuleVersion {
                rule_id: rule_id.into(), version: ver, snapshot, changed_at: now(), reason: reason.into(),
            });
            self.log_audit("rule", rule_id, "updated", "system", json!({"version": ver, "reason": reason}));
            true
        } else {
            false
        }
    }

    pub fn get_active_rules(&self) -> Vec<PricingRule> {
        let now_str = now();
        let mut rules: Vec<_> = self.rules.lock().unwrap().iter().filter(|r| {
            if !r.active { return false; }
            if let Some(ref from) = r.schedule_from { if now_str < *from { return false; } }
            if let Some(ref until) = r.schedule_until { if now_str > *until { return false; } }
            true
        }).cloned().collect();
        rules.sort_by_key(|r| r.priority);
        rules
    }

    // Segment CRUD
    pub fn add_segment(&self, mut s: Segment) -> String {
        s.id = format!("seg_{}", uid());
        let id = s.id.clone();
        self.log_audit("segment", &id, "created", "system", json!({"name": s.name}));
        self.segments.lock().unwrap().push(s);
        id
    }

    // Promotion CRUD
    pub fn add_promotion(&self, mut p: Promotion) -> String {
        p.id = format!("promo_{}", uid());
        let id = p.id.clone();
        self.log_audit("promotion", &id, "created", "system", json!({"name": p.name}));
        self.promotions.lock().unwrap().push(p);
        id
    }

    pub fn get_active_promotions(&self) -> Vec<Promotion> {
        self.promotions.lock().unwrap().iter().filter(|p| p.active).cloned().collect()
    }

    // Quote CRUD
    pub fn create_quote(&self, mut q: Quote) -> String {
        q.id = format!("qt_{}", uid());
        q.created_at = now();
        let id = q.id.clone();
        self.log_audit("quote", &id, "created", "system", json!({"customer": q.customer_id}));
        self.quotes.lock().unwrap().insert(id.clone(), q);
        id
    }
}
