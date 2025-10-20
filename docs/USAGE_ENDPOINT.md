# Claude Code Usage API Endpoint

## Discovery Summary

This document details the discovery and usage of Claude Code's usage tracking endpoint for Max Plan subscribers.

## Background

Claude Code has two separate authentication systems:
1. **API Keys** (`sk-ant-api-...`) - For business/Console users, requires Admin API keys for usage data
2. **OAuth Tokens** - For individual Max Plan subscribers on claude.ai

The Max Plan uses OAuth authentication with two token types stored in `~/.claude/.credentials.json`:
- **Access Token** (`sk-ant-oat01-...`) - Short-lived token for API calls
- **Refresh Token** (`sk-ant-ort01-...`) - Long-lived token for renewing access tokens

## The Usage Endpoint

### Endpoint Details

```
GET https://api.anthropic.com/api/oauth/usage
```

### Authentication

**Headers:**
```
Authorization: Bearer {accessToken}
anthropic-beta: oauth-2025-04-20
```

**Important:** Use the `accessToken` from `.credentials.json`, NOT the `refreshToken`.

### Response Format

```json
{
  "five_hour": {
    "utilization": 19,
    "resets_at": "2025-10-20T02:59:59.764670+00:00"
  },
  "seven_day": {
    "utilization": 33,
    "resets_at": "2025-10-23T21:59:59.764687+00:00"
  },
  "seven_day_oauth_apps": {
    "utilization": 0,
    "resets_at": null
  },
  "seven_day_opus": {
    "utilization": 0,
    "resets_at": null
  }
}
```

### Response Fields

- **five_hour**: Current 5-hour rolling window usage
  - `utilization`: Percentage used (0-100)
  - `resets_at`: ISO 8601 timestamp when the limit resets

- **seven_day**: Weekly usage across all models
  - `utilization`: Percentage used (0-100)
  - `resets_at`: ISO 8601 timestamp when the limit resets

- **seven_day_oauth_apps**: OAuth app-specific usage (typically 0 for CLI)
  - `utilization`: Percentage used (0-100)
  - `resets_at`: ISO 8601 timestamp or null if not applicable

- **seven_day_opus**: Weekly Opus-specific usage
  - `utilization`: Percentage used (0-100)
  - `resets_at`: ISO 8601 timestamp or null if not applicable

## Discovery Process

### Initial Investigation

The journey started with trying to intercept `/usage` command traffic using Fiddler, but Node.js wasn't using the proxy by default.

### Key Breakthrough

Searched the minified Claude Code CLI source at:
```
C:\Users\William\AppData\Roaming\npm\node_modules\@anthropic-ai\claude-code\cli.js
```

Found the critical code:
```javascript
Q=`${d4().BASE_API_URL}/api/oauth/usage`;
return(await u2.get(Q,{headers:B,timeout:5000})).data
```

### Configuration Discovery

Found `BASE_API_URL` definition:
```javascript
NCA={
  ...OCA,
  BASE_API_URL:"https://api.anthropic.com",
  // ... other OAuth config
}
```

### Authentication Pattern

The code showed OAuth Bearer token authentication:
```javascript
let B={"Content-Type":"application/json","User-Agent":Nz(),...A.headers},
```

Where `A.headers` contained:
```javascript
{Authorization:`Bearer ${B.accessToken}`,"anthropic-beta":It}
```

And `It` was defined as:
```javascript
It="oauth-2025-04-20"
```

## Differences from Admin API

The Admin API endpoints (`/v1/organizations/usage_report/*`) require:
- Admin API keys (`sk-ant-admin-...`)
- Organization-level access
- Different response format with detailed breakdowns

The OAuth usage endpoint:
- Uses OAuth access tokens from Max Plan subscriptions
- Returns simple percentage-based utilization
- No admin privileges required
- Works with individual user accounts

## Example Usage

### cURL
```bash
curl -X GET "https://api.anthropic.com/api/oauth/usage" \
  -H "Authorization: Bearer sk-ant-oat01-..." \
  -H "anthropic-beta: oauth-2025-04-20"
```

### Response Interpretation

- **utilization < 80**: Safe usage levels
- **utilization 80-95**: Approaching limit, use conservatively
- **utilization > 95**: Very close to limit, may be throttled
- **utilization 100**: Limit reached, requests will be rejected until reset

## Implementation Notes

1. **Token Management**: Access tokens expire, refresh tokens are used to obtain new access tokens
2. **Error Handling**: 401 responses indicate expired/invalid tokens
3. **Rate Limiting**: This endpoint itself should be called sparingly (max once per minute)
4. **Caching**: Cache responses for 30-60 seconds to avoid excessive calls

## Related Files

- `~/.claude/.credentials.json` - Contains OAuth tokens
- `~/.claude/settings.json` - User settings
- Claude Code source: `@anthropic-ai/claude-code/cli.js`

## References

- Claude Code CLI: https://docs.claude.com/en/docs/claude-code
- OAuth 2.0 specification: https://oauth.net/2/
