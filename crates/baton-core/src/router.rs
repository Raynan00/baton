//! Transport routing decision (BATON.md §3.2). Pure function so it is table-testable.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Lan,
    Relay,
    RelayWithPush,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipientKind {
    Desktop,
    Phone,
}

#[derive(Debug, Clone, Copy)]
pub struct Ctx {
    pub recipient: RecipientKind,
    /// Live LAN link, or reachable within the last 10 s.
    pub lan_reachable: bool,
    /// Phone reported `foreground=true` and holds a LAN link or relay WebSocket.
    pub phone_foreground: bool,
    /// The message wants an APNs push (alert / Live Activity).
    pub wants_push: bool,
    pub lan_only: bool,
}

pub fn decide(c: &Ctx) -> Route {
    let phone_closed = c.recipient == RecipientKind::Phone && !c.phone_foreground;
    // Handoff to a backgrounded phone skips LAN: iOS can't listen in the background.
    if c.lan_reachable && !phone_closed {
        return Route::Lan;
    }
    if c.lan_only {
        return Route::Lan; // caller surfaces "will not reach a closed iPhone"
    }
    if c.recipient == RecipientKind::Phone && c.wants_push {
        Route::RelayWithPush
    } else {
        Route::Relay
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use RecipientKind::*;

    fn ctx(recipient: RecipientKind, lan: bool, fg: bool, push: bool) -> Ctx {
        Ctx {
            recipient,
            lan_reachable: lan,
            phone_foreground: fg,
            wants_push: push,
            lan_only: false,
        }
    }

    #[test]
    fn table() {
        let cases = [
            (ctx(Desktop, true, false, false), Route::Lan),
            (ctx(Desktop, false, false, false), Route::Relay),
            (ctx(Phone, true, true, true), Route::Lan),
            (ctx(Phone, true, false, true), Route::RelayWithPush),
            (ctx(Phone, false, false, true), Route::RelayWithPush),
            (ctx(Phone, false, true, false), Route::Relay),
        ];
        for (c, want) in cases {
            assert_eq!(decide(&c), want, "{c:?}");
        }
    }
}
