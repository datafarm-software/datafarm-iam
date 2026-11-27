//! Shared test constants. Every value here mirrors a row in `fixtures/*.sql`.
//! If you change a fixture, change it here too (and vice versa).
//!
//! Fixture load order: nodes -> users -> grants -> devices
//!   #[sqlx::test(fixtures("nodes", "users", "grants", "devices"))]

#![allow(dead_code)]

use uuid::{Uuid, uuid};

/// Tree: DataFarm -> CompanyA -> ClientA
///                            |
///                            -> ClientB
pub mod node {
    use super::*;

    pub const DATAFARM: Uuid = uuid!("00000000-0000-0000-0000-000000000001");
    pub const COMPANY_A: Uuid = uuid!("00000000-0000-0000-0000-000000000002");
    pub const CLIENT_A: Uuid = uuid!("00000000-0000-0000-0000-000000000003");
    pub const CLIENT_B: Uuid = uuid!("00000000-0000-0000-0000-000000000004");

    pub const DATAFARM_NAME: &str = "DataFarm";
    pub const COMPANY_A_NAME: &str = "CompanyA";
    pub const CLIENT_A_NAME: &str = "ClientA";
    pub const CLIENT_B_NAME: &str = "ClientB";

    /// Tenant (org) boundaries. DataFarm has none.
    pub const COMPANY_A_ORG: &str = "seed-org-companya";
    pub const CLIENT_A_ORG: &str = "seed-org-clienta";
    pub const CLIENT_B_ORG: &str = "seed-org-clientb";
}

pub mod user {
    use super::*;

    pub const DF_USER: Uuid = uuid!("10000000-0000-0000-0000-000000000001");
    pub const BOB: Uuid = uuid!("10000000-0000-0000-0000-000000000002");
    pub const TOM: Uuid = uuid!("10000000-0000-0000-0000-000000000003");

    pub const DF_USER_SUB: &str = "seed-sub-df";
    pub const BOB_SUB: &str = "seed-sub-bob";
    pub const TOM_SUB: &str = "seed-sub-tom";

    pub const DF_USER_EMAIL: &str = "df@datafarm.co.za";
    pub const BOB_EMAIL: &str = "bob@companya.com";
    pub const TOM_EMAIL: &str = "tom@clienta.com";

    // Home nodes
    pub const DF_USER_HOME: Uuid = node::DATAFARM;
    pub const BOB_HOME: Uuid = node::COMPANY_A;
    pub const TOM_HOME: Uuid = node::CLIENT_A;
}

pub mod device {
    pub const DEV_1001: &str = "dev-1001"; // placed on ClientA
}

pub mod role {
    pub const ADMIN: &str = "admin";
    pub const MANAGER: &str = "manager";
    pub const MEMBER: &str = "member";
}

// ---------------------------------------------------------------------
// ltree helpers. Paths are built from ids with '-' replaced by '_', as in
// iam.nodes_set_path(), so these let tests assert on `nodes.path` and on
// the `node_path` values returned by the iam_api_v1 contract.
// ---------------------------------------------------------------------

/// One ltree label for a node id.
pub fn ltree_label(id: Uuid) -> String {
    id.to_string().replace('-', "_")
}

/// Dotted ltree path for a chain of ids, root first.
pub fn ltree_path(chain: &[Uuid]) -> String {
    chain
        .iter()
        .map(|id| ltree_label(*id))
        .collect::<Vec<_>>()
        .join(".")
}

pub fn datafarm_path() -> String {
    ltree_path(&[node::DATAFARM])
}

pub fn company_a_path() -> String {
    ltree_path(&[node::DATAFARM, node::COMPANY_A])
}

pub fn client_a_path() -> String {
    ltree_path(&[node::DATAFARM, node::COMPANY_A, node::CLIENT_A])
}
