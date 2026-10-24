use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};
use sysinfo::{self, System};

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(get_system_info));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();
    println!("Server running on http://localhost:8080");
    axum::serve(listener, app).await.unwrap();
}

#[derive(Serialize, Deserialize)]
struct ServerInfo {
    os_name: String,
    os_version: String,
    kernel_version: String,
    hostname: String,
    total_memory: String,
    used_memory: String,
    total_swap: String,
    used_swap: String,
}

async fn get_system_info() -> Json<ServerInfo> {
    let mut system = sysinfo::System::new_all();
    system.refresh_all();

    Json(ServerInfo {
        os_name: System::name().unwrap_or_default(),
        os_version: System::os_version().unwrap_or_default(),
        kernel_version: System::kernel_version().unwrap_or_default(),
        hostname: System::host_name().unwrap_or_default(),
        total_memory: system.total_memory().to_string(),
        used_memory: system.used_memory().to_string(),
        total_swap: system.total_swap().to_string(),
        used_swap: system.used_swap().to_string(),
    })
}
