use uuid::Uuid;

pub struct Grant {
    pub user_id: Uuid,
    pub node_id: Uuid,
    pub role: &'static str,
    pub granted_by: Option<Uuid>,
}

