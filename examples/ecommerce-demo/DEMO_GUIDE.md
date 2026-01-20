# eCommerce Demo Guide

This guide walks through key demo scenarios that showcase agentgateway's virtual tools and composition patterns.

## Setup

Before starting, ensure all services are running:

```bash
./start_services.sh
```

Services:
- Customer UI: http://localhost:8080
- Merchandiser UI: http://localhost:8081
- Gateway: http://localhost:3000

## Demo Scenarios

### Scenario 1: Customer Shopping Flow

**Demonstrates**: Basic tool operations, semantic search

1. **Open Customer UI** (http://localhost:8080)

2. **Search for Products**
   - Type "wireless headphones" in the search bar
   - Notice how semantic search finds relevant products even with different wording
   - Try "bluetooth speaker" or "workout equipment"

3. **Browse by Category**
   - Click category buttons to filter
   - Explore Electronics, Home & Kitchen, Sports & Outdoors

4. **Add Items to Cart**
   - Click "Add to Cart" on a product
   - Notice cart count updates in the header
   - Add multiple items

5. **Checkout**
   - Go to Cart page
   - Adjust quantities
   - Enter a shipping address
   - Complete checkout
   - View order confirmation

### Scenario 2: Merchandiser Inventory Management

**Demonstrates**: Dashboard aggregation, low stock alerts

1. **Open Merchandiser UI** (http://localhost:8081)

2. **Review Dashboard**
   - See key metrics: total products, units, inventory value, revenue
   - Notice alert badges for low stock and pending POs

3. **Identify Low Stock Items**
   - Click "Low Stock Items" alert or navigate to Inventory
   - Filter to show only low stock products
   - Notice products like "Hiking Backpack 40L" is out of stock

4. **Create Purchase Order**
   - Click "Order Now" on a low stock item
   - Select a supplier (note lead times and reliability scores)
   - Set quantity and create PO

5. **View Purchase Orders**
   - Navigate to Purchase Orders page
   - See pending orders with expected delivery dates

### Scenario 3: Pipeline Pattern - Restock Report

**Demonstrates**: Parallel data aggregation in pipeline

The `restock_report` virtual tool demonstrates pipeline composition:

```bash
# Using an MCP client
mcp call restock_report '{}'
```

This tool:
1. Fetches low stock alerts
2. Fetches available suppliers
3. Fetches pending purchase orders
4. Combines all data into a unified report

The pipeline runs steps 1-3 in parallel, then combines results.

### Scenario 4: Saga Pattern - Safe Checkout

**Demonstrates**: Distributed transaction with compensation

The `safe_checkout` virtual tool demonstrates saga pattern:

1. **Step 1**: Get cart contents
2. **Step 2**: Reserve inventory (compensates with release)
3. **Step 3**: Create order (compensates with cancel)

If any step fails, previous steps are automatically rolled back.

To test:
1. Add items to cart via Customer UI
2. Call `safe_checkout` through gateway
3. If successful, inventory is reserved and order created
4. If inventory reservation fails, nothing changes

### Scenario 5: Scatter-Gather - Dashboard Data

**Demonstrates**: Parallel fetching with aggregation

The `merchandiser_dashboard` virtual tool:

```bash
mcp call merchandiser_dashboard '{}'
```

Fetches in parallel:
- Inventory report
- Low stock alerts
- Pending PO summary
- Sales report

Returns all data combined for dashboard display.

### Scenario 6: Schema Transform - Customer vs Internal Views

**Demonstrates**: Data filtering for different audiences

Compare these two tools:

**Customer view** (`get_product_details`):
```json
{
  "id": "prod-001",
  "name": "Wireless Bluetooth Headphones",
  "price": 149.99,
  "in_stock": true
}
```

**Internal view** (`get_product_internal`):
```json
{
  "id": "prod-001",
  "name": "Wireless Bluetooth Headphones",
  "price": 149.99,
  "cost": 75.00,
  "stock_quantity": 45,
  "reorder_threshold": 15
}
```

The customer view hides sensitive cost and stock data.

### Scenario 7: Delivery Simulation

**Demonstrates**: Time-based events

1. **Create Purchase Orders**
   - Create a few POs in Merchandiser UI

2. **Advance Time**
   - Click "Advance Deliveries" button on dashboard
   - Or call `advance_deliveries` tool with days parameter

3. **Watch Deliveries Arrive**
   - POs with passed expected delivery dates get processed
   - Inventory automatically updates
   - Supplier reliability affects delivery success

### Scenario 8: Cached Categories

**Demonstrates**: Read-through caching

The `cached_categories` virtual tool:
- First call: Fetches from database
- Subsequent calls within 5 minutes: Returns cached data
- Uses `staleWhileRevalidate` for background refresh

### Scenario 9: Timeout Pattern

**Demonstrates**: Deadline enforcement

The `quick_search` virtual tool:
- Wraps `search_products` with 3-second timeout
- If search takes too long, returns fallback response
- Useful for maintaining responsiveness

## Tool Categories

### Customer-Facing Tools
| Tool | Description |
|------|-------------|
| `browse_products` | List products with pagination |
| `search_products` | Semantic search |
| `get_product_details` | Product info (no cost) |
| `get_categories` | Category list |
| `view_cart` | Cart contents |
| `add_to_cart` | Add item |
| `safe_checkout` | Complete purchase |

### Merchandiser Tools
| Tool | Description |
|------|-------------|
| `get_product_internal` | Full product data |
| `get_inventory_report` | Stock levels |
| `get_low_stock_alerts` | Reorder alerts |
| `adjust_inventory` | Manual adjustments |
| `create_purchase_order` | Order from supplier |
| `receive_shipment` | Mark PO received |
| `restock_report` | Aggregated restock data |
| `merchandiser_dashboard` | All dashboard data |

### Composition Patterns
| Pattern | Example Tool |
|---------|--------------|
| Pipeline | `restock_report` |
| Saga | `safe_checkout` |
| Scatter-Gather | `merchandiser_dashboard` |
| Cache | `cached_categories` |
| Timeout | `quick_search` |
| Schema Transform | `get_product_details` |

## Testing with MCP Inspector

Use the agentgateway UI to test tools:

1. Open http://localhost:15000/ui
2. Navigate to Tools section
3. Select a tool and provide input
4. Execute and view results

## Troubleshooting

### Services won't start
- Check if ports 3000, 8080, 8081 are available
- Ensure databases are seeded: `make seed`

### Vector search not working
- Ensure `sentence-transformers` is installed
- Re-seed databases to regenerate embeddings

### Gateway shows no tools
- Check registry file path in config.yaml
- Verify MCP services are running

### tmux issues
- Kill existing session: `tmux kill-session -t ecommerce-demo`
- Start fresh: `./start_services.sh`
