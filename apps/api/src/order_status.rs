/// Mirrors the CHECK constraint on `orders.status` (last changed in
/// `migrations/20261101132739_catering_quotes.up.sql`), and is
/// mirrored in turn by `ORDER_STATUSES` in `apps/web/lib/orders.ts`.
/// Adding a status means editing all three (Phase 20 open risk 4);
/// `tests/order_status_schema.rs` fails if this list and the CHECK drift.
pub const ORDER_STATUSES: [&str; 8] = [
    "awaiting_payment",
    "payment_claimed",
    "processing",
    "ready_for_collection",
    "out_for_delivery",
    // On-site catering only (Phase 23 Decision 12).
    "serving_on_site",
    "completed",
    "cancelled",
];

pub fn is_valid_status(value: &str) -> bool {
    ORDER_STATUSES.contains(&value)
}

/// Statuses from which the customer may confirm they received or
/// collected the order (Phase 22 Decision 8), or that their event was
/// catered (Phase 23 Decision 12).
pub const CUSTOMER_COMPLETABLE: [&str; 3] = [
    "ready_for_collection",
    "out_for_delivery",
    "serving_on_site",
];

pub fn customer_can_complete(status: &str) -> bool {
    CUSTOMER_COMPLETABLE.contains(&status)
}

/// Statuses an admin may move an order to next. `payment_claimed` is never
/// here: only the customer's "I've paid" sets it. Confirming payment
/// (`processing`) is allowed straight from `awaiting_payment` for a
/// customer who paid but never pressed the button (Phase 20 Decision 3).
/// An order the customer completed can be reopened to its ready status, in
/// case they pressed "received" too early; one the admin completed can't
/// (Phase 22 Decision 8). `on_site` orders only come from accepted catering
/// quotes (Phase 23 Decision 12).
pub fn admin_next(
    status: &str,
    delivery_type: &str,
    completed_by: Option<&str>,
) -> &'static [&'static str] {
    match (status, delivery_type, completed_by) {
        ("awaiting_payment" | "payment_claimed", _, _) => &["processing", "cancelled"],
        ("processing", "pickup", _) => &["ready_for_collection", "cancelled"],
        ("processing", "on_site", _) => &["serving_on_site", "cancelled"],
        ("processing", _, _) => &["out_for_delivery", "cancelled"],
        ("ready_for_collection" | "out_for_delivery" | "serving_on_site", _, _) => {
            &["completed", "cancelled"]
        }
        ("completed", "pickup", Some("customer")) => &["ready_for_collection"],
        ("completed", "on_site", Some("customer")) => &["serving_on_site"],
        ("completed", _, Some("customer")) => &["out_for_delivery"],
        _ => &[],
    }
}

/// True when `to` takes a customer-completed order back to its ready
/// status. The reopen sends the customer no email (Phase 22 Decision 8).
pub fn is_reopen(from: &str, to: &str) -> bool {
    from == "completed" && customer_can_complete(to)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allowed(from: &str, to: &str, delivery_type: &str) -> bool {
        admin_next(from, delivery_type, None).contains(&to)
    }

    #[test]
    fn payment_can_be_confirmed_with_or_without_a_claim() {
        for delivery_type in ["pickup", "delivery", "on_site"] {
            assert!(allowed("awaiting_payment", "processing", delivery_type));
            assert!(allowed("payment_claimed", "processing", delivery_type));
        }
    }

    #[test]
    fn admin_can_never_set_payment_claimed() {
        for from in ORDER_STATUSES {
            for delivery_type in ["pickup", "delivery", "on_site"] {
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
        assert!(allowed("processing", "serving_on_site", "on_site"));
        assert!(!allowed("processing", "out_for_delivery", "on_site"));
        assert!(!allowed("processing", "ready_for_collection", "on_site"));
        assert!(!allowed("processing", "serving_on_site", "delivery"));
    }

    #[test]
    fn handed_over_orders_can_complete() {
        assert!(allowed("ready_for_collection", "completed", "pickup"));
        assert!(allowed("out_for_delivery", "completed", "delivery"));
        assert!(allowed("serving_on_site", "completed", "on_site"));
    }

    #[test]
    fn no_skipping_ahead() {
        assert!(!allowed("awaiting_payment", "completed", "pickup"));
        assert!(!allowed(
            "payment_claimed",
            "ready_for_collection",
            "pickup"
        ));
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
            ("serving_on_site", "on_site"),
        ] {
            assert!(allowed(from, "cancelled", delivery_type));
        }
    }

    #[test]
    fn terminal_statuses_have_no_next_step() {
        for delivery_type in ["pickup", "delivery", "on_site"] {
            assert!(admin_next("completed", delivery_type, None).is_empty());
            assert!(admin_next("completed", delivery_type, Some("admin")).is_empty());
            assert!(admin_next("cancelled", delivery_type, None).is_empty());
        }
    }

    #[test]
    fn every_target_is_a_known_status() {
        for from in ORDER_STATUSES {
            for delivery_type in ["pickup", "delivery", "on_site"] {
                for completed_by in [None, Some("customer"), Some("admin")] {
                    for to in admin_next(from, delivery_type, completed_by) {
                        assert!(is_valid_status(to));
                    }
                }
            }
        }
    }

    #[test]
    fn customer_completed_orders_reopen_to_their_ready_status() {
        assert_eq!(
            admin_next("completed", "pickup", Some("customer")),
            &["ready_for_collection"]
        );
        assert_eq!(
            admin_next("completed", "delivery", Some("customer")),
            &["out_for_delivery"]
        );
        assert_eq!(
            admin_next("completed", "on_site", Some("customer")),
            &["serving_on_site"]
        );
        assert!(is_reopen("completed", "serving_on_site"));
        assert!(is_reopen("completed", "ready_for_collection"));
        assert!(!is_reopen("processing", "ready_for_collection"));
    }

    #[test]
    fn customer_can_complete_only_handed_over_orders() {
        for status in ORDER_STATUSES {
            assert_eq!(
                customer_can_complete(status),
                matches!(
                    status,
                    "ready_for_collection" | "out_for_delivery" | "serving_on_site"
                ),
                "{status}"
            );
        }
    }
}
