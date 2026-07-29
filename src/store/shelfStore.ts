import { create } from 'zustand';

export interface SmartRule {
  field: string; // "author", "tag", "format", "rating", "series", "added_date"
  operator: string; // "equals", "contains", "greater_than", "less_than", "in_last"
  value: string;
  matchType: string; // "all" or "any"
}

export interface Shelf {
  id?: number;
  name: string;
  description?: string;
  parentId?: number;
  isSmart: boolean;
  smartRules?: string; // JSON string of SmartRule[]
  icon?: string;
  color?: string;
  shelfType: string;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
  bookCount?: number;
  children: Shelf[];
}

interface ShelfState {
  shelves: Shelf[];
  selectedShelf: Shelf | null;
  isLoading: boolean;
  favoritesShelf: Shelf | null;
  
  // Actions
  setShelfs: (shelves: Shelf[]) => void;
  addShelf: (shelf: Shelf) => void;
  updateShelf: (id: number, shelf: Partial<Shelf>) => void;
  removeShelf: (id: number) => void;
  selectShelf: (shelf: Shelf | null) => void;
  setLoading: (isLoading: boolean) => void;
  setFavoritesShelf: (shelf: Shelf | null) => void;
}

export const useShelfStore = create<ShelfState>((set) => ({
  shelves: [],
  selectedShelf: null,
  isLoading: false,
  favoritesShelf: null,

  setShelfs: (shelves) => set({ shelves }),

  addShelf: (shelf) =>
    set((state) => ({
      shelves: [...state.shelves, shelf],
    })),

  updateShelf: (id, updatedShelf) =>
    set((state) => ({
      shelves: state.shelves.map((c) =>
        c.id === id ? { ...c, ...updatedShelf } : c
      ),
    })),

  removeShelf: (id) =>
    set((state) => ({
      shelves: state.shelves.filter((c) => c.id !== id),
      selectedShelf:
        state.selectedShelf?.id === id ? null : state.selectedShelf,
    })),

  selectShelf: (shelf) => set({ selectedShelf: shelf }),

  setLoading: (isLoading) => set({ isLoading }),

  setFavoritesShelf: (favoritesShelf) => set({ favoritesShelf }),

}));
