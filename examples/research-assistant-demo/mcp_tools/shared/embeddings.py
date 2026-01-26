"""Shared embedding utilities using sentence-transformers."""

import functools
from typing import Any

import numpy as np


@functools.lru_cache(maxsize=1)
def get_embedding_model():
    """Get the shared embedding model (cached)."""
    from sentence_transformers import SentenceTransformer

    # Using a small, fast model for local dev
    return SentenceTransformer("all-MiniLM-L6-v2")


def embed_text(text: str) -> list[float]:
    """Generate an embedding for a single text."""
    model = get_embedding_model()
    embedding = model.encode(text, convert_to_numpy=True)
    return embedding.tolist()


def embed_texts(texts: list[str]) -> list[list[float]]:
    """Generate embeddings for multiple texts."""
    model = get_embedding_model()
    embeddings = model.encode(texts, convert_to_numpy=True)
    return embeddings.tolist()


def cosine_similarity(a: list[float], b: list[float]) -> float:
    """Compute cosine similarity between two vectors."""
    a_arr = np.array(a)
    b_arr = np.array(b)
    return float(np.dot(a_arr, b_arr) / (np.linalg.norm(a_arr) * np.linalg.norm(b_arr)))


def serialize_embedding(embedding: list[float]) -> bytes:
    """Serialize an embedding for sqlite-vec storage."""
    return np.array(embedding, dtype=np.float32).tobytes()
