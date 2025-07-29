# Braidpool Node RPC API

This directory contains the RPC server implementation and documentation for the Braidpool node.

## Files

- `src/rpc_server.rs` - Main RPC server implementation
- `RPC_API_DOCUMENTATION.md` - Comprehensive API documentation for all endpoints

## Available RPC Endpoints

1. **`getbead`** - Retrieve a specific bead by hash
2. **`addbead`** - Add a new bead to the braid
3. **`gettips`** - Get current DAG tip beads
4. **`getgeneses`** - Get genesis beads (root nodes)
5. **`getbeadcount`** - Get total number of beads
6. **`getcohortcount`** - Get total number of cohorts
7. **`getbeadsincohort`** - Get beads from a specific cohort

## Quick Start

1. Start the node server:
   ```bash
   ./node
   ```

2. The RPC server will be available at `http://127.0.0.1:6682`

3. Test with curl:
   ```bash
   curl -X POST http://127.0.0.1:6682 \
     -H "Content-Type: application/json" \
     -d '{"jsonrpc": "2.0", "method": "getbeadcount", "params": [], "id": 1}'
   ```

For detailed documentation, see [RPC_API_DOCUMENTATION.md](./RPC_API_DOCUMENTATION.md).
