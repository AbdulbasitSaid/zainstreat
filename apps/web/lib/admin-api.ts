import { cookies } from "next/headers";

const API_BASE_URL = process.env.API_BASE_URL;

/**
 * Server-to-server fetch against the Rust API with the browser's session
 * cookie forwarded. Same posture as lib/api.ts's apiFetch — API_BASE_URL is
 * container-internal and never reaches the browser.
 */
export async function adminApiFetch(path: string): Promise<Response> {
  if (!API_BASE_URL) {
    throw new Error("API_BASE_URL is not set — check docker-compose.yml's web service.");
  }

  return fetch(`${API_BASE_URL}${path}`, {
    headers: { Cookie: (await cookies()).toString() },
    cache: "no-store",
  });
}

export async function adminApiJson<T>(path: string): Promise<T> {
  const response = await adminApiFetch(path);

  if (!response.ok) {
    throw new Error(`Admin API request to ${path} failed with status ${response.status}`);
  }

  return (await response.json()) as T;
}
