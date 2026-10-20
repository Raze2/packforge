# 06 — Licensing & Tiers

## 1. Plans

| Plan | Who | Delivery |
|---|---|---|
| **Free** | Anyone | Desktop + web, starter library, SVG/PDF (watermark), 3D preview, basic stats |
| **Premium** | Designers, small shops | Desktop license or web account: full library, DXF/CF2/HPGL, clean PDF, custom components, pallet/case tools, reports |
| **Enterprise / API** | Integrators (Laravel, ERP) | API keys for `pf-api` (hosted or self-hosted), plan-based rate limits, white-label web component |

Feature-to-tier mapping lives in [01-product-spec.md §4](01-product-spec.md) and in code as a
single `Entitlement` enum so UI, core, and API agree.

## 2. Desktop: signed offline tokens

- Server holds an **Ed25519 private key**; apps ship only the **public key**.
- Token = base64url(`payload` JSON) + `.` + base64url(signature). Payload:
  `{ "lic": "id", "sub": "customer", "plan": "premium", "ent": [...], "seats": 1,
     "device": "<hash>", "iat": 1730000000, "exp": 1761536000, "refresh_by": 1731209600 }`
- `pf-license::verify(token, public_key, now)` checks signature, expiry, product, and (optionally)
  device binding. `now` is supplied by the shell.
- Device binding uses a hash of a stable machine id (salted). Users can move a seat by
  deactivating in their account.
- Online refresh renews `refresh_by`; offline grace = 14 days past `refresh_by`.
- Never store secrets in the client; never roll our own crypto beyond using `ed25519-dalek`.

Honest limit: any client-side check can be patched by a determined attacker. The goal is to
keep honest users honest and make premium value (updates, library, cloud features) worth paying.

## 3. Web: server-side premium

- Browser-only (WASM) mode = Free tier, enforced in the UI.
- Premium exports and full library are served by `pf-api`, which checks the API key/plan.
- The embedding site proxies requests through its backend (Laravel route) so keys never reach
  the browser.

## 4. API keys

- Format `pk_live_…` / `pk_test_…`; stored hashed (SHA-256) server-side.
- Per-key plan, rate limit (token bucket), monthly quota, allowed origins (for CORS).
- Key management via a small admin CLI in v1; dashboard later.

## 5. Payments (later stage)

Stripe / Paddle (Paddle handles VAT as merchant of record). Webhook → issue/renew token or
upgrade API plan. Not required for the first releases.
