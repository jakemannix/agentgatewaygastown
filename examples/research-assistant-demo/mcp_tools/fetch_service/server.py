"""Fetch Service - URL fetching and content extraction.

This service handles fetching web content and extracting URLs from text.
Designed to be composed with search services for deep research workflows.
"""

import re
from typing import Annotated
from urllib.parse import urljoin, urlparse

import httpx
from bs4 import BeautifulSoup
from mcp.server.fastmcp import FastMCP
from pydantic import Field

mcp = FastMCP(
    name="fetch-service",
    instructions="Service for fetching URL content and extracting URLs from text",
)


# URL regex pattern for extraction
URL_PATTERN = re.compile(
    r'https?://(?:[-\w.]|(?:%[\da-fA-F]{2}))+(?:/(?:[-\w._~:/?#\[\]@!$&\'()*+,;=%])*)?',
    re.IGNORECASE
)


@mcp.tool()
async def url_fetch(
    url: Annotated[str, Field(description="The URL to fetch")],
    extract_text: Annotated[bool, Field(description="Extract text content (removes HTML)")] = True,
    max_length: Annotated[int, Field(description="Maximum content length to return", ge=100, le=100000)] = 10000,
    include_metadata: Annotated[bool, Field(description="Include page metadata (title, description, links)")] = True,
) -> dict:
    """Fetch content from a URL.

    Retrieves the content at the given URL, optionally extracting clean text
    and metadata. Useful for reading documentation, papers, and web pages.
    """
    result = {
        "url": url,
        "success": False,
        "content": "",
        "metadata": {},
    }

    try:
        async with httpx.AsyncClient(follow_redirects=True) as client:
            response = await client.get(
                url,
                headers={
                    "User-Agent": "Mozilla/5.0 (compatible; ResearchBot/1.0; +https://example.com/bot)",
                    "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
                },
                timeout=30.0,
            )
            response.raise_for_status()

            content_type = response.headers.get("content-type", "")
            result["metadata"]["content_type"] = content_type
            result["metadata"]["status_code"] = response.status_code

            raw_content = response.text

            if "html" in content_type.lower() and extract_text:
                # Parse HTML and extract text
                soup = BeautifulSoup(raw_content, "html.parser")

                # Remove script and style elements
                for element in soup(["script", "style", "nav", "footer", "header"]):
                    element.decompose()

                if include_metadata:
                    # Extract title
                    title = soup.find("title")
                    result["metadata"]["title"] = title.get_text(strip=True) if title else ""

                    # Extract meta description
                    meta_desc = soup.find("meta", attrs={"name": "description"})
                    if meta_desc:
                        result["metadata"]["description"] = meta_desc.get("content", "")

                    # Extract all links
                    links = []
                    for link in soup.find_all("a", href=True):
                        href = link["href"]
                        if href.startswith("/"):
                            href = urljoin(url, href)
                        if href.startswith("http"):
                            links.append({
                                "url": href,
                                "text": link.get_text(strip=True)[:100],
                            })
                    result["metadata"]["links"] = links[:50]  # Limit to 50 links
                    result["metadata"]["link_count"] = len(links)

                # Get text content
                text = soup.get_text(separator="\n", strip=True)
                # Clean up excessive whitespace
                text = re.sub(r'\n{3,}', '\n\n', text)
                result["content"] = text[:max_length]
            else:
                # Return raw content for non-HTML
                result["content"] = raw_content[:max_length]

            result["success"] = True
            result["metadata"]["content_length"] = len(result["content"])
            result["metadata"]["truncated"] = len(raw_content) > max_length

    except httpx.HTTPStatusError as e:
        result["error"] = f"HTTP error: {e.response.status_code}"
        result["metadata"]["status_code"] = e.response.status_code
    except httpx.TimeoutException:
        result["error"] = "Request timed out"
    except Exception as e:
        result["error"] = f"Fetch error: {str(e)}"

    return result


@mcp.tool()
async def extract_urls(
    text: Annotated[str, Field(description="Text to extract URLs from")],
    include_domains: Annotated[list[str] | None, Field(description="Only include URLs from these domains")] = None,
    exclude_domains: Annotated[list[str] | None, Field(description="Exclude URLs from these domains")] = None,
    deduplicate: Annotated[bool, Field(description="Remove duplicate URLs")] = True,
) -> dict:
    """Extract URLs from text content.

    Uses regex to find all URLs in the provided text. Can filter by domain
    to focus on specific sources (e.g., only arXiv links).
    """
    # Find all URLs
    urls = URL_PATTERN.findall(text)

    # Apply domain filtering
    filtered_urls = []
    for url in urls:
        try:
            parsed = urlparse(url)
            domain = parsed.netloc.lower()

            # Check include filter
            if include_domains:
                if not any(d.lower() in domain for d in include_domains):
                    continue

            # Check exclude filter
            if exclude_domains:
                if any(d.lower() in domain for d in exclude_domains):
                    continue

            filtered_urls.append({
                "url": url,
                "domain": domain,
                "path": parsed.path,
            })
        except Exception:
            continue

    # Deduplicate if requested
    if deduplicate:
        seen = set()
        unique_urls = []
        for u in filtered_urls:
            if u["url"] not in seen:
                seen.add(u["url"])
                unique_urls.append(u)
        filtered_urls = unique_urls

    return {
        "total": len(filtered_urls),
        "urls": filtered_urls,
    }


@mcp.tool()
async def batch_fetch(
    urls: Annotated[list[str], Field(description="List of URLs to fetch")],
    extract_text: Annotated[bool, Field(description="Extract text content (removes HTML)")] = True,
    max_length_per_url: Annotated[int, Field(description="Maximum content length per URL", ge=100, le=50000)] = 5000,
) -> dict:
    """Fetch content from multiple URLs in parallel.

    Fetches multiple URLs concurrently for efficiency. Returns results
    for each URL, with errors noted for any that fail.
    """
    import asyncio

    async def fetch_one(url: str) -> dict:
        return await url_fetch(
            url=url,
            extract_text=extract_text,
            max_length=max_length_per_url,
            include_metadata=True,
        )

    # Limit concurrency to avoid overwhelming servers
    semaphore = asyncio.Semaphore(5)

    async def fetch_with_semaphore(url: str) -> dict:
        async with semaphore:
            return await fetch_one(url)

    results = await asyncio.gather(
        *[fetch_with_semaphore(url) for url in urls],
        return_exceptions=True
    )

    fetched = []
    for url, result in zip(urls, results):
        if isinstance(result, Exception):
            fetched.append({
                "url": url,
                "success": False,
                "error": str(result),
            })
        else:
            fetched.append(result)

    success_count = sum(1 for r in fetched if r.get("success", False))

    return {
        "total": len(urls),
        "success_count": success_count,
        "failure_count": len(urls) - success_count,
        "results": fetched,
    }


if __name__ == "__main__":
    from mcp_tools.shared.http_runner import run_http_server
    run_http_server(mcp, default_port=8002)
