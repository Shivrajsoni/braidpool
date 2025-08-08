import { useState, useEffect } from 'react';
import { Miner, MinerStatus } from './Types';


const DeviceCard = ({
  miner,
  onActivateLight,
  onDelete
}: {
  miner: Miner;
  onActivateLight: (id: string) => void;
  onDelete: (id: string) => void;
}) => {
  const getStatusColor = (status: MinerStatus) => {
    switch (status) {
      case 'online':
        return 'bg-green-500';
      case 'warning':
        return 'bg-yellow-400';
      case 'offline':
        return 'bg-red-500';
      default:
        return 'bg-gray-500';
    }
  };

  const statusColor = getStatusColor(miner.status);

  return (
    <div className="relative w-full max-w-[360px]  border border-gray-700 rounded-xl p-5 backdrop-blur-sm transition-transform duration-200 hover:-translate-y-1 hover:shadow-xl">
      <div
        className={`absolute top-3 right-3 w-3 h-3 rounded-full ${statusColor}`}
      />
      {miner.alerts > 0 && (
        <div className="absolute top-2 right-10 px-2 py-0.5 text-xs rounded-full bg-red-100 dark:bg-red-900 text-red-700 dark:text-red-300">
          ⚠ {miner.alerts}
        </div>
      )}

      <h3 className="text-lg font-semibold text-white mb-1">{miner.name}</h3>
      <div className="flex justify-between text-xs text-gray-400 mb-3">
        <span>Location: {miner.location}</span>
        <span>Last Seen: {miner.lastSeen}</span>
      </div>

      <div className="text-sm text-gray-300 space-y-2 mb-4">
        <div className="flex justify-between">
          <span>Hashrate: {Number(miner.hashrate).toFixed(2)} GH/s</span>
          <span>Best Difficulty {miner.bestDiff}</span>
        </div>
        <div className="flex justify-between">
          <span>ASICModel: {miner.ASICModel}</span>
          <span>Uptime: {miner.uptime}</span>
        </div>
        <div className="flex justify-between">
          <span>Power: {Number(miner.powerDraw).toFixed(2)} W</span>
          <span>MaxPower: {miner.maxPower} W</span>
        </div>
        <div className="flex justify-between">
          <span>Efficiency: {Number(miner.efficiency).toFixed(2)}</span>
          <span>Frequency: {miner.frequency}MHz</span>
        </div>
        <div className="flex justify-between">
          <span>FanSpeed: {miner.fanspeed}</span>
          <span>Temp: {miner.temp}°C</span>
        </div>
      </div>
    </div>
  );
};

const MinerInventoryDashboard = () => {
  const [miners, setMiners] = useState<Miner[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [activeLight, setActiveLight] = useState<string | null>(null);
  const [newMinerIP, setNewMinerIP] = useState('');
  const [syncing, setSyncing] = useState(false);

  const API_BASE_URL = 'http://localhost:5000';

  // Fetch all miners from RPC storage
  const fetchMiners = async () => {
    try {
      const response = await fetch(`${API_BASE_URL}/api/miners`, {
        headers: { Accept: 'application/json' },
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const data = await response.json();
      setMiners(Array.isArray(data) ? data : []);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error occurred');
      console.error('Error fetching miners:', err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchMiners();
    // Fetch every 30 seconds
    const interval = setInterval(fetchMiners, 30000);
    return () => clearInterval(interval);
  }, []);

  const handleActivateLight = (id: string) => {
    setActiveLight(id);
    console.log(`Activating locate light for miner ${id}`);
    setTimeout(() => setActiveLight(null), 5000);
  };

  const addMinerByIP = async () => {
    if (!newMinerIP.trim()) {
      alert('Please enter a valid IP address');
      return;
    }

    try {
      const response = await fetch(`${API_BASE_URL}/api/miners`, {
        method: 'POST',
        headers: { 
          'Content-Type': 'application/json',
          'Accept': 'application/json' 
        },
        body: JSON.stringify({ ip: newMinerIP.trim() })
      });

      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.error || `HTTP ${response.status}`);
      }

      const newMiner = await response.json();
      console.log('Miner added successfully:', newMiner);
      
      // Refresh the miners list
      await fetchMiners();
      setNewMinerIP('');
      
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Unknown error';
      alert(`Failed to add miner: ${errorMsg}`);
      console.error('Add miner error:', err);
    }
  };

  const deleteMiner = async (id: string) => {
    if (!confirm('Are you sure you want to remove this miner?')) {
      return;
    }

    try {
      const response = await fetch(`${API_BASE_URL}/api/miners/${encodeURIComponent(id)}`, {
        method: 'DELETE',
      });

      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.error || `HTTP ${response.status}`);
      }

      // Refresh the miners list
      await fetchMiners();
      console.log(`Miner ${id} deleted successfully`);
      
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Unknown error';
      alert(`Failed to delete miner: ${errorMsg}`);
      console.error('Delete miner error:', err);
    }
  };

  const syncAllMiners = async () => {
    setSyncing(true);
    try {
      const response = await fetch(`${API_BASE_URL}/api/miners/sync`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
      });

      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.error || `HTTP ${response.status}`);
      }

      const result = await response.json();
      console.log('Sync completed:', result);
      
      // Refresh the miners list
      await fetchMiners();
      
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Unknown error';
      alert(`Sync failed: ${errorMsg}`);
      console.error('Sync error:', err);
    } finally {
      setSyncing(false);
    }
  };

 

  const totalMiners = miners.length;
  const onlineMiners = miners.filter((m) => m.status === 'online').length;
  const warningMiners = miners.filter((m) => m.status === 'warning').length;
  const offlineMiners = miners.filter((m) => m.status === 'offline').length;

  if (loading) {
    return (
      <div className="flex justify-center items-center py-8 gap-3">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
        <h2 className="text-white">Loading miner data from storage...</h2>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-red-900 border border-red-700 text-red-100 px-4 py-3 rounded relative" role="alert">
        <strong className="font-bold">Error: </strong>
        <span className="block sm:inline">{error}</span>
        <button 
          onClick={fetchMiners} 
          className="mt-2 px-3 py-1 bg-red-700 hover:bg-red-600 rounded text-sm"
        >
          Retry
        </button>
      </div>
    );
  }

  return (
    <>
      <div className="text-center mb-8">
        <p className="text-base text-gray-400 mt-1">
          Status of all mining devices (stored in RPC database)
        </p>

        <div className="flex justify-center items-center gap-2 mt-4 flex-wrap">
          <input
            type="text"
            value={newMinerIP}
            onChange={(e) => setNewMinerIP(e.target.value)}
            placeholder="Enter miner IP (e.g. 192.168.1.100)"
            className="px-3 py-1 text-sm bg-gray-800 border border-gray-600 rounded text-white placeholder-gray-400 w-64"
            onKeyPress={(e) => e.key === 'Enter' && addMinerByIP()}
          />
          <button
            onClick={addMinerByIP}
            className="px-3 py-1 text-sm bg-gray-800 hover:bg-blue-700 text-white rounded"
          >
            Add Miner
          </button>
          <button
            onClick={syncAllMiners}
            disabled={syncing}
            className="px-3 py-1 text-sm bg-gray-800  hover:bg-green-700 disabled:bg-green-800 text-white rounded"
          >
            {syncing ? 'Syncing...' : 'Sync All'}
          </button>
          
        </div>

        <div className="flex flex-wrap justify-center gap-3 mt-6 text-sm">
          <div className="px-4 py-1 rounded-md border border-gray-600 text-green-400">
            {onlineMiners} Online
          </div>
          <div className="px-4 py-1 rounded-md border border-gray-600 text-yellow-400">
            {warningMiners} Warning
          </div>
          <div className="px-4 py-1 rounded-md border border-gray-600 text-red-400">
            {offlineMiners} Offline
          </div>
          <div className="px-4 py-1 rounded-md border border-gray-600 text-blue-400">
            {totalMiners} Total
          </div>
        </div>
      </div>

      {miners.length === 0 ? (
        <div className="text-center py-8 text-gray-500">
          <p>No mining devices found in storage.</p>
          <p className="text-sm mt-2">Add miners by IP address or check your braidpool RPC server.</p>
        </div>
      ) : (
        <div className="flex overflow-x-auto space-x-4 pb-4 ml-6">
          {miners.map((miner) => (
            <DeviceCard
              key={miner.id}
              miner={miner}
              onActivateLight={handleActivateLight}
              onDelete={deleteMiner}
            />
          ))}
        </div>
      )}
    </>
  );
};

export default MinerInventoryDashboard;