import { create } from 'zustand';

// ────────────────────────────────────────────────────────────
// DOODLE STROKE TYPES
// ────────────────────────────────────────────────────────────

export interface DoodleStroke {
    id: string;
    tool: 'pen' | 'eraser';
    color: string;
    width: number;
    points: [number, number, number][]; // [x%, y%, pressure]
    timestamp: number;
}

export type DoodleAction =
    | { type: 'addStroke'; stroke: DoodleStroke }
    | { type: 'removeStroke'; strokeId: string; stroke: DoodleStroke }
    | { type: 'clearAll'; strokes: DoodleStroke[] };

// ────────────────────────────────────────────────────────────
// DOODLE UI STATE (ephemeral, not persisted)
// ────────────────────────────────────────────────────────────

interface DoodleState {
    // Mode
    isDoodleMode: boolean;

    // Tool settings
    tool: 'pen' | 'eraser';
    penColor: string;
    penWidth: number;

    // Active page tracking
    activePageId: string | null;
    setActivePage: (pageId: string) => void;

    // Strokes map for multiple pages (pageId -> strokes)
    strokesMap: Record<string, DoodleStroke[]>;

    // Undo/Redo stacks per page
    undoStackMap: Record<string, DoodleAction[]>;
    redoStackMap: Record<string, DoodleAction[]>;

    // Dirty flags per page
    isDirtyMap: Record<string, boolean>;

    // Actions
    toggleDoodleMode: () => void;
    setDoodleMode: (active: boolean) => void;
    setTool: (tool: 'pen' | 'eraser') => void;
    setPenColor: (color: string) => void;
    setPenWidth: (width: number) => void;

    // Stroke operations
    addStroke: (pageId: string, stroke: DoodleStroke) => void;
    undo: (pageId: string) => void;
    redo: (pageId: string) => void;
    clearAll: (pageId: string) => void;

    // Page lifecycle
    loadStrokes: (pageId: string, strokes: DoodleStroke[]) => void;
    resetPage: (pageId: string) => void;
    markClean: (pageId: string) => void;
}

const MAX_UNDO_STACK = 50;
const MAX_STROKES_PER_PAGE = 500;

export const useDoodleStore = create<DoodleState>((set) => ({
    isDoodleMode: false,
    tool: 'pen',
    penColor: '#1A1A2E',
    penWidth: 3,
    activePageId: null,
    strokesMap: {},
    undoStackMap: {},
    redoStackMap: {},
    isDirtyMap: {},

    setActivePage: (pageId) => set({ activePageId: pageId }),

    toggleDoodleMode: () => set((state) => ({
        isDoodleMode: !state.isDoodleMode,
        ...(state.isDoodleMode ? {} : { tool: 'pen' as const }),
    })),

    setDoodleMode: (active) => set({ isDoodleMode: active }),

    setTool: (tool) => set({ tool }),

    setPenColor: (color) => set({ penColor: color }),

    setPenWidth: (width) => set({ penWidth: Math.max(1, Math.min(20, width)) }),

    addStroke: (pageId, stroke) => set((state) => {
        let newStrokes = [...(state.strokesMap[pageId] || []), stroke];

        if (newStrokes.length > MAX_STROKES_PER_PAGE) {
            newStrokes = newStrokes.slice(newStrokes.length - MAX_STROKES_PER_PAGE);
        }

        const action: DoodleAction = { type: 'addStroke', stroke };
        let newUndoStack = [...(state.undoStackMap[pageId] || []), action];
        if (newUndoStack.length > MAX_UNDO_STACK) {
            newUndoStack = newUndoStack.slice(1);
        }

        return {
            strokesMap: { ...state.strokesMap, [pageId]: newStrokes },
            undoStackMap: { ...state.undoStackMap, [pageId]: newUndoStack },
            redoStackMap: { ...state.redoStackMap, [pageId]: [] }, // Clear redo
            isDirtyMap: { ...state.isDirtyMap, [pageId]: true },
        };
    }),

    undo: (pageId) => set((state) => {
        const undoStack = state.undoStackMap[pageId] || [];
        if (undoStack.length === 0) return state;

        const action = undoStack[undoStack.length - 1];
        const newUndoStack = undoStack.slice(0, -1);

        let newStrokes = [...(state.strokesMap[pageId] || [])];
        let redoAction: DoodleAction;

        switch (action.type) {
            case 'addStroke':
                newStrokes = newStrokes.filter((s) => s.id !== action.stroke.id);
                redoAction = action;
                break;
            case 'removeStroke':
                newStrokes.push(action.stroke);
                redoAction = action;
                break;
            case 'clearAll':
                newStrokes = action.strokes;
                redoAction = action;
                break;
        }

        const redoStack = state.redoStackMap[pageId] || [];
        return {
            strokesMap: { ...state.strokesMap, [pageId]: newStrokes },
            undoStackMap: { ...state.undoStackMap, [pageId]: newUndoStack },
            redoStackMap: { ...state.redoStackMap, [pageId]: [...redoStack, redoAction!] },
            isDirtyMap: { ...state.isDirtyMap, [pageId]: true },
        };
    }),

    redo: (pageId) => set((state) => {
        const redoStack = state.redoStackMap[pageId] || [];
        if (redoStack.length === 0) return state;

        const action = redoStack[redoStack.length - 1];
        const newRedoStack = redoStack.slice(0, -1);

        let newStrokes = [...(state.strokesMap[pageId] || [])];

        switch (action.type) {
            case 'addStroke':
                newStrokes.push(action.stroke);
                break;
            case 'removeStroke':
                newStrokes = newStrokes.filter((s) => s.id !== action.stroke.id);
                break;
            case 'clearAll':
                newStrokes = [];
                break;
        }

        const undoStack = state.undoStackMap[pageId] || [];
        return {
            strokesMap: { ...state.strokesMap, [pageId]: newStrokes },
            undoStackMap: { ...state.undoStackMap, [pageId]: [...undoStack, action] },
            redoStackMap: { ...state.redoStackMap, [pageId]: newRedoStack },
            isDirtyMap: { ...state.isDirtyMap, [pageId]: true },
        };
    }),

    clearAll: (pageId) => set((state) => {
        const strokes = state.strokesMap[pageId] || [];
        if (strokes.length === 0) return state;

        const action: DoodleAction = { type: 'clearAll', strokes: [...strokes] };
        const undoStack = state.undoStackMap[pageId] || [];
        let newUndoStack = [...undoStack, action];
        if (newUndoStack.length > MAX_UNDO_STACK) {
            newUndoStack = newUndoStack.slice(1);
        }

        return {
            strokesMap: { ...state.strokesMap, [pageId]: [] },
            undoStackMap: { ...state.undoStackMap, [pageId]: newUndoStack },
            redoStackMap: { ...state.redoStackMap, [pageId]: [] },
            isDirtyMap: { ...state.isDirtyMap, [pageId]: true },
        };
    }),

    loadStrokes: (pageId, strokes) => set((state) => ({
        strokesMap: { ...state.strokesMap, [pageId]: strokes },
        undoStackMap: { ...state.undoStackMap, [pageId]: [] },
        redoStackMap: { ...state.redoStackMap, [pageId]: [] },
        isDirtyMap: { ...state.isDirtyMap, [pageId]: false },
    })),

    resetPage: (pageId) => set((state) => ({
        strokesMap: { ...state.strokesMap, [pageId]: [] },
        undoStackMap: { ...state.undoStackMap, [pageId]: [] },
        redoStackMap: { ...state.redoStackMap, [pageId]: [] },
        isDirtyMap: { ...state.isDirtyMap, [pageId]: false },
    })),

    markClean: (pageId) => set((state) => ({
        isDirtyMap: { ...state.isDirtyMap, [pageId]: false },
    })),
}));
