import { isOrderToken, type OrderResponse } from "@/lib/orders";

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

export function getCategories(): Promise<Category[]> {
  return apiFetch<Category[]>("/api/categories");
}

export function getMenuItems(categoryId?: number): Promise<MenuItem[]> {
  const query = categoryId !== undefined ? `?category_id=${categoryId}` : "";
  return apiFetch<MenuItem[]>(`/api/menu-items${query}`);
}

// `null` for an unknown or malformed token, so the page can 404 without
// calling the API for something that can't match (Phase 20 Decision 4).
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
