#!/usr/bin/env python3
"""Seed script to populate the ecommerce demo databases with sample data.

Run from the ecommerce-demo directory:
    python data/seed_data.py
"""

import sys
from pathlib import Path

# Add parent (ecommerce-demo) to path for proper module imports
parent_dir = str(Path(__file__).parent.parent)
if parent_dir not in sys.path:
    sys.path.insert(0, parent_dir)

from sentence_transformers import SentenceTransformer

from mcp_tools.catalog_service.database import CatalogDatabase
from mcp_tools.supplier_service.database import SupplierDatabase

# Sample product data with varied categories
PRODUCTS = [
    # Electronics
    {
        "id": "prod-001",
        "name": "Wireless Bluetooth Headphones",
        "description": "Premium noise-canceling wireless headphones with 30-hour battery life. Features deep bass, comfortable ear cushions, and foldable design.",
        "price": 149.99,
        "cost": 75.00,
        "category": "Electronics",
        "stock_quantity": 45,
        "reorder_threshold": 15,
    },
    {
        "id": "prod-002",
        "name": "USB-C Fast Charger",
        "description": "65W GaN charger with multiple ports. Compatible with laptops, tablets, and phones. Compact travel-friendly design.",
        "price": 49.99,
        "cost": 22.00,
        "category": "Electronics",
        "stock_quantity": 120,
        "reorder_threshold": 30,
    },
    {
        "id": "prod-003",
        "name": "Portable Bluetooth Speaker",
        "description": "Waterproof portable speaker with 360-degree sound. Perfect for outdoor adventures with 20-hour playtime.",
        "price": 79.99,
        "cost": 35.00,
        "category": "Electronics",
        "stock_quantity": 60,
        "reorder_threshold": 20,
    },
    {
        "id": "prod-004",
        "name": "Smart Watch Fitness Tracker",
        "description": "Advanced fitness watch with heart rate monitor, GPS, sleep tracking, and water resistance to 50m.",
        "price": 199.99,
        "cost": 95.00,
        "category": "Electronics",
        "stock_quantity": 8,
        "reorder_threshold": 10,
    },
    # Home & Kitchen
    {
        "id": "prod-005",
        "name": "Stainless Steel Coffee Maker",
        "description": "12-cup programmable coffee maker with thermal carafe. Features auto brew, strength control, and easy cleaning.",
        "price": 89.99,
        "cost": 42.00,
        "category": "Home & Kitchen",
        "stock_quantity": 35,
        "reorder_threshold": 12,
    },
    {
        "id": "prod-006",
        "name": "Non-Stick Cookware Set",
        "description": "10-piece ceramic non-stick cookware set. Includes pots, pans, and utensils. Dishwasher safe and PFOA-free.",
        "price": 129.99,
        "cost": 55.00,
        "category": "Home & Kitchen",
        "stock_quantity": 25,
        "reorder_threshold": 8,
    },
    {
        "id": "prod-007",
        "name": "Electric Stand Mixer",
        "description": "Professional 5-quart stand mixer with 10 speeds. Perfect for baking with dough hook, flat beater, and whisk attachments.",
        "price": 249.99,
        "cost": 120.00,
        "category": "Home & Kitchen",
        "stock_quantity": 3,
        "reorder_threshold": 5,
    },
    {
        "id": "prod-008",
        "name": "Vacuum Insulated Water Bottle",
        "description": "32oz stainless steel water bottle keeps drinks cold 24hrs or hot 12hrs. Leak-proof lid and wide mouth.",
        "price": 34.99,
        "cost": 12.00,
        "category": "Home & Kitchen",
        "stock_quantity": 200,
        "reorder_threshold": 50,
    },
    # Sports & Outdoors
    {
        "id": "prod-009",
        "name": "Yoga Mat Premium",
        "description": "Extra thick 6mm eco-friendly yoga mat with alignment lines. Non-slip surface and carrying strap included.",
        "price": 44.99,
        "cost": 18.00,
        "category": "Sports & Outdoors",
        "stock_quantity": 75,
        "reorder_threshold": 25,
    },
    {
        "id": "prod-010",
        "name": "Camping Tent 4-Person",
        "description": "Waterproof family tent with easy setup. Features rainfly, mesh windows, and storage pockets. 3-season design.",
        "price": 179.99,
        "cost": 85.00,
        "category": "Sports & Outdoors",
        "stock_quantity": 15,
        "reorder_threshold": 5,
    },
    {
        "id": "prod-011",
        "name": "Adjustable Dumbbell Set",
        "description": "Space-saving adjustable dumbbells from 5 to 52.5 lbs each. Quick-change weight system for home workouts.",
        "price": 299.99,
        "cost": 150.00,
        "category": "Sports & Outdoors",
        "stock_quantity": 12,
        "reorder_threshold": 4,
    },
    {
        "id": "prod-012",
        "name": "Hiking Backpack 40L",
        "description": "Lightweight hiking backpack with hydration bladder compartment. Ventilated back panel and rain cover included.",
        "price": 89.99,
        "cost": 40.00,
        "category": "Sports & Outdoors",
        "stock_quantity": 0,
        "reorder_threshold": 8,
    },
    # Books & Office
    {
        "id": "prod-013",
        "name": "Leather Journal Notebook",
        "description": "Handcrafted genuine leather journal with 200 lined pages. Vintage design with ribbon bookmark and closure strap.",
        "price": 29.99,
        "cost": 10.00,
        "category": "Books & Office",
        "stock_quantity": 150,
        "reorder_threshold": 40,
    },
    {
        "id": "prod-014",
        "name": "Ergonomic Office Chair",
        "description": "Adjustable mesh office chair with lumbar support. Features armrests, headrest, and tilt mechanism.",
        "price": 349.99,
        "cost": 175.00,
        "category": "Books & Office",
        "stock_quantity": 7,
        "reorder_threshold": 3,
    },
    {
        "id": "prod-015",
        "name": "Wireless Keyboard and Mouse Combo",
        "description": "Slim wireless keyboard and ergonomic mouse with USB receiver. Quiet keys and long battery life.",
        "price": 59.99,
        "cost": 25.00,
        "category": "Books & Office",
        "stock_quantity": 85,
        "reorder_threshold": 25,
    },
    # Beauty & Personal Care
    {
        "id": "prod-016",
        "name": "Organic Skincare Set",
        "description": "5-piece organic skincare kit with cleanser, toner, serum, moisturizer, and eye cream. Suitable for all skin types.",
        "price": 79.99,
        "cost": 30.00,
        "category": "Beauty & Personal Care",
        "stock_quantity": 55,
        "reorder_threshold": 15,
    },
    {
        "id": "prod-017",
        "name": "Electric Toothbrush Pro",
        "description": "Sonic electric toothbrush with 5 cleaning modes, pressure sensor, and 2-minute timer. Includes 3 brush heads.",
        "price": 69.99,
        "cost": 28.00,
        "category": "Beauty & Personal Care",
        "stock_quantity": 40,
        "reorder_threshold": 12,
    },
    {
        "id": "prod-018",
        "name": "Hair Dryer Professional",
        "description": "1875W ionic hair dryer with diffuser and concentrator attachments. Fast drying with reduced frizz technology.",
        "price": 54.99,
        "cost": 22.00,
        "category": "Beauty & Personal Care",
        "stock_quantity": 5,
        "reorder_threshold": 10,
    },
    # Clothing
    {
        "id": "prod-019",
        "name": "Merino Wool Sweater",
        "description": "Premium merino wool crewneck sweater. Soft, breathable, and temperature-regulating for year-round comfort.",
        "price": 89.99,
        "cost": 38.00,
        "category": "Clothing",
        "stock_quantity": 30,
        "reorder_threshold": 10,
    },
    {
        "id": "prod-020",
        "name": "Running Shoes Performance",
        "description": "Lightweight running shoes with responsive cushioning and breathable mesh upper. Great for road running.",
        "price": 119.99,
        "cost": 55.00,
        "category": "Clothing",
        "stock_quantity": 48,
        "reorder_threshold": 15,
    },
]

# Sample suppliers
SUPPLIERS = [
    {
        "id": "sup-001",
        "name": "TechSource Global",
        "lead_time_days": 5,
        "reliability_score": 0.95,
        "contact_email": "orders@techsource.example.com",
    },
    {
        "id": "sup-002",
        "name": "HomeGoods Direct",
        "lead_time_days": 7,
        "reliability_score": 0.88,
        "contact_email": "supply@homegoods.example.com",
    },
    {
        "id": "sup-003",
        "name": "SportsWare International",
        "lead_time_days": 10,
        "reliability_score": 0.82,
        "contact_email": "fulfillment@sportsware.example.com",
    },
    {
        "id": "sup-004",
        "name": "Premium Distributors",
        "lead_time_days": 3,
        "reliability_score": 0.98,
        "contact_email": "orders@premiumdist.example.com",
    },
    {
        "id": "sup-005",
        "name": "Value Wholesale Co",
        "lead_time_days": 14,
        "reliability_score": 0.75,
        "contact_email": "bulk@valuewholesale.example.com",
    },
]


def seed_databases(data_dir: Path):
    """Seed all databases with sample data."""
    print("Loading embedding model...")
    model = SentenceTransformer("all-MiniLM-L6-v2")

    print("Initializing databases...")
    catalog_db = CatalogDatabase(data_dir)
    supplier_db = SupplierDatabase(data_dir)

    # Seed suppliers
    print("Seeding suppliers...")
    for supplier in SUPPLIERS:
        try:
            supplier_db.create_supplier(**supplier)
            print(f"  Created supplier: {supplier['name']}")
        except Exception as e:
            print(f"  Supplier {supplier['name']} may already exist: {e}")

    # Seed products with embeddings
    print("Seeding products...")
    for product in PRODUCTS:
        try:
            # Generate embedding from name + description
            text = f"{product['name']}. {product['description']}"
            embedding = model.encode(text).tolist()

            catalog_db.create_product(
                product_id=product["id"],
                name=product["name"],
                description=product["description"],
                price=product["price"],
                cost=product["cost"],
                category=product["category"],
                stock_quantity=product["stock_quantity"],
                reorder_threshold=product["reorder_threshold"],
                embedding=embedding,
            )
            print(f"  Created product: {product['name']}")
        except Exception as e:
            print(f"  Product {product['name']} may already exist: {e}")

    print("\nSeeding complete!")
    print(f"  Products: {len(PRODUCTS)}")
    print(f"  Suppliers: {len(SUPPLIERS)}")

    # Show some stats
    stats = catalog_db.get_inventory_stats()
    print(f"\nInventory Stats:")
    print(f"  Total products: {stats['total_products']}")
    print(f"  Total units: {stats['total_units']}")
    print(f"  Total value (at cost): ${stats['total_value']:.2f}")
    print(f"  Low stock items: {stats['low_stock_count']}")
    print(f"  Out of stock items: {stats['out_of_stock_count']}")


def reset_databases(data_dir: Path):
    """Delete all database files."""
    import os

    db_files = [
        "catalog.db",
        "cart.db",
        "order.db",
        "inventory.db",
        "supplier.db",
    ]
    for db_file in db_files:
        db_path = data_dir / db_file
        if db_path.exists():
            os.remove(db_path)
            print(f"Deleted: {db_path}")


if __name__ == "__main__":
    data_dir = Path(__file__).parent

    if len(sys.argv) > 1 and sys.argv[1] == "--reset":
        print("Resetting databases...")
        reset_databases(data_dir)

    seed_databases(data_dir)
