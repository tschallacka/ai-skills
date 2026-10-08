"""Inventory pricing helpers for the warehouse service.

Seed fixture for the ai-text-editor-usage benchmark. Deliberately not
production code: just large enough and varied enough (several call sites of
one deprecated pattern, one constant an external edit will touch, several
independent functions) to exercise span-addressed edits, regex search, and
collision detection in a single short task. Indented at 3 spaces per level
on purpose -- see external_reformat.py for why.
"""

TAX_RATE = 0.07
SHIPPING_FLAT_FEE = 4.99
BULK_DISCOUNT_THRESHOLD = 50


def legacy_log(message):
   """Deprecated: prints instead of using the real logger. TODO: remove once
   every call site below has moved to logger.info."""
   print(f"[legacy] {message}")


def add_item(cart, sku, quantity, unit_price):
   legacy_log(f"adding {quantity} of {sku} at {unit_price}")
   cart.append({"sku": sku, "quantity": quantity, "unit_price": unit_price})
   return cart


def remove_item(cart, sku):
   legacy_log(f"removing {sku}")
   return [item for item in cart if item["sku"] != sku]


def subtotal(cart):
   legacy_log("computing subtotal")
   return sum(item["quantity"] * item["unit_price"] for item in cart)


def apply_bulk_discount(cart_subtotal, item_count):
   """10% off once the cart crosses BULK_DISCOUNT_THRESHOLD items."""
   if item_count >= BULK_DISCOUNT_THRESHOLD:
      return cart_subtotal * 0.90
   return cart_subtotal


def apply_tax(amount):
   legacy_log(f"applying tax at rate {TAX_RATE}")
   return amount * (1 + TAX_RATE)


def shipping_cost(cart_subtotal):
   legacy_log("computing shipping")
   if cart_subtotal >= 75:
      return 0.0
   return SHIPPING_FLAT_FEE


def calculate_total(cart):
   """End-to-end total: subtotal -> bulk discount -> tax -> shipping."""
   base = subtotal(cart)
   discounted = apply_bulk_discount(base, sum(item["quantity"] for item in cart))
   taxed = apply_tax(discounted)
   return taxed + shipping_cost(discounted)


def format_receipt(cart):
   legacy_log("formatting receipt")
   lines = [f"{item['sku']}: {item['quantity']} x {item['unit_price']}" for item in cart]
   lines.append(f"Total: {calculate_total(cart):.2f}")
   return "\n".join(lines)
