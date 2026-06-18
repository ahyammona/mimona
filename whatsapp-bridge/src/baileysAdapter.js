import {
  default as makeWASocket,
  useMultiFileAuthState,
  DisconnectReason,
  fetchLatestBaileysVersion,
} from "@whiskeysockets/baileys";
import { Boom } from "@hapi/boom";
import pino from "pino";
import path from "node:path";
import fs from "node:fs";
import qrcode from "qrcode";
import qrcodeTerminal from "qrcode-terminal";

import { AUTH_DIR } from "./config.js";
import { getAssistantReply, setLinkStatus, linkBaileysSession } from "./mimonaClient.js";

const logger = pino({ level: "warn" });

/**
 * Baileys doesn't need a phone number upfront — the user just scans a QR
 * code with whatever WhatsApp account they want to link, and the real
 * number only becomes known after a successful connection (read off
 * `sock.user.id`). So sessions are keyed by an opaque session_id chosen
 * by the link UI (a fresh one per "Connect" click), not by phone number.
 *
 * Once connected, the entry also records `phoneNumber` so later lookups
 * (status polling, message routing) work either by session_id or by the
 * resolved number.
 */
const sessions = new Map();

function sessionDir(sessionId) {
  const safe = sessionId.replace(/[^a-zA-Z0-9]/g, "_");
  return path.join(AUTH_DIR, safe);
}

/**
 * Extracts a clean E.164-ish phone number from a Baileys JID like
 * "15551234567:12@s.whatsapp.net" or "15551234567@s.whatsapp.net".
 */
function phoneFromJid(jid) {
  if (!jid) return null;
  const digits = jid.split("@")[0].split(":")[0];
  return digits ? `+${digits}` : null;
}

export function getLatestQr(sessionId) {
  const session = sessions.get(sessionId);
  return session?.latestQrDataUrl || null;
}

export function getConnectionState(sessionId) {
  const session = sessions.get(sessionId);
  return session?.state || "not_started";
}

export function getResolvedPhoneNumber(sessionId) {
  const session = sessions.get(sessionId);
  return session?.phoneNumber || null;
}

/**
 * Starts (or resumes) a Baileys session under this session_id. Safe to
 * call repeatedly while the link UI polls — if a session is already
 * connecting/connected under this id, it's a no-op.
 */
export async function startBaileysSession(sessionId) {
  if (sessions.has(sessionId)) {
    const existing = sessions.get(sessionId);
    if (existing.state === "connected" || existing.state === "connecting") {
      return existing;
    }
  }

  const dir = sessionDir(sessionId);
  fs.mkdirSync(dir, { recursive: true });

  const { state, saveCreds } = await useMultiFileAuthState(dir);
  const { version } = await fetchLatestBaileysVersion();

  const sessionEntry = {
    state: "connecting",
    latestQrDataUrl: null,
    socket: null,
    phoneNumber: null,
  };
  sessions.set(sessionId, sessionEntry);

  const sock = makeWASocket({
    version,
    auth: state,
    logger,
    printQRInTerminal: false,
  });
  sessionEntry.socket = sock;

  sock.ev.on("creds.update", saveCreds);

  sock.ev.on("connection.update", async (update) => {
    const { connection, lastDisconnect, qr } = update;

    if (qr) {
      sessionEntry.latestQrDataUrl = await qrcode.toDataURL(qr);
      sessionEntry.state = "awaiting_qr_scan";
      qrcodeTerminal.generate(qr, { small: true });
    }

    if (connection === "open") {
      const resolvedNumber = phoneFromJid(sock.user?.id);
      sessionEntry.state = "connected";
      sessionEntry.latestQrDataUrl = null;
      sessionEntry.phoneNumber = resolvedNumber;

      console.log(`[baileys] session ${sessionId} connected as ${resolvedNumber}`);

      if (resolvedNumber) {
        // Re-key the in-memory map so status polls and message routing
        // can find this session by phone number immediately.
        sessions.set(resolvedNumber, sessionEntry);

        // Register the user with Mimona and mark connected right away —
        // these don't depend on the creds being saved to disk.
        await linkBaileysSession(resolvedNumber).catch((err) =>
          console.error(`[baileys] failed to register ${resolvedNumber} with Mimona:`, err)
        );
        await setLinkStatus(resolvedNumber, "connected").catch(() => {});

        // Delay the directory rename until after Baileys has had time to
        // finish writing creds.json. Renaming immediately races with
        // Baileys' own creds.update handler which is still writing to the
        // old path — the 3s wait lets that settle without blocking the
        // connection response.
        setTimeout(() => {
          const oldDir = sessionDir(sessionId);
          const newDir = sessionDir(resolvedNumber);
          if (oldDir === newDir) return;
          if (!fs.existsSync(oldDir)) return;
          if (fs.existsSync(newDir)) return;
          try {
            fs.renameSync(oldDir, newDir);
            console.log(`[baileys] session dir renamed to phone number for ${resolvedNumber}`);
          } catch (err) {
            console.error(`[baileys] could not rename session dir for ${resolvedNumber}:`, err);
          }
        }, 3000);
      }
    }

    if (connection === "close") {
      const statusCode = new Boom(lastDisconnect?.error)?.output?.statusCode;
      const loggedOut = statusCode === DisconnectReason.loggedOut;

      sessionEntry.state = "disconnected";
      console.log(
        `[baileys] session ${sessionId} disconnected (loggedOut=${loggedOut})`
      );
      if (sessionEntry.phoneNumber) {
        await setLinkStatus(sessionEntry.phoneNumber, "disconnected").catch(() => {});
      }

      if (!loggedOut) {
        setTimeout(() => startBaileysSession(sessionId), 3000);
      } else {
        sessions.delete(sessionId);
        fs.rmSync(dir, { recursive: true, force: true });
      }
    }
  });

  sock.ev.on("messages.upsert", async ({ messages, type }) => {
    if (type !== "notify") return;
    if (!sessionEntry.phoneNumber) return; // not resolved yet, ignore

    for (const msg of messages) {
      if (msg.key.fromMe) continue;
      const text =
        msg.message?.conversation ||
        msg.message?.extendedTextMessage?.text ||
        null;
      if (!text) continue;

      const from = msg.key.remoteJid;
      try {
        const reply = await getAssistantReply(sessionEntry.phoneNumber, text);
        await sock.sendMessage(from, { text: reply });
      } catch (err) {
        console.error(`[baileys] reply failed for ${sessionEntry.phoneNumber}:`, err);
        await sock
          .sendMessage(from, {
            text: "Sorry, something went wrong generating a reply. Please try again shortly.",
          })
          .catch(() => {});
      }
    }
  });

  return sessionEntry;
}

export function stopBaileysSession(sessionId) {
  const session = sessions.get(sessionId);
  if (session?.socket) {
    session.socket.end(undefined);
  }
  sessions.delete(sessionId);
}

/**
 * Resume sessions for users already marked connected in Mimona after a
 * bridge restart. Their session dir is keyed by phone number in this
 * case (see config note in index.js) since session_id values aren't
 * persisted across restarts — only the resolved phone number is.
 */
export async function startBaileysSessionForKnownNumber(phoneNumber) {
  return startBaileysSession(phoneNumber);
}