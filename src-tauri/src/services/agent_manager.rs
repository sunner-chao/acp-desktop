use crate::models::{Agent, AgentConfig, CreateAgentInput, DriverType, UpdateAgentInput};
use crate::services::db::Database;
use rusqlite::{params, Result};
use std::sync::Mutex;

pub struct AgentManager {
    db: Mutex<Database>,
}

impl AgentManager {
    pub fn new(db: Database) -> Self {
        Self { db: Mutex::new(db) }
    }

    pub fn create_agent(&self, input: CreateAgentInput) -> Result<Agent> {
        let db = self.db.lock().unwrap();
        let agent = Agent::new(input);

        db.get_connection().execute(
            "INSERT INTO agents (id, name, description, driver_type, address, config, is_online, session_id, last_active, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
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
        )?;

        log::info!("Created agent: {} ({})", agent.name, agent.id);
        Ok(agent)
    }

    pub fn get_agents(&self) -> Result<Vec<Agent>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.get_connection().prepare(
            "SELECT id, name, description, driver_type, address, config, is_online, session_id, last_active, created_at FROM agents"
        )?;

        let agents = stmt.query_map([], |row| {
            let driver_type_str: String = row.get(3)?;
            let config_str: String = row.get(5)?;

            let driver_type: DriverType =
                serde_json::from_str(&driver_type_str).unwrap_or(DriverType::Script);
            let config: AgentConfig =
                serde_json::from_str(&config_str).unwrap_or_else(|_| AgentConfig::default());

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
        })?;

        agents.collect()
    }

    pub fn update_agent(&self, input: UpdateAgentInput) -> Result<Agent> {
        let db = self.db.lock().unwrap();

        // Get existing agent
        let mut stmt = db.get_connection().prepare(
            "SELECT id, name, description, driver_type, address, config, is_online, session_id, last_active, created_at FROM agents WHERE id = ?1"
        )?;

        let agent: Agent = stmt.query_row([&input.id], |row| {
            let driver_type_str: String = row.get(3)?;
            let config_str: String = row.get(5)?;

            let driver_type: DriverType =
                serde_json::from_str(&driver_type_str).unwrap_or(DriverType::Script);
            let config: AgentConfig =
                serde_json::from_str(&config_str).unwrap_or_else(|_| AgentConfig::default());

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
        })?;

        let updated_agent = Agent {
            id: agent.id,
            name: input.name.unwrap_or(agent.name),
            description: input.description.or(agent.description),
            driver_type: agent.driver_type,
            address: agent.address,
            config: input.config.unwrap_or(agent.config),
            is_online: agent.is_online,
            session_id: agent.session_id,
            last_active: agent.last_active,
            created_at: agent.created_at,
        };

        db.get_connection().execute(
            "UPDATE agents SET name = ?1, description = ?2, config = ?3 WHERE id = ?4",
            params![
                updated_agent.name,
                updated_agent.description,
                serde_json::to_string(&updated_agent.config).unwrap(),
                updated_agent.id,
            ],
        )?;

        log::info!(
            "Updated agent: {} ({})",
            updated_agent.name,
            updated_agent.id
        );
        Ok(updated_agent)
    }

    pub fn delete_agent(&self, id: &str) -> Result<()> {
        let db = self.db.lock().unwrap();
        db.get_connection()
            .execute("DELETE FROM agents WHERE id = ?1", [id])?;
        log::info!("Deleted agent: {}", id);
        Ok(())
    }

    pub fn set_online_status(&self, id: &str, is_online: bool) -> Result<()> {
        let db = self.db.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        db.get_connection().execute(
            "UPDATE agents SET is_online = ?1, last_active = ?2 WHERE id = ?3",
            params![is_online as i32, now, id],
        )?;
        Ok(())
    }
}
