export interface AdminCategory {
  id: number;
  name: string;
  description: string | null;
  is_archived: boolean;
  updated_at: string;
}
