/**
 * Shelf Suggestion Utility
 * Extracts folder names from file paths and suggests creating shelves
 */

// Generic folder names to skip (too common, not meaningful)
const GENERIC_NAMES = new Set([
  'books',
  'downloads',
  'documents',
  'ebooks',
  'new folder',
  'archive',
  'backup',
  'temp',
  'tmp',
  'home',
  'desktop',
  'library',
  'storage',
  'media',
  'files',
  'documents',
  'folder',
  'misc',
  'various',
]);

export interface ShelfSuggestion {
  name: string;
  bookCount: number;
  filePaths: string[];
}

/**
 * Extract meaningful folder names from file paths
 * Example: "/Users/john/Books/Fiction/SciFi/dune.epub" → "SciFi"
 * Takes last 1-2 folder levels but skips generic names
 */
function extractFolderName(filePath: string): string | null {
  const normalized = filePath.replace(/\\/g, '/');
  const parts = normalized.split('/').filter(p => p.length > 0);
  
  if (parts.length === 0) return null;
  
  let folderIndex = parts.length - 2;
  if (folderIndex < 0) return null;
  
  while (folderIndex >= 0) {
    const folderName = parts[folderIndex].toLowerCase();
    
    if (folderName.length < 2 || GENERIC_NAMES.has(folderName)) {
      folderIndex--;
      continue;
    }
    
    return parts[folderIndex];
  }
  
  return null;
}

/**
 * Generate shelf suggestions from imported file paths
 * Groups books by folder name and suggests creating shelves
 */
import { Book } from './tauri';

export function generateShelfSuggestions(
  books: Book[]
): ShelfSuggestion[] {
  const suggestionsMap = new Map<string, string[]>();
  
  // Group by folder name AND series
  for (const book of books) {
    const path = book.file_path;
    
    // 1. Group by Series (if available)
    if (book.series) {
      const normalizedSeries = book.series.trim();
      if (normalizedSeries.length > 0) {
        if (!suggestionsMap.has(normalizedSeries)) {
          suggestionsMap.set(normalizedSeries, []);
        }
        suggestionsMap.get(normalizedSeries)!.push(path);
      }
    }
    
    // 2. Group by Folder Name
    const folderName = extractFolderName(path);
    if (folderName) {
      // Normalize folder name (Title Case)
      const normalizedName = folderName
        .split(/[-_\s]+/)
        .map(word => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
        .join(' ');
      
      // If we already grouped this book by a series that matches the folder name, skip to avoid duplicates
      if (book.series && book.series.trim().toLowerCase() === normalizedName.toLowerCase()) {
         continue;
      }

      if (!suggestionsMap.has(normalizedName)) {
        suggestionsMap.set(normalizedName, []);
      }
      // Only push if not already in this folder suggestion
      if (!suggestionsMap.get(normalizedName)!.includes(path)) {
        suggestionsMap.get(normalizedName)!.push(path);
      }
    }
  }
  
  // Convert to array and filter out single-book suggestions
  // (only suggest if 2+ books in same folder/series)
  const suggestions: ShelfSuggestion[] = Array.from(
    suggestionsMap.entries()
  )
    .filter(([_, paths]) => paths.length > 1)
    .map(([name, paths]) => ({
      name,
      bookCount: paths.length,
      filePaths: paths,
    }))
    .sort((a, b) => b.bookCount - a.bookCount); // Sort by count descending
  
  return suggestions;
}

/**
 * Get book IDs from file paths (used to add books to newly created shelves)
 * This is used after we know which books were successfully imported
 * The books returned from ImportResult.success are file paths
 */
export function filterPathsForShelf(
  allPaths: string[],
  shelfPaths: string[]
): string[] {
  const pathSet = new Set(shelfPaths);
  return allPaths.filter(p => pathSet.has(p));
}
