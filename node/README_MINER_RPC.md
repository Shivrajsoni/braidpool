# Miner Management RPC Endpoints

This document describes the new RPC endpoints for managing miner data in the braidpool node.

## Database

A SQLite database named `database.db3` is automatically created when the RPC server starts. The database contains a `miner` table with all the fields from the BitAxe device information.

## RPC Endpoints

### 1. Add or Update Miner

**Method:** `addminer`

**Description:** Adds a new miner or updates an existing one (based on MAC address as primary key).

**Example Request:**
```bash
curl -X POST http://127.0.0.1:6682 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "addminer",
    "params": ["{\"id\":\"00:11:22:33:44:55-192.168.1.100\",\"mac_addr\":\"00:11:22:33:44:55\",\"asic_model\":\"BM1370\",\"name\":\"bitaxe\",\"status\":\"online\",\"temp\":68.75,\"hashrate\":\"843.95\",\"efficiency\":\"43.87\",\"power_draw\":\"19.25\",\"max_power\":40.0,\"uptime\":\"3 min\",\"location\":\"IP: 192.168.1.100\",\"last_seen\":\"11:27 AM\",\"alerts\":0,\"frequency\":400,\"fanspeed\":\"100\",\"best_diff\":\"28.03M\",\"hostname\":\"bitaxe\",\"power\":19.2547607421875,\"voltage\":5132.8125,\"current\":13515.625,\"vr_temp\":68.0,\"nominal_voltage\":5.0,\"expected_hashrate\":816.0,\"pool_difficulty\":4096,\"is_using_fallback_stratum\":0,\"is_psram_available\":1,\"free_heap\":8403524,\"core_voltage\":1060,\"core_voltage_actual\":1043,\"ssid\":\"jio\",\"wifi_status\":\"Connected!\",\"wifi_rssi\":-62,\"ap_enabled\":0,\"shares_accepted\":10,\"shares_rejected\":0,\"uptime_seconds\":180,\"small_core_count\":2040,\"stratum_url\":\"public-pool.io\",\"stratum_port\":21496,\"stratum_user\":\"\",\"stratum_suggested_difficulty\":1000,\"stratum_extranonce_subscribe\":0,\"fallback_stratum_url\":\"solo.ckpool.org\",\"fallback_stratum_port\":3333,\"fallback_stratum_user\":\"\",\"fallback_stratum_suggested_difficulty\":1000,\"fallback_stratum_extranonce_subscribe\":0,\"response_time\":364.972,\"version\":\"v2.9.0\",\"axe_os_version\":\"v2.9.0\",\"idf_version\":\"v5.4.1\",\"board_version\":\"602\",\"running_partition\":\"ota_0\",\"overheat_mode\":0,\"overclock_enabled\":0,\"display\":\"SSD1306 (128x32)\",\"rotation\":0,\"invert_screen\":0,\"display_timeout\":-1,\"auto_fanspeed\":0,\"temp_target\":65,\"fan_rpm\":7307,\"stats_frequency\":0}"],
    "id": 1
  }'
```

**Example Response:**
```json
{
  "jsonrpc": "2.0",
  "result": "Miner added/updated successfully",
  "id": 1
}
```

### 2. Get All Miners

**Method:** `getminers`

**Description:** Retrieves all miners from the database.

**Example Request:**
```bash
curl -X POST http://127.0.0.1:6682 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "getminers",
    "params": [],
    "id": 1
  }'
```

**Example Response:**
```json
{
  "jsonrpc": "2.0",
  "result": "[{\"id\":\"00:11:22:33:44:55-192.168.1.100\",\"mac_addr\":\"00:11:22:33:44:55\",\"asic_model\":\"BM1370\",\"name\":\"bitaxe\",\"status\":\"online\",\"temp\":68.75,\"hashrate\":\"843.95\",\"efficiency\":\"43.87\",\"power_draw\":\"19.25\",\"max_power\":40.0,\"uptime\":\"3 min\",\"location\":\"IP: 192.168.1.100\",\"last_seen\":\"11:27 AM\",\"alerts\":0,\"frequency\":400,\"fanspeed\":\"100\",\"best_diff\":\"28.03M\",\"hostname\":\"bitaxe\",\"power\":19.2547607421875,\"voltage\":5132.8125,\"current\":13515.625,\"vr_temp\":68.0,\"nominal_voltage\":5.0,\"expected_hashrate\":816.0,\"pool_difficulty\":4096,\"is_using_fallback_stratum\":0,\"is_psram_available\":1,\"free_heap\":8403524,\"core_voltage\":1060,\"core_voltage_actual\":1043,\"ssid\":\"jio\",\"wifi_status\":\"Connected!\",\"wifi_rssi\":-62,\"ap_enabled\":0,\"shares_accepted\":10,\"shares_rejected\":0,\"uptime_seconds\":180,\"small_core_count\":2040,\"stratum_url\":\"public-pool.io\",\"stratum_port\":21496,\"stratum_user\":\"\",\"stratum_suggested_difficulty\":1000,\"stratum_extranonce_subscribe\":0,\"fallback_stratum_url\":\"solo.ckpool.org\",\"fallback_stratum_port\":3333,\"fallback_stratum_user\":\"\",\"fallback_stratum_suggested_difficulty\":1000,\"fallback_stratum_extranonce_subscribe\":0,\"response_time\":364.972,\"version\":\"v2.9.0\",\"axe_os_version\":\"v2.9.0\",\"idf_version\":\"v5.4.1\",\"board_version\":\"602\",\"running_partition\":\"ota_0\",\"overheat_mode\":0,\"overclock_enabled\":0,\"display\":\"SSD1306 (128x32)\",\"rotation\":0,\"invert_screen\":0,\"display_timeout\":-1,\"auto_fanspeed\":0,\"temp_target\":65,\"fan_rpm\":7307,\"stats_frequency\":0}]",
  "id": 1
}
```

### 3. Get Specific Miner

**Method:** `getminer`

**Description:** Retrieves a specific miner by ID.

**Example Request:**
```bash
curl -X POST http://127.0.0.1:6682 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "getminer",
    "params": ["00:11:22:33:44:55-192.168.1.100"],
    "id": 1
  }'
```

### 4. Delete Miner

**Method:** `deleteminer`

**Description:** Deletes a miner from the database.

**Example Request:**
```bash
curl -X POST http://127.0.0.1:6682 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "deleteminer",
    "params": ["00:11:22:33:44:55-192.168.1.100"],
    "id": 1
  }'
```

**Example Response:**
```json
{
  "jsonrpc": "2.0",
  "result": "Miner deleted successfully",
  "id": 1
}
```

## Miner Data Structure

The miner data structure includes all fields from the BitAxe device response:

- **id**: Primary key (combination of MAC address and IP)
- **mac_addr**: MAC address (unique identifier)
- **asic_model**: ASIC model (e.g., "BM1370")
- **name**: Device hostname
- **status**: Device status ("online", "offline", "warning")
- **temp**: Temperature in Celsius
- **hashrate**: Current hashrate as string
- **efficiency**: Power efficiency (hashrate/power)
- **power_draw**: Current power consumption
- **max_power**: Maximum power rating
- **uptime**: Human-readable uptime
- **location**: Device location (IP address)
- **last_seen**: Last seen timestamp
- **alerts**: Number of alerts
- **frequency**: Clock frequency
- **fanspeed**: Fan speed percentage
- **best_diff**: Best difficulty achieved
- And many more fields from the original BitAxe response...

## Integration with Frontend

The frontend can use these endpoints to:

1. **POST new miner data** when a miner is discovered or updated
2. **GET all miners** to populate the dashboard
3. **GET specific miner** for detailed views
4. **DELETE miners** when they're inactive for too long

The workflow described in the conversation would be:
1. Frontend fetches miner data from BitAxe device
2. Frontend transforms the data to match the expected structure
3. Frontend POSTs the miner data to the `addminer` endpoint
4. Frontend GETs all miners from the `getminers` endpoint to update the dashboard

## Running the Server

To start the RPC server with miner endpoints:

```bash
cd /home/aritra/braidpool/braidpool/node
cargo run
```

The server will start on `http://127.0.0.1:6682` and automatically create the SQLite database.
