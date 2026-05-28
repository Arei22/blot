use crate::database::schemas::servers;
use diesel::{Queryable, Selectable};

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = servers)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Servers {
    pub id: i64,
    pub name: String,
    pub version: String,
    pub crack: bool,
    pub port: i64,
    pub started: bool,
}
