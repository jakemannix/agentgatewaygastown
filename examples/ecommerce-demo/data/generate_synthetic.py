#!/usr/bin/env python3
"""Synthetic data generator using LLMs for the ecommerce demo.

This script generates realistic synthetic data for products, suppliers, etc.
using LLM APIs (Anthropic, OpenAI, or Google).

Usage:
    # Generate 50 products
    python generate_synthetic.py products --count 50

    # Generate 10 suppliers
    python generate_synthetic.py suppliers --count 10

    # Generate products in a specific category
    python generate_synthetic.py products --count 20 --prompt "luxury watches and jewelry"

    # Use a specific provider
    python generate_synthetic.py products --count 10 --provider openai

    # Output to file instead of seeding database
    python generate_synthetic.py products --count 10 --output products.json

    # Dry run (print generated data without saving)
    python generate_synthetic.py products --count 5 --dry-run
"""

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Optional

# Add parent to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent / "mcp-tools"))


def get_llm_client(provider: Optional[str] = None):
    """Get the appropriate LLM client based on available API keys."""
    if provider:
        provider = provider.lower()

    # Check for explicit provider or detect from environment
    if provider == "anthropic" or (not provider and os.environ.get("ANTHROPIC_API_KEY")):
        try:
            import anthropic
            return ("anthropic", anthropic.Anthropic())
        except ImportError:
            print("Error: anthropic package not installed. Run: pip install anthropic")
            sys.exit(1)

    elif provider == "openai" or (not provider and os.environ.get("OPENAI_API_KEY")):
        try:
            import openai
            return ("openai", openai.OpenAI())
        except ImportError:
            print("Error: openai package not installed. Run: pip install openai")
            sys.exit(1)

    elif provider == "google" or (not provider and os.environ.get("GOOGLE_API_KEY")):
        try:
            import google.generativeai as genai
            genai.configure(api_key=os.environ.get("GOOGLE_API_KEY"))
            return ("google", genai.GenerativeModel("gemini-1.5-flash"))
        except ImportError:
            print("Error: google-generativeai package not installed. Run: pip install google-generativeai")
            sys.exit(1)

    else:
        print("Error: No LLM API key found.")
        print("Set one of: ANTHROPIC_API_KEY, OPENAI_API_KEY, or GOOGLE_API_KEY")
        sys.exit(1)


def call_llm(client_info: tuple, prompt: str, max_tokens: int = 4096) -> str:
    """Call the LLM with the given prompt."""
    provider, client = client_info

    if provider == "anthropic":
        response = client.messages.create(
            model="claude-sonnet-4-20250514",
            max_tokens=max_tokens,
            messages=[{"role": "user", "content": prompt}],
        )
        return response.content[0].text

    elif provider == "openai":
        response = client.chat.completions.create(
            model="gpt-4o",
            max_tokens=max_tokens,
            messages=[{"role": "user", "content": prompt}],
        )
        return response.choices[0].message.content

    elif provider == "google":
        response = client.generate_content(prompt)
        return response.text

    else:
        raise ValueError(f"Unknown provider: {provider}")


def generate_products(
    client_info: tuple,
    count: int,
    custom_prompt: Optional[str] = None,
    batch_size: int = 10,
) -> list[dict]:
    """Generate synthetic product data using LLM."""
    products = []
    generated = 0

    categories = [
        "Electronics", "Home & Kitchen", "Sports & Outdoors",
        "Books & Office", "Beauty & Personal Care", "Clothing",
        "Toys & Games", "Automotive", "Garden & Outdoor", "Pet Supplies"
    ]

    while generated < count:
        batch = min(batch_size, count - generated)

        if custom_prompt:
            category_hint = f"Focus on: {custom_prompt}"
        else:
            # Rotate through categories
            cat_idx = (generated // batch_size) % len(categories)
            category_hint = f"Focus on the '{categories[cat_idx]}' category"

        prompt = f"""Generate exactly {batch} realistic ecommerce product entries as a JSON array.
{category_hint}

Each product must have these exact fields:
- id: unique string like "prod-XXX" where XXX is a 3-digit number starting from {generated + 1:03d}
- name: descriptive product name (3-6 words)
- description: detailed product description (2-3 sentences, 30-60 words)
- price: retail price as a number (between $10-$500, realistic for the product)
- cost: wholesale cost as a number (typically 40-60% of price)
- category: one of {json.dumps(categories)}
- stock_quantity: integer between 0-200 (some items should have low stock: 0-10)
- reorder_threshold: integer between 3-30 (lower for expensive items)

Return ONLY a valid JSON array with no additional text or markdown formatting.
Make the products varied, realistic, and interesting. Include some that are:
- Popular everyday items
- Specialty/niche products
- Premium/luxury variants
- Budget-friendly options"""

        print(f"  Generating products {generated + 1}-{generated + batch}...")
        response = call_llm(client_info, prompt)

        # Parse JSON from response
        try:
            # Handle potential markdown code blocks
            if "```json" in response:
                response = response.split("```json")[1].split("```")[0]
            elif "```" in response:
                response = response.split("```")[1].split("```")[0]

            batch_products = json.loads(response.strip())
            products.extend(batch_products)
            generated += len(batch_products)
        except json.JSONDecodeError as e:
            print(f"  Warning: Failed to parse batch, retrying... ({e})")
            continue

    return products[:count]  # Trim to exact count


def generate_suppliers(
    client_info: tuple,
    count: int,
    custom_prompt: Optional[str] = None,
) -> list[dict]:
    """Generate synthetic supplier data using LLM."""
    if custom_prompt:
        context = f"These suppliers specialize in: {custom_prompt}"
    else:
        context = "Include a mix of large distributors, specialty suppliers, and regional wholesalers"

    prompt = f"""Generate exactly {count} realistic supplier/vendor entries for an ecommerce business as a JSON array.
{context}

Each supplier must have these exact fields:
- id: unique string like "sup-XXX" where XXX is a 3-digit number starting from 001
- name: realistic company name (2-4 words)
- lead_time_days: integer between 2-21 (shipping/fulfillment time)
- reliability_score: float between 0.70-0.99 (higher = more reliable)
- contact_email: realistic business email like "orders@company.example.com"

Return ONLY a valid JSON array with no additional text or markdown formatting.
Make the suppliers varied:
- Some fast but expensive (short lead time, high reliability)
- Some slow but cheap (long lead time, lower reliability)
- Mix of domestic and international sounding names
- Include specialty suppliers and general distributors"""

    print(f"  Generating {count} suppliers...")
    response = call_llm(client_info, prompt)

    # Parse JSON from response
    try:
        if "```json" in response:
            response = response.split("```json")[1].split("```")[0]
        elif "```" in response:
            response = response.split("```")[1].split("```")[0]

        suppliers = json.loads(response.strip())
        return suppliers[:count]
    except json.JSONDecodeError as e:
        print(f"Error parsing supplier data: {e}")
        print(f"Raw response: {response[:500]}...")
        return []


def seed_products(products: list[dict], data_dir: Path):
    """Seed products into the catalog database."""
    from sentence_transformers import SentenceTransformer
    from catalog_service.database import CatalogDatabase

    print("Loading embedding model...")
    model = SentenceTransformer("all-MiniLM-L6-v2")

    print("Connecting to catalog database...")
    catalog_db = CatalogDatabase(data_dir)

    print("Seeding products...")
    success = 0
    for product in products:
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
            success += 1
        except Exception as e:
            print(f"  Warning: Failed to create {product['id']}: {e}")

    print(f"  Created {success}/{len(products)} products")


def seed_suppliers(suppliers: list[dict], data_dir: Path):
    """Seed suppliers into the supplier database."""
    from supplier_service.database import SupplierDatabase

    print("Connecting to supplier database...")
    supplier_db = SupplierDatabase(data_dir)

    print("Seeding suppliers...")
    success = 0
    for supplier in suppliers:
        try:
            supplier_db.create_supplier(**supplier)
            success += 1
        except Exception as e:
            print(f"  Warning: Failed to create {supplier['id']}: {e}")

    print(f"  Created {success}/{len(suppliers)} suppliers")


def main():
    parser = argparse.ArgumentParser(
        description="Generate synthetic data for the ecommerce demo using LLMs",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument(
        "type",
        choices=["products", "suppliers"],
        help="Type of data to generate",
    )
    parser.add_argument(
        "--count", "-n",
        type=int,
        default=10,
        help="Number of records to generate (default: 10)",
    )
    parser.add_argument(
        "--prompt", "-p",
        type=str,
        help="Custom prompt to guide generation (e.g., 'outdoor camping gear')",
    )
    parser.add_argument(
        "--provider",
        choices=["anthropic", "openai", "google"],
        help="LLM provider to use (auto-detected from API keys if not specified)",
    )
    parser.add_argument(
        "--output", "-o",
        type=str,
        help="Output file path (JSON). If not specified, seeds database directly.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print generated data without saving",
    )
    parser.add_argument(
        "--batch-size",
        type=int,
        default=10,
        help="Number of records to generate per LLM call (default: 10)",
    )

    args = parser.parse_args()

    print(f"Synthetic Data Generator")
    print(f"========================")
    print(f"Type: {args.type}")
    print(f"Count: {args.count}")
    if args.prompt:
        print(f"Custom prompt: {args.prompt}")
    print()

    # Get LLM client
    print("Initializing LLM client...")
    client_info = get_llm_client(args.provider)
    print(f"  Using provider: {client_info[0]}")
    print()

    # Generate data
    if args.type == "products":
        data = generate_products(
            client_info,
            args.count,
            args.prompt,
            args.batch_size,
        )
    elif args.type == "suppliers":
        data = generate_suppliers(client_info, args.count, args.prompt)
    else:
        print(f"Unknown type: {args.type}")
        sys.exit(1)

    if not data:
        print("Error: No data generated")
        sys.exit(1)

    print(f"\nGenerated {len(data)} {args.type}")

    # Output or seed
    if args.dry_run:
        print("\n--- Generated Data (dry run) ---")
        print(json.dumps(data, indent=2))

    elif args.output:
        output_path = Path(args.output)
        with open(output_path, "w") as f:
            json.dump(data, f, indent=2)
        print(f"\nSaved to: {output_path}")

    else:
        # Seed database
        data_dir = Path(__file__).parent
        print(f"\nSeeding database in: {data_dir}")

        if args.type == "products":
            seed_products(data, data_dir)
        elif args.type == "suppliers":
            seed_suppliers(data, data_dir)

        print("\nDone!")


if __name__ == "__main__":
    main()
