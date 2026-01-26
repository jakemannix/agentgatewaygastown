"""Entity Service - Subject-relation-object store with vector search.

This service manages a knowledge graph of entities and their relationships.
Uses sqlite-vec for semantic search over entity descriptions.
"""

from typing import Annotated

from mcp.server.fastmcp import FastMCP
from pydantic import Field

from .database import get_db

mcp = FastMCP(
    name="entity-service",
    instructions="Knowledge graph service for entities and relations with semantic search",
)


@mcp.tool()
def entity_search(
    query: Annotated[str, Field(description="Search query for finding relevant entities")],
    entity_type: Annotated[str | None, Field(description="Filter by entity type (e.g., 'concept', 'paper', 'person')")] = None,
    limit: Annotated[int, Field(description="Maximum number of results", ge=1, le=50)] = 10,
) -> dict:
    """Search for entities using semantic similarity.

    Finds entities whose descriptions are semantically similar to the query.
    Uses vector embeddings for accurate semantic matching.
    """
    db = get_db()
    results = db.search_entities(query, entity_type=entity_type, limit=limit)

    return {
        "query": query,
        "entity_type_filter": entity_type,
        "total": len(results),
        "entities": results,
    }


@mcp.tool()
def get_entity(
    entity_id: Annotated[str, Field(description="The entity ID to retrieve")],
    include_relations: Annotated[bool, Field(description="Include the entity's relations")] = True,
) -> dict:
    """Get an entity by its ID.

    Retrieves full details of an entity, optionally including all its relations.
    """
    db = get_db()
    entity = db.get_entity(entity_id)

    if not entity:
        return {"error": f"Entity not found: {entity_id}", "found": False}

    result = {"found": True, "entity": entity}

    if include_relations:
        result["relations"] = db.get_relations(entity_id)

    return result


@mcp.tool()
def create_entity(
    name: Annotated[str, Field(description="Name of the entity")],
    entity_type: Annotated[str, Field(description="Type of entity (e.g., 'concept', 'paper', 'person', 'organization')")],
    description: Annotated[str | None, Field(description="Detailed description of the entity")] = None,
    properties: Annotated[dict | None, Field(description="Additional properties as key-value pairs")] = None,
) -> dict:
    """Create a new entity in the knowledge graph.

    Creates an entity with the given name, type, and optional description.
    The description is embedded for semantic search.
    """
    db = get_db()
    entity = db.create_entity(
        name=name,
        entity_type=entity_type,
        description=description,
        properties=properties,
    )

    return {"success": True, "entity": entity}


@mcp.tool()
def update_entity(
    entity_id: Annotated[str, Field(description="ID of the entity to update")],
    name: Annotated[str | None, Field(description="New name for the entity")] = None,
    description: Annotated[str | None, Field(description="New description for the entity")] = None,
    properties: Annotated[dict | None, Field(description="New or updated properties")] = None,
) -> dict:
    """Update an existing entity.

    Updates the specified fields of an entity. Only provided fields are updated.
    """
    db = get_db()
    entity = db.update_entity(
        entity_id=entity_id,
        name=name,
        description=description,
        properties=properties,
    )

    if not entity:
        return {"success": False, "error": f"Entity not found: {entity_id}"}

    return {"success": True, "entity": entity}


@mcp.tool()
def delete_entity(
    entity_id: Annotated[str, Field(description="ID of the entity to delete")],
) -> dict:
    """Delete an entity and all its relations.

    Permanently removes an entity and any relations it's involved in.
    """
    db = get_db()
    success = db.delete_entity(entity_id)

    return {
        "success": success,
        "message": "Entity deleted" if success else f"Entity not found: {entity_id}",
    }


@mcp.tool()
def search_relations(
    entity_id: Annotated[str, Field(description="Entity ID to find relations for")],
    direction: Annotated[str, Field(description="Relation direction: 'outgoing', 'incoming', or 'both'")] = "both",
    predicate: Annotated[str | None, Field(description="Filter by relation type (e.g., 'related_to', 'authored_by')")] = None,
) -> dict:
    """Find relations for an entity.

    Returns all relations where the entity is either subject (outgoing)
    or object (incoming), optionally filtered by predicate.
    """
    db = get_db()

    # Validate entity exists
    entity = db.get_entity(entity_id)
    if not entity:
        return {"error": f"Entity not found: {entity_id}", "found": False}

    relations = db.get_relations(entity_id, direction=direction, predicate=predicate)

    return {
        "entity_id": entity_id,
        "entity_name": entity["name"],
        "direction": direction,
        "predicate_filter": predicate,
        "total": len(relations),
        "relations": relations,
    }


@mcp.tool()
def create_relation(
    subject_id: Annotated[str, Field(description="ID of the subject entity (source)")],
    predicate: Annotated[str, Field(description="Relation type (e.g., 'related_to', 'part_of', 'authored_by')")],
    object_id: Annotated[str, Field(description="ID of the object entity (target)")],
    properties: Annotated[dict | None, Field(description="Additional properties for the relation")] = None,
    confidence: Annotated[float, Field(description="Confidence score for the relation", ge=0.0, le=1.0)] = 1.0,
) -> dict:
    """Create a relation between two entities.

    Creates a directed relation from subject to object with the given predicate.
    Example: (Transformers, "introduced_by", Attention Paper)
    """
    db = get_db()
    relation = db.create_relation(
        subject_id=subject_id,
        predicate=predicate,
        object_id=object_id,
        properties=properties,
        confidence=confidence,
    )

    if not relation:
        return {"success": False, "error": "One or both entities not found"}

    return {"success": True, "relation": relation}


@mcp.tool()
def delete_relation(
    relation_id: Annotated[str, Field(description="ID of the relation to delete")],
) -> dict:
    """Delete a relation between entities.

    Removes a specific relation without affecting the entities.
    """
    db = get_db()
    success = db.delete_relation(relation_id)

    return {
        "success": success,
        "message": "Relation deleted" if success else f"Relation not found: {relation_id}",
    }


@mcp.tool()
def list_entity_types() -> dict:
    """List all entity types in the knowledge graph.

    Returns the distinct types of entities that have been created.
    """
    db = get_db()
    types = db.list_entity_types()

    return {"entity_types": types, "total": len(types)}


@mcp.tool()
def list_predicates() -> dict:
    """List all relation predicates in the knowledge graph.

    Returns the distinct types of relations that have been created.
    """
    db = get_db()
    predicates = db.list_predicates()

    return {"predicates": predicates, "total": len(predicates)}


if __name__ == "__main__":
    import sys
    import os
    sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
    from mcp_tools.shared.http_runner import run_http_server
    run_http_server(mcp, default_port=8003)
