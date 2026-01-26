"""Seed the databases with initial data for the research assistant demo.

Creates:
- Sample entities (concepts, papers, people)
- Sample relations between entities
- Category taxonomy for ML/AI topics
- Sample tagged content

Run with: uv run python data/seed_data.py
"""

import sys
import os
from pathlib import Path

# Add parent to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent))

from mcp_tools.entity_service.database import EntityDatabase
from mcp_tools.category_service.database import CategoryDatabase
from mcp_tools.tag_service.database import TagDatabase


def seed_entities(db: EntityDatabase):
    """Create sample entities."""
    print("Seeding entities...")

    entities = [
        # Core concepts
        {
            "name": "Transformer",
            "entity_type": "concept",
            "description": "Neural network architecture based on self-attention mechanisms. "
                          "Introduced in 'Attention Is All You Need' paper. Foundation for modern LLMs.",
            "properties": {"year_introduced": 2017}
        },
        {
            "name": "Self-Attention",
            "entity_type": "concept",
            "description": "Mechanism that allows a model to weigh the importance of different positions "
                          "in an input sequence when computing representations.",
            "properties": {"complexity": "O(n^2)"}
        },
        {
            "name": "State Space Model",
            "entity_type": "concept",
            "description": "Class of models that process sequences through latent state transitions. "
                          "Recent variants like Mamba achieve linear complexity for sequence modeling.",
            "properties": {"complexity": "O(n)"}
        },
        {
            "name": "Mamba",
            "entity_type": "concept",
            "description": "Selective state space model with input-dependent dynamics. "
                          "Achieves competitive performance with transformers at linear complexity.",
            "properties": {"year_introduced": 2023, "authors": ["Albert Gu", "Tri Dao"]}
        },
        {
            "name": "Flash Attention",
            "entity_type": "concept",
            "description": "IO-aware algorithm for computing exact attention with reduced memory footprint. "
                          "Enables longer context windows in transformer models.",
            "properties": {"year_introduced": 2022}
        },

        # Papers
        {
            "name": "Attention Is All You Need",
            "entity_type": "paper",
            "description": "Seminal paper introducing the Transformer architecture. "
                          "Replaced RNNs with self-attention for sequence transduction.",
            "properties": {"year": 2017, "venue": "NeurIPS", "citations": 100000}
        },
        {
            "name": "Mamba: Linear-Time Sequence Modeling with Selective State Spaces",
            "entity_type": "paper",
            "description": "Paper introducing Mamba architecture with selective state spaces "
                          "and hardware-efficient implementation.",
            "properties": {"year": 2023, "arxiv_id": "2312.00752"}
        },

        # People
        {
            "name": "Ashish Vaswani",
            "entity_type": "person",
            "description": "Lead author of the Transformer paper. Former Google researcher.",
            "properties": {"affiliation": "Essential AI"}
        },
        {
            "name": "Albert Gu",
            "entity_type": "person",
            "description": "Researcher known for work on state space models and Mamba architecture.",
            "properties": {"affiliation": "CMU / Cartesia AI"}
        },
    ]

    created = []
    for entity in entities:
        result = db.create_entity(**entity)
        created.append(result)
        print(f"  Created entity: {result['name']} ({result['id'][:8]}...)")

    return {e["name"]: e["id"] for e in created}


def seed_relations(db: EntityDatabase, entity_ids: dict):
    """Create relationships between entities."""
    print("\nSeeding relations...")

    relations = [
        # Concept relations
        ("Transformer", "based_on", "Self-Attention"),
        ("State Space Model", "alternative_to", "Transformer"),
        ("Mamba", "implements", "State Space Model"),
        ("Flash Attention", "optimizes", "Self-Attention"),

        # Paper relations
        ("Attention Is All You Need", "introduces", "Transformer"),
        ("Mamba: Linear-Time Sequence Modeling with Selective State Spaces", "introduces", "Mamba"),

        # Author relations
        ("Ashish Vaswani", "authored", "Attention Is All You Need"),
        ("Albert Gu", "authored", "Mamba: Linear-Time Sequence Modeling with Selective State Spaces"),
    ]

    for subject_name, predicate, object_name in relations:
        subject_id = entity_ids.get(subject_name)
        object_id = entity_ids.get(object_name)
        if subject_id and object_id:
            result = db.create_relation(subject_id, predicate, object_id)
            if result:
                print(f"  {subject_name} --[{predicate}]--> {object_name}")


def seed_categories(db: CategoryDatabase):
    """Create category taxonomy."""
    print("\nSeeding categories...")

    # Root categories
    ml = db.create_category(
        name="Machine Learning",
        description="Algorithms and models that learn from data"
    )
    print(f"  Created: {ml['name']}")

    nlp = db.create_category(
        name="Natural Language Processing",
        description="Processing and understanding human language",
        parent_id=ml["id"]
    )
    print(f"  Created: {nlp['name']} (under ML)")

    cv = db.create_category(
        name="Computer Vision",
        description="Understanding and processing visual information",
        parent_id=ml["id"]
    )
    print(f"  Created: {cv['name']} (under ML)")

    # Architectures
    arch = db.create_category(
        name="Neural Architectures",
        description="Fundamental neural network architectures",
        parent_id=ml["id"]
    )
    print(f"  Created: {arch['name']} (under ML)")

    transformers = db.create_category(
        name="Transformers",
        description="Attention-based sequence models",
        parent_id=arch["id"]
    )
    print(f"  Created: {transformers['name']} (under Architectures)")

    ssm = db.create_category(
        name="State Space Models",
        description="Sequence models with latent state dynamics",
        parent_id=arch["id"]
    )
    print(f"  Created: {ssm['name']} (under Architectures)")

    attention = db.create_category(
        name="Attention Mechanisms",
        description="Methods for computing relevance between positions",
        parent_id=transformers["id"]
    )
    print(f"  Created: {attention['name']} (under Transformers)")

    # Optimization
    opt = db.create_category(
        name="Optimization",
        description="Methods for training and improving models",
        parent_id=ml["id"]
    )
    print(f"  Created: {opt['name']} (under ML)")

    efficiency = db.create_category(
        name="Efficient ML",
        description="Techniques for reducing computational requirements",
        parent_id=opt["id"]
    )
    print(f"  Created: {efficiency['name']} (under Optimization)")

    return {
        "machine_learning": ml["id"],
        "transformers": transformers["id"],
        "ssm": ssm["id"],
        "attention": attention["id"],
        "efficiency": efficiency["id"],
    }


def seed_content(db: TagDatabase, category_ids: dict):
    """Create sample tagged content."""
    print("\nSeeding content...")

    content_items = [
        {
            "url": "https://arxiv.org/abs/1706.03762",
            "title": "Attention Is All You Need",
            "content_type": "paper",
            "summary": "Introduces the Transformer, a model architecture based entirely on attention mechanisms.",
            "source": "arxiv",
            "category_id": category_ids["transformers"],
        },
        {
            "url": "https://arxiv.org/abs/2312.00752",
            "title": "Mamba: Linear-Time Sequence Modeling",
            "content_type": "paper",
            "summary": "Presents Mamba, a selective state space model achieving linear complexity for sequence modeling.",
            "source": "arxiv",
            "category_id": category_ids["ssm"],
        },
        {
            "url": "https://github.com/state-spaces/mamba",
            "title": "Mamba Official Implementation",
            "content_type": "repo",
            "summary": "Official PyTorch implementation of Mamba selective state space models.",
            "source": "github",
            "category_id": category_ids["ssm"],
        },
        {
            "url": "https://arxiv.org/abs/2205.14135",
            "title": "FlashAttention: Fast and Memory-Efficient Exact Attention",
            "content_type": "paper",
            "summary": "IO-aware algorithm for computing exact attention with sub-quadratic memory.",
            "source": "arxiv",
            "category_id": category_ids["efficiency"],
        },
        {
            "url": "https://huggingface.co/state-spaces/mamba-2.8b",
            "title": "Mamba 2.8B Model",
            "content_type": "model",
            "summary": "Pretrained Mamba language model with 2.8 billion parameters.",
            "source": "huggingface",
            "category_id": category_ids["ssm"],
        },
    ]

    for item in content_items:
        category_id = item.pop("category_id")
        result = db.create_or_update_content(**item)
        print(f"  Registered: {item['title']}")

        # Tag with category
        if result.get("id"):
            db.tag_content(result["id"], category_id, confidence=1.0, added_by="seed")
            print(f"    Tagged with category")


def main():
    print("=" * 60)
    print("Research Assistant Demo - Database Seeding")
    print("=" * 60)

    data_dir = Path(__file__).parent

    # Initialize databases
    entity_db = EntityDatabase(data_dir)
    category_db = CategoryDatabase(data_dir)
    tag_db = TagDatabase(data_dir)

    # Seed data
    entity_ids = seed_entities(entity_db)
    seed_relations(entity_db, entity_ids)
    category_ids = seed_categories(category_db)
    seed_content(tag_db, category_ids)

    print("\n" + "=" * 60)
    print("Seeding complete!")
    print("=" * 60)


if __name__ == "__main__":
    main()
