import { isOrderToken, type OrderResponse } from "@/lib/orders";
import { previewFetchInit } from "@/lib/preview";
import { isQuoteToken, type PublicQuote } from "@/lib/quotes";
import { EMPTY_SITE_CONTENT, type SiteContent } from "@/lib/site-content";

export interface Category {
  id: number;
  name: string;
  description: string | null;
}

export interface MenuItemPriceOption {
  id: number;
  label: string;
  price: string;
}

export interface MenuItem {
  id: number;
  category_id: number;
  name: string;
  description: string | null;
  price: string | null;
  image_url: string | null;
  is_available: boolean;
  is_featured: boolean;
  price_options: MenuItemPriceOption[];
}

const API_BASE_URL = process.env.API_BASE_URL;

async function apiFetch<T>(path: string): Promise<T> {
  if (!API_BASE_URL) {
    throw new Error("API_BASE_URL is not set — check docker-compose.yml's web service.");
  }

  const response = await fetch(`${API_BASE_URL}${path}`, { cache: "no-store" });

  if (!response.ok) {
    throw new Error(`API request to ${path} failed with status ${response.status}`);
  }

  return (await response.json()) as T;
}

// In draft mode, read the admin preview endpoint (drafts overlaid,
// unpublished rows included); without an admin session it 401s and the
// page falls back to published data (Phase 25 Decision 21).
async function previewFetch<T>(path: string): Promise<T | null> {
  const init = await previewFetchInit();
  if (!init || !API_BASE_URL) return null;

  const response = await fetch(`${API_BASE_URL}/api/admin/preview${path}`, init);
  if (!response.ok) return null;
  return (await response.json()) as T;
}

export async function getCategories(): Promise<Category[]> {
  return (await previewFetch<Category[]>("/categories")) ?? apiFetch<Category[]>("/api/categories");
}

export async function getMenuItems(categoryId?: number): Promise<MenuItem[]> {
  const query = categoryId !== undefined ? `?category_id=${categoryId}` : "";
  return (
    (await previewFetch<MenuItem[]>(`/menu-items${query}`)) ??
    apiFetch<MenuItem[]>(`/api/menu-items${query}`)
  );
}

// Fails soft to the bundled defaults. The published document is cached
// under a tag (revalidated on publish) rather than fetched no-store, so the
// layout reading it doesn't make about/services/contact dynamic.
export async function getSiteContent(): Promise<SiteContent> {
  try {
    const preview = await previewFetch<SiteContent>("/site-content");
    if (preview) return preview;
    if (!API_BASE_URL) return EMPTY_SITE_CONTENT;

    const response = await fetch(`${API_BASE_URL}/api/site-content`, {
      next: { tags: ["site-content"], revalidate: 300 },
    });
    if (!response.ok) return EMPTY_SITE_CONTENT;
    return (await response.json()) as SiteContent;
  } catch {
    return EMPTY_SITE_CONTENT;
  }
}

// `null` for an unknown or malformed token, so the page can 404 without
// calling the API for something that can't match (Phase 20 Decision 4).
export async function getQuoteByToken(token: string): Promise<PublicQuote | null> {
  if (!isQuoteToken(token)) return null;
  if (!API_BASE_URL) {
    throw new Error("API_BASE_URL is not set — check docker-compose.yml's web service.");
  }

  const response = await fetch(`${API_BASE_URL}/api/quotes/by-token/${token}`, { cache: "no-store" });
  if (response.status === 404) return null;
  if (!response.ok) {
    throw new Error(`Quote lookup failed with status ${response.status}`);
  }

  return (await response.json()) as PublicQuote;
}

export async function getOrderByToken(token: string): Promise<OrderResponse | null> {
  if (!isOrderToken(token)) return null;
  if (!API_BASE_URL) {
    throw new Error("API_BASE_URL is not set — check docker-compose.yml's web service.");
  }

  const response = await fetch(`${API_BASE_URL}/api/orders/by-token/${token}`, { cache: "no-store" });
  if (response.status === 404) return null;
  if (!response.ok) {
    throw new Error(`Order lookup failed with status ${response.status}`);
  }

  return (await response.json()) as OrderResponse;
}
