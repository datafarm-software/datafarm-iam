//! Every value here composes a mock sub-division.
//! Used in testing to expand the existing fixtures tree,
//! according to access restrictions.

#![allow(dead_code)]

use uuid::{Uuid, uuid};

/// Tree: DataFarm -> CompanyA -> ClientA
pub mod node {
    use super::*;
    pub const SUB_DIVISION: Uuid = uuid!("00000000-0000-0000-0000-000000000009");
    pub const SUB_DIVISION_NAME: &str = "Sub-Division";
    pub const SUB_DIVISION_TAG: &str = "sub-division";
}

pub mod user {
    pub const DAN_SUB: &str = "seed-sub-dan";
    pub const DAN_EMAIL: &str = "dan@clienta.com";
}
