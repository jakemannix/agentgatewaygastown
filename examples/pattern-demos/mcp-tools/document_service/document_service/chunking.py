"""Document chunking utilities."""

from __future__ import annotations

import re
import uuid
from dataclasses import dataclass


@dataclass
class ChunkConfig:
    """Configuration for chunking."""

    chunk_size: int = 500
    chunk_overlap: int = 50
    separator: str = "\n\n"
    fallback_separators: tuple[str, ...] = ("\n", ". ", " ")


def chunk_text(
    text: str,
    chunk_size: int = 500,
    chunk_overlap: int = 50,
    separator: str = "\n\n",
    fallback_separators: tuple[str, ...] = ("\n", ". ", " "),
) -> list[str]:
    """Split text into chunks with overlap.

    Uses a hierarchical splitting strategy:
    1. First tries to split on the primary separator (e.g., paragraphs)
    2. If chunks are too large, recursively splits on fallback separators
    3. Finally falls back to character-level splitting if needed

    Args:
        text: The text to chunk.
        chunk_size: Target size for each chunk in characters.
        chunk_overlap: Number of characters to overlap between chunks.
        separator: Primary separator to split on.
        fallback_separators: Fallback separators to try if chunks are too large.

    Returns:
        List of text chunks.
    """
    if len(text) <= chunk_size:
        return [text] if text.strip() else []

    # Split on primary separator
    splits = text.split(separator)
    splits = [s for s in splits if s.strip()]

    if not splits:
        return _split_by_characters(text, chunk_size, chunk_overlap)

    chunks = []
    current_chunk = ""

    for split in splits:
        # If this split alone is too large, recursively chunk it
        if len(split) > chunk_size:
            # Flush current chunk first
            if current_chunk.strip():
                chunks.append(current_chunk.strip())
                current_chunk = ""

            # Try fallback separators
            if fallback_separators:
                sub_chunks = chunk_text(
                    split,
                    chunk_size=chunk_size,
                    chunk_overlap=chunk_overlap,
                    separator=fallback_separators[0],
                    fallback_separators=fallback_separators[1:],
                )
                chunks.extend(sub_chunks)
            else:
                # Final fallback: character-level splitting
                sub_chunks = _split_by_characters(split, chunk_size, chunk_overlap)
                chunks.extend(sub_chunks)
            continue

        # Check if adding this split would exceed chunk size
        potential = current_chunk + separator + split if current_chunk else split
        if len(potential) <= chunk_size:
            current_chunk = potential
        else:
            # Flush current chunk and start new one with overlap
            if current_chunk.strip():
                chunks.append(current_chunk.strip())
                # Add overlap from end of current chunk
                overlap_text = _get_overlap(current_chunk, chunk_overlap)
                current_chunk = overlap_text + separator + split if overlap_text else split
            else:
                current_chunk = split

    # Don't forget the last chunk
    if current_chunk.strip():
        chunks.append(current_chunk.strip())

    return chunks


def _split_by_characters(
    text: str, chunk_size: int, chunk_overlap: int
) -> list[str]:
    """Split text by characters with overlap.

    Args:
        text: The text to split.
        chunk_size: Size of each chunk.
        chunk_overlap: Overlap between chunks.

    Returns:
        List of text chunks.
    """
    chunks = []
    start = 0
    text_len = len(text)

    while start < text_len:
        end = min(start + chunk_size, text_len)
        chunk = text[start:end]

        # Try to end at a word boundary
        if end < text_len and not text[end].isspace():
            last_space = chunk.rfind(" ")
            if last_space > chunk_size // 2:
                end = start + last_space
                chunk = text[start:end]

        chunks.append(chunk.strip())
        start = end - chunk_overlap
        if start >= text_len - chunk_overlap:
            break

    return [c for c in chunks if c]


def _get_overlap(text: str, overlap_size: int) -> str:
    """Get the overlap text from the end of a string.

    Args:
        text: The source text.
        overlap_size: Desired overlap size.

    Returns:
        The overlap text.
    """
    if len(text) <= overlap_size:
        return text

    overlap = text[-overlap_size:]
    # Try to start at a word boundary
    first_space = overlap.find(" ")
    if first_space > 0 and first_space < len(overlap) // 2:
        overlap = overlap[first_space + 1:]

    return overlap


def generate_chunk_id(document_id: str, chunk_index: int) -> str:
    """Generate a unique chunk ID.

    Args:
        document_id: The parent document ID.
        chunk_index: The index of the chunk.

    Returns:
        A unique chunk ID.
    """
    return f"{document_id}_chunk_{chunk_index}"
