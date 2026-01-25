# Design: eCommerce Service Separation for Meaningful Compositions

## Problem Statement

The current ecommerce demo services have overlapping concerns that don't require multi-step compositions. For example:
- `catalog-service` returns products with `price`, `stock` status, etc.
- This means a single tool call gets everything the customer needs
- Compositions feel forced rather than natural

**Goal:** Restructure services so that realistic customer queries *naturally require* multi-step pipelines, making the construct binding and data flow features genuinely useful.

## Proposed Service Architecture

### 1. Catalog Service (Product Master Data)
**Purpose:** Static product information - what exists in the catalog.

```
Tools:
  - search_products(query, category?, limit?) → ProductMatch[]
  - get_product(product_id) → ProductDetails
  - get_categories() → Category[]
  - get_similar_products(product_id) → ProductMatch[]

ProductMatch:
  - id: string
  - name: string
  - category: string
  - brand: string
  - description: string
  - image_url: string
  - attributes: { size?, color?, material?, ... }

ProductDetails:
  - ...ProductMatch
  - long_description: string
  - specifications: { key: value }[]
  - created_at: timestamp
```

**Note:** No price, no stock. Those are separate concerns.

---

### 2. Offers Service (Pricing & Availability)
**Purpose:** Current offers/listings for products. Simulates a marketplace where multiple sellers may offer the same product.

```
Tools:
  - get_offers(product_id) → Offer[]
  - get_offers_batch(product_ids[]) → { product_id: Offer[] }
  - get_best_offer(product_id) → Offer

Offer:
  - offer_id: string
  - product_id: string
  - seller_id: string
  - seller_name: string
  - price: number
  - original_price?: number  (for showing discounts)
  - shipping_days: number
  - shipping_cost: number
  - offer_expires_at?: timestamp
  - condition: "new" | "refurbished" | "used"
  - in_stock: boolean
  - quantity_available?: number
```

**Why separate?** Prices change frequently, multiple sellers exist, offers expire. This is fundamentally different from catalog data.

---

### 3. Inventory Service (Warehouse Stock)
**Purpose:** Internal stock management for merchandisers. Not customer-facing.

```
Tools:
  - check_stock(product_id) → StockStatus
  - check_stock_batch(product_ids[]) → StockStatus[]
  - get_inventory_report() → InventoryReport
  - get_low_stock_alerts(threshold?) → LowStockAlert[]
  - adjust_inventory(product_id, adjustment, reason) → StockStatus
  - reserve_stock(product_id, quantity, reservation_id) → Reservation

StockStatus:
  - product_id: string
  - warehouse_id: string
  - quantity_on_hand: number
  - quantity_reserved: number
  - quantity_available: number  (on_hand - reserved)
  - reorder_point: number
  - reorder_quantity: number
  - last_restock_date: timestamp

LowStockAlert:
  - product_id: string
  - product_name: string
  - quantity_available: number
  - reorder_point: number
  - deficit: number  (reorder_point - quantity_available)
  - days_of_stock_remaining: number
  - urgency: "critical" | "high" | "medium" | "low"
```

---

### 4. Supplier Service (Procurement)
**Purpose:** Supplier management and purchase orders.

```
Tools:
  - list_suppliers() → Supplier[]
  - get_supplier(supplier_id) → SupplierDetails
  - get_quotes(product_id, quantity) → Quote[]
  - get_quotes_batch(items: {product_id, quantity}[]) → Quote[]
  - create_purchase_order(supplier_id, items[]) → PurchaseOrder
  - list_purchase_orders(status?) → PurchaseOrder[]
  - receive_shipment(po_id, received_items[]) → ReceiptConfirmation

Quote:
  - supplier_id: string
  - supplier_name: string
  - product_id: string
  - quantity: number
  - unit_price: number
  - total_price: number
  - lead_time_days: number
  - valid_until: timestamp
  - reliability_score: number (0-1)
```

---

### 5. Cart Service (Shopping Cart)
**Purpose:** Cart management. Stores product_ids and quantities only.

```
Tools:
  - get_cart(user_id) → Cart
  - add_to_cart(user_id, product_id, quantity, offer_id?) → Cart
  - update_cart_item(user_id, item_id, quantity) → Cart
  - remove_from_cart(user_id, item_id) → Cart
  - clear_cart(user_id) → Cart

Cart:
  - cart_id: string
  - user_id: string
  - items: CartItem[]
  - created_at: timestamp
  - updated_at: timestamp

CartItem:
  - item_id: string
  - product_id: string
  - offer_id?: string  (which seller's offer was selected)
  - quantity: number
  - added_at: timestamp
```

**Note:** No prices in cart! Must join with offers to show totals.

---

### 6. Order Service (Order Management)
**Purpose:** Checkout and order tracking.

```
Tools:
  - checkout(user_id, payment_info, shipping_address) → Order
  - get_order(order_id) → Order
  - list_orders(user_id?, status?) → Order[]
  - cancel_order(order_id, reason) → Order
  - get_sales_report(date_range?) → SalesReport

Order:
  - order_id: string
  - user_id: string
  - items: OrderItem[]  (snapshot of product/price at purchase time)
  - subtotal: number
  - shipping: number
  - tax: number
  - total: number
  - status: "pending" | "confirmed" | "shipped" | "delivered" | "cancelled"
  - shipping_address: Address
  - created_at: timestamp
```

---

## Natural Composition Examples

With this separation, useful compositions emerge naturally:

### 1. `product_with_offers` (Customer-facing)
**Use case:** "Show me this product with current prices"

```
Pipeline:
  Step 1: catalog.get_product(product_id) → product details
  Step 2: offers.get_offers(product_id) → available offers

Output: Product info + list of buying options with prices
```

### 2. `search_with_prices` (Customer-facing)
**Use case:** "Search for coffee makers" - need to show prices in results

```
Pipeline:
  Step 1: catalog.search_products(query) → product matches (no prices!)
  Step 2: offers.get_offers_batch($.matches[*].id) → prices for each

Construct for Step 2:
  - product_ids: fromStep("search", "$.matches[*].id")
```

### 3. `cart_with_totals` (Customer-facing)
**Use case:** "Show my cart" - cart only has IDs, need product names and prices

```
Pipeline:
  Step 1: cart.get_cart(user_id) → cart items (just IDs and quantities)
  Step 2: catalog.get_products_batch($.items[*].product_id) → product names
  Step 3: offers.get_offers_batch($.items[*].product_id) → current prices

Construct for Step 2:
  - product_ids: fromStep("cart", "$.items[*].product_id")

Construct for Step 3:
  - product_ids: fromStep("cart", "$.items[*].product_id")
```

### 4. `smart_restock` (Merchandiser)
**Use case:** "What needs restocking and what will it cost?"

```
Pipeline:
  Step 1: inventory.get_low_stock_alerts() → products needing restock
  Step 2: supplier.get_quotes_batch(alerts) → quotes for each product

Construct for Step 2:
  - items: fromStep("alerts", "$.alerts").map(a => ({
      product_id: a.product_id,
      quantity: a.deficit
    }))
```

### 5. `order_fulfillment_check` (Merchandiser)
**Use case:** "Can we fulfill this order?"

```
Pipeline:
  Step 1: order.get_order(order_id) → order details
  Step 2: inventory.check_stock_batch($.items[*].product_id) → stock for each item

Construct for Step 2:
  - product_ids: fromStep("order", "$.items[*].product_id")
```

### 6. `product_intelligence` (Merchandiser)
**Use case:** "Full intel on a product - catalog, inventory, and supplier options"

```
Pipeline:
  Step 1: catalog.get_product(product_id) → product details
  Step 2: inventory.check_stock(product_id) → current stock
  Step 3: supplier.get_quotes(product_id, quantity) → reorder options

Construct for Step 3:
  - product_id: fromInput("$.product_id")
  - quantity: fromStep("stock", "$.reorder_quantity")  // use configured reorder qty
```

---

## Implementation Plan

### Phase 1: Schema Updates
1. Update catalog service to remove price/stock fields
2. Create new offers service with pricing data
3. Update inventory service with cleaner stock model
4. Update cart service to store only IDs

### Phase 2: New Tools
1. Add batch operations: `get_offers_batch`, `check_stock_batch`, `get_quotes_batch`
2. These are essential for efficient composition (avoid N+1 queries)

### Phase 3: Update Registry
1. Create new virtual tools using the compositions above
2. Update agent ALLOWED_TOOLS lists
3. Update agent system prompts to explain the new tool semantics

### Phase 4: Test & Document
1. Update integration tests
2. Add example conversations showing the compositions in action
3. Update tool-builder with the new service tools

---

## Benefits

1. **Realistic microservice architecture** - mirrors real ecommerce systems
2. **Natural compositions** - multi-step pipelines are *required*, not optional
3. **Clear data ownership** - each service owns specific data
4. **Demonstrates value** - construct binding becomes essential, not just nice-to-have
5. **Better demo** - shows agentgateway solving real orchestration problems

---

## Open Questions

1. **Batch vs. single operations:** Should compositions call batch APIs or loop with map-each?
   - Batch is more efficient but requires the batch API to exist
   - Map-each is more flexible but potentially slower

2. **Caching:** Should offers be cached? They change frequently but hitting the service per-request is expensive.

3. **Output merging:** How do we merge data from multiple steps into a unified response?
   - Current outputTransform only reshapes final step output
   - May need a "merge" or "join" pattern for combining catalog + offers data

4. **Error handling:** What if offers service is down but catalog works?
   - Partial results vs. fail-fast?
   - Per-step error handling configuration?
