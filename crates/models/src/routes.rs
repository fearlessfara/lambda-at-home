use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiRoute {
    pub route_id: Uuid,
    pub path: String,          // e.g. "/api-caller" or "/v1/items"
    pub method: Option<String>,// e.g. "GET" | "POST" | None for any
    pub function_name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateApiRouteRequest {
    pub path: String,
    pub method: Option<String>,
    pub function_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListApiRoutesResponse {
    pub routes: Vec<ApiRoute>,
}

