# Task 9: Cross-Site Linking + Token Badges

## Overview
Add consistent token badges and transaction linking across miner dashboard, status explorer, and faucet UI.

## Components to Modify

### 1. Miner Dashboard
**Location:** `gateway.tribewarez.com/testnet.rpc.gateway.tribewarez.com/.../miner/views/index.html`

**Changes:**
- Add click handlers to transaction hashes
- Link to validator's `/api/tx/:hash` endpoint
- Style transaction hashes as clickable links (underline, pointer cursor)

```html
<a href="#" onclick="viewTransaction('${txHash}'); return false;">
  ${txHash.substring(0, 8)}...
</a>

<script>
function viewTransaction(txHash) {
  const validatorUrl = 'http://localhost:8900'; // or configurable
  fetch(`${validatorUrl}/api/tx/${txHash}`)
    .then(r => r.json())
    .then(tx => {
      // Display modal or redirect to explorer
      console.log('TX Details:', tx);
    });
}
</script>
```

### 2. Status Explorer
**Location:** `gateway.tribewarez.com/testnet.rpc.gateway.tribewarez.com/.../status/...`

**Changes:**
- Add token type badges with consistent colors
- Display next to amounts in transactions/balances
- Implement CSS classes for token styling

```html
<!-- Token Badge Component -->
<span class="token-badge token-tribe">TRIBE</span>
<span class="token-badge token-aum">AUM</span>
<span class="token-badge token-stomp">STOMP</span>
<span class="token-badge token-ravecoin">RAVECOIN</span>

<style>
.token-badge {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 4px;
  font-weight: bold;
  font-size: 0.85em;
}

.token-tribe {
  background-color: #0066FF;
  color: white;
}

.token-aum {
  background-color: #FFD700;
  color: black;
}

.token-stomp {
  background-color: #FF8C00;
  color: white;
}

.token-ravecoin {
  background-color: #9933FF;
  color: white;
}
</style>
```

**Integration:**
- Show token badges in transaction lists
- Display in balance queries (e.g., `/api/token/balance/:address/:token_type`)
- Add to supply queries via `/api/supply` endpoint

### 3. Faucet UI
**Location:** `gateway.tribewarez.com/testnet.rpc.gateway.tribewarez.com/.../faucet/index.html`

**Changes:**
- Add token icons/color swatches
- Show claim amounts per token
- Display transaction confirmation with link

```html
<!-- Token Selector -->
<div class="token-selector">
  <button class="token-button tribe" data-token="TRIBE">
    🪙 TRIBE (1.0)
  </button>
  <button class="token-button aum" data-token="AUM">
    ✨ AUM (100)
  </button>
  <button class="token-button stomp" data-token="STOMP">
    🔥 STOMP (1000)
  </button>
  <button class="token-button ravecoin" data-token="RAVECOIN">
    🎵 RAVECOIN (100)
  </button>
</div>

<!-- Transaction Confirmation -->
<div id="claim-result" style="display:none;">
  <p>Claimed <span id="claim-amount"></span> <span id="claim-token"></span></p>
  <p>TX Hash: <a href="#" id="tx-link"></a></p>
</div>

<style>
.token-button {
  margin: 5px;
  padding: 10px 20px;
  border: none;
  border-radius: 6px;
  font-weight: bold;
  cursor: pointer;
}

.token-button.tribe { background-color: #0066FF; color: white; }
.token-button.aum { background-color: #FFD700; color: black; }
.token-button.stomp { background-color: #FF8C00; color: white; }
.token-button.ravecoin { background-color: #9933FF; color: white; }
</style>
```

## API Integration

### Endpoints Used

1. **Supply Endpoint** (Task 6)
   ```
   GET /api/supply
   Returns: { "TRIBE": 1234567, "AUM": 5000000, ... }
   ```

2. **Transaction Detail Endpoint** (Task 8)
   ```
   GET /api/tx/:hash
   Returns: { 
     "tx_hash": "...",
     "from": "...",
     "to": "...", 
     "token": "STOMP",
     "amount": 1000,
     "fee": 10,
     "block_height": 123,
     "timestamp": 1234567890 
   }
   ```

3. **Token Balance Endpoint** (existing)
   ```
   GET /token/balance/:address/:token_type
   ```

## Consistent Naming

Across all sites, use:
- `TRIBE` - native chain token
- `AUM` - AUM token
- `STOMP` - STOMP token  
- `RAVECOIN` - RAVECOIN token
- `PTtC` - Pumped TRIBE test coin
- `NMTC` - Numerologic Master Coin
- `AI3` - AI3 token

## Color Scheme

| Token | Color | Hex |
|-------|-------|-----|
| TRIBE | Blue | #0066FF |
| AUM | Gold | #FFD700 |
| STOMP | Orange | #FF8C00 |
| RAVECOIN | Purple | #9933FF |

## Testing Checklist

- [ ] Miner dashboard shows transaction hashes as links
- [ ] Clicking transaction hash opens `/api/tx/:hash` details
- [ ] Status explorer shows token badges next to amounts
- [ ] Faucet buttons allow selecting token type
- [ ] Faucet claim shows token badge + claim amount
- [ ] Token colors consistent across all three sites
- [ ] Supply endpoint returns all token types
- [ ] Transaction detail endpoint returns complete TX info

## Notes

- Validator API runs on port 8900 (or configurable via env)
- CORS may need to be enabled on validator for cross-origin requests
- Cache token list periodically to avoid repeated API calls
- Fallback gracefully if explorer unavailable
