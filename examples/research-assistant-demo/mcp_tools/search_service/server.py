"""Search Service - External search APIs for web, arXiv, GitHub, HuggingFace.

This service provides unified search interfaces across multiple external sources.
Each search tool returns a normalized result format for easy composition.
"""

import os
import re
from datetime import datetime
from typing import Annotated

import httpx
from mcp.server.fastmcp import FastMCP
from pydantic import Field

mcp = FastMCP(
    name="search-service",
    instructions="External search service for web, arXiv, GitHub, and HuggingFace",
)


# Normalized result schema (all search tools return this format)
def normalize_result(
    source: str,
    title: str,
    url: str,
    snippet: str,
    score: float = 1.0,
    metadata: dict | None = None,
) -> dict:
    """Create a normalized search result."""
    return {
        "source": source,
        "title": title,
        "url": url,
        "snippet": snippet,
        "score": score,
        "metadata": metadata or {},
        "timestamp": datetime.utcnow().isoformat(),
    }


@mcp.tool()
async def exa_search(
    query: Annotated[str, Field(description="Search query for web search")],
    num_results: Annotated[int, Field(description="Number of results to return", ge=1, le=20)] = 10,
    use_autoprompt: Annotated[bool, Field(description="Use Exa's autoprompt feature")] = True,
) -> dict:
    """Search the web using Exa API.

    Exa provides high-quality web search results optimized for AI applications.
    Returns normalized search results with URLs, titles, and snippets.
    """
    exa_api_key = os.getenv("EXA_API_KEY")

    if exa_api_key:
        # Real Exa API call
        async with httpx.AsyncClient() as client:
            response = await client.post(
                "https://api.exa.ai/search",
                headers={"Authorization": f"Bearer {exa_api_key}"},
                json={
                    "query": query,
                    "numResults": num_results,
                    "useAutoprompt": use_autoprompt,
                    "type": "neural",
                },
                timeout=30.0,
            )
            response.raise_for_status()
            data = response.json()

            results = [
                normalize_result(
                    source="exa",
                    title=r.get("title", ""),
                    url=r.get("url", ""),
                    snippet=r.get("text", r.get("highlight", ""))[:500],
                    score=r.get("score", 1.0),
                    metadata={"published_date": r.get("publishedDate")},
                )
                for r in data.get("results", [])
            ]
    else:
        # Mock results for demo
        results = [
            normalize_result(
                source="exa",
                title=f"Web Result: {query} - Article {i+1}",
                url=f"https://example.com/article/{query.replace(' ', '-')}-{i+1}",
                snippet=f"This is a simulated web search result for '{query}'. "
                        f"In production, this would contain actual content from the web.",
                score=0.9 - (i * 0.05),
                metadata={"published_date": "2025-01-15"},
            )
            for i in range(min(num_results, 5))
        ]

    return {
        "query": query,
        "source": "exa",
        "total": len(results),
        "results": results,
    }


@mcp.tool()
async def arxiv_search(
    query: Annotated[str, Field(description="Search query for arXiv papers")],
    num_results: Annotated[int, Field(description="Number of papers to return", ge=1, le=50)] = 10,
    sort_by: Annotated[str, Field(description="Sort order: relevance, lastUpdatedDate, submittedDate")] = "relevance",
) -> dict:
    """Search arXiv for academic papers.

    Searches the arXiv preprint server for papers matching the query.
    Returns paper metadata including title, authors, abstract, and PDF URL.
    """
    # arXiv API is free and doesn't require a key
    async with httpx.AsyncClient() as client:
        # Build arXiv API query
        search_query = query.replace(" ", "+")
        params = {
            "search_query": f"all:{search_query}",
            "start": 0,
            "max_results": num_results,
            "sortBy": sort_by,
            "sortOrder": "descending",
        }

        try:
            response = await client.get(
                "http://export.arxiv.org/api/query",
                params=params,
                timeout=30.0,
            )
            response.raise_for_status()

            # Parse Atom XML response
            from xml.etree import ElementTree as ET

            root = ET.fromstring(response.text)
            ns = {"atom": "http://www.w3.org/2005/Atom", "arxiv": "http://arxiv.org/schemas/atom"}

            results = []
            for entry in root.findall("atom:entry", ns):
                title = entry.find("atom:title", ns)
                summary = entry.find("atom:summary", ns)
                published = entry.find("atom:published", ns)

                # Get PDF link
                pdf_url = ""
                for link in entry.findall("atom:link", ns):
                    if link.get("title") == "pdf":
                        pdf_url = link.get("href", "")
                        break

                # Get authors
                authors = [
                    a.find("atom:name", ns).text
                    for a in entry.findall("atom:author", ns)
                    if a.find("atom:name", ns) is not None
                ]

                # Get arXiv ID
                arxiv_id = entry.find("atom:id", ns)
                arxiv_id = arxiv_id.text if arxiv_id is not None else ""
                if "abs/" in arxiv_id:
                    arxiv_id = arxiv_id.split("abs/")[-1]

                # Get categories
                categories = [
                    cat.get("term", "")
                    for cat in entry.findall("atom:category", ns)
                ]

                results.append(normalize_result(
                    source="arxiv",
                    title=title.text.strip() if title is not None else "",
                    url=pdf_url or f"https://arxiv.org/abs/{arxiv_id}",
                    snippet=(summary.text.strip()[:500] if summary is not None else ""),
                    score=1.0 - (len(results) * 0.02),  # Relevance-ordered
                    metadata={
                        "arxiv_id": arxiv_id,
                        "authors": authors[:5],  # Limit to first 5 authors
                        "published": published.text if published is not None else None,
                        "categories": categories[:3],
                    },
                ))

        except Exception as e:
            # Fallback to mock results on error
            results = [
                normalize_result(
                    source="arxiv",
                    title=f"[arXiv] {query} - Paper {i+1}",
                    url=f"https://arxiv.org/abs/2501.{10000+i}",
                    snippet=f"Mock arXiv paper about {query}. Error fetching real results: {e}",
                    score=0.9 - (i * 0.05),
                    metadata={"arxiv_id": f"2501.{10000+i}", "authors": ["Author A", "Author B"]},
                )
                for i in range(min(num_results, 5))
            ]

    return {
        "query": query,
        "source": "arxiv",
        "total": len(results),
        "results": results,
    }


@mcp.tool()
async def github_search(
    query: Annotated[str, Field(description="Search query for GitHub repositories")],
    num_results: Annotated[int, Field(description="Number of repositories to return", ge=1, le=30)] = 10,
    search_type: Annotated[str, Field(description="What to search: repositories, code")] = "repositories",
) -> dict:
    """Search GitHub for repositories or code.

    Searches GitHub's public repositories for projects matching the query.
    Returns repository metadata including name, description, stars, and URL.
    """
    github_token = os.getenv("GITHUB_TOKEN")
    headers = {"Accept": "application/vnd.github.v3+json"}
    if github_token:
        headers["Authorization"] = f"token {github_token}"

    async with httpx.AsyncClient() as client:
        try:
            if search_type == "repositories":
                response = await client.get(
                    "https://api.github.com/search/repositories",
                    headers=headers,
                    params={"q": query, "per_page": num_results, "sort": "stars"},
                    timeout=30.0,
                )
            else:
                response = await client.get(
                    "https://api.github.com/search/code",
                    headers=headers,
                    params={"q": query, "per_page": num_results},
                    timeout=30.0,
                )

            response.raise_for_status()
            data = response.json()

            if search_type == "repositories":
                results = [
                    normalize_result(
                        source="github",
                        title=r.get("full_name", ""),
                        url=r.get("html_url", ""),
                        snippet=r.get("description", "") or "No description",
                        score=min(1.0, r.get("stargazers_count", 0) / 10000),
                        metadata={
                            "stars": r.get("stargazers_count", 0),
                            "forks": r.get("forks_count", 0),
                            "language": r.get("language"),
                            "topics": r.get("topics", [])[:5],
                            "updated_at": r.get("updated_at"),
                        },
                    )
                    for r in data.get("items", [])
                ]
            else:
                results = [
                    normalize_result(
                        source="github",
                        title=f"{r.get('repository', {}).get('full_name', '')}/{r.get('name', '')}",
                        url=r.get("html_url", ""),
                        snippet=f"Code file in {r.get('repository', {}).get('full_name', '')}",
                        score=0.9,
                        metadata={
                            "path": r.get("path"),
                            "repository": r.get("repository", {}).get("full_name"),
                        },
                    )
                    for r in data.get("items", [])
                ]

        except Exception as e:
            # Mock results on error
            results = [
                normalize_result(
                    source="github",
                    title=f"example/{query.replace(' ', '-')}-{i+1}",
                    url=f"https://github.com/example/{query.replace(' ', '-')}-{i+1}",
                    snippet=f"Mock GitHub repository for {query}. API error: {e}",
                    score=0.9 - (i * 0.05),
                    metadata={"stars": 1000 - (i * 100), "language": "Python"},
                )
                for i in range(min(num_results, 5))
            ]

    return {
        "query": query,
        "source": "github",
        "total": len(results),
        "results": results,
    }


@mcp.tool()
async def huggingface_search(
    query: Annotated[str, Field(description="Search query for HuggingFace")],
    num_results: Annotated[int, Field(description="Number of results to return", ge=1, le=30)] = 10,
    search_type: Annotated[str, Field(description="What to search: models, datasets, spaces")] = "models",
) -> dict:
    """Search HuggingFace for models, datasets, or spaces.

    Searches the HuggingFace Hub for ML models, datasets, or Spaces matching the query.
    Returns metadata including name, description, downloads, and URL.
    """
    hf_token = os.getenv("HF_TOKEN") or os.getenv("HUGGINGFACE_TOKEN")
    headers = {"Accept": "application/json"}
    if hf_token:
        headers["Authorization"] = f"Bearer {hf_token}"

    async with httpx.AsyncClient() as client:
        try:
            # HuggingFace API endpoint
            base_url = f"https://huggingface.co/api/{search_type}"
            params = {"search": query, "limit": num_results, "sort": "downloads", "direction": "-1"}

            response = await client.get(
                base_url,
                headers=headers,
                params=params,
                timeout=30.0,
            )
            response.raise_for_status()
            data = response.json()

            results = []
            for item in data:
                if search_type == "models":
                    results.append(normalize_result(
                        source="huggingface",
                        title=item.get("id", ""),
                        url=f"https://huggingface.co/{item.get('id', '')}",
                        snippet=item.get("description", item.get("pipeline_tag", "")) or "ML model",
                        score=min(1.0, item.get("downloads", 0) / 1000000),
                        metadata={
                            "downloads": item.get("downloads", 0),
                            "likes": item.get("likes", 0),
                            "pipeline_tag": item.get("pipeline_tag"),
                            "tags": item.get("tags", [])[:5],
                            "library_name": item.get("library_name"),
                        },
                    ))
                elif search_type == "datasets":
                    results.append(normalize_result(
                        source="huggingface",
                        title=item.get("id", ""),
                        url=f"https://huggingface.co/datasets/{item.get('id', '')}",
                        snippet=item.get("description", "") or "Dataset",
                        score=min(1.0, item.get("downloads", 0) / 100000),
                        metadata={
                            "downloads": item.get("downloads", 0),
                            "likes": item.get("likes", 0),
                            "tags": item.get("tags", [])[:5],
                        },
                    ))
                else:  # spaces
                    results.append(normalize_result(
                        source="huggingface",
                        title=item.get("id", ""),
                        url=f"https://huggingface.co/spaces/{item.get('id', '')}",
                        snippet=item.get("description", "") or "HuggingFace Space",
                        score=item.get("likes", 0) / 1000,
                        metadata={
                            "likes": item.get("likes", 0),
                            "sdk": item.get("sdk"),
                        },
                    ))

        except Exception as e:
            # Mock results on error
            results = [
                normalize_result(
                    source="huggingface",
                    title=f"org/{query.replace(' ', '-')}-{search_type[:-1]}-{i+1}",
                    url=f"https://huggingface.co/{search_type}/org/{query.replace(' ', '-')}-{i+1}",
                    snippet=f"Mock HuggingFace {search_type[:-1]} for {query}. API error: {e}",
                    score=0.9 - (i * 0.05),
                    metadata={"downloads": 10000 - (i * 1000)},
                )
                for i in range(min(num_results, 5))
            ]

    return {
        "query": query,
        "source": f"huggingface_{search_type}",
        "total": len(results),
        "results": results,
    }


if __name__ == "__main__":
    from mcp_tools.shared.http_runner import run_http_server
    run_http_server(mcp, default_port=8001)
