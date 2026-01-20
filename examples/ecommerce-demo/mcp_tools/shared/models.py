"""Shared Pydantic models for ecommerce MCP services."""

from datetime import datetime
from enum import Enum
from typing import Optional

from pydantic import BaseModel, Field


class CartStatus(str, Enum):
    ACTIVE = "active"
    CHECKED_OUT = "checked_out"
    ABANDONED = "abandoned"


class OrderStatus(str, Enum):
    PENDING = "pending"
    CONFIRMED = "confirmed"
    SHIPPED = "shipped"
    DELIVERED = "delivered"
    CANCELLED = "cancelled"


class PurchaseOrderStatus(str, Enum):
    PENDING = "pending"
    CONFIRMED = "confirmed"
    SHIPPED = "shipped"
    RECEIVED = "received"
    CANCELLED = "cancelled"


class Product(BaseModel):
    """Product in the catalog."""

    id: str
    name: str
    description: Optional[str] = None
    price: float
    cost: float  # Hidden from customers
    category: Optional[str] = None
    stock_quantity: int = 0
    reorder_threshold: int = 10
    created_at: Optional[datetime] = None
    updated_at: Optional[datetime] = None


class ProductPublic(BaseModel):
    """Customer-facing product view (no cost info)."""

    id: str
    name: str
    description: Optional[str] = None
    price: float
    category: Optional[str] = None
    in_stock: bool = True


class CartItem(BaseModel):
    """Item in a shopping cart."""

    id: str
    cart_id: str
    product_id: str
    quantity: int = 1
    # Enriched fields (not stored, added at query time)
    product_name: Optional[str] = None
    product_price: Optional[float] = None
    line_total: Optional[float] = None


class Cart(BaseModel):
    """Shopping cart."""

    id: str
    user_id: str
    status: CartStatus = CartStatus.ACTIVE
    created_at: Optional[datetime] = None
    updated_at: Optional[datetime] = None
    items: list[CartItem] = Field(default_factory=list)
    total: float = 0.0


class OrderItem(BaseModel):
    """Item in an order."""

    id: str
    order_id: str
    product_id: str
    quantity: int
    price_at_time: float
    # Enriched fields
    product_name: Optional[str] = None
    line_total: Optional[float] = None


class Order(BaseModel):
    """Customer order."""

    id: str
    user_id: str
    cart_id: Optional[str] = None
    total: float
    status: OrderStatus = OrderStatus.PENDING
    shipping_address: Optional[str] = None
    created_at: Optional[datetime] = None
    updated_at: Optional[datetime] = None
    items: list[OrderItem] = Field(default_factory=list)


class Supplier(BaseModel):
    """Supplier for purchase orders."""

    id: str
    name: str
    lead_time_days: int = 7
    reliability_score: float = 0.9  # 0-1, affects delivery simulation
    contact_email: Optional[str] = None


class PurchaseOrder(BaseModel):
    """Purchase order to supplier."""

    id: str
    product_id: str
    supplier_id: str
    quantity_ordered: int
    unit_cost: float
    status: PurchaseOrderStatus = PurchaseOrderStatus.PENDING
    expected_delivery: Optional[datetime] = None
    actual_delivery: Optional[datetime] = None
    created_at: Optional[datetime] = None
    notes: Optional[str] = None
    # Enriched fields
    product_name: Optional[str] = None
    supplier_name: Optional[str] = None


class InventoryAdjustment(BaseModel):
    """Record of inventory adjustment."""

    id: str
    product_id: str
    quantity_change: int
    reason: str
    created_at: Optional[datetime] = None


class LowStockAlert(BaseModel):
    """Alert for low stock products."""

    product_id: str
    product_name: str
    current_stock: int
    reorder_threshold: int
    deficit: int  # How many units below threshold


class SalesReportItem(BaseModel):
    """Sales data for a product."""

    product_id: str
    product_name: str
    units_sold: int
    revenue: float
    avg_price: float


class SalesReport(BaseModel):
    """Sales analytics report."""

    start_date: Optional[datetime] = None
    end_date: Optional[datetime] = None
    total_revenue: float
    total_units: int
    items: list[SalesReportItem] = Field(default_factory=list)


class InventoryReport(BaseModel):
    """Full inventory status."""

    total_products: int
    total_units: int
    total_value: float  # At cost
    low_stock_count: int
    out_of_stock_count: int
    products: list[Product] = Field(default_factory=list)
