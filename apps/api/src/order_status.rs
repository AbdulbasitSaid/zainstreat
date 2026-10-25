/// Mirrors the CHECK constraint on `orders.status` in
/// `migrations/20261025120000_order_payment_lifecycle.up.sql`, and is
/// mirrored in turn by `ORDER_STATUSES` in `apps/web/lib/orders.ts`.
/// Adding a status means editing all three (Phase 20 open risk 4);
/// `tests/order_status_schema.rs` fails if this list and the CHECK drift.
pub const ORDER_STATUSES: [&str; 7] = [
    "awaiting_payment",
    "payment_claimed",
    "processing",
    "ready_for_collection",
    "out_for_delivery",
    "completed",
    "cancelled",
];

pub fn is_valid_status(value: &str) -> bool {
    ORDER_STATUSES.contains(&value)
}

/// Statuses an admin may move an order to next. `payment_claimed` is never
/// here: only the customer's "I've paid" sets it. Confirming payment
/// (`processing`) is allowed straight from `awaiting_payment` for a
/// customer who paid but never pressed the button (Phase 20 Decision 3).
pub fn admin_next(status: &str, delivery_type: &str) -> &'static [&'static str] {
    match (status, delivery_type) {
        ("awaiting_payment" | "payment_claimed", _) => &["processing", "cancelled"],
        ("processing", "pickup") => &["ready_for_collection", "cancelled"],
        ("processing", _) => &["out_for_delivery", "cancelled"],
        ("ready_for_collection" | "out_for_delivery", _) => &["completed", "cancelled"],
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allowed(from: &str, to: &str, delivery_type: &str) -> bool {
        admin_next(from, delivery_type).contains(&to)
    }

    #[test]
    fn payment_can_be_confirmed_with_or_without_a_claim() {
        for delivery_type in ["pickup", "delivery"] {
            assert!(allowed("awaiting_payment", "processing", delivery_type));
            assert!(allowed("payment_claimed", "processing", delivery_type));
        }
    }

    #[test]
    fn admin_can_never_set_payment_claimed() {
        for from in ORDER_STATUSES {
            for delivery_type in ["pickup", "delivery"] {
                assert!(!allowed(from, "payment_claimed", delivery_type));
            }
        }
    }

    #[test]
    fn processing_branches_on_delivery_type() {
        assert!(allowed("processing", "ready_for_collection", "pickup"));
        assert!(!allowed("processing", "out_for_delivery", "pickup"));
        assert!(allowed("processing", "out_for_delivery", "delivery"));
        assert!(!allowed("processing", "ready_for_collection", "delivery"));
    }

    #[test]
    fn handed_over_orders_can_complete() {
        assert!(allowed("ready_for_collection", "completed", "pickup"));
        assert!(allowed("out_for_delivery", "completed", "delivery"));
    }

    #[test]
    fn no_skipping_ahead() {
        assert!(!allowed("awaiting_payment", "completed", "pickup"));
        assert!(!allowed("payment_claimed", "ready_for_collection", "pickup"));
        assert!(!allowed("processing", "completed", "delivery"));
    }

    #[test]
    fn every_open_status_can_be_cancelled() {
        for (from, delivery_type) in [
            ("awaiting_payment", "pickup"),
            ("payment_claimed", "pickup"),
            ("processing", "pickup"),
            ("ready_for_collection", "pickup"),
            ("out_for_delivery", "delivery"),
        ] {
            assert!(allowed(from, "cancelled", delivery_type));
        }
    }

    #[test]
    fn terminal_statuses_have_no_next_step() {
        for delivery_type in ["pickup", "delivery"] {
            assert!(admin_next("completed", delivery_type).is_empty());
            assert!(admin_next("cancelled", delivery_type).is_empty());
        }
    }

    #[test]
    fn every_target_is_a_known_status() {
        for from in ORDER_STATUSES {
            for delivery_type in ["pickup", "delivery"] {
                for to in admin_next(from, delivery_type) {
                    assert!(is_valid_status(to));
                }
            }
        }
    }
}
