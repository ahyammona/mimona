import http from "node:http";
import { URL } from "node:url";
import crypto from "node:crypto";

import { BRIDGE_PORT, OFFICIAL_API_CONFIGURED } from "./config.js";
import {
  startBaileysSession,
  startBaileysSessionForKnownNumber,
  getLatestQr,
  getConnectionState,
  getResolvedPhoneNumber,
  stopBaileysSession,
} from "./baileysAdapter.js";
import {
  verifyWebhook,
  handleIncomingWebhook,
  registerOfficialUser,
} from "./officialAdapter.js";
import { listWhatsAppUsers } from "./mimonaClient.js";

// The link UI (frontend/whatsapp.html) is served by Mimona on a
// different port (11435) than this bridge (3344). Even though both run
// on the same machine, the browser treats localhost:11435 and
// localhost:3344 as different origins and enforces CORS — without these
// headers, every fetch() from the page to this server gets silently
// blocked client-side, which looks identical to "bridge is down" even
// though curl (which ignores CORS) reaches it fine.
const CORS_HEADERS = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Methods": "GET, POST, DELETE, OPTIONS",
  "Access-Control-Allow-Headers": "Content-Type",
};

function sendJson(res, status, payload) {
  res.writeHead(status, { "Content-Type": "application/json", ...CORS_HEADERS });
  res.end(JSON.stringify(payload));
}

function readBody(req) {
  return new Promise((resolve, reject) => {
    let data = "";
    req.on("data", (chunk) => (data += chunk));
    req.on("end", () => {
      if (!data) return resolve({});
      try {
        resolve(JSON.parse(data));
      } catch (e) {
        reject(e);
      }
    });
    req.on("error", reject);
  });
}

/**
 * On boot, resume any Baileys sessions for users who were already
 * connected before the bridge restarted (e.g. after a deploy). Once a
 * session resolves to a real phone number, its auth creds directory is
 * renamed from the temporary session_id to that number (see
 * baileysAdapter.js), so resuming under the phone number here finds the
 * same saved credentials and reconnects without a fresh QR scan.
 */
async function resumeExistingBaileysSessions() {
  try {
    const users = await listWhatsAppUsers();
    for (const u of users) {
      if (u.connection_method === "baileys" && u.status === "connected") {
        startBaileysSessionForKnownNumber(u.phone_number).catch((err) =>
          console.error(`[bridge] failed to resume session for ${u.phone_number}:`, err)
        );
      }
    }
  } catch (err) {
    console.warn(
      "[bridge] Could not reach Mimona server to resume sessions yet:",
      err.message
    );
  }
}

const server = http.createServer(async (req, res) => {
  const url = new URL(req.url, `http://${req.headers.host}`);

  // Handle CORS preflight requests. Browsers send OPTIONS before any
  // cross-origin POST with a JSON body — without this, Chrome/Firefox
  // never sends the actual request.
  if (req.method === "OPTIONS") {
    res.writeHead(204, CORS_HEADERS);
    return res.end();
  }

  try {
    // ── Baileys: start a fresh QR pairing session. No phone number is
    //    needed here — the link UI just clicks "Connect" and gets back a
    //    session_id to poll. The real number is only known after the
    //    user scans the QR with their phone. ─────────────────────────────
    if (url.pathname === "/baileys/start" && req.method === "POST") {
      const body = await readBody(req);
      const sessionId = body.session_id || crypto.randomUUID();
      await startBaileysSession(sessionId);
      return sendJson(res, 200, {
        ok: true,
        session_id: sessionId,
        state: getConnectionState(sessionId),
      });
    }

    // ── Baileys: poll for QR / connection state / resolved number. The
    //    link UI calls this every ~2s while showing the QR code. ────────
    if (url.pathname === "/baileys/status" && req.method === "GET") {
      const sessionId = url.searchParams.get("session_id");
      if (!sessionId) {
        return sendJson(res, 400, { error: "session_id query param is required" });
      }
      return sendJson(res, 200, {
        state: getConnectionState(sessionId),
        qr: getLatestQr(sessionId),
        phone_number: getResolvedPhoneNumber(sessionId),
      });
    }

    if (url.pathname === "/baileys/stop" && req.method === "POST") {
      const { session_id } = await readBody(req);
      if (session_id) stopBaileysSession(session_id);
      return sendJson(res, 200, { ok: true });
    }

    // ── Official Cloud API: webhook verification handshake (GET) ───────
    if (url.pathname === "/official/webhook" && req.method === "GET") {
      const query = Object.fromEntries(url.searchParams.entries());
      const challenge = verifyWebhook(query);
      if (challenge !== null) {
        res.writeHead(200, { "Content-Type": "text/plain" });
        return res.end(challenge);
      }
      return sendJson(res, 403, { error: "Webhook verification failed" });
    }

    // ── Official Cloud API: incoming messages (POST) ────────────────────
    if (url.pathname === "/official/webhook" && req.method === "POST") {
      const body = await readBody(req);
      await handleIncomingWebhook(body);
      return sendJson(res, 200, { ok: true });
    }

    // ── Official Cloud API: mark a user as connected once Meta creds are
    //    in place (no QR flow needed — just a confirmation step). Still
    //    needs a phone number since Official requires one upfront. ─────
    if (url.pathname === "/official/register" && req.method === "POST") {
      const { phone_number } = await readBody(req);
      if (!phone_number) {
        return sendJson(res, 400, { error: "phone_number is required" });
      }
      try {
        await registerOfficialUser(phone_number);
        return sendJson(res, 200, { ok: true });
      } catch (err) {
        return sendJson(res, 409, { error: err.message });
      }
    }

    if (url.pathname === "/health" && req.method === "GET") {
      return sendJson(res, 200, {
        ok: true,
        official_api_configured: OFFICIAL_API_CONFIGURED,
      });
    }

    sendJson(res, 404, { error: "Not found" });
  } catch (err) {
    console.error("[bridge] request error:", err);
    sendJson(res, 500, { error: err.message });
  }
});

server.listen(BRIDGE_PORT, () => {
  console.log(`Mimona WhatsApp bridge listening on http://localhost:${BRIDGE_PORT}`);
  console.log(
    OFFICIAL_API_CONFIGURED
      ? "Official Cloud API: configured"
      : "Official Cloud API: not configured (set META_PHONE_NUMBER_ID / META_ACCESS_TOKEN to enable)"
  );
  resumeExistingBaileysSessions();
});