import React, { useState, useEffect } from 'react';
import { useShelfStore } from '../../store/shelfStore';
import { useUIStore } from '../../store/uiStore';
import { CreateShelfDialog } from './CreateShelfDialog';
import { ShelfGrid } from './ShelfGrid';
import { ShelfBookGrid } from './ShelfBookGrid';
import { Shelf, Book, api } from '../../lib/tauri';
import { Loader2 } from 'lucide-react';
import { logger } from '@/lib/logger';

export function ShelfView() {
  const setCurrentView = useUIStore(state => state.setCurrentView);
  const selectedShelf = useShelfStore(state => state.selectedShelf);
  const selectShelf = useShelfStore(state => state.selectShelf);
  const shelves = useShelfStore(state => state.shelves);
  const setShelfs = useShelfStore(state => state.setShelfs);
  
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editShelf, setEditShelf] = useState<Shelf | null>(null);
  const [parentId, setParentId] = useState<number | undefined>(undefined);

  const [books, setBooks] = useState<Book[]>([]);
  const [loadingBooks, setLoadingBooks] = useState(false);
  const [loadingShelves, setLoadingShelves] = useState(true);

  useEffect(() => {
    async function loadShelfs() {
      setLoadingShelves(true);
      try {
        const nested = await api.getNestedShelfs();
        
        // Also load special shelves
        const [favs, shelfList] = await Promise.all([
          api.getShelfsByType('favorites'),
          api.getShelfsByType('shelf'),
        ]);
        
        const allShelves = [
          ...(favs || []),
          ...(shelfList || []),
          ...(nested || [])
        ];
        
        // Deduplicate shelves by ID
        const uniqueShelves = Array.from(new Map(allShelves.map(s => [s.id, s])).values());
        setShelfs(uniqueShelves);
      } catch (error) {
        logger.error('Failed to load shelves:', error);
      } finally {
        setLoadingShelves(false);
      }
    }
    loadShelfs();
  }, [setShelfs]);

  useEffect(() => {
    async function loadBooks() {
      if (!selectedShelf || selectedShelf.id === undefined) {
        setBooks([]);
        return;
      }
      
      setLoadingBooks(true);
      try {
        const shelfBooks = await api.getShelfBooks(selectedShelf.id);
        setBooks(shelfBooks || []);
      } catch (error) {
        logger.error('Failed to load shelf books:', error);
        setBooks([]);
      } finally {
        setLoadingBooks(false);
      }
    }
    
    loadBooks();
  }, [selectedShelf]);

  const handleCreateShelf = (parentShelfId?: number) => {
    setEditShelf(null);
    setParentId(parentShelfId);
    setDialogOpen(true);
  };

  const handleEditShelf = (shelf: Shelf) => {
    setEditShelf(shelf);
    setParentId(undefined);
    setDialogOpen(true);
  };

  return (
    <div className="flex flex-col h-full w-full bg-[#0a0a0a] pt-16 md:pt-0 overflow-hidden relative">
      {/* Dynamic Content */}
      <div className="flex-1 overflow-hidden relative">
        {loadingShelves ? (
          <div className="flex items-center justify-center h-full">
            <Loader2 className="w-8 h-8 animate-spin text-white/30" />
          </div>
        ) : !selectedShelf ? (
          <ShelfGrid 
            shelves={shelves} 
            onSelectShelf={selectShelf} 
            onCreateShelf={() => handleCreateShelf()} 
          />
        ) : loadingBooks ? (
          <div className="flex items-center justify-center h-full">
            <Loader2 className="w-8 h-8 animate-spin text-white/30" />
          </div>
        ) : (
          <ShelfBookGrid shelf={selectedShelf} books={books} onBack={() => selectShelf(null)} />
        )}
      </div>

      <CreateShelfDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        editShelf={editShelf}
        parentId={parentId}
      />
    </div>
  );
}
