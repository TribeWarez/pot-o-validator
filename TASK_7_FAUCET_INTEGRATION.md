# Task 7: Faucet AUM/STOMP/RAVECOIN Support

## Location
This task requires modifications to the gateway.tribewarez.com submodule:
- Path: `gateway.tribewarez.com/testnet.rpc.gateway.tribewarez.com/faucet/server.js`
- Also: faucet HTML templates in the same directory

## Changes Required

### 1. Update TOKENS Configuration
In `server.js`, update the TOKENS object to include:

```javascript
const TOKENS = {
  TRIBE: {
    symbol: 'TRIBE',
    decimals: 9,
    claim_amount: 1000000000,  // 1 TRIBE per claim
    claim_interval: 3600,      // 1 hour between claims
  },
  AUM: {
    symbol: 'AUM',
    decimals: 9,
    claim_amount: 100000000000, // 100 AUM per claim
    claim_interval: 3600,
  },
  STOMP: {
    symbol: 'STOMP',
    decimals: 9,
    claim_amount: 1000000000000, // 1000 STOMP per claim
    claim_interval: 3600,
  },
  RAVECOIN: {
    symbol: 'RAVECOIN',
    decimals: 9,
    claim_amount: 100000000000, // 100 RAVECOIN per claim
    claim_interval: 3600,
  },
};
```

### 2. Update Faucet Routes
Add token claim routes:
- `GET /api/faucet/tokens` - list available tokens
- `POST /api/faucet/claim/:token/:address` - claim tokens for specified token type

### 3. Update Web UI
In HTML template:
- Add token selection buttons (TRIBE, AUM, STOMP, RAVECOIN)
- Show claim amounts per token
- Display token icons/colors:
  - TRIBE: Blue (#0066FF)
  - AUM: Gold (#FFD700)
  - STOMP: Orange (#FF8C00)
  - RAVECOIN: Purple (#9933FF)

### 4. Token Verification
Before claiming, verify:
- Token is in TOKENS config
- User has waited required interval since last claim
- Reserve accounts have sufficient balance

## Integration Points

- Faucet backend calls `POST /api/tx` endpoint on pot-o-validator
- Response confirms transaction hash after claim
- Show transaction hash link to `/api/tx/:hash` explorer

## Testing

```bash
# Check available tokens
curl http://localhost:3000/api/faucet/tokens

# Claim AUM
curl -X POST http://localhost:3000/api/faucet/claim/AUM/Tc1TestAddress

# Verify with validator
curl http://localhost:8900/api/tx/[tx_hash]
```

## Notes

- All amounts are in base units (9 decimals as per TokenType)
- Use Tc1StompReserve, Tc1AumReserve, Tc1RavecoinReserve as sources
- Leverage existing TRIBE claim logic as template
