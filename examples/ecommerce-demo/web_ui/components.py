"""Shared UI components for ecommerce demo."""

from fasthtml.common import *


def page_layout(title: str, content, nav_items: list = None, extra_head: list = None):
    """Create a page with common layout."""
    nav_items = nav_items or []
    extra_head = extra_head or []

    return Html(
        Head(
            Title(title),
            Meta(charset="utf-8"),
            Meta(name="viewport", content="width=device-width, initial-scale=1"),
            Link(rel="stylesheet", href="/static/style.css"),
            Script(src="https://unpkg.com/htmx.org@1.9.10"),
            Script(src="https://unpkg.com/htmx.org/dist/ext/json-enc.js"),
            *extra_head,
        ),
        Body(
            Header(
                Nav(
                    Div(A(title, href="/", cls="logo"), cls="nav-brand"),
                    Div(*nav_items, cls="nav-links") if nav_items else None,
                    cls="nav-container",
                ),
                cls="main-header",
            ),
            Main(content, cls="main-content"),
            Footer(
                P("eCommerce Demo - Powered by AgentGateway"),
                cls="main-footer",
            ),
        ),
    )


def product_card(product: dict, show_add_cart: bool = True, user_id: str = None):
    """Render a product card."""
    in_stock = product.get("in_stock", product.get("stock_quantity", 0) > 0)
    stock_class = "in-stock" if in_stock else "out-of-stock"
    stock_text = "In Stock" if in_stock else "Out of Stock"

    return Div(
        Div(
            H3(product["name"], cls="product-name"),
            P(product.get("description", "")[:100] + "..." if len(product.get("description", "")) > 100 else product.get("description", ""), cls="product-desc"),
            cls="product-info",
        ),
        Div(
            Span(f"${product['price']:.2f}", cls="product-price"),
            Span(product.get("category", ""), cls="product-category"),
            cls="product-meta",
        ),
        Div(
            Span(stock_text, cls=f"stock-badge {stock_class}"),
            Button(
                "Add to Cart",
                hx_post=f"/api/cart/add?user_id={user_id}&product_id={product['id']}",
                hx_target="#cart-count",
                hx_swap="innerHTML",
                cls="btn btn-primary",
                disabled=not in_stock,
            ) if show_add_cart and user_id else None,
            cls="product-actions",
        ),
        A("View Details", href=f"/product/{product['id']}", cls="product-link"),
        cls="product-card",
        id=f"product-{product['id']}",
    )


def cart_item_row(item: dict):
    """Render a cart item row."""
    return Tr(
        Td(item.get("product_name", "Unknown")),
        Td(f"${item.get('product_price', 0):.2f}"),
        Td(
            Input(
                type="number",
                value=str(item["quantity"]),
                min="0",
                max="99",
                hx_post=f"/api/cart/update/{item['id']}",
                hx_trigger="change",
                hx_target="#cart-table",
                hx_swap="outerHTML",
                name="quantity",
                cls="quantity-input",
            ),
        ),
        Td(f"${item.get('line_total', 0):.2f}"),
        Td(
            Button(
                "Remove",
                hx_delete=f"/api/cart/remove/{item['id']}",
                hx_target="#cart-table",
                hx_swap="outerHTML",
                cls="btn btn-danger btn-sm",
            ),
        ),
        id=f"cart-item-{item['id']}",
    )


def order_card(order: dict):
    """Render an order card."""
    status_class = f"status-{order['status']}"

    return Div(
        Div(
            H4(f"Order #{order['id'][:8]}..."),
            Span(order["status"].title(), cls=f"order-status {status_class}"),
            cls="order-header",
        ),
        Div(
            P(f"Total: ${order['total']:.2f}"),
            P(f"Items: {len(order.get('items', []))}"),
            P(f"Date: {order.get('created_at', 'N/A')[:10]}"),
            cls="order-details",
        ),
        A("View Details", href=f"/order/{order['id']}", cls="btn btn-secondary"),
        cls="order-card",
    )


def inventory_row(product: dict):
    """Render an inventory table row."""
    stock = product.get("stock_quantity", 0)
    threshold = product.get("reorder_threshold", 10)
    status_class = "low-stock" if stock < threshold else "ok"
    if stock == 0:
        status_class = "out-of-stock"

    return Tr(
        Td(product["name"]),
        Td(product.get("category", "N/A")),
        Td(str(stock), cls=f"stock-cell {status_class}"),
        Td(str(threshold)),
        Td(f"${product.get('cost', 0):.2f}"),
        Td(f"${stock * product.get('cost', 0):.2f}"),
        Td(
            Button(
                "Adjust",
                hx_get=f"/modal/adjust/{product['id']}",
                hx_target="#modal-container",
                cls="btn btn-sm",
            ),
            Button(
                "Order",
                hx_get=f"/modal/order/{product['id']}",
                hx_target="#modal-container",
                cls="btn btn-sm btn-primary",
            ) if stock < threshold else None,
        ),
        cls=status_class,
    )


def purchase_order_row(po: dict):
    """Render a purchase order table row."""
    status_class = f"po-status-{po['status']}"

    return Tr(
        Td(po["id"][:8] + "..."),
        Td(po.get("product_name", "Unknown")),
        Td(po.get("supplier_name", "Unknown")),
        Td(str(po["quantity_ordered"])),
        Td(f"${po['unit_cost']:.2f}"),
        Td(f"${po['quantity_ordered'] * po['unit_cost']:.2f}"),
        Td(Span(po["status"].title(), cls=status_class)),
        Td(po.get("expected_delivery", "N/A")[:10] if po.get("expected_delivery") else "N/A"),
        Td(
            Button(
                "Receive",
                hx_post=f"/api/po/receive/{po['id']}",
                hx_target="#po-table",
                hx_swap="outerHTML",
                cls="btn btn-sm btn-success",
            ) if po["status"] in ["pending", "confirmed", "shipped"] else None,
        ),
    )


def alert_badge(count: int, label: str, color: str = "red"):
    """Render an alert badge."""
    if count == 0:
        color = "green"

    return Div(
        Span(str(count), cls=f"badge badge-{color}"),
        Span(label, cls="badge-label"),
        cls="alert-badge",
    )


def stat_card(title: str, value: str, subtitle: str = None):
    """Render a statistics card."""
    return Div(
        H4(title, cls="stat-title"),
        P(value, cls="stat-value"),
        P(subtitle, cls="stat-subtitle") if subtitle else None,
        cls="stat-card",
    )


def modal(title: str, content, modal_id: str = "modal"):
    """Render a modal dialog."""
    return Div(
        Div(
            Div(
                H3(title),
                Button("&times;", onclick=f"document.getElementById('{modal_id}').remove()", cls="modal-close"),
                cls="modal-header",
            ),
            Div(content, cls="modal-body"),
            cls="modal-content",
        ),
        cls="modal-overlay",
        id=modal_id,
    )


def chat_panel(endpoint: str, user_id: str):
    """Render a chat panel for agent interaction."""
    return Div(
        Div(
            H4("Chat with Agent"),
            Button("&times;", onclick="toggleChat()", cls="chat-close"),
            cls="chat-header",
        ),
        Div(id="chat-messages", cls="chat-messages"),
        Form(
            Input(
                type="text",
                name="message",
                placeholder="Ask about products, orders...",
                autocomplete="off",
                cls="chat-input",
            ),
            Button("Send", type="submit", cls="btn btn-primary"),
            hx_post=endpoint,
            hx_target="#chat-messages",
            hx_swap="beforeend",
            hx_vals=f'{{"user_id": "{user_id}"}}',
            cls="chat-form",
        ),
        cls="chat-panel",
        id="chat-panel",
    )
