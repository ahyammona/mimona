# Mimona WhatsApp Bridge

Connects WhatsApp to Mimona. Runs as a small standalone Node.js service
alongside `mimona serve` — it doesn't touch Mimona's Rust code at runtime,
it just calls Mimona's existing HTTP API.

## Two connection methods

**Baileys** (`@whiskeysockets/baileys`) — works today, no setup. The user
scans a QR code with their own WhatsApp app, same as linking WhatsApp Web.
This is against WhatsApp's Terms of Service for automated use; numbers can
be banned without warning, especially at volume. Good for prototyping and
low-volume personal use, not a permanent backend for paying customers.

**Official (Meta Cloud API)** — the legitimate, ToS-compliant path. Needs:
1. A Meta Business account, verified.
2. A WhatsApp Business app created in the Meta App Dashboard.
3. A phone number registered to it (a new number, or one ported in — it
   can no longer be used in the regular WhatsApp app afterward).
4. The webhook URL (`http://your-server:3344/official/webhook`) configured
   in the Meta dashboard, with a verify token matching `META_VERIFY_TOKEN`.
5. `META_PHONE_NUMBER_ID` and `META_ACCESS_TOKEN` from that dashboard.

Pricing is per-conversation (24-hour windows), varies by country/category,
and changes periodically — check Meta's current WhatsApp Business Platform
pricing page before estimating costs. This path is scaffolded and fully
wired in this codebase; it just stays inert (`OFFICIAL_API_CONFIGURED =
false`) until you add the env vars above.

## Setup

```bash
cd whatsapp-bridge
cp .env.example .env
npm install
npm start
```

Make sure `mimona serve` is running first (default `http://localhost:11435`) —
the bridge calls it for every chat reply and for reading/writing user state.

## How a user links via Baileys

1. User opens `frontend/whatsapp.html`, picks "Scan QR Code", clicks Connect.
   No phone number is needed yet — Baileys doesn't know which account will
   scan the code until it happens.
2. UI calls the bridge's `POST /baileys/start` (no body needed); the bridge
   generates a `session_id` and starts a fresh Baileys pairing under it.
3. UI polls `GET /baileys/status?session_id=...` every ~2s; once a `qr`
   value appears, it's rendered as an `<img>`.
4. User scans it. Baileys resolves the real WhatsApp number from the
   connected session (`sock.user.id`), and the bridge:
   - calls Mimona's `POST /api/whatsapp/link` to create the user record
     under that real number (method: `baileys`)
   - renames the session's auth-creds directory from `session_id` to the
     resolved number, so a bridge restart can resume it later without a
     fresh QR scan
   - flips status to `connected`
5. UI's next status poll sees `state: "connected"` plus the resolved
   `phone_number`, and moves to the prompt editor.
6. From then on, any WhatsApp message to that linked number is forwarded to
   `getAssistantReply()`, which re-reads the user's current `system_prompt`
   from Mimona before calling `/v1/chat/completions` — so prompt edits made
   in the UI take effect on the very next message, no relink required.

## How a user links via Official

Same `link` call, but with `connection_method: "official"`. There's no QR
step — once Meta verification + webhook are set up, the UI calls
`POST /official/register` to confirm the link, and incoming messages arrive
via the `/official/webhook` route instead of a Baileys socket.

## Session storage

Baileys credentials are saved per-number under `auth_sessions/<phone>/`.
Treat this directory like a password — anyone with these files can send
messages as that WhatsApp account. It's `.gitignore`'d; back it up
separately if you need session persistence across deploys.