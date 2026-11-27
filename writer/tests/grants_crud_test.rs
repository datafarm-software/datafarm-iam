mod common;
use common::*;
use iam_writer::db::node::{MoveNode, NewNode, UpdateNode};
use iam_writer::error::Error;
use iam_writer::writer::IamWriter;
use sqlx::PgPool;
use uuid::uuid;

#[sqlx::test(
    migrations = "../migrations",
    fixtures("nodes", "users", "grants", "devices")
)]
async fn test_crud_grant(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {

