# marketplace (v1 skeleton)

A minimal multi-vendor marketplace backend in Rust: Axum + SQLx + PostgreSQL.

## Deliberate v1 scope (see our chat for the reasoning)

**In scope:**
- Users, vendors, products, orders
- One vendor per order (no split-cart checkout yet)
- Flat 10% commission rate per vendor (stored but not yet paid out anywhere)
- JWT auth, Argon2 password hashing
- Order state machine: `placed -> paid -> fulfilled`, or `placed/paid -> cancelled`

**Deliberately deferred:**
- Multi-vendor carts / split payments (Stripe Connect)
- Real payout calculation and disbursement
- Event-driven architecture / CQRS
- Multi-currency, multi-region, storefront theming

Get this loop working end-to-end first, then layer in complexity with real
experience instead of guessing at the schema up front.

## Project layout

```
crates/
  domain/   pure types: User, Vendor, Product, Order, OrderStatus state machine — no I/O
  db/       SQLx queries + transactions against Postgres
  api/      Axum HTTP layer: routes, JWT auth extractor, handlers
migrations/ SQL migrations (run automatically on startup via sqlx::migrate!)
```

## Running it

1. Install Rust (https://rustup.rs) and PostgreSQL.
2. Create a database: `createdb marketplace`
3. Copy the env file: `cp .env.example .env` and adjust `DATABASE_URL` / `JWT_SECRET`.
4. Run it: `cargo run -p api`

Migrations run automatically on startup. The server listens on `:8080`.

## Endpoints

| Method | Path                  | Auth      | Description |
|--------|------------------------|-----------|---|
| POST   | `/auth/register`      | none      | `{ email, password }` -> `{ token }` |
| POST   | `/auth/login`         | none      | `{ email, password }` -> `{ token }` |
| POST   | `/vendors`             | Bearer    | `{ name, slug }` — becomes a vendor |
| GET    | `/products`            | none      | list active products |
| POST   | `/products`            | Bearer    | `{ vendor_id, title, description, price_cents, inventory_count }` — must own the vendor |
| GET    | `/products/:id`        | none      | fetch one product |
| POST   | `/orders`               | Bearer    | `{ items: [{ product_id, quantity }] }` — checkout, single vendor only |
| GET    | `/orders/:id`           | Bearer    | fetch an order you placed |
| PATCH  | `/orders/:id`           | Bearer    | `{ status: "paid" \| "fulfilled" \| "cancelled" }` |

Auth: send `Authorization: Bearer <token>` from register/login.

## Known gaps to fill in next (not yet implemented)

- Real Stripe payment capture on checkout (order is created as `placed`, nothing charges a card yet)
- Vendor-side "my orders" listing (currently only the buyer can view an order)
- Ownership check on `PATCH /orders/:id` (currently any authenticated user can transition any order — restrict to the owning vendor once a vendor-order lookup exists)
- Password reset / email verification
- Pagination on `GET /products`
