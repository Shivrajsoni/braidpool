use rusqlite::{Connection, Result, params};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Miner {
    pub id: String,
    pub asic_model: String,
    pub name: String,
    pub status: String,
    pub temp: f64,
    pub hashrate: String,
    pub efficiency: String,
    pub power_draw: String,
    pub max_power: f64,
    pub uptime: String,
    pub location: String,
    pub last_seen: String,
    pub alerts: i32,
    pub frequency: i32,
    pub fanspeed: String,
    pub best_diff: String,
    pub mac_addr: String,
    pub hostname: String,
    pub power: f64,
    pub voltage: f64,
    pub current: f64,
    pub vr_temp: f64,
    pub nominal_voltage: f64,
    pub expected_hashrate: f64,
    pub pool_difficulty: i32,
    pub is_using_fallback_stratum: i32,
    pub is_psram_available: i32,
    pub free_heap: i64,
    pub core_voltage: i32,
    pub core_voltage_actual: i32,
    pub ssid: String,
    pub wifi_status: String,
    pub wifi_rssi: i32,
    pub ap_enabled: i32,
    pub shares_accepted: i32,
    pub shares_rejected: i32,
    pub uptime_seconds: i64,
    pub small_core_count: i32,
    pub stratum_url: String,
    pub stratum_port: i32,
    pub stratum_user: String,
    pub stratum_suggested_difficulty: i32,
    pub stratum_extranonce_subscribe: i32,
    pub fallback_stratum_url: String,
    pub fallback_stratum_port: i32,
    pub fallback_stratum_user: String,
    pub fallback_stratum_suggested_difficulty: i32,
    pub fallback_stratum_extranonce_subscribe: i32,
    pub response_time: f64,
    pub version: String,
    pub axe_os_version: String,
    pub idf_version: String,
    pub board_version: String,
    pub running_partition: String,
    pub overheat_mode: i32,
    pub overclock_enabled: i32,
    pub display: String,
    pub rotation: i32,
    pub invert_screen: i32,
    pub display_timeout: i32,
    pub auto_fanspeed: i32,
    pub temp_target: i32,
    pub fan_rpm: i32,
    pub stats_frequency: i32,
}

pub struct Database {
    db_path: String,
}

impl Database {
    pub fn new(db_path: &str) -> Result<Self> {
        let db = Database { 
            db_path: db_path.to_string(),
        };
        db.create_tables()?;
        Ok(db)
    }

    fn get_connection(&self) -> Result<Connection> {
        Connection::open(&self.db_path)
    }

    fn create_tables(&self) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS miner (
                id TEXT PRIMARY KEY,
                mac_addr TEXT UNIQUE NOT NULL,
                asic_model TEXT,
                name TEXT,
                status TEXT,
                temp REAL,
                hashrate TEXT,
                efficiency TEXT,
                power_draw TEXT,
                max_power REAL,
                uptime TEXT,
                location TEXT,
                last_seen TEXT,
                alerts INTEGER,
                frequency INTEGER,
                fanspeed TEXT,
                best_diff TEXT,
                hostname TEXT,
                power REAL,
                voltage REAL,
                current REAL,
                vr_temp REAL,
                nominal_voltage REAL,
                expected_hashrate REAL,
                pool_difficulty INTEGER,
                is_using_fallback_stratum INTEGER,
                is_psram_available INTEGER,
                free_heap INTEGER,
                core_voltage INTEGER,
                core_voltage_actual INTEGER,
                ssid TEXT,
                wifi_status TEXT,
                wifi_rssi INTEGER,
                ap_enabled INTEGER,
                shares_accepted INTEGER,
                shares_rejected INTEGER,
                uptime_seconds INTEGER,
                small_core_count INTEGER,
                stratum_url TEXT,
                stratum_port INTEGER,
                stratum_user TEXT,
                stratum_suggested_difficulty INTEGER,
                stratum_extranonce_subscribe INTEGER,
                fallback_stratum_url TEXT,
                fallback_stratum_port INTEGER,
                fallback_stratum_user TEXT,
                fallback_stratum_suggested_difficulty INTEGER,
                fallback_stratum_extranonce_subscribe INTEGER,
                response_time REAL,
                version TEXT,
                axe_os_version TEXT,
                idf_version TEXT,
                board_version TEXT,
                running_partition TEXT,
                overheat_mode INTEGER,
                overclock_enabled INTEGER,
                display TEXT,
                rotation INTEGER,
                invert_screen INTEGER,
                display_timeout INTEGER,
                auto_fanspeed INTEGER,
                temp_target INTEGER,
                fan_rpm INTEGER,
                stats_frequency INTEGER,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        Ok(())
    }

    pub fn insert_or_update_miner(&self, miner: &Miner) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT OR REPLACE INTO miner (
                id, mac_addr, asic_model, name, status, temp, hashrate, efficiency, 
                power_draw, max_power, uptime, location, last_seen, alerts, frequency, 
                fanspeed, best_diff, hostname, power, voltage, current, vr_temp, 
                nominal_voltage, expected_hashrate, pool_difficulty, is_using_fallback_stratum, 
                is_psram_available, free_heap, core_voltage, core_voltage_actual, ssid, 
                wifi_status, wifi_rssi, ap_enabled, shares_accepted, shares_rejected, 
                uptime_seconds, small_core_count, stratum_url, stratum_port, stratum_user, 
                stratum_suggested_difficulty, stratum_extranonce_subscribe, fallback_stratum_url, 
                fallback_stratum_port, fallback_stratum_user, fallback_stratum_suggested_difficulty, 
                fallback_stratum_extranonce_subscribe, response_time, version, axe_os_version, 
                idf_version, board_version, running_partition, overheat_mode, overclock_enabled, 
                display, rotation, invert_screen, display_timeout, auto_fanspeed, temp_target, 
                fan_rpm, stats_frequency, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, 
                ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34, 
                ?35, ?36, ?37, ?38, ?39, ?40, ?41, ?42, ?43, ?44, ?45, ?46, ?47, ?48, ?49, ?50, 
                ?51, ?52, ?53, ?54, ?55, ?56, ?57, ?58, ?59, ?60, ?61, ?62, ?63, ?64, 
                CURRENT_TIMESTAMP
            )",
            params![
                miner.id, miner.mac_addr, miner.asic_model, miner.name, miner.status, 
                miner.temp, miner.hashrate, miner.efficiency, miner.power_draw, miner.max_power, 
                miner.uptime, miner.location, miner.last_seen, miner.alerts, miner.frequency, 
                miner.fanspeed, miner.best_diff, miner.hostname, miner.power, miner.voltage, 
                miner.current, miner.vr_temp, miner.nominal_voltage, miner.expected_hashrate, 
                miner.pool_difficulty, miner.is_using_fallback_stratum, miner.is_psram_available, 
                miner.free_heap, miner.core_voltage, miner.core_voltage_actual, miner.ssid, 
                miner.wifi_status, miner.wifi_rssi, miner.ap_enabled, miner.shares_accepted, 
                miner.shares_rejected, miner.uptime_seconds, miner.small_core_count, miner.stratum_url, 
                miner.stratum_port, miner.stratum_user, miner.stratum_suggested_difficulty, 
                miner.stratum_extranonce_subscribe, miner.fallback_stratum_url, miner.fallback_stratum_port, 
                miner.fallback_stratum_user, miner.fallback_stratum_suggested_difficulty, 
                miner.fallback_stratum_extranonce_subscribe, miner.response_time, miner.version, 
                miner.axe_os_version, miner.idf_version, miner.board_version, miner.running_partition, 
                miner.overheat_mode, miner.overclock_enabled, miner.display, miner.rotation, 
                miner.invert_screen, miner.display_timeout, miner.auto_fanspeed, miner.temp_target, 
                miner.fan_rpm, miner.stats_frequency
            ],
        )?;
        Ok(())
    }

    pub fn get_all_miners(&self) -> Result<Vec<Miner>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, mac_addr, asic_model, name, status, temp, hashrate, efficiency, 
             power_draw, max_power, uptime, location, last_seen, alerts, frequency, 
             fanspeed, best_diff, hostname, power, voltage, current, vr_temp, 
             nominal_voltage, expected_hashrate, pool_difficulty, is_using_fallback_stratum, 
             is_psram_available, free_heap, core_voltage, core_voltage_actual, ssid, 
             wifi_status, wifi_rssi, ap_enabled, shares_accepted, shares_rejected, 
             uptime_seconds, small_core_count, stratum_url, stratum_port, stratum_user, 
             stratum_suggested_difficulty, stratum_extranonce_subscribe, fallback_stratum_url, 
             fallback_stratum_port, fallback_stratum_user, fallback_stratum_suggested_difficulty, 
             fallback_stratum_extranonce_subscribe, response_time, version, axe_os_version, 
             idf_version, board_version, running_partition, overheat_mode, overclock_enabled, 
             display, rotation, invert_screen, display_timeout, auto_fanspeed, temp_target, 
             fan_rpm, stats_frequency FROM miner ORDER BY last_seen DESC"
        )?;

        let miner_iter = stmt.query_map([], |row| {
            Ok(Miner {
                id: row.get(0)?,
                mac_addr: row.get(1)?,
                asic_model: row.get(2)?,
                name: row.get(3)?,
                status: row.get(4)?,
                temp: row.get(5)?,
                hashrate: row.get(6)?,
                efficiency: row.get(7)?,
                power_draw: row.get(8)?,
                max_power: row.get(9)?,
                uptime: row.get(10)?,
                location: row.get(11)?,
                last_seen: row.get(12)?,
                alerts: row.get(13)?,
                frequency: row.get(14)?,
                fanspeed: row.get(15)?,
                best_diff: row.get(16)?,
                hostname: row.get(17)?,
                power: row.get(18)?,
                voltage: row.get(19)?,
                current: row.get(20)?,
                vr_temp: row.get(21)?,
                nominal_voltage: row.get(22)?,
                expected_hashrate: row.get(23)?,
                pool_difficulty: row.get(24)?,
                is_using_fallback_stratum: row.get(25)?,
                is_psram_available: row.get(26)?,
                free_heap: row.get(27)?,
                core_voltage: row.get(28)?,
                core_voltage_actual: row.get(29)?,
                ssid: row.get(30)?,
                wifi_status: row.get(31)?,
                wifi_rssi: row.get(32)?,
                ap_enabled: row.get(33)?,
                shares_accepted: row.get(34)?,
                shares_rejected: row.get(35)?,
                uptime_seconds: row.get(36)?,
                small_core_count: row.get(37)?,
                stratum_url: row.get(38)?,
                stratum_port: row.get(39)?,
                stratum_user: row.get(40)?,
                stratum_suggested_difficulty: row.get(41)?,
                stratum_extranonce_subscribe: row.get(42)?,
                fallback_stratum_url: row.get(43)?,
                fallback_stratum_port: row.get(44)?,
                fallback_stratum_user: row.get(45)?,
                fallback_stratum_suggested_difficulty: row.get(46)?,
                fallback_stratum_extranonce_subscribe: row.get(47)?,
                response_time: row.get(48)?,
                version: row.get(49)?,
                axe_os_version: row.get(50)?,
                idf_version: row.get(51)?,
                board_version: row.get(52)?,
                running_partition: row.get(53)?,
                overheat_mode: row.get(54)?,
                overclock_enabled: row.get(55)?,
                display: row.get(56)?,
                rotation: row.get(57)?,
                invert_screen: row.get(58)?,
                display_timeout: row.get(59)?,
                auto_fanspeed: row.get(60)?,
                temp_target: row.get(61)?,
                fan_rpm: row.get(62)?,
                stats_frequency: row.get(63)?,
            })
        })?;

        let mut miners = Vec::new();
        for miner in miner_iter {
            miners.push(miner?);
        }
        Ok(miners)
    }

    pub fn get_miner_by_id(&self, id: &str) -> Result<Option<Miner>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, mac_addr, asic_model, name, status, temp, hashrate, efficiency, 
             power_draw, max_power, uptime, location, last_seen, alerts, frequency, 
             fanspeed, best_diff, hostname, power, voltage, current, vr_temp, 
             nominal_voltage, expected_hashrate, pool_difficulty, is_using_fallback_stratum, 
             is_psram_available, free_heap, core_voltage, core_voltage_actual, ssid, 
             wifi_status, wifi_rssi, ap_enabled, shares_accepted, shares_rejected, 
             uptime_seconds, small_core_count, stratum_url, stratum_port, stratum_user, 
             stratum_suggested_difficulty, stratum_extranonce_subscribe, fallback_stratum_url, 
             fallback_stratum_port, fallback_stratum_user, fallback_stratum_suggested_difficulty, 
             fallback_stratum_extranonce_subscribe, response_time, version, axe_os_version, 
             idf_version, board_version, running_partition, overheat_mode, overclock_enabled, 
             display, rotation, invert_screen, display_timeout, auto_fanspeed, temp_target, 
             fan_rpm, stats_frequency FROM miner WHERE id = ?1"
        )?;

        let mut miner_iter = stmt.query_map([id], |row| {
            Ok(Miner {
                id: row.get(0)?,
                mac_addr: row.get(1)?,
                asic_model: row.get(2)?,
                name: row.get(3)?,
                status: row.get(4)?,
                temp: row.get(5)?,
                hashrate: row.get(6)?,
                efficiency: row.get(7)?,
                power_draw: row.get(8)?,
                max_power: row.get(9)?,
                uptime: row.get(10)?,
                location: row.get(11)?,
                last_seen: row.get(12)?,
                alerts: row.get(13)?,
                frequency: row.get(14)?,
                fanspeed: row.get(15)?,
                best_diff: row.get(16)?,
                hostname: row.get(17)?,
                power: row.get(18)?,
                voltage: row.get(19)?,
                current: row.get(20)?,
                vr_temp: row.get(21)?,
                nominal_voltage: row.get(22)?,
                expected_hashrate: row.get(23)?,
                pool_difficulty: row.get(24)?,
                is_using_fallback_stratum: row.get(25)?,
                is_psram_available: row.get(26)?,
                free_heap: row.get(27)?,
                core_voltage: row.get(28)?,
                core_voltage_actual: row.get(29)?,
                ssid: row.get(30)?,
                wifi_status: row.get(31)?,
                wifi_rssi: row.get(32)?,
                ap_enabled: row.get(33)?,
                shares_accepted: row.get(34)?,
                shares_rejected: row.get(35)?,
                uptime_seconds: row.get(36)?,
                small_core_count: row.get(37)?,
                stratum_url: row.get(38)?,
                stratum_port: row.get(39)?,
                stratum_user: row.get(40)?,
                stratum_suggested_difficulty: row.get(41)?,
                stratum_extranonce_subscribe: row.get(42)?,
                fallback_stratum_url: row.get(43)?,
                fallback_stratum_port: row.get(44)?,
                fallback_stratum_user: row.get(45)?,
                fallback_stratum_suggested_difficulty: row.get(46)?,
                fallback_stratum_extranonce_subscribe: row.get(47)?,
                response_time: row.get(48)?,
                version: row.get(49)?,
                axe_os_version: row.get(50)?,
                idf_version: row.get(51)?,
                board_version: row.get(52)?,
                running_partition: row.get(53)?,
                overheat_mode: row.get(54)?,
                overclock_enabled: row.get(55)?,
                display: row.get(56)?,
                rotation: row.get(57)?,
                invert_screen: row.get(58)?,
                display_timeout: row.get(59)?,
                auto_fanspeed: row.get(60)?,
                temp_target: row.get(61)?,
                fan_rpm: row.get(62)?,
                stats_frequency: row.get(63)?,
            })
        })?;

        match miner_iter.next() {
            Some(miner) => Ok(Some(miner?)),
            None => Ok(None),
        }
    }

    pub fn delete_miner(&self, id: &str) -> Result<usize> {
        let conn = self.get_connection()?;
        let affected_rows = conn.execute("DELETE FROM miner WHERE id = ?1", [id])?;
        Ok(affected_rows)
    }
}

// Implement Send and Sync for Database manually
unsafe impl Send for Database {}
unsafe impl Sync for Database {}
