import React, { useState, useRef, useEffect } from 'react';
import { Book, Shelf } from '../../lib/tauri';
import { Star, X, BookOpen, ArrowLeft } from 'lucide-react';
import { cn } from '@/lib/utils';
import { motion, AnimatePresence } from 'framer-motion';

interface ShelfBookGridProps {
  shelf: Shelf;
  books: Book[];
  onBack: () => void;
}

export function ShelfBookGrid({ shelf, books, onBack }: ShelfBookGridProps) {
  const [selectedBookId, setSelectedBookId] = useState<number | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [columns, setColumns] = useState(5);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const width = entry.contentRect.width;
        if (width < 400) setColumns(2);
        else if (width < 600) setColumns(3);
        else if (width < 900) setColumns(4);
        else if (width < 1200) setColumns(5);
        else setColumns(6);
      }
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  // Group books into rows based on the dynamic column count
  const rows = [];
  for (let i = 0; i < books.length; i += columns) {
    rows.push(books.slice(i, i + columns));
  }

  const shelfColor = shelf.color || '#f59e0b'; // Default amber like the screenshot

  return (
    <div className="p-4 sm:p-8 h-full overflow-y-auto" ref={containerRef}>
      <div className="max-w-[1400px] mx-auto">
        {/* Header */}
        <div className="mb-12 relative">
          <button 
            onClick={onBack}
            className="mb-6 flex items-center gap-2 text-white/50 hover:text-white transition-colors group"
          >
            <div className="w-8 h-8 rounded-full bg-white/5 flex items-center justify-center group-hover:bg-white/10 transition-colors">
              <ArrowLeft size={16} />
            </div>
            <span className="text-sm font-medium">Back to Shelves</span>
          </button>

          <div className="text-[11px] font-bold tracking-[0.2em] text-white/50 uppercase mb-3">
            MY SHELF · {books.length} BOOKS
          </div>
          <h1 className="text-5xl font-bold tracking-tight text-white mb-3" style={{ fontFamily: 'var(--font-serif)', letterSpacing: '-0.02em' }}>
            {shelf.name}
          </h1>
          <p className="text-white/40 text-sm">
            Tap a book to open its card — tap it again to close.
          </p>
        </div>

        {/* Grid */}
        <div className="flex flex-col gap-6 relative">
          {rows.map((rowBooks, rowIndex) => {
            const hasSelected = rowBooks.some(b => b.id === selectedBookId);
            const selectedIndex = rowBooks.findIndex(b => b.id === selectedBookId);
            const selectedBook = rowBooks[selectedIndex];
            const showAbove = rowIndex > 0;

            const ExpandedCard = () => (
              <AnimatePresence>
                {hasSelected && selectedBook && (
                  <motion.div
                    initial={{ height: 0, opacity: 0, [showAbove ? 'marginBottom' : 'marginTop']: -12 }}
                    animate={{ height: 'auto', opacity: 1, [showAbove ? 'marginBottom' : 'marginTop']: 0 }}
                    exit={{ height: 0, opacity: 0, [showAbove ? 'marginBottom' : 'marginTop']: -12 }}
                    transition={{ duration: 0.3, ease: [0.16, 1, 0.3, 1] }}
                    className="overflow-hidden"
                  >
                    <div 
                      className={cn(
                        "relative rounded-2xl p-4 sm:p-5",
                        showAbove ? "mb-4" : "mt-4"
                      )}
                      style={{ backgroundColor: '#1a1a1a', border: `1px solid ${shelfColor}40` }}
                    >
                      {/* Triangle Pointer */}
                      <div 
                        className={cn(
                          "absolute w-6 h-6 rotate-45 bg-[#1a1a1a]",
                          showAbove ? "-bottom-3 border-r border-b" : "-top-3 border-l border-t"
                        )}
                        style={{
                          left: `calc(${(selectedIndex + 0.5) / columns * 100}% - 12px)`,
                          borderColor: `${shelfColor}40`,
                          transition: 'left 0.3s ease-out'
                        }}
                      />

                      {/* Top/Bottom glowing line accent */}
                      <div 
                        className={cn(
                          "absolute left-0 right-0 h-[2px]",
                          showAbove ? "bottom-0 rounded-b-2xl" : "top-0 rounded-t-2xl"
                        )}
                        style={{ 
                          background: `linear-gradient(90deg, transparent, ${shelfColor}, transparent)`,
                          opacity: 0.5
                        }}
                      />

                      <div className="flex justify-between items-start mb-3">
                        <div className="pr-4">
                          <h2 className="text-xl font-bold text-white italic mb-1" style={{ fontFamily: 'var(--font-serif)' }}>
                            {selectedBook.title}
                          </h2>
                          <div className="text-white/50 text-xs">
                            {selectedBook.authors && selectedBook.authors.length > 0 ? selectedBook.authors.map(a => a.name).join(', ') : 'Unknown Author'}
                            {selectedBook.pubdate && ` · ${new Date(selectedBook.pubdate).getFullYear()}`}
                          </div>
                        </div>
                        
                        <div className="flex bg-white/5 rounded-full p-1 border border-white/10 shrink-0 items-center">
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              window.dispatchEvent(
                                new CustomEvent('open-book', { detail: { bookId: selectedBook.id } })
                              );
                            }}
                            className="px-3 py-1 rounded-full text-white/80 hover:text-white hover:bg-white/10 transition-colors text-xs font-medium flex items-center gap-1.5"
                          >
                            <BookOpen className="w-3.5 h-3.5" />
                            Read
                          </button>
                          <div className="w-px h-3 bg-white/20 mx-1"></div>
                          <button
                            onClick={() => setSelectedBookId(null)}
                            className="p-1.5 rounded-full text-white/60 hover:text-white hover:bg-white/10 transition-colors"
                          >
                            <X className="w-3.5 h-3.5" />
                          </button>
                        </div>
                      </div>

                      {selectedBook.notes ? (
                         <p className="text-white/80 text-sm leading-relaxed max-w-3xl mb-4 line-clamp-2">
                           {selectedBook.notes}
                         </p>
                      ) : (
                         <p className="text-white/80 text-sm leading-relaxed max-w-3xl mb-4 italic opacity-60 line-clamp-2">
                           No description or notes available.
                         </p>
                      )}
                      
                      <div className="flex items-center gap-4 text-xs">
                        {(selectedBook.rating !== undefined && selectedBook.rating !== null) && (
                          <div className="flex items-center gap-1 text-white/90">
                            <Star className="w-3.5 h-3.5 fill-amber-500 text-amber-500" />
                            <span className="font-medium">{selectedBook.rating}</span>
                          </div>
                        )}
                        
                        {(selectedBook.page_count !== undefined && selectedBook.page_count !== null) && (
                          <div className="text-white/50 font-medium">
                            {selectedBook.page_count} pages
                          </div>
                        )}

                        {selectedBook.tags && selectedBook.tags.length > 0 && (
                          <div 
                            className="px-2 py-0.5 rounded-full border text-[10px] font-medium"
                            style={{ 
                              borderColor: `${shelfColor}50`, 
                              color: shelfColor,
                              backgroundColor: `${shelfColor}10` 
                            }}
                          >
                            {selectedBook.tags[0].name}
                          </div>
                        )}
                      </div>
                    </div>
                  </motion.div>
                )}
              </AnimatePresence>
            );

            return (
              <React.Fragment key={rowIndex}>
                {showAbove && <ExpandedCard />}
                <div 
                  className="grid gap-4 sm:gap-6" 
                  style={{ gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` }}
                >
                  {rowBooks.map((book) => {
                    const isSelected = book.id === selectedBookId;
                    // Generate a deterministic color based on book ID for the flat covers
                    const hue = (book.id || 0) * 137.508 % 360;
                    const coverColor = `hsl(${hue}, 40%, 30%)`;
                    
                    return (
                      <button
                        key={book.id}
                        onClick={() => setSelectedBookId(isSelected ? null : book.id!)}
                        className={cn(
                          "relative aspect-[2/3] w-full rounded-xl overflow-hidden transition-all duration-300 text-left flex flex-col p-4",
                          isSelected ? "ring-2 ring-offset-4 ring-offset-[#0a0a0a] scale-[1.02] shadow-2xl z-10" : "hover:scale-[1.02] hover:shadow-xl opacity-90 hover:opacity-100"
                        )}
                        style={{
                          background: `linear-gradient(135deg, ${coverColor} 0%, hsl(${hue}, 50%, 20%) 100%)`,
                          boxShadow: isSelected ? `0 0 20px ${shelfColor}40` : undefined,
                          borderColor: isSelected ? shelfColor : 'transparent',
                        }}
                      >
                        {/* Flat colored covers from the mockup */}
                        <div className="font-serif text-white/90 font-medium text-lg leading-tight line-clamp-4 shadow-black drop-shadow-md">
                          {book.title}
                        </div>
                        <div className="mt-auto text-xs text-white/60 font-medium truncate">
                          {book.authors && book.authors.length > 0 ? book.authors.map(a => a.name).join(', ') : 'Unknown Author'}
                        </div>
                      </button>
                    );
                  })}
                </div>
                {!showAbove && <ExpandedCard />}
              </React.Fragment>
            );
          })}
        </div>
        
        {books.length === 0 && (
          <div className="flex flex-col items-center justify-center py-20 text-center">
            <h2 className="text-xl font-semibold mb-2 text-white">Shelf is empty</h2>
            <p className="text-white/50 mb-8 max-w-sm">
              You haven't added any books to this shelf yet.
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
