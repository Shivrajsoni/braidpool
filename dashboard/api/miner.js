import express from 'express';
import fetch from 'node-fetch';
import cors from 'cors';

const app = express();
const PORT = 5000;
const RPC_URL = 'http://127.0.0.1:6682';

app.use(cors());
app.use(express.json());

async function makeRPCCall(method, params = []) {
  try {
    const response = await fetch(RPC_URL, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        jsonrpc: '2.0',
        method: method,
        params: params,
        id: Date.now()
      })
    });

    if (!response.ok) {
      throw new Error(`RPC call failed: ${response.statusText}`);
    }

    const result = await response.json();
    if (result.error) {
      throw new Error(`RPC error: ${result.error.message || result.error}`);
    }

    return result.result;
  } catch (error) {
    console.error(`RPC call error for ${method}:`, error);
    throw error;
  }
}

function getDeviceStatus(data, reachable = true) {
  if (!reachable) return 'offline';
  
  // Critical issues
  if (data.overheat_mode) return 'warning';
  if ((data.temp || 0) > 80) return 'warning';
  if ((data.power || 0) < 3) return 'warning'; // Too low power
  if (!data.stratum_url) return 'warning'; // No pool
  
  const accepted = data.shares_accepted || 0;
  const rejected = data.shares_rejected || 0;
  if (accepted > 0 && rejected / (accepted + rejected) > 0.1) return 'warning';
  
  return 'online'; // Hashrate can be 0 and still be online!
}

function transformMinerData(bitaxeData, ip, reachable = true) {
  const id = `${bitaxeData.mac_addr || 'unknown'}-${ip}`;
  const status = getDeviceStatus(bitaxeData, reachable);
  
  return {
    frontendData: {
      id: id,
      name: bitaxeData.hostname || 'Unknown',
      status: status,
      temp: bitaxeData.temp || 0,
      hashrate: (bitaxeData.hashRate || 0).toFixed(2),
      efficiency: (bitaxeData.hashRate / bitaxeData.power || 0).toFixed(2),
      powerDraw: (bitaxeData.power || 0).toFixed(2),
      maxPower: bitaxeData.max_power || 0,
      uptime: `${Math.floor((bitaxeData.uptimeSeconds || 0) / 60)} min`,
      location: `IP: ${ip}`,
      lastSeen: new Date().toLocaleTimeString(),
      alerts: status === 'warning' ? 1 : status === 'offline' ? 1 : 0,
      frequency: bitaxeData.frequency || 0,
      fanspeed: bitaxeData.fanspeed || 0,
      bestDiff: bitaxeData.bestDiff || 'N/A',
      ASICModel: bitaxeData.ASICModel || 'Unknown'
    },
    
    rpcData: {
      id: id,
      mac_addr: bitaxeData.mac_addr || 'unknown',
      asic_model: bitaxeData.ASICModel || 'Unknown',
      name: bitaxeData.hostname || 'Unknown',
      status: status,
      temp: bitaxeData.temp || 0,
      hashrate: (bitaxeData.hashRate || 0).toString(),
      efficiency: (bitaxeData.hashRate / bitaxeData.power || 0).toString(),
      power_draw: (bitaxeData.power || 0).toString(),
      max_power: bitaxeData.max_power || 0,
      uptime: `${Math.floor((bitaxeData.uptimeSeconds || 0) / 60)} min`,
      location: `IP: ${ip}`,
      last_seen: new Date().toLocaleTimeString(),
      alerts: status === 'warning' ? 1 : status === 'offline' ? 1 : 0,
      frequency: bitaxeData.frequency || 0,
      fanspeed: bitaxeData.fanspeed?.toString() || '0',
      best_diff: bitaxeData.bestDiff || 'N/A',
      hostname: bitaxeData.hostname || 'Unknown',
      power: bitaxeData.power || 0,
      voltage: bitaxeData.voltage || 0,
      current: bitaxeData.current || 0,
      vr_temp: bitaxeData.vr_temp || 0,
      nominal_voltage: bitaxeData.nominal_voltage || 0,
      expected_hashrate: bitaxeData.expected_hashrate || 0,
      pool_difficulty: bitaxeData.pool_difficulty || 0,
      is_using_fallback_stratum: bitaxeData.is_using_fallback_stratum || 0,
      is_psram_available: bitaxeData.is_psram_available || 0,
      free_heap: bitaxeData.free_heap || 0,
      core_voltage: bitaxeData.core_voltage || 0,
      core_voltage_actual: bitaxeData.core_voltage_actual || 0,
      ssid: bitaxeData.ssid || '',
      wifi_status: bitaxeData.wifi_status || 'Unknown',
      wifi_rssi: bitaxeData.wifi_rssi || 0,
      ap_enabled: bitaxeData.ap_enabled || 0,
      shares_accepted: bitaxeData.shares_accepted || 0,
      shares_rejected: bitaxeData.shares_rejected || 0,
      uptime_seconds: bitaxeData.uptimeSeconds || 0,
      small_core_count: bitaxeData.small_core_count || 0,
      stratum_url: bitaxeData.stratum_url || '',
      stratum_port: bitaxeData.stratum_port || 0,
      stratum_user: bitaxeData.stratum_user || '',
      stratum_suggested_difficulty: bitaxeData.stratum_suggested_difficulty || 0,
      stratum_extranonce_subscribe: bitaxeData.stratum_extranonce_subscribe || 0,
      fallback_stratum_url: bitaxeData.fallback_stratum_url || '',
      fallback_stratum_port: bitaxeData.fallback_stratum_port || 0,
      fallback_stratum_user: bitaxeData.fallback_stratum_user || '',
      fallback_stratum_suggested_difficulty: bitaxeData.fallback_stratum_suggested_difficulty || 0,
      fallback_stratum_extranonce_subscribe: bitaxeData.fallback_stratum_extranonce_subscribe || 0,
      response_time: bitaxeData.response_time || 0,
      version: bitaxeData.version || '',
      axe_os_version: bitaxeData.axe_os_version || '',
      idf_version: bitaxeData.idf_version || '',
      board_version: bitaxeData.board_version || '',
      running_partition: bitaxeData.running_partition || '',
      overheat_mode: bitaxeData.overheat_mode || 0,
      overclock_enabled: bitaxeData.overclock_enabled || 0,
      display: bitaxeData.display || '',
      rotation: bitaxeData.rotation || 0,
      invert_screen: bitaxeData.invert_screen || 0,
      display_timeout: bitaxeData.display_timeout || -1,
      auto_fanspeed: bitaxeData.auto_fanspeed || 0,
      temp_target: bitaxeData.temp_target || 0,
      fan_rpm: bitaxeData.fan_rpm || 0,
      stats_frequency: bitaxeData.stats_frequency || 0
    }
  };
}

// Get all miners
app.get('/api/miners', async (req, res) => {
  try {
    const result = await makeRPCCall('getminers');
    const miners = JSON.parse(result || '[]');
    
    const frontendMiners = miners.map(miner => ({
      id: miner.id,
      name: miner.name,
      status: miner.status,
      temp: miner.temp,
      hashrate: miner.hashrate,
      efficiency: miner.efficiency,
      powerDraw: miner.power_draw,
      maxPower: miner.max_power,
      uptime: miner.uptime,
      location: miner.location,
      lastSeen: miner.last_seen,
      alerts: miner.alerts,
      frequency: miner.frequency,
      fanspeed: miner.fanspeed,
      bestDiff: miner.best_diff,
      ASICModel: miner.asic_model
    }));
    
    res.json(frontendMiners);
  } catch (error) {
    console.error('Error fetching miners:', error);
    res.status(500).json({ error: 'Failed to fetch miners' });
  }
});

// Add/Update miner
app.post('/api/miners', async (req, res) => {
  const { ip } = req.body;
  
  if (!ip) {
    return res.status(400).json({ error: 'IP required' });
  }

  try {
    const controller = new AbortController();
    setTimeout(() => controller.abort(), 5000);
    
    const response = await fetch(`http://${ip}/api/system/info`, { signal: controller.signal });
    if (!response.ok) throw new Error(`Device returned ${response.status}`);
    
    const bitaxeData = await response.json();
    const { frontendData, rpcData } = transformMinerData(bitaxeData, ip, true);
    
    await makeRPCCall('addminer', [JSON.stringify(rpcData)]);
    res.json(frontendData);
    
  } catch (error) {
    console.error(`Failed to add miner ${ip}:`, error);
    res.status(500).json({ error: `Could not connect to ${ip}` });
  }
});

// Get specific miner
app.get('/api/miners/:id', async (req, res) => {
  try {
    const result = await makeRPCCall('getminer', [req.params.id]);
    const miner = JSON.parse(result || 'null');
    
    if (!miner) {
      return res.status(404).json({ error: 'Miner not found' });
    }
    
    res.json({
      id: miner.id,
      name: miner.name,
      status: miner.status,
      temp: miner.temp,
      hashrate: miner.hashrate,
      efficiency: miner.efficiency,
      powerDraw: miner.power_draw,
      maxPower: miner.max_power,
      uptime: miner.uptime,
      location: miner.location,
      lastSeen: miner.last_seen,
      alerts: miner.alerts,
      frequency: miner.frequency,
      fanspeed: miner.fanspeed,
      bestDiff: miner.best_diff,
      ASICModel: miner.asic_model
    });
  } catch (error) {
    console.error(`Error fetching miner:`, error);
    res.status(500).json({ error: 'Failed to fetch miner' });
  }
});

// Delete miner
app.delete('/api/miners/:id', async (req, res) => {
  try {
    await makeRPCCall('deleteminer', [req.params.id]);
    res.json({ message: 'Deleted' });
  } catch (error) {
    console.error(`Error deleting miner:`, error);
    res.status(500).json({ error: 'Failed to delete' });
  }
});

app.post('/api/miners/sync', async (req, res) => {
  try {
    const result = await makeRPCCall('getminers');
    const storedMiners = JSON.parse(result || '[]');
    const syncResults = [];
    
    for (const miner of storedMiners) {
      try {
        const ipMatch = miner.location.match(/IP: (.+)/);
        if (!ipMatch) continue;
        
        const ip = ipMatch[1];
        const controller = new AbortController();
        setTimeout(() => controller.abort(), 5000);
        
        const response = await fetch(`http://${ip}/api/system/info`, { signal: controller.signal });
        
        if (!response.ok) throw new Error('Not reachable');
        
        const bitaxeData = await response.json();
        const { rpcData } = transformMinerData(bitaxeData, ip, true);
        
        await makeRPCCall('addminer', [JSON.stringify(rpcData)]);
        syncResults.push({ id: miner.id, status: 'updated' });
        
      } catch (error) {
        const { last_seen, ...rest } = miner;
        const offlineData = { 
          ...rest, 
          status: 'offline',
          last_seen // keep the previous last_seen value
        };
        await makeRPCCall('addminer', [JSON.stringify(offlineData)]);
        syncResults.push({ id: miner.id, status: 'offline' });
      }
    }
    
    res.json({ message: 'Sync completed', results: syncResults });
  } catch (error) {
    console.error('Sync error:', error);
    res.status(500).json({ error: 'Sync failed' });
  }
});

app.listen(PORT, () => {
  console.log(`Server running on http://localhost:${PORT}`);
});