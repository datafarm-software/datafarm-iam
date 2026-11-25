//! Every value here composes a mock sub-division.
//! Used in testing to expand the existing fixtures tree,
//! according to access restrictions.

#![allow(dead_code)]

use crate::common::fixtures::{grant::Grant, role, user::DF_USER};
use uuid::{Uuid, uuid};

/// Tree: DataFarm -> CompanyA -> ClientA
pub mod node {
    use super::*;
    pub const SUB_DIVISION: Uuid = uuid!("00000000-0000-0000-0000-000000000004");
    pub const SUB_DIVISION_NAME: &str = "Sub-Division";
}

pub mod user {
    use super::*;
    pub const DAN: Uuid = uuid!("10000000-0000-0000-0000-000000000004");
    pub const DAN_SUB: &str = "seed-sub-dan";
    pub const DAN_EMAIL: &str = "dan@clienta.com";
    // Home node
    pub const DAN_HOME: Uuid = node::SUB_DIVISION;
}

pub mod grant {
    use super::*;
    pub const DAN_MEMBER_ON_CLIENT_A: Grant = Grant {
        user: user::DAN,
        node: node::SUB_DIVISION,
        role: role::MEMBER,
        granted_by: Some(DF_USER),
    };
}
