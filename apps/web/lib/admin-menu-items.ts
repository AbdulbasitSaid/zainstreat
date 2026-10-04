export interface AdminPriceOption {
  id: number;
  label: string;
  price: string;
}

export interface AdminMenuItem {
  id: number;
  category_id: number;
  category_name: string;
  name: string;
  description: string | null;
  price: string | null;
  image_url: string | null;
  is_available: boolean;
  is_featured: boolean;
  is_archived: boolean;
  price_options: AdminPriceOption[];
  updated_at: string;
}
