module archimeters::kiosk_name_ext;

use std::string::String;
use sui::dynamic_field as df;
use sui::kiosk::{Kiosk, KioskOwnerCap};

/// The dynamic field key for the Kiosk Name Extension
public struct KioskName has copy, store, drop {}

/// Add a name to the Kiosk (in this implementation can be called only once)
public fun add(self: &mut Kiosk, cap: &KioskOwnerCap, name: String) {
    let uid_mut = self.uid_mut_as_owner(cap);
    df::add(uid_mut, KioskName {}, name)
}

/// Try to read the name of the Kiosk - if set - return Some(String), if not - None
public fun name(self: &Kiosk): Option<String> {
    if (df::exists_(self.uid(), KioskName {})) {
        option::some(*df::borrow(self.uid(), KioskName {}))
    } else {
        option::none()
    }
}