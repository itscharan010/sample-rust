create extension if not exists pgcrypto;

create table users (
    id uuid primary key default gen_random_uuid(),
    email text not null unique,
    password_hash text not null,
    is_vendor boolean not null default false,
    created_at timestamptz not null default now()
);

create table vendors (
    id uuid primary key default gen_random_uuid(),
    owner_user_id uuid not null references users (id),
    name text not null,
    slug text not null unique,
    commission_rate numeric(5, 4) not null default 0.10,
    created_at timestamptz not null default now()
);
create index idx_vendors_owner on vendors (owner_user_id);

create table products (
    id uuid primary key default gen_random_uuid(),
    vendor_id uuid not null references vendors (id),
    title text not null,
    description text not null default '',
    price_cents bigint not null check (price_cents >= 0),
    inventory_count integer not null default 0 check (inventory_count >= 0),
    is_active boolean not null default true,
    created_at timestamptz not null default now()
);
create index idx_products_vendor on products (vendor_id);
create index idx_products_active on products (is_active);

-- v1 constraint: one vendor per order (no split-payment cart yet).
-- `items` stores a JSON snapshot of what was ordered so historical
-- orders are unaffected by later price/title edits.
create table orders (
    id uuid primary key default gen_random_uuid(),
    buyer_user_id uuid not null references users (id),
    vendor_id uuid not null references vendors (id),
    status text not null default 'placed'
        check (status in ('placed', 'paid', 'fulfilled', 'cancelled')),
    items jsonb not null,
    total_cents bigint not null check (total_cents >= 0),
    created_at timestamptz not null default now()
);
create index idx_orders_buyer on orders (buyer_user_id);
create index idx_orders_vendor on orders (vendor_id);
