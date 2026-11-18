// Phase 36 Decisions 8, 9 (apps/api's AdminRentalResponse / RentalFields).
export interface AdminRental {
  id: number;
  name: string;
  description: string | null;
  price: string;
  image_url: string | null;
  is_available: boolean;
  display_order: number;
  is_archived: boolean;
  updated_at: string;
}

export interface RentalFields {
  name: string;
  description: string | null;
  price: string;
  image_url: string | null;
  is_available: boolean;
  display_order: number;
}
