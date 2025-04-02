use axum::{
    routing::{get, post},
    Json, Router,
};
use rusqlite::{params, Connection, OptionalExtension, Result};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

/**
 * 资源类型
 */
#[derive(Serialize, Deserialize)]
struct ResourceType {
    id: Option<u32>, // 资源id
    name: String,    // 资源名称
}

/**
 * 资源
 */
#[derive(Serialize, Deserialize)]
struct Resource {
    id: Option<u32>,
    type_id: u32,     // 资源类型id
    amount: u32,      // 数量
    activity_id: u32, // 来源名称
}

/**
 * 资源来源活动
 */
#[derive(Serialize, Deserialize)]
struct Activity {
    id: Option<u32>,
    name: String, // 活动名称
    count: u32,   // 活动总刷取次数
    consume: u32, // 活动总体力消耗
}

#[derive(Deserialize)]
struct BatchResourceRequest {
    records: Vec<Resource>,
    stamina_cost: u32, // 每次提交的体力消耗
}

#[tokio::main]
async fn main() {
    // 设置数据库
    let conn = establish_connection();
    create_tables(&conn).expect("Failed to create tables");

    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);

    // 设置 Axum 路由
    let app = Router::new()
        .route("/resource_types/:activity_id", get(get_resource_types)) // 按活动获取资源类型
        .route("/resource_types/:activity_id", post(add_resource_type))
        .route("/activities", post(add_activity).get(get_activities))
        .route("/resources", post(add_resource))
        .route("/batch_resources", post(add_resources))
        .route("/statistics", get(fetch_statistics))
        .route("/activity_resources/:activity_id", get(get_activity_resources))
        .layer(cors);

    // 启动服务器
    let addr = SocketAddr::from(([127, 0, 0, 1], 9909));
    println!("Listening on {}", addr);
    axum::Server::bind(&addr).serve(app.into_make_service()).await.unwrap();
}

fn establish_connection() -> Connection {
    Connection::open("game_resources.db").expect("无法连接到数据库")
}

fn create_tables(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS resource_types (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS activities (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            count INTEGER DEFAULT 0, -- 活动总刷取次数
            consume INTEGER DEFAULT 0 -- 活动总体力消耗
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS resources (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            type_id INTEGER NOT NULL,
            amount INTEGER NOT NULL,
            activity_id INTEGER NOT NULL,
            FOREIGN KEY (type_id) REFERENCES resource_types(id),
            FOREIGN KEY (activity_id) REFERENCES activities(id)
        )",
        [],
    )?;
    Ok(())
}

async fn add_resource_type(
    axum::extract::Path(activity_id): axum::extract::Path<u32>,
    Json(resource_type): Json<ResourceType>,
) -> Json<ResourceType> {
    let conn = establish_connection();

    // 插入资源类型
    conn.execute("INSERT INTO resource_types (name) VALUES (?1)", params![resource_type.name])
        .expect("Failed to add resource type");

    let type_id = conn.last_insert_rowid() as u32;

    // 插入资源记录，关联活动
    conn.execute(
        "INSERT INTO resources (type_id, amount, activity_id) VALUES (?1, 0, ?2)",
        params![type_id, activity_id],
    )
    .expect("Failed to add resource record");

    Json(ResourceType { id: Some(type_id), name: resource_type.name })
}

async fn get_resource_types(
    axum::extract::Path(activity_id): axum::extract::Path<u32>,
) -> Json<Vec<ResourceType>> {
    let conn = establish_connection();
    let mut stmt = conn
        .prepare(
            "SELECT rt.id, rt.name
             FROM resource_types rt
             JOIN resources r ON rt.id = r.type_id
             WHERE r.activity_id = ?1",
        )
        .expect("Failed to prepare query");

    let resource_types = stmt
        .query_map([activity_id], |row| {
            Ok(ResourceType { id: Some(row.get(0)?), name: row.get(1)? })
        })
        .expect("Failed to execute query")
        .map(|res| res.unwrap())
        .collect();

    Json(resource_types)
}

async fn add_activity(Json(activity): Json<Activity>) -> Json<Activity> {
    let conn = establish_connection();
    conn.execute("INSERT INTO activities (name) VALUES (?1)", params![activity.name])
        .expect("Failed to add activity");

    let id = conn.last_insert_rowid() as u32;
    Json(Activity { id: Some(id), name: activity.name, count: 0, consume: 0 })
}

async fn get_activities() -> Json<Vec<Activity>> {
    let conn = establish_connection();
    let mut stmt = conn
        .prepare("SELECT id, name, count, consume FROM activities")
        .expect("Failed to prepare query");

    let activities = stmt
        .query_map([], |row| {
            Ok(Activity {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                count: row.get(2)?,
                consume: row.get(3)?,
            })
        })
        .expect("Failed to execute query")
        .map(|res| res.unwrap())
        .collect();

    Json(activities)
}

async fn add_resource(Json(resource): Json<Resource>) -> Json<Resource> {
    let conn = establish_connection();
    conn.execute(
        "INSERT INTO resources (type_id, amount, activity_id) VALUES (?1, ?2, ?3)",
        params![resource.type_id, resource.amount, resource.activity_id],
    )
    .expect("Failed to add resource");

    // 更新活动的刷取次数和总体力消耗
    conn.execute(
        "UPDATE activities SET count = count + 1, consume = consume + ?1 WHERE id = ?2",
        params![resource.amount, resource.activity_id],
    )
    .expect("Failed to update activity");

    let id = conn.last_insert_rowid() as u32;
    Json(Resource {
        id: Some(id),
        type_id: resource.type_id,
        amount: resource.amount,
        activity_id: resource.activity_id,
    })
}

async fn add_resources(Json(payload): Json<BatchResourceRequest>) -> Json<()> {
    let mut conn = establish_connection();
    let tx = conn.transaction().expect("Failed to start transaction");
    let a_id = payload.records.get(0).unwrap().activity_id;

    for record in payload.records {
        // 检查是否已经存在相同活动和资源类型的记录
        let existing_amount: Option<u32> = tx
            .query_row(
                "SELECT amount FROM resources WHERE type_id = ?1 AND activity_id = ?2",
                params![record.type_id, record.activity_id],
                |row| row.get(0),
            )
            .optional()
            .expect("Failed to query existing resource");

        if let Some(_amount) = existing_amount {
            // 如果记录已存在，更新数量
            tx.execute(
                "UPDATE resources SET amount = amount + ?1 WHERE type_id = ?2 AND activity_id = ?3",
                params![record.amount, record.type_id, record.activity_id],
            )
            .expect("Failed to update resource amount");
        } else {
            // 如果记录不存在，插入新记录
            tx.execute(
                "INSERT INTO resources (type_id, amount, activity_id) VALUES (?1, ?2, ?3)",
                params![record.type_id, record.amount, record.activity_id],
            )
            .expect("Failed to add new resource");
        }
    }
    // 更新活动的刷取次数和总体力消耗
    tx.execute(
        "UPDATE activities SET count = count + 1, consume = consume + ?1 WHERE id = ?2",
        params![payload.stamina_cost, a_id],
    )
    .expect("Failed to update activity");

    tx.commit().expect("Failed to commit transaction");
    Json(())
}

async fn fetch_statistics() -> Json<Vec<Activity>> {
    let conn = establish_connection();
    let mut stmt = conn
        .prepare("SELECT id, name, count, consume FROM activities")
        .expect("Failed to prepare query");

    let statistics = stmt
        .query_map([], |row| {
            Ok(Activity {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                count: row.get(2)?,
                consume: row.get(3)?,
            })
        })
        .expect("Failed to execute query")
        .map(|res| res.unwrap())
        .collect();

    Json(statistics)
}

async fn get_activity_resources(
    axum::extract::Path(activity_id): axum::extract::Path<u32>,
) -> Json<Vec<(String, u32)>> {
    let conn = establish_connection();
    let mut stmt = conn
        .prepare(
            "SELECT rt.name, r.amount
             FROM resources r
             JOIN resource_types rt ON r.type_id = rt.id
             WHERE r.activity_id = ?1",
        )
        .expect("Failed to prepare query");

    let resources = stmt
        .query_map([activity_id], |row| {
            Ok((row.get(0)?, row.get(1)?)) // 返回资源名称和数量
        })
        .expect("Failed to execute query")
        .map(|res| res.unwrap())
        .collect();

    Json(resources)
}
