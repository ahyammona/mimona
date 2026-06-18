import fetch from "node-fetch";
import {
  META_PHONE_NUMBER_ID,
  META_ACCESS_TOKEN,
  META_VERIFY_TOKEN,
  OFFICIAL_API_CONFIGURED,
} from "./config.js";
import { getAssistantReply, setLinkStatus } from "./mimonaClient.js";

const GRAPH_BASE = "https://graph.facebook.com/v20.0";

/**
 * Handles Meta's webhook verification handshake (GET request with a
 * challenge token) when you first configure the webhook URL in the Meta
 * App dashboard. Returns the challenge string to echo back, or null if
 * verification fails.
 */
export function verifyWebhook(query) {
  const mode = query["hub.mode"];
  const token = query["hub.verify_token"];
  const challenge = query["hub.challenge"];

  if (mode === "subscribe" && token === META_VERIFY_TOKEN && META_VERIFY_TOKEN) {
    return challenge;
  }
  return null;
}

/**
 * Processes an incoming webhook POST body from Meta. Meta batches
 * messages inside entry[].changes[].value.messages[]; this walks that
 * shape, replies via Mimona, and sends the reply back through the Cloud
 * API.
 */
export async function handleIncomingWebhook(body) {
  if (!OFFICIAL_API_CONFIGURED) {
    console.warn(
      "[official] Received a webhook but META_PHONE_NUMBER_ID / META_ACCESS_TOKEN are not set — ignoring."
    );
    return;
  }

  const entries = body?.entry || [];
  for (const entry of entries) {
    for (const change of entry.changes || []) {
      const value = change.value || {};
      const messages = value.messages || [];

      for (const msg of messages) {
        const from = msg.from; // sender's E.164 number, no '+'
        const text = msg.text?.body;
        if (!text) continue;

        const phoneNumber = `+${from}`;
        try {
          const reply = await getAssistantReply(phoneNumber, text);
          await sendOfficialMessage(from, reply);
        } catch (err) {
          console.error(`[official] reply failed for ${phoneNumber}:`, err);
        }
      }
    }
  }
}

/**
 * Sends a text message via the Cloud API. `to` is the recipient's number
 * without a leading '+' (Meta's convention).
 */
export async function sendOfficialMessage(to, text) {
  if (!OFFICIAL_API_CONFIGURED) {
    throw new Error(
      "Official API not configured — set META_PHONE_NUMBER_ID and META_ACCESS_TOKEN."
    );
  }

  const res = await fetch(
    `${GRAPH_BASE}/${META_PHONE_NUMBER_ID}/messages`,
    {
      method: "POST",
      headers: {
        Authorization: `Bearer ${META_ACCESS_TOKEN}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        messaging_product: "whatsapp",
        to,
        type: "text",
        text: { body: text },
      }),
    }
  );

  if (!res.ok) {
    throw new Error(
      `Cloud API send failed: ${res.status} ${await res.text()}`
    );
  }
  return res.json();
}

/**
 * Marks a user's record as connected via the official path. There's no
 * QR scan here — once Meta verification is done and the number is
 * assigned, the link UI just records the phone number and this method
 * confirms the assistant is live.
 */
export async function registerOfficialUser(phoneNumber) {
  if (!OFFICIAL_API_CONFIGURED) {
    throw new Error(
      "Official API is not configured yet. Add META_PHONE_NUMBER_ID, META_ACCESS_TOKEN " +
        "(and META_VERIFY_TOKEN for webhook setup) once your Meta Business verification " +
        "is complete, then restart the bridge."
    );
  }
  await setLinkStatus(phoneNumber, "connected");
}

export { OFFICIAL_API_CONFIGURED };
