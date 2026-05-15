use crate::models::{Agent, CreateAgentInput, UpdateAgentInput};
use crate::AppState;
use tauri::State;

pub fn fetch_agent_by_id(db: &crate::services::Database, id: &str) -> Result<Agent, String> {
    let mut stmt = db
        .get_connection()
        .prepare(
        "SELECT id, name, description, driver_type, address, config, is_online, session_id, last_active, created_at FROM agents WHERE id = ?1"
        )
        .map_err(|e| e.to_string())?;

    let agent = stmt
        .query_row([id], |row| {
            let driver_type_str: String = row.get(3)?;
            let config_str: String = row.get(5)?;

            let driver_type: crate::models::DriverType =
                serde_json::from_str(&driver_type_str).unwrap_or(crate::models::DriverType::Script);
            let config: crate::models::AgentConfig =
                serde_json::from_str(&config_str).unwrap_or_default();

            Ok(Agent {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                driver_type,
                address: row.get(4)?,
                config,
                is_online: row.get::<_, i32>(6)? != 0,
                session_id: row.get(7)?,
                last_active: row.get(8)?,
                created_at: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?;

    Ok(agent)
}

pub fn set_agent_online_status(
    db: &crate::services::Database,
    id: &str,
    is_online: bool,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    db.get_connection()
        .execute(
            "UPDATE agents SET is_online = ?1, last_active = ?2 WHERE id = ?3",
            rusqlite::params![is_online as i32, now, id],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_agents(state: State<AppState>) -> Result<Vec<Agent>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db.get_connection()
        .prepare(
        "SELECT id, name, description, driver_type, address, config, is_online, session_id, last_active, created_at FROM agents
         ORDER BY is_online DESC, COALESCE(last_active, created_at) DESC, created_at DESC"
    )
    .map_err(|e| e.to_string())?;

    let agents = stmt
        .query_map([], |row| {
            let driver_type_str: String = row.get(3)?;
            let config_str: String = row.get(5)?;
            let agent_id: String = row.get(0)?;

            let driver_type: crate::models::DriverType =
                serde_json::from_str(&driver_type_str).unwrap_or(crate::models::DriverType::Script);
            let config: crate::models::AgentConfig =
                serde_json::from_str(&config_str).unwrap_or_default();

            Ok(Agent {
                id: agent_id.clone(),
                name: row.get(1)?,
                description: row.get(2)?,
                driver_type,
                address: row.get(4)?,
                config,
                is_online: row.get::<_, i32>(6)? != 0,
                session_id: row.get(7)?,
                last_active: row.get(8)?,
                created_at: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(agents)
}

#[tauri::command]
pub fn create_agent(state: State<AppState>, input: CreateAgentInput) -> Result<Agent, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let agent = Agent::new(input);

    db.get_connection()
        .execute(
            "INSERT INTO agents (id, name, description, driver_type, address, config, is_online, session_id, last_active, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                agent.id,
                agent.name,
                agent.description,
                serde_json::to_string(&agent.driver_type).unwrap(),
                agent.address,
                serde_json::to_string(&agent.config).unwrap(),
                agent.is_online,
                agent.session_id,
                agent.last_active,
                agent.created_at,
            ],
        )
        .map_err(|e| e.to_string())?;

    log::info!("Created agent: {} ({})", agent.name, agent.id);
    Ok(agent)
}

#[tauri::command]
pub fn start_agent_session(state: State<AppState>, id: String) -> Result<Agent, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let agent = fetch_agent_by_id(&db, &id)?;
    set_agent_online_status(&db, &id, true)?;
    log::info!("Started agent session: {} ({})", agent.name, agent.id);
    fetch_agent_by_id(&db, &id)
}

#[tauri::command]
pub fn stop_agent_session(state: State<AppState>, id: String) -> Result<Agent, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let agent = fetch_agent_by_id(&db, &id)?;
    set_agent_online_status(&db, &id, false)?;
    log::info!("Stopped agent session: {} ({})", agent.name, agent.id);
    fetch_agent_by_id(&db, &id)
}

#[tauri::command]
pub fn update_agent(state: State<AppState>, input: UpdateAgentInput) -> Result<Agent, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db.get_connection()
        .prepare(
            "SELECT id, name, description, driver_type, address, config, is_online, session_id, last_active, created_at FROM agents WHERE id = ?1"
        )
        .map_err(|e| e.to_string())?;

    let agent: Agent = stmt
        .query_row([&input.id], |row| {
            let driver_type_str: String = row.get(3)?;
            let config_str: String = row.get(5)?;

            let driver_type: crate::models::DriverType =
                serde_json::from_str(&driver_type_str).unwrap_or(crate::models::DriverType::Script);
            let config: crate::models::AgentConfig =
                serde_json::from_str(&config_str).unwrap_or_default();

            Ok(Agent {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                driver_type,
                address: row.get(4)?,
                config,
                is_online: row.get::<_, i32>(6)? != 0,
                session_id: row.get(7)?,
                last_active: row.get(8)?,
                created_at: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let has_new_name = input.name.is_some();
    let new_name = input.name.unwrap_or(agent.name.clone());
    let new_address = if has_new_name {
        format!("agent://local/{}", new_name)
    } else {
        agent.address.clone()
    };

    let updated_agent = Agent {
        id: agent.id,
        name: new_name,
        description: input.description.or(agent.description),
        driver_type: agent.driver_type,
        address: new_address,
        config: input.config.unwrap_or(agent.config),
        is_online: agent.is_online,
        session_id: agent.session_id,
        last_active: agent.last_active,
        created_at: agent.created_at,
    };

    db.get_connection()
        .execute(
            "UPDATE agents SET name = ?1, description = ?2, config = ?3, address = ?4 WHERE id = ?5",
            rusqlite::params![
                updated_agent.name,
                updated_agent.description,
                serde_json::to_string(&updated_agent.config).unwrap(),
                updated_agent.address,
                updated_agent.id,
            ],
        )
        .map_err(|e| e.to_string())?;

    log::info!(
        "Updated agent: {} ({})",
        updated_agent.name,
        updated_agent.id
    );
    Ok(updated_agent)
}

#[tauri::command]
pub fn delete_agent(state: State<AppState>, id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_connection()
        .execute("DELETE FROM agents WHERE id = ?1", [&id])
        .map_err(|e| e.to_string())?;
    log::info!("Deleted agent: {}", id);
    Ok(())
}

#[tauri::command]
pub fn import_agents(state: State<AppState>, json: String) -> Result<Vec<Agent>, String> {
    let agents: Vec<Agent> = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let db = state.db.lock().map_err(|e| e.to_string())?;

    for agent in &agents {
        db.get_connection()
            .execute(
                "INSERT OR REPLACE INTO agents (id, name, description, driver_type, address, config, is_online, session_id, last_active, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    agent.id,
                    agent.name,
                    agent.description,
                    serde_json::to_string(&agent.driver_type).unwrap(),
                    agent.address,
                    serde_json::to_string(&agent.config).unwrap(),
                    agent.is_online,
                    agent.session_id,
                    agent.last_active,
                    agent.created_at,
                ],
            )
            .map_err(|e| e.to_string())?;
    }

    log::info!("Imported {} agents", agents.len());
    Ok(agents)
}

#[tauri::command]
pub fn export_agents(state: State<AppState>) -> Result<String, String> {
    let agents = get_agents(state)?;
    serde_json::to_string_pretty(&agents).map_err(|e| e.to_string())
}
