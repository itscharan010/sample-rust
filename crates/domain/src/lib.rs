use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------- Newtype IDs (compile-time safety against mixing up IDs) ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VendorId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProductId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrderId(pub Uuid);

// ---------- User ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub is_vendor: bool,
    pub created_at: DateTime<Utc>,
}

// ---------- Vendor ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vendor {
    pub id: VendorId,
    pub owner_user_id: UserId,
    pub name: String,
    pub slug: String,
    /// Flat commission rate for now, e.g. 0.10 = 10%.
    pub commission_rate: Decimal,
    pub created_at: DateTime<Utc>,
}

// ---------- Product ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: ProductId,
    pub vendor_id: VendorId,
    pub title: String,
    pub description: String,
    pub price_cents: i64,
    pub inventory_count: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

// ---------- Order ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    Placed,
    Paid,
    Fulfilled,
    Cancelled,
}

impl OrderStatus {
    /// Enforce valid transitions in one place instead of scattering checks
    /// across handlers.
    pub fn can_transition_to(self, next: OrderStatus) -> bool {
        use OrderStatus::*;
        matches!(
            (self, next),
            (Placed, Paid) | (Placed, Cancelled) | (Paid, Fulfilled) | (Paid, Cancelled)
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    pub product_id: ProductId,
    pub title_snapshot: String,
    pub unit_price_cents: i64,
    pub quantity: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: OrderId,
    pub buyer_user_id: UserId,
    /// v1 constraint: one vendor per order. Revisit once split payments
    /// (Stripe Connect) are wired up.
    pub vendor_id: VendorId,
    pub status: OrderStatus,
    pub items: Vec<OrderItem>,
    pub total_cents: i64,
    pub created_at: DateTime<Utc>,
}

impl Order {
    pub fn compute_total_cents(items: &[OrderItem]) -> i64 {
        items
            .iter()
            .map(|i| i.unit_price_cents * i.quantity as i64)
            .sum()
    }
}
