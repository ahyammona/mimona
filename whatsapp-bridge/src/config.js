/**
 * Central config for the bridge. Everything that might change between
 * dev / staging / prod lives here, read from env vars with sane defaults.
 */

export const MIMONA_BASE_URL =
  process.env.MIMONA_BASE_URL || "http://localhost:11435";

export const BRIDGE_PORT = parseInt(process.env.BRIDGE_PORT || "3344", 10);

export const AUTH_DIR =
  process.env.WA_AUTH_DIR || new URL("../auth_sessions", import.meta.url).pathname;

// Official Cloud API (Meta) — only used once you've completed business
// verification and have these. Left undefined intentionally; the official
// adapter checks for these and reports itself as "not configured" until set.
export const META_PHONE_NUMBER_ID = process.env.META_PHONE_NUMBER_ID || null;
export const META_ACCESS_TOKEN = process.env.META_ACCESS_TOKEN || null;
export const META_VERIFY_TOKEN = process.env.META_VERIFY_TOKEN || null;
export const META_APP_SECRET = process.env.META_APP_SECRET || null;

export const OFFICIAL_API_CONFIGURED = Boolean(
  META_PHONE_NUMBER_ID && META_ACCESS_TOKEN
);
