import fetch from "node-fetch";
import { MIMONA_BASE_URL } from "./config.js";

/**
 * Thin wrapper around Mimona's HTTP API. Keeps all the bridge's knowledge
 * of Mimona's URL shapes in one place.
 */

export async function getWhatsAppUser(phoneNumber) {
  const res = await fetch(
    `${MIMONA_BASE_URL}/api/whatsapp/users/${encodeURIComponent(phoneNumber)}`
  );
  if (res.status === 404) return null;
  if (!res.ok) {
    throw new Error(`getWhatsAppUser failed: ${res.status} ${await res.text()}`);
  }
  const data = await res.json();
  return data.user;
}

export async function listWhatsAppUsers() {
  const res = await fetch(`${MIMONA_BASE_URL}/api/whatsapp/users`);
  if (!res.ok) {
    throw new Error(`listWhatsAppUsers failed: ${res.status} ${await res.text()}`);
  }
  const data = await res.json();
  return data.users;
}

export async function setLinkStatus(phoneNumber, status) {
  const res = await fetch(
    `${MIMONA_BASE_URL}/api/whatsapp/users/${encodeURIComponent(phoneNumber)}/status`,
    {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ status }),
    }
  );
  if (!res.ok) {
    throw new Error(`setLinkStatus failed: ${res.status} ${await res.text()}`);
  }
  return res.json();
}

/**
 * Creates (or updates) the Mimona user record for a number that just
 * finished QR pairing. Unlike the Official flow, Baileys never knows the
 * phone number until after a successful scan — so this is called from
 * baileysAdapter's "connection open" handler rather than from the link
 * UI's initial "Connect" click.
 */
export async function linkBaileysSession(phoneNumber) {
  const res = await fetch(`${MIMONA_BASE_URL}/api/whatsapp/link`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      phone_number: phoneNumber,
      connection_method: "baileys",
    }),
  });
  if (!res.ok) {
    throw new Error(`linkBaileysSession failed: ${res.status} ${await res.text()}`);
  }
  const data = await res.json();
  return data.user;
}

/**
 * Sends the user's message + their configured system prompt to Mimona's
 * OpenAI-compatible chat endpoint and returns the assistant's reply text.
 * Always re-fetches the user record first, so a prompt edited in the UI
 * a minute ago — or a second ago — is what's actually used here.
 */
export async function getAssistantReply(phoneNumber, userMessage) {
  const user = await getWhatsAppUser(phoneNumber);
  if (!user) {
    throw new Error(`No linked Mimona user for ${phoneNumber}`);
  }

  const res = await fetch(`${MIMONA_BASE_URL}/v1/chat/completions`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      model: user.model,
      messages: [
        { role: "system", content: user.system_prompt },
        { role: "user", content: userMessage },
      ],
      temperature: 0.7,
      max_tokens: 512,
      stream: false,
    }),
  });

  if (!res.ok) {
    throw new Error(`chat completion failed: ${res.status} ${await res.text()}`);
  }

  const data = await res.json();
  const reply = data?.choices?.[0]?.message?.content;
  return reply && reply.trim().length > 0
    ? reply
    : "Sorry, I couldn't generate a reply just now. Try again in a moment.";
}
