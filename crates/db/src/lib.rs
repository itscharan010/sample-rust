use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use domain::{
    Order, OrderId, OrderItem, OrderStatus, Product, ProductId, User, UserId, Vendor, VendorId,
};
use rust_decimal::Decimal;
use sqlx::{postgres::PgPoolOptions, FromRow, PgPool};
use uuid::Uuid;

pub async fn connect(database_url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;
    Ok(pool)
}

// ---------------- Users ----------------

#[derive(FromRow)]
struct UserRow {
    id: Uuid,
    email: String,
    password_hash: String,
    is_vendor: bool,
    created_at: DateTime<Utc>,
}

impl From<UserRow> for User {
    fn from(r: UserRow) -> Self {
        User {
            id: UserId(r.id),
            email: r.email,
            password_hash: r.password_hash,
            is_vendor: r.is_vendor,
            created_at: r.created_at,
        }
    }
}

pub async fn create_user(pool: &PgPool, email: &str, password_hash: &str) -> Result<User> {
    let row = sqlx::query_as::<_, UserRow>(
        r#"insert into users (id, email, password_hash, is_vendor)
           values (gen_random_uuid(), $1, $2, false)
           returning id, email, password_hash, is_vendor, created_at"#,
    )
    .bind(email)
    .bind(password_hash)
    .fetch_one(pool)
    .await?;
    Ok(row.into())
}

pub async fn find_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>> {
    let row = sqlx::query_as::<_, UserRow>(
        r#"select id, email, password_hash, is_vendor, created_at
           from users where email = $1"#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(Into::into))
}

// ---------------- Vendors ----------------

#[derive(FromRow)]
struct VendorRow {
    id: Uuid,
    owner_user_id: Uuid,
    name: String,
    slug: String,
    commission_rate: Decimal,
    created_at: DateTime<Utc>,
}

impl From<VendorRow> for Vendor {
    fn from(r: VendorRow) -> Self {
        Vendor {
            id: VendorId(r.id),
            owner_user_id: UserId(r.owner_user_id),
            name: r.name,
            slug: r.slug,
            commission_rate: r.commission_rate,
            created_at: r.created_at,
        }
    }
}

pub async fn create_vendor(
    pool: &PgPool,
    owner_user_id: UserId,
    name: &str,
    slug: &str,
) -> Result<Vendor> {
    // Mark the owning user as a vendor and create the vendor row in one
    // transaction so they can't end up out of sync.
    let mut tx = pool.begin().await?;

    sqlx::query("update users set is_vendor = true where id = $1")
        .bind(owner_user_id.0)
        .execute(&mut *tx)
        .await?;

    let row = sqlx::query_as::<_, VendorRow>(
        r#"insert into vendors (id, owner_user_id, name, slug, commission_rate)
           values (gen_random_uuid(), $1, $2, $3, 0.10)
           returning id, owner_user_id, name, slug, commission_rate, created_at"#,
    )
    .bind(owner_user_id.0)
    .bind(name)
    .bind(slug)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(row.into())
}

pub async fn get_vendor(pool: &PgPool, id: VendorId) -> Result<Option<Vendor>> {
    let row = sqlx::query_as::<_, VendorRow>(
        r#"select id, owner_user_id, name, slug, commission_rate, created_at
           from vendors where id = $1"#,
    )
    .bind(id.0)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(Into::into))
}

// ---------------- Products ----------------

#[derive(FromRow)]
struct ProductRow {
    id: Uuid,
    vendor_id: Uuid,
    title: String,
    description: String,
    price_cents: i64,
    inventory_count: i32,
    is_active: bool,
    created_at: DateTime<Utc>,
}

impl From<ProductRow> for Product {
    fn from(r: ProductRow) -> Self {
        Product {
            id: ProductId(r.id),
            vendor_id: VendorId(r.vendor_id),
            title: r.title,
            description: r.description,
            price_cents: r.price_cents,
            inventory_count: r.inventory_count,
            is_active: r.is_active,
            created_at: r.created_at,
        }
    }
}

pub async fn create_product(
    pool: &PgPool,
    vendor_id: VendorId,
    title: &str,
    description: &str,
    price_cents: i64,
    inventory_count: i32,
) -> Result<Product> {
    let row = sqlx::query_as::<_, ProductRow>(
        r#"insert into products (id, vendor_id, title, description, price_cents, inventory_count, is_active)
           values (gen_random_uuid(), $1, $2, $3, $4, $5, true)
           returning id, vendor_id, title, description, price_cents, inventory_count, is_active, created_at"#,
    )
    .bind(vendor_id.0)
    .bind(title)
    .bind(description)
    .bind(price_cents)
    .bind(inventory_count)
    .fetch_one(pool)
    .await?;
    Ok(row.into())
}

pub async fn list_active_products(pool: &PgPool) -> Result<Vec<Product>> {
    let rows = sqlx::query_as::<_, ProductRow>(
        r#"select id, vendor_id, title, description, price_cents, inventory_count, is_active, created_at
           from products where is_active = true order by created_at desc"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn get_product(pool: &PgPool, id: ProductId) -> Result<Option<Product>> {
    let row = sqlx::query_as::<_, ProductRow>(
        r#"select id, vendor_id, title, description, price_cents, inventory_count, is_active, created_at
           from products where id = $1"#,
    )
    .bind(id.0)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(Into::into))
}

// ---------------- Orders ----------------
// v1 constraint: every order belongs to exactly one vendor. Items are
// stored as a JSON snapshot column so historical orders don't change if a
// product's price/title changes later.

#[derive(FromRow)]
struct OrderRow {
    id: Uuid,
    buyer_user_id: Uuid,
    vendor_id: Uuid,
    status: String,
    items: serde_json::Value,
    total_cents: i64,
    created_at: DateTime<Utc>,
}

fn status_from_str(s: &str) -> Result<OrderStatus> {
    Ok(match s {
        "placed" => OrderStatus::Placed,
        "paid" => OrderStatus::Paid,
        "fulfilled" => OrderStatus::Fulfilled,
        "cancelled" => OrderStatus::Cancelled,
        other => return Err(anyhow!("unknown order status: {other}")),
    })
}

fn status_to_str(s: OrderStatus) -> &'static str {
    match s {
        OrderStatus::Placed => "placed",
        OrderStatus::Paid => "paid",
        OrderStatus::Fulfilled => "fulfilled",
        OrderStatus::Cancelled => "cancelled",
    }
}

impl TryFrom<OrderRow> for Order {
    type Error = anyhow::Error;
    fn try_from(r: OrderRow) -> Result<Self> {
        let items: Vec<OrderItem> = serde_json::from_value(r.items)?;
        Ok(Order {
            id: OrderId(r.id),
            buyer_user_id: UserId(r.buyer_user_id),
            vendor_id: VendorId(r.vendor_id),
            status: status_from_str(&r.status)?,
            items,
            total_cents: r.total_cents,
            created_at: r.created_at,
        })
    }
}

/// Places an order for items from a single vendor. Rejects the whole cart
/// if any item belongs to a different vendor than the first item, or if
/// there isn't enough inventory — and decrements inventory atomically in
/// the same transaction.
pub async fn place_single_vendor_order(
    pool: &PgPool,
    buyer_user_id: UserId,
    line_items: &[(ProductId, i32)], // (product_id, quantity)
) -> Result<Order> {
    if line_items.is_empty() {
        return Err(anyhow!("order must contain at least one item"));
    }

    let mut tx = pool.begin().await?;

    let mut vendor_id: Option<VendorId> = None;
    let mut items = Vec::with_capacity(line_items.len());

    for (product_id, quantity) in line_items {
        let row: ProductRow = sqlx::query_as(
            r#"select id, vendor_id, title, description, price_cents, inventory_count, is_active, created_at
               from products where id = $1 for update"#,
        )
        .bind(product_id.0)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| anyhow!("product not found: {}", product_id.0))?;

        let this_vendor = VendorId(row.vendor_id);
        match vendor_id {
            None => vendor_id = Some(this_vendor),
            Some(v) if v.0 != this_vendor.0 => {
                return Err(anyhow!(
                    "all items in one order must belong to the same vendor (v1 constraint)"
                ));
            }
            _ => {}
        }

        if row.inventory_count < *quantity {
            return Err(anyhow!("insufficient inventory for product {}", row.title));
        }

        sqlx::query("update products set inventory_count = inventory_count - $1 where id = $2")
            .bind(quantity)
            .bind(product_id.0)
            .execute(&mut *tx)
            .await?;

        items.push(OrderItem {
            product_id: *product_id,
            title_snapshot: row.title,
            unit_price_cents: row.price_cents,
            quantity: *quantity,
        });
    }

    let vendor_id = vendor_id.expect("checked non-empty above");
    let total_cents = Order::compute_total_cents(&items);
    let items_json = serde_json::to_value(&items)?;

    let row: OrderRow = sqlx::query_as(
        r#"insert into orders (id, buyer_user_id, vendor_id, status, items, total_cents)
           values (gen_random_uuid(), $1, $2, 'placed', $3, $4)
           returning id, buyer_user_id, vendor_id, status, items, total_cents, created_at"#,
    )
    .bind(buyer_user_id.0)
    .bind(vendor_id.0)
    .bind(&items_json)
    .bind(total_cents)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    row.try_into()
}

pub async fn get_order(pool: &PgPool, id: OrderId) -> Result<Option<Order>> {
    let row: Option<OrderRow> = sqlx::query_as(
        r#"select id, buyer_user_id, vendor_id, status, items, total_cents, created_at
           from orders where id = $1"#,
    )
    .bind(id.0)
    .fetch_optional(pool)
    .await?;
    row.map(TryInto::try_into).transpose()
}

/// Transitions an order's status, enforcing the state machine defined on
/// `OrderStatus` so an invalid transition (e.g. Fulfilled -> Placed)
/// can't be written to the DB.
pub async fn update_order_status(pool: &PgPool, id: OrderId, next: OrderStatus) -> Result<Order> {
    let mut tx = pool.begin().await?;

    let current: OrderRow = sqlx::query_as(
        r#"select id, buyer_user_id, vendor_id, status, items, total_cents, created_at
           from orders where id = $1 for update"#,
    )
    .bind(id.0)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| anyhow!("order not found: {}", id.0))?;

    let current_status = status_from_str(&current.status)?;
    if !current_status.can_transition_to(next) {
        return Err(anyhow!(
            "invalid order transition: {current_status:?} -> {next:?}"
        ));
    }

    sqlx::query("update orders set status = $1 where id = $2")
        .bind(status_to_str(next))
        .bind(id.0)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    let mut order: Order = current.try_into()?;
    order.status = next;
    Ok(order)
}
