use uuid::Uuid;

pub struct NewNode {
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub tags: Vec<String>,
    pub org_ref: Option<String>,
}
