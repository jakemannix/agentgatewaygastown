/**
 * eCommerce Demo Registry - TypeScript DSL version
 *
 * This file demonstrates building the same virtual tools as ecommerce_registry_v2.json
 * using the @vmcp/dsl TypeScript DSL instead of raw JSON.
 *
 * Benefits:
 * - Type safety: TypeScript catches schema errors at compile time
 * - IDE support: Autocomplete, refactoring, go-to-definition
 * - Composition: Build complex pipelines with fluent builder API
 * - DRY: Reuse schemas, helper functions, and patterns
 *
 * Run with:
 *   cd packages/vmcp-dsl && npx tsx ../../examples/ecommerce-demo/gateway-configs/ecommerce_registry.ts
 *
 * Or compile to JSON:
 *   npx tsx examples/ecommerce-demo/gateway-configs/ecommerce_registry.ts > registry.json
 */

import {
  // Registry v2 builders
  registryV2,
  schema,
  server,
  toolV2,
  agent,
  skill,
  toolDep,
  schemaRef,
  // Pattern builders
  pipeline,
  step,
  // Binding helpers
  fromInput,
  fromStep,
  constant,
  construct,
  // Output transform
  outputTransform,
} from '../../../packages/vmcp-dsl/src/index.js';

// =============================================================================
// SCHEMAS - Reusable type definitions
// =============================================================================

const ProductSummary = schema('ProductSummary', '1.0.0')
  .description('Basic product info shown in search results and listings')
  .schema({
    type: 'object',
    properties: {
      id: { type: 'string' },
      name: { type: 'string' },
      price: { type: 'number' },
      description: { type: 'string' },
      in_stock: { type: 'boolean' },
    },
  })
  .build();

const ProductListOutput = schema('ProductListOutput', '1.0.0')
  .description('Standard product search/list result')
  .schema({
    type: 'object',
    properties: {
      products: {
        type: 'array',
        items: { $ref: '#ProductSummary' },
      },
      total_found: { type: 'integer' },
    },
  })
  .build();

const ProductQueryInput = schema('ProductQueryInput', '1.0.0')
  .description('Common input for product searches')
  .schema({
    type: 'object',
    properties: {
      query: { type: 'string', description: 'What are you looking for?' },
      category: { type: 'string', description: 'Optional category filter' },
    },
    required: ['query'],
  })
  .build();

const ProductIdInput = schema('ProductIdInput', '1.0.0')
  .description('Input requiring a product ID')
  .schema({
    type: 'object',
    properties: {
      product_id: { type: 'string', description: 'Product ID' },
    },
    required: ['product_id'],
  })
  .build();

const PersonalizedSearchInput = schema('PersonalizedSearchInput', '1.0.0')
  .description('Input for personalized search')
  .schema({
    type: 'object',
    properties: {
      query: { type: 'string', description: 'Search query' },
      user_id: { type: 'string', description: 'User ID for personalization' },
      limit: { type: 'integer', description: 'Max results', default: 10 },
    },
    required: ['query'],
  })
  .build();

// =============================================================================
// SERVERS - Backend MCP services
// =============================================================================

const catalogService = server('catalog-service', '1.0.0')
  .description('Product catalog with search and browse capabilities')
  .provides('search_products', '1.0.0')
  .provides('list_products', '1.0.0')
  .provides('get_product', '1.0.0')
  .provides('get_categories', '1.0.0')
  .provides('search_index', '1.0.0')
  .provides('hydrate_products', '1.0.0')
  .provides('personalize_ranking', '1.0.0')
  .metadata('owner', 'catalog-team')
  .build();

const cartService = server('cart-service', '1.0.0')
  .description('Shopping cart management')
  .provides('view_cart', '1.0.0')
  .provides('add_to_cart', '1.0.0')
  .provides('update_cart_item', '1.0.0')
  .provides('remove_from_cart', '1.0.0')
  .provides('clear_cart', '1.0.0')
  .build();

const orderService = server('order-service', '1.0.0')
  .description('Order processing and management')
  .provides('checkout', '1.0.0')
  .provides('get_order', '1.0.0')
  .provides('list_orders', '1.0.0')
  .provides('get_sales_report', '1.0.0')
  .provides('update_order_status', '1.0.0')
  .build();

const inventoryService = server('inventory-service', '1.0.0')
  .description('Stock tracking and inventory management')
  .provides('check_stock', '1.0.0')
  .provides('get_inventory_report', '1.0.0')
  .provides('get_low_stock_alerts', '1.0.0')
  .provides('adjust_inventory', '1.0.0')
  .build();

const supplierService = server('supplier-service', '1.0.0')
  .description('Supplier and purchase order management')
  .provides('list_suppliers', '1.0.0')
  .provides('get_supplier', '1.0.0')
  .provides('create_purchase_order', '1.0.0')
  .provides('list_purchase_orders', '1.0.0')
  .provides('receive_shipment', '1.0.0')
  .provides('get_all_quotes', '1.0.0')
  .build();

// =============================================================================
// TOOLS - Virtual tool definitions
// =============================================================================

// --- Simple Source Tools (1:1 mapping) ---

const findProducts = toolV2('find_products', '1.0.0')
  .description("Search for products using natural language (e.g., 'comfortable running shoes under $100')")
  .source('catalog-service', '1.0.0', 'search_products')
  .sourceConfig({
    server: 'catalog-service',
    serverVersion: '1.0.0',
    tool: 'search_products',
    defaults: {
      in_stock_only: true,
      limit: 8,
    },
    hideFields: ['include_embeddings', 'debug_scores', 'sort_order', 'page'],
  })
  .inputSchemaRef(schemaRef('ProductQueryInput', '1.0.0'))
  .outputSchemaRef(schemaRef('ProductListOutput', '1.0.0'))
  .outputTransform({
    mappings: {
      products: { path: '$.products' },
      total_found: { path: '$.results_count' },
    },
  })
  .build();

const browseProducts = toolV2('browse_products', '1.0.0')
  .description('Browse the product catalog with optional category filter')
  .source('catalog-service', '1.0.0', 'list_products')
  .build();

const getProductDetails = toolV2('get_product_details', '1.0.0')
  .description('Get detailed information about a specific product')
  .source('catalog-service', '1.0.0', 'get_product')
  .build();

// --- Composition Tools (N:1 mapping with pipelines) ---

/**
 * Personalized Search Pipeline
 *
 * A 3-step pipeline demonstrating cross-step data flow:
 * 1. search_index - Fast token matching, returns (id, score) pairs
 * 2. hydrate_products - Fetch full product details for IDs
 * 3. personalize_ranking - Re-rank based on user preferences
 *
 * Uses construct bindings to build inputs from multiple sources.
 */
const personalizedSearch = toolV2('personalized_search', '1.0.0')
  .description('Search products and personalize results based on your preferences')
  .spec(
    pipeline()
      // Step 1: Fast index search
      .addStep(
        step('search')
          .tool('catalog-service_search_index')
          .fromInput('$')
          .build()
      )
      // Step 2: Hydrate with full product details
      .addStep({
        id: 'hydrate',
        operation: { tool: { name: 'catalog-service_hydrate_products' } },
        input: construct({
          product_ids: fromStep('search', '$.matches[*].id'),
          fields: constant(['id', 'name', 'price', 'category', 'brand', 'description', 'image_url']),
        }),
      })
      // Step 3: Personalize ranking
      .addStep({
        id: 'personalize',
        operation: { tool: { name: 'catalog-service_personalize_ranking' } },
        input: construct({
          items: fromStep('hydrate', '$.products'),
          scores: fromStep('search', '$.matches[*].score'),
          user_id: fromInput('$.user_id'),
        }),
      })
      .build()
  )
  .inputSchemaRef(schemaRef('PersonalizedSearchInput', '1.0.0'))
  .outputTransform({
    mappings: {
      products: { path: '$.products' },
    },
  })
  .build();

/**
 * Top Restock Quote
 *
 * A 2-step pipeline showcasing JSONPath field extraction:
 * 1. Get low stock alerts (sorted by deficit - most critical first)
 * 2. Get supplier quotes for the FIRST product using its deficit as quantity
 *
 * The construct binding extracts $.alerts[0].product_id and $.alerts[0].deficit
 * from step 1's output to build step 2's input.
 */
const topRestockQuote = toolV2('top_restock_quote', '1.0.0')
  .description('Get supplier quotes for the most critical low-stock product')
  .spec(
    pipeline()
      // Step 1: Get low stock alerts
      .addStep(
        step('alerts')
          .tool('inventory-service_get_low_stock_alerts')
          .fromInput('$')
          .build()
      )
      // Step 2: Get quotes for the top alert
      .addStep({
        id: 'quotes',
        operation: { tool: { name: 'supplier-service_get_all_quotes' } },
        input: construct({
          product_id: fromStep('alerts', '$.alerts[0].product_id'),
          quantity: fromStep('alerts', '$.alerts[0].deficit'),
        }),
      })
      .build()
  )
  .inputSchema({
    type: 'object',
    properties: {
      threshold: { type: 'integer', description: 'Optional custom stock threshold' },
    },
  })
  .outputTransform({
    mappings: {
      product_id: { path: '$.product_id' },
      product_name: { path: '$.product_name' },
      quantity_needed: { path: '$.quantity_requested' },
      quotes: { path: '$.quotes' },
      best_price: { path: '$.best_price' },
      fastest_delivery: { path: '$.fastest_delivery' },
    },
  })
  .build();

/**
 * Product Intelligence
 *
 * A 3-step pipeline calling 3 different services for a single product:
 * 1. Get product details from catalog
 * 2. Check real-time inventory status
 * 3. Get supplier quotes for restocking
 *
 * Demonstrates constant injection (quantity: 50) for the quotes step.
 */
const productIntelligence = toolV2('product_intelligence', '1.0.0')
  .description('Get complete product intelligence: details, inventory status, and supplier options')
  .spec(
    pipeline()
      .addStep(
        step('product')
          .tool('catalog-service_get_product')
          .fromInput('$')
          .build()
      )
      .addStep(
        step('stock')
          .tool('inventory-service_check_stock')
          .fromInput('$')
          .build()
      )
      .addStep({
        id: 'quotes',
        operation: { tool: { name: 'supplier-service_get_all_quotes' } },
        input: construct({
          product_id: fromInput('$.product_id'),
          quantity: constant(50),
        }),
      })
      .build()
  )
  .inputSchemaRef(schemaRef('ProductIdInput', '1.0.0'))
  .build();

/**
 * Order with Context
 *
 * A 3-step pipeline that enriches an order with business intelligence:
 * 1. Get order details
 * 2. Get sales context (how is this product selling?)
 * 3. Get inventory to show remaining stock after this order
 *
 * Demonstrates using constant({}) to call tools with no input from prior steps.
 */
const orderWithContext = toolV2('order_with_context', '1.0.0')
  .description('Get order details with sales context and inventory impact')
  .spec(
    pipeline()
      .addStep(
        step('order')
          .tool('order-service_get_order')
          .fromInput('$')
          .build()
      )
      .addStep(
        step('sales')
          .tool('order-service_get_sales_report')
          .constant({})
          .build()
      )
      .addStep(
        step('inventory')
          .tool('inventory-service_get_inventory_report')
          .constant({})
          .build()
      )
      .build()
  )
  .inputSchema({
    type: 'object',
    properties: {
      order_id: { type: 'string', description: 'Order ID to look up' },
    },
    required: ['order_id'],
  })
  .build();

// =============================================================================
// AGENTS - Agent definitions with SBOM
// =============================================================================

const customerAgent = agent('customer-agent', '1.0.0')
  .description('Customer-facing shopping assistant (Google ADK)')
  .url('http://localhost:9001')
  .streaming()
  .sbom([
    toolDep('virtual_find_products', '1.0.0'),
    toolDep('virtual_browse_products', '1.0.0'),
    toolDep('virtual_get_product_details', '1.0.0'),
    toolDep('virtual_personalized_search', '1.0.0'),
  ])
  .build();

const merchandiserAgent = agent('merchandiser-agent', '1.0.0')
  .description('Internal merchandiser assistant for inventory and suppliers (LangGraph)')
  .url('http://localhost:9002')
  .streaming()
  .sbom([
    toolDep('virtual_top_restock_quote', '1.0.0'),
    toolDep('virtual_product_intelligence', '1.0.0'),
    toolDep('virtual_order_with_context', '1.0.0'),
  ])
  .build();

// =============================================================================
// BUILD REGISTRY
// =============================================================================

export const registry = registryV2()
  // Schemas
  .schemas(
    ProductSummary,
    ProductListOutput,
    ProductQueryInput,
    ProductIdInput,
    PersonalizedSearchInput
  )
  // Servers
  .servers(catalogService, cartService, orderService, inventoryService, supplierService)
  // Tools (compositions)
  .tools(
    findProducts,
    browseProducts,
    getProductDetails,
    personalizedSearch,
    topRestockQuote,
    productIntelligence,
    orderWithContext
  )
  // Agents
  .agents(customerAgent, merchandiserAgent)
  .build();

// Output JSON when run directly
console.log(JSON.stringify(registry, null, 2));
