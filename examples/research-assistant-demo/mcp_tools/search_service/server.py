"""Search Service - External search APIs for web, arXiv, GitHub, HuggingFace.

This service provides search interfaces to multiple external sources.
Each tool returns its NATIVE API format - normalization is done declaratively
via outputTransform mappings in the gateway registry.
"""

import os
from dataclasses import dataclass, field
from typing import Annotated

import httpx
from mcp.server.fastmcp import FastMCP
from pydantic import Field

mcp = FastMCP(
    name="search-service",
    instructions="External search service for web, arXiv, GitHub, and HuggingFace",
)


# =============================================================================
# Exa Search - Native Response Types
# =============================================================================


@dataclass
class ExaResult:
    """Single result from Exa API."""
    title: str
    url: str
    text: str | None = None
    highlight: str | None = None
    score: float = 1.0
    published_date: str | None = None


@dataclass
class ExaSearchResponse:
    """Native Exa API response format."""
    query: str
    results: list[ExaResult] = field(default_factory=list)
    error: str | None = None


@mcp.tool()
async def exa_search(
    query: Annotated[str, Field(description="Search query for web search")],
    num_results: Annotated[int, Field(description="Number of results to return", ge=1, le=20)] = 10,
    use_autoprompt: Annotated[bool, Field(description="Use Exa's autoprompt feature")] = True,
) -> ExaSearchResponse:
    """Search the web using Exa API.

    Exa provides high-quality web search results optimized for AI applications.
    Requires EXA_API_KEY environment variable.
    """
    exa_api_key = os.getenv("EXA_API_KEY")

    if not exa_api_key:
        return ExaSearchResponse(
            query=query,
            error="EXA_API_KEY environment variable not set. Get an API key at https://exa.ai"
        )

    try:
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
                ExaResult(
                    title=r.get("title", ""),
                    url=r.get("url", ""),
                    text=r.get("text"),
                    highlight=r.get("highlight"),
                    score=r.get("score", 1.0),
                    published_date=r.get("publishedDate"),
                )
                for r in data.get("results", [])
            ]

            return ExaSearchResponse(query=query, results=results)

    except httpx.HTTPStatusError as e:
        return ExaSearchResponse(
            query=query,
            error=f"Exa API error: HTTP {e.response.status_code}"
        )
    except httpx.TimeoutException:
        return ExaSearchResponse(query=query, error="Exa API timeout")
    except Exception as e:
        return ExaSearchResponse(query=query, error=f"Exa API error: {e}")


# =============================================================================
# arXiv Search - Native Response Types
# =============================================================================


@dataclass
class ArxivPaper:
    """Single paper from arXiv API."""
    arxiv_id: str
    title: str
    abstract: str
    authors: list[str] = field(default_factory=list)
    categories: list[str] = field(default_factory=list)
    pdf_url: str | None = None
    abs_url: str | None = None
    published: str | None = None
    updated: str | None = None


@dataclass
class ArxivSearchResponse:
    """Native arXiv API response format."""
    query: str
    papers: list[ArxivPaper] = field(default_factory=list)
    error: str | None = None


@mcp.tool()
async def arxiv_search(
    query: Annotated[str, Field(description="Search query for arXiv papers")],
    num_results: Annotated[int, Field(description="Number of papers to return", ge=1, le=50)] = 10,
    sort_by: Annotated[str, Field(description="Sort order: relevance, lastUpdatedDate, submittedDate")] = "relevance",
) -> ArxivSearchResponse:
    """Search arXiv for academic papers.

    Searches the arXiv preprint server for papers matching the query.
    Returns paper metadata including title, authors, abstract, and PDF URL.
    No API key required - arXiv API is free.
    """
    try:
        async with httpx.AsyncClient() as client:
            search_query = query.replace(" ", "+")
            params = {
                "search_query": f"all:{search_query}",
                "start": 0,
                "max_results": num_results,
                "sortBy": sort_by,
                "sortOrder": "descending",
            }

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

            papers = []
            for entry in root.findall("atom:entry", ns):
                title_el = entry.find("atom:title", ns)
                summary_el = entry.find("atom:summary", ns)
                published_el = entry.find("atom:published", ns)
                updated_el = entry.find("atom:updated", ns)

                # Get PDF and abs links
                pdf_url = None
                abs_url = None
                for link in entry.findall("atom:link", ns):
                    if link.get("title") == "pdf":
                        pdf_url = link.get("href", "")
                    elif link.get("type") == "text/html":
                        abs_url = link.get("href", "")

                # Get authors
                authors = [
                    a.find("atom:name", ns).text
                    for a in entry.findall("atom:author", ns)
                    if a.find("atom:name", ns) is not None
                ]

                # Get arXiv ID from entry id
                id_el = entry.find("atom:id", ns)
                arxiv_id = ""
                if id_el is not None and id_el.text:
                    arxiv_id = id_el.text
                    if "abs/" in arxiv_id:
                        arxiv_id = arxiv_id.split("abs/")[-1]

                # Get categories
                categories = [
                    cat.get("term", "")
                    for cat in entry.findall("atom:category", ns)
                    if cat.get("term")
                ]

                papers.append(ArxivPaper(
                    arxiv_id=arxiv_id,
                    title=title_el.text.strip() if title_el is not None and title_el.text else "",
                    abstract=summary_el.text.strip() if summary_el is not None and summary_el.text else "",
                    authors=authors,
                    categories=categories,
                    pdf_url=pdf_url,
                    abs_url=abs_url or f"https://arxiv.org/abs/{arxiv_id}",
                    published=published_el.text if published_el is not None else None,
                    updated=updated_el.text if updated_el is not None else None,
                ))

            return ArxivSearchResponse(query=query, papers=papers)

    except httpx.HTTPStatusError as e:
        return ArxivSearchResponse(
            query=query,
            error=f"arXiv API error: HTTP {e.response.status_code}"
        )
    except httpx.TimeoutException:
        return ArxivSearchResponse(query=query, error="arXiv API timeout")
    except ET.ParseError as e:
        return ArxivSearchResponse(query=query, error=f"arXiv XML parse error: {e}")
    except Exception as e:
        return ArxivSearchResponse(query=query, error=f"arXiv API error: {e}")


# =============================================================================
# GitHub Search - Native Response Types
# =============================================================================


@dataclass
class GitHubRepo:
    """Single repository from GitHub API."""
    full_name: str
    html_url: str
    description: str | None = None
    stargazers_count: int = 0
    forks_count: int = 0
    language: str | None = None
    topics: list[str] = field(default_factory=list)
    updated_at: str | None = None


@dataclass
class GitHubCodeResult:
    """Single code search result from GitHub API."""
    name: str
    path: str
    html_url: str
    repository_full_name: str


@dataclass
class GitHubSearchResponse:
    """Native GitHub API response format."""
    query: str
    search_type: str
    total_count: int = 0
    repos: list[GitHubRepo] = field(default_factory=list)
    code_results: list[GitHubCodeResult] = field(default_factory=list)
    error: str | None = None


@mcp.tool()
async def github_search(
    query: Annotated[str, Field(description="Search query for GitHub repositories")],
    num_results: Annotated[int, Field(description="Number of repositories to return", ge=1, le=30)] = 10,
    search_type: Annotated[str, Field(description="What to search: repositories, code")] = "repositories",
) -> GitHubSearchResponse:
    """Search GitHub for repositories or code.

    Searches GitHub's public repositories for projects matching the query.
    GITHUB_TOKEN is optional but recommended to avoid rate limits.
    """
    github_token = os.getenv("GITHUB_TOKEN")
    headers = {"Accept": "application/vnd.github.v3+json"}
    if github_token:
        headers["Authorization"] = f"token {github_token}"

    try:
        async with httpx.AsyncClient() as client:
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
                repos = [
                    GitHubRepo(
                        full_name=r.get("full_name", ""),
                        html_url=r.get("html_url", ""),
                        description=r.get("description"),
                        stargazers_count=r.get("stargazers_count", 0),
                        forks_count=r.get("forks_count", 0),
                        language=r.get("language"),
                        topics=r.get("topics", [])[:5],
                        updated_at=r.get("updated_at"),
                    )
                    for r in data.get("items", [])
                ]
                return GitHubSearchResponse(
                    query=query,
                    search_type=search_type,
                    total_count=data.get("total_count", 0),
                    repos=repos,
                )
            else:
                code_results = [
                    GitHubCodeResult(
                        name=r.get("name", ""),
                        path=r.get("path", ""),
                        html_url=r.get("html_url", ""),
                        repository_full_name=r.get("repository", {}).get("full_name", ""),
                    )
                    for r in data.get("items", [])
                ]
                return GitHubSearchResponse(
                    query=query,
                    search_type=search_type,
                    total_count=data.get("total_count", 0),
                    code_results=code_results,
                )

    except httpx.HTTPStatusError as e:
        error_msg = f"GitHub API error: HTTP {e.response.status_code}"
        if e.response.status_code == 403:
            error_msg += " (rate limited - set GITHUB_TOKEN to increase limits)"
        return GitHubSearchResponse(
            query=query, search_type=search_type, error=error_msg
        )
    except httpx.TimeoutException:
        return GitHubSearchResponse(
            query=query, search_type=search_type, error="GitHub API timeout"
        )
    except Exception as e:
        return GitHubSearchResponse(
            query=query, search_type=search_type, error=f"GitHub API error: {e}"
        )


# =============================================================================
# HuggingFace Search - Native Response Types
# =============================================================================


@dataclass
class HuggingFaceModel:
    """Single model from HuggingFace API."""
    id: str
    url: str
    description: str | None = None
    downloads: int = 0
    likes: int = 0
    pipeline_tag: str | None = None
    tags: list[str] = field(default_factory=list)
    library_name: str | None = None


@dataclass
class HuggingFaceDataset:
    """Single dataset from HuggingFace API."""
    id: str
    url: str
    description: str | None = None
    downloads: int = 0
    likes: int = 0
    tags: list[str] = field(default_factory=list)


@dataclass
class HuggingFaceSpace:
    """Single space from HuggingFace API."""
    id: str
    url: str
    description: str | None = None
    likes: int = 0
    sdk: str | None = None


@dataclass
class HuggingFaceSearchResponse:
    """Native HuggingFace API response format."""
    query: str
    search_type: str
    models: list[HuggingFaceModel] = field(default_factory=list)
    datasets: list[HuggingFaceDataset] = field(default_factory=list)
    spaces: list[HuggingFaceSpace] = field(default_factory=list)
    error: str | None = None


@mcp.tool()
async def huggingface_search(
    query: Annotated[str, Field(description="Search query for HuggingFace")],
    num_results: Annotated[int, Field(description="Number of results to return", ge=1, le=30)] = 10,
    search_type: Annotated[str, Field(description="What to search: models, datasets, spaces")] = "models",
) -> HuggingFaceSearchResponse:
    """Search HuggingFace for models, datasets, or spaces.

    Searches the HuggingFace Hub for ML models, datasets, or Spaces.
    HF_TOKEN is optional but recommended to avoid rate limits.
    """
    hf_token = os.getenv("HF_TOKEN") or os.getenv("HUGGINGFACE_TOKEN")
    headers = {"Accept": "application/json"}
    if hf_token:
        headers["Authorization"] = f"Bearer {hf_token}"

    try:
        async with httpx.AsyncClient() as client:
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

            if search_type == "models":
                models = [
                    HuggingFaceModel(
                        id=item.get("id", ""),
                        url=f"https://huggingface.co/{item.get('id', '')}",
                        description=item.get("description") or item.get("pipeline_tag"),
                        downloads=item.get("downloads", 0),
                        likes=item.get("likes", 0),
                        pipeline_tag=item.get("pipeline_tag"),
                        tags=item.get("tags", [])[:5],
                        library_name=item.get("library_name"),
                    )
                    for item in data
                ]
                return HuggingFaceSearchResponse(
                    query=query, search_type=search_type, models=models
                )

            elif search_type == "datasets":
                datasets = [
                    HuggingFaceDataset(
                        id=item.get("id", ""),
                        url=f"https://huggingface.co/datasets/{item.get('id', '')}",
                        description=item.get("description"),
                        downloads=item.get("downloads", 0),
                        likes=item.get("likes", 0),
                        tags=item.get("tags", [])[:5],
                    )
                    for item in data
                ]
                return HuggingFaceSearchResponse(
                    query=query, search_type=search_type, datasets=datasets
                )

            else:  # spaces
                spaces = [
                    HuggingFaceSpace(
                        id=item.get("id", ""),
                        url=f"https://huggingface.co/spaces/{item.get('id', '')}",
                        description=item.get("description"),
                        likes=item.get("likes", 0),
                        sdk=item.get("sdk"),
                    )
                    for item in data
                ]
                return HuggingFaceSearchResponse(
                    query=query, search_type=search_type, spaces=spaces
                )

    except httpx.HTTPStatusError as e:
        return HuggingFaceSearchResponse(
            query=query,
            search_type=search_type,
            error=f"HuggingFace API error: HTTP {e.response.status_code}"
        )
    except httpx.TimeoutException:
        return HuggingFaceSearchResponse(
            query=query, search_type=search_type, error="HuggingFace API timeout"
        )
    except Exception as e:
        return HuggingFaceSearchResponse(
            query=query, search_type=search_type, error=f"HuggingFace API error: {e}"
        )


if __name__ == "__main__":
    from mcp_tools.shared.http_runner import run_http_server
    run_http_server(mcp, default_port=8001)
