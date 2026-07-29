import React, { useState, useRef, useEffect } from 'react';
import { Book, Shelf } from '../../lib/tauri';
import { Star, X, BookOpen, ArrowLeft } from 'lucide-react';
import { cn } from '@/lib/utils';
import { motion, AnimatePresence } from 'framer-motion';
import { useCoverImage } from '../common/hooks/useCoverImage';

interface ShelfBookGridProps {
  shelf: Shelf;
  books: Book[];
  onBack: () => void;
}

function ShelfBookCard({ book, isSelected, onClick, shelfColor }: { book: Book, isSelected: boolean, onClick: () => void, shelfColor: string }) {
  const { coverUrl, loading } = useCoverImage(book.id, book.cover_path);
  const [imgLoaded, setImgLoaded] = useState(false);
  const [imgError, setImgError] = useState(false);
  
  const hue = (book.id || 0) * 137.508 % 360;
  const coverColor = `hsl(${hue}, 40%, 30%)`;

  return (
    <button
      onClick={onClick}
      className={cn(
        "relative aspect-[2/3] w-full rounded-xl overflow-hidden transition-all duration-300 text-left flex flex-col p-4 group",
        isSelected ? "ring-2 ring-offset-4 ring-offset-background scale-[1.02] shadow-2xl z-10" : "hover:scale-[1.02] hover:shadow-xl opacity-90 hover:opacity-100"
      )}
      style={{
        boxShadow: isSelected ? `0 0 20px ${shelfColor}40` : undefined,
        borderColor: isSelected ? shelfColor : 'transparent',
      }}
    >
      <div 
        className="absolute inset-0 z-0 p-4 flex flex-col"
        style={{
          background: `linear-gradient(135deg, ${coverColor} 0%, hsl(${hue}, 50%, 20%) 100%)`,
        }}
      >
        <div className="font-serif text-white/90 font-medium text-lg leading-tight line-clamp-4 shadow-black drop-shadow-md">
          {book.title}
        </div>
        <div className="mt-auto text-xs text-white/60 font-medium truncate">
          {book.authors && book.authors.length > 0 ? book.authors.map(a => a.name).join(', ') : 'Unknown Author'}
        </div>
      </div>

      {coverUrl && !imgError && (
        <img
          src={coverUrl}
          alt={book.title}
          loading="lazy"
          onLoad={() => setImgLoaded(true)}
          onError={() => setImgError(true)}
          className={cn(
            'absolute inset-0 w-full h-full object-cover bg-muted z-10',
            'transition-opacity duration-500',
            imgLoaded ? 'opacity-100' : 'opacity-0'
          )}
        />
      )}
      
      <div className="absolute inset-0 bg-gradient-to-tr from-transparent via-white/0 to-white/20 pointer-events-none mix-blend-overlay opacity-0 group-hover:opacity-100 transition-opacity duration-500 z-20" />
      <div className="absolute inset-0 shadow-[inset_0_0_0_1px_rgba(0,0,0,0.1)] dark:shadow-[inset_0_0_0_1px_rgba(255,255,255,0.1)] pointer-events-none z-20" />
    </button>
  );
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

  const rows = [];
  for (let i = 0; i < books.length; i += columns) {
    rows.push(books.slice(i, i + columns));
  }

  const shelfColor = shelf.color || '#f59e0b';

  return (
    <div className="p-4 sm:p-8 h-full overflow-y-auto" ref={containerRef}>
      <div className="max-w-[1400px] mx-auto">
        <div className="mb-8 relative sticky top-0 z-10 bg-background/90 backdrop-blur-md pt-2 pb-4 -mx-4 px-4 sm:-mx-8 sm:px-8 border-b border-border/50">
          <button 
            onClick={onBack}
            className="mb-6 flex items-center gap-2 text-muted-foreground hover:text-foreground transition-colors group"
          >
            <div className="w-8 h-8 rounded-full bg-foreground/5 flex items-center justify-center group-hover:bg-foreground/10 transition-colors">
              <ArrowLeft size={16} />
            </div>
            <span className="text-sm font-medium">Back to Shelves</span>
          </button>

          <div className="text-[11px] font-bold tracking-[0.2em] text-muted-foreground uppercase mb-3 mt-2">
            MY SHELF · {books.length} BOOKS
          </div>
          <h1 className="text-4xl sm:text-5xl font-bold tracking-tight text-foreground mb-2" style={{ fontFamily: 'var(--font-serif)', letterSpacing: '-0.02em' }}>
            {shelf.name}
          </h1>
          <p className="text-muted-foreground/70 text-sm">
            Tap a book to open its card — tap it again to close.
          </p>
        </div>

        <div className="flex flex-col gap-6 relative">
          {rows.map((rowBooks, rowIndex) => {
            const hasSelected = rowBooks.some(b => b.id === selectedBookId);
            const selectedIndex = rowBooks.findIndex(b => b.id === selectedBookId);
            const selectedBook = rowBooks[selectedIndex];
            const showAbove = rowIndex < (rows.length / 2);

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
                        "relative rounded-2xl p-4 sm:p-5 bg-card",
                        showAbove ? "mb-4" : "mt-4"
                      )}
                      style={{ border: `1px solid ${shelfColor}40` }}
                    >
                      <div 
                        className={cn(
                          "absolute w-6 h-6 rotate-45 bg-card",
                          showAbove ? "-bottom-3 border-r border-b" : "-top-3 border-l border-t"
                        )}
                        style={{
                          left: `calc(${(selectedIndex + 0.5) / columns * 100}% - 12px)`,
                          borderColor: `${shelfColor}40`,
                          transition: 'left 0.3s ease-out'
                        }}
                      />

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
                          <h2 className="text-xl font-bold text-foreground italic mb-1" style={{ fontFamily: 'var(--font-serif)' }}>
                            {selectedBook.title}
                          </h2>
                          <div className="text-muted-foreground text-xs flex flex-wrap items-center gap-2">
                            <span>{selectedBook.authors && selectedBook.authors.length > 0 ? selectedBook.authors.map(a => a.name).join(', ') : 'Unknown Author'}</span>
                            {selectedBook.pubdate && <span>· {new Date(selectedBook.pubdate).getFullYear()}</span>}
                            {selectedBook.rating ? (
                              <span className="flex items-center gap-0.5 text-yellow-500">
                                · ★ {selectedBook.rating}
                              </span>
                            ) : null}
                            {selectedBook.page_count ? (
                              <span>· {selectedBook.page_count} pages</span>
                            ) : null}
                          </div>
                        </div>
                        
                        <div className="flex bg-foreground/5 rounded-full p-1 border border-border/50 shrink-0 items-center">
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              window.dispatchEvent(
                                new CustomEvent('open-book', { detail: { bookId: selectedBook.id } })
                              );
                            }}
                            className="px-3 py-1 rounded-full text-foreground/80 hover:text-foreground hover:bg-foreground/10 transition-colors text-xs font-medium flex items-center gap-1.5"
                          >
                            <BookOpen className="w-3.5 h-3.5" />
                            Read
                          </button>
                          <div className="w-px h-3 bg-border mx-1"></div>
                          <button
                            onClick={() => setSelectedBookId(null)}
                            className="p-1.5 rounded-full text-muted-foreground hover:text-foreground hover:bg-foreground/10 transition-colors"
                          >
                            <X className="w-3.5 h-3.5" />
                          </button>
                        </div>
                      </div>

                      {selectedBook.notes ? (
                         <p className="text-foreground/80 text-sm leading-relaxed max-w-3xl mb-4 line-clamp-2">
                           {selectedBook.notes}
                         </p>
                      ) : (
                         <p className="text-muted-foreground text-sm leading-relaxed max-w-3xl mb-4 italic line-clamp-2">
                           No description or notes available.
                         </p>
                      )}
                      
                      <div className="flex items-center gap-4 text-xs">
                        <div className="flex items-center gap-1 text-foreground/90">
                          <Star className="w-3.5 h-3.5 fill-amber-500 text-amber-500" />
                          <span className="font-medium">
                            {selectedBook.rating !== undefined && selectedBook.rating !== null ? selectedBook.rating : 'N/A'}
                          </span>
                        </div>
                        
                        <div className="text-muted-foreground font-medium">
                          {selectedBook.page_count !== undefined && selectedBook.page_count !== null ? selectedBook.page_count : '0'} pages
                        </div>

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
                    return (
                      <ShelfBookCard 
                        key={book.id}
                        book={book} 
                        isSelected={isSelected} 
                        onClick={() => setSelectedBookId(isSelected ? null : book.id!)} 
                        shelfColor={shelfColor} 
                      />
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
            <h2 className="text-xl font-semibold mb-2 text-foreground">Shelf is empty</h2>
            <p className="text-muted-foreground mb-8 max-w-sm">
              You haven't added any books to this shelf yet.
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
