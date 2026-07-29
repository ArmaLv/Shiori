import { useState, useEffect } from 'react';
import * as Dialog from '@radix-ui/react-dialog';
import { motion, AnimatePresence } from 'framer-motion';
import { X, Folder, Sparkles, BookMarked, Loader2, Library, Star, Heart, Bookmark, BookOpen, Target, Lightbulb, Palette, Flame, FolderOpen } from 'lucide-react';
import { api, Shelf, SmartRule } from '../../lib/tauri';
import { logger } from '@/lib/logger';
import { useShelfStore } from '../../store/shelfStore';
import { SmartShelfEditor } from './SmartShelfEditor';
import { useToast } from '@/store/toastStore';
import { cn } from '@/lib/utils';

interface CreateShelfDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  editShelf?: Shelf | null;
  parentId?: number;
}

const PRESET_COLORS = [
  '#3b82f6', // blue
  '#ef4444', // red
  '#10b981', // green
  '#f59e0b', // amber
  '#8b5cf6', // purple
  '#ec4899', // pink
  '#06b6d4', // cyan
  '#84cc16', // lime
];

const PRESET_ICONS = [
  { id: 'library', icon: Library },
  { id: 'star', icon: Star },
  { id: 'heart', icon: Heart },
  { id: 'bookmark', icon: Bookmark },
  { id: 'bookopen', icon: BookOpen },
  { id: 'target', icon: Target },
  { id: 'sparkles', icon: Sparkles },
  { id: 'lightbulb', icon: Lightbulb },
  { id: 'palette', icon: Palette },
  { id: 'flame', icon: Flame },
];

export const CreateShelfDialog = ({
  open,
  onOpenChange,
  editShelf,
  parentId,
}: CreateShelfDialogProps) => {
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [color, setColor] = useState(PRESET_COLORS[0]);
  const [icon, setIcon] = useState('');
  const [isSmart, setIsSmart] = useState(false);
  const [shelfType, setShelfType] = useState<'regular' | 'books' | 'manga' | 'mixed' | 'shelf'>('regular');
  const [smartRules, setSmartRules] = useState<SmartRule[]>([]);
  const [selectedParentId, setSelectedParentId] = useState<number | null>(null);
  const [allShelfs, setAllShelfs] = useState<Shelf[]>([]);
  const [loading, setLoading] = useState(false);
  const [errors, setErrors] = useState<{ name?: string; rules?: string }>({});
  const [previewCount, setPreviewCount] = useState<number | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);

  const toast = useToast();

  const addShelf = useShelfStore(state => state.addShelf);
  const updateShelf = useShelfStore(state => state.updateShelf);

  useEffect(() => {
    if (open) {
      loadShelfs();
      
      if (editShelf) {
        setName(editShelf.name);
        setDescription(editShelf.description || '');
        setColor(editShelf.color || PRESET_COLORS[0]);
        setIcon(editShelf.icon || '');
        setIsSmart(editShelf.isSmart);
        setShelfType((editShelf.shelfType as any) || 'regular');
        setSelectedParentId(editShelf.parentId || null);

        if (editShelf.smartRules) {
          try {
            const rules = JSON.parse(editShelf.smartRules);
            setSmartRules(rules);
          } catch (e) {
             logger.error('Failed to parse smart rules:', e);
          }
        }
      } else {
        resetForm();
        setSelectedParentId(parentId || null);
      }
    }
  }, [open, editShelf, parentId]);

  const loadShelfs = async () => {
    try {
      const cols = await api.getShelfs();
      setAllShelfs(cols || []);
    } catch (error) {
      logger.error('Failed to load shelves:', error);
    }
  };

  const resetForm = () => {
    setName('');
    setDescription('');
    setColor(PRESET_COLORS[0]);
    setIcon('');
    setIsSmart(false);
    setShelfType('regular');
    setSmartRules([]);
    setSelectedParentId(null);
    setErrors({});
    setPreviewCount(null);
  };

  const updatePreview = async (rules: SmartRule[]) => {
    if (rules.length === 0) {
      setPreviewCount(null);
      return;
    }

    setPreviewLoading(true);
    try {
      const count = await api.previewSmartShelf(JSON.stringify(rules));
      setPreviewCount(count);
    } catch (error) {
      logger.error('Failed to preview smart shelf:', error);
      setPreviewCount(null);
    } finally {
      setPreviewLoading(false);
    }
  };

  const validate = () => {
    const newErrors: typeof errors = {};
    if (!name.trim()) newErrors.name = "Required";
    if (isSmart && smartRules.length === 0) {
      newErrors.rules = "Needs at least one rule";
    }
    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!validate()) return;

    setLoading(true);
    try {
      const shelfData = {
        name: name.trim(),
        description: description.trim() || null,
        parent_id: selectedParentId,
        is_smart: isSmart,
        smart_rules: isSmart ? JSON.stringify(smartRules) : null,
        icon: icon || null,
        color: color || null,
        shelf_type: shelfType,
      };

      if (editShelf && editShelf.id !== undefined) {
        const updated = await api.updateShelf(editShelf.id, shelfData);
        updateShelf(editShelf.id, updated);
      } else {
        const created = await api.createShelf(shelfData);
        addShelf(created);
      }

      onOpenChange(false);
      resetForm();
    } catch (error) {
      logger.error('Failed to save shelf:', error);
      toast.error('Failed to save shelf', error instanceof Error ? error.message : 'Unknown error');
    } finally {
      setLoading(false);
    }
  };

  const getAvailableParentShelfs = () => {
    if (!editShelf) return allShelfs;
    const excludedIds = new Set<number>([editShelf.id!]);
    const findDescendants = (parentId: number) => {
      allShelfs.forEach(c => {
        if (c.parentId === parentId && !excludedIds.has(c.id!)) {
          excludedIds.add(c.id!);
          findDescendants(c.id!);
        }
      });
    };
    findDescendants(editShelf.id!);
    return allShelfs.filter(c => !excludedIds.has(c.id!));
  };

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/60 backdrop-blur-xl z-[200] data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0" />
        <Dialog.Content 
          aria-describedby={undefined} 
          className="fixed left-[50%] top-[50%] z-[200] w-[95vw] max-w-[800px] translate-x-[-50%] translate-y-[-50%] bg-[#0a0a0a]/90 backdrop-blur-2xl p-6 md:p-10 shadow-2xl border border-white/10 rounded-3xl duration-200 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[state=closed]:slide-out-to-left-1/2 data-[state=closed]:slide-out-to-top-[48%] data-[state=open]:slide-in-from-left-1/2 data-[state=open]:slide-in-from-top-[48%] max-h-[90vh] overflow-y-auto"
        >
          <div className="flex items-center justify-between mb-8">
            <Dialog.Title className="text-2xl font-bold tracking-tight text-white/90">
              {editShelf ? 'Edit Shelf' : 'Create Shelf'}
            </Dialog.Title>
            <Dialog.Close asChild>
              <button className="rounded-full p-2 text-white/50 hover:bg-white/10 hover:text-white transition-colors">
                <X className="h-5 w-5" />
                <span className="sr-only">Close</span>
              </button>
            </Dialog.Close>
          </div>

          <form onSubmit={handleSubmit} className="flex flex-col gap-8">
            {/* Type Toggle */}
            <div className="flex p-1 bg-white/5 rounded-2xl border border-white/5 w-full max-w-sm">
              <button
                type="button"
                onClick={() => setIsSmart(false)}
                className={cn(
                  "flex flex-1 items-center justify-center gap-2 py-2.5 rounded-xl text-sm font-medium transition-all duration-300",
                  !isSmart ? "bg-white/10 text-white shadow-sm" : "text-white/40 hover:text-white/80 hover:bg-white/5"
                )}
              >
                <Folder className="w-4 h-4" />
                Manual
              </button>
              <button
                type="button"
                onClick={() => setIsSmart(true)}
                className={cn(
                  "flex flex-1 items-center justify-center gap-2 py-2.5 rounded-xl text-sm font-medium transition-all duration-300",
                  isSmart ? "bg-white/10 text-white shadow-sm" : "text-white/40 hover:text-white/80 hover:bg-white/5"
                )}
              >
                <Sparkles className="w-4 h-4" />
                Smart
              </button>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-x-12 gap-y-8">
              {/* Left Column: Basic Info */}
              <div className="space-y-6">
                <div className="space-y-2">
                  <label className="text-sm font-medium text-white/70">Name</label>
                  <input
                    type="text"
                    value={name}
                    onChange={(e) => {
                      setName(e.target.value);
                      setErrors(prev => ({ ...prev, name: undefined }));
                    }}
                    className={cn(
                      "flex w-full rounded-xl border-b-2 bg-transparent px-2 py-3 text-lg text-white/90 placeholder:text-white/30 transition-all focus:outline-none",
                      errors.name ? "border-rose-500/50" : "border-white/10 focus:border-white/40 hover:border-white/30"
                    )}
                    placeholder="e.g. Science Fiction"
                    required
                  />
                  {errors.name && <p className="text-xs text-rose-500">{errors.name}</p>}
                </div>

                <div className="space-y-2">
                  <label className="text-sm font-medium text-white/70">Description (Optional)</label>
                  <textarea
                    value={description}
                    onChange={(e) => setDescription(e.target.value)}
                    className="flex min-h-[60px] w-full rounded-xl border-b-2 border-white/10 bg-transparent px-2 py-3 text-sm text-white/90 placeholder:text-white/30 transition-all focus:outline-none focus:border-white/40 hover:border-white/30 resize-none"
                    placeholder="What's this shelf about?"
                  />
                </div>

                <div className="space-y-2">
                  <label className="text-sm font-medium text-white/70">Parent Shelf (Optional)</label>
                  <select
                    value={selectedParentId || ''}
                    onChange={(e) => setSelectedParentId(e.target.value ? Number(e.target.value) : null)}
                    className="flex w-full appearance-none rounded-xl border-b-2 border-white/10 bg-transparent px-2 py-3 text-sm text-white/90 focus:outline-none focus:border-white/40 hover:border-white/30 transition-all"
                  >
                    <option value="" className="bg-[#1a1a1a] text-white">None (Top Level)</option>
                    {getAvailableParentShelfs().map((col) => (
                      <option key={col.id} value={col.id} className="bg-[#1a1a1a] text-white">{col.name}</option>
                    ))}
                  </select>
                </div>

                {!isSmart && (
                  <div className="space-y-3 pt-2">
                    <label className="text-sm font-medium text-white/70">Content Type</label>
                    <div className="grid grid-cols-3 gap-3">
                      {[
                        { id: 'mixed', label: 'Mixed', icon: FolderOpen },
                        { id: 'books', label: 'Books', icon: BookMarked },
                        { id: 'manga', label: 'Manga', icon: Sparkles }
                      ].map(type => {
                        const Icon = type.icon;
                        const isActive = shelfType === type.id || (type.id === 'mixed' && shelfType === 'regular');
                        return (
                          <button
                            key={type.id}
                            type="button"
                            onClick={() => setShelfType(type.id as any)}
                            className={cn(
                              "flex flex-col items-center justify-center gap-2 py-3 rounded-2xl border text-xs font-medium transition-all duration-300",
                              isActive ? "bg-white/10 border-white/20 text-white shadow-sm" : "bg-transparent border-white/5 text-white/40 hover:bg-white/5 hover:text-white/80"
                            )}
                          >
                            <Icon className={cn("w-5 h-5", isActive ? "text-white" : "")} />
                            {type.label}
                          </button>
                        )
                      })}
                    </div>
                  </div>
                )}
              </div>

              {/* Right Column: Styling & Smart Rules */}
              <div className="space-y-8">
                {/* Theme Color */}
                <div className="space-y-4">
                  <label className="text-sm font-medium text-white/70">Color Theme</label>
                  <div className="flex flex-wrap gap-3">
                    {PRESET_COLORS.map((presetColor) => (
                      <button
                        key={presetColor}
                        type="button"
                        onClick={() => setColor(presetColor)}
                        className={cn(
                          "w-8 h-8 rounded-full transition-all duration-300 outline-none",
                          color === presetColor ? "ring-2 ring-white/80 ring-offset-2 ring-offset-[#0a0a0a] scale-110 shadow-lg" : "opacity-50 hover:opacity-100 hover:scale-110"
                        )}
                        style={{ backgroundColor: presetColor, boxShadow: color === presetColor ? `0 0 15px ${presetColor}80` : 'none' }}
                      />
                    ))}
                  </div>
                </div>

                {/* SVG Icon Picker */}
                <div className="space-y-4">
                  <label className="text-sm font-medium text-white/70">Icon Symbol</label>
                  <div className="flex flex-wrap gap-2">
                    <button
                      type="button"
                      onClick={() => setIcon('')}
                      className={cn(
                        "w-10 h-10 rounded-xl flex items-center justify-center transition-all duration-300",
                        !icon ? "bg-white/20 text-white shadow-lg" : "bg-transparent text-white/30 hover:bg-white/5 hover:text-white"
                      )}
                    >
                      <X className="w-5 h-5" />
                    </button>
                    {PRESET_ICONS.map((preset) => {
                      const IconComponent = preset.icon;
                      return (
                        <button
                          key={preset.id}
                          type="button"
                          onClick={() => setIcon(preset.id)}
                          className={cn(
                            "w-10 h-10 rounded-xl flex items-center justify-center transition-all duration-300",
                            icon === preset.id ? "bg-white/20 text-white shadow-lg" : "bg-transparent text-white/30 hover:bg-white/5 hover:text-white"
                          )}
                        >
                          <IconComponent className="w-5 h-5" />
                        </button>
                      )
                    })}
                  </div>
                </div>

                {isSmart && (
                  <div className="space-y-3 pt-2">
                    <label className="text-sm font-medium text-white/70">Smart Rules</label>
                    <div className={cn("rounded-2xl border overflow-hidden", errors.rules ? "border-rose-500/50" : "border-white/10")}>
                      <SmartShelfEditor 
                        rules={smartRules} 
                        onChange={(rules) => {
                          setSmartRules(rules);
                          setErrors(prev => ({ ...prev, rules: undefined }));
                          updatePreview(rules);
                        }} 
                      />
                    </div>
                    {errors.rules && <p className="text-xs text-rose-500">{errors.rules}</p>}
                    
                    {previewLoading ? (
                      <p className="text-xs text-white/40 flex items-center gap-1"><Loader2 className="w-3 h-3 animate-spin" /> Calculating preview...</p>
                    ) : previewCount !== null ? (
                      <p className="text-xs text-green-400/80 font-medium">Matches {previewCount} books</p>
                    ) : null}
                  </div>
                )}
              </div>
            </div>

            <div className="flex justify-end pt-6 mt-4 border-t border-white/10">
              <button
                type="submit"
                disabled={loading}
                className="w-full md:w-auto px-8 py-3 text-sm font-semibold rounded-xl bg-white text-black hover:bg-white/90 transition-all duration-300 flex items-center justify-center gap-2 disabled:opacity-50"
              >
                {loading && <Loader2 className="w-4 h-4 animate-spin" />}
                {loading ? 'Saving...' : editShelf ? 'Update Shelf' : 'Create Shelf'}
              </button>
            </div>
          </form>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
};
