import { useState } from 'react';
import { Plus, Trash2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { SmartRule } from '../../lib/tauri';

interface SmartShelfEditorProps {
  rules: SmartRule[];
  onChange: (rules: SmartRule[]) => void;
}

type RuleField = 'author' | 'tag' | 'format' | 'series' | 'rating' | 'added_date' | 'title' | 'publisher' | 'language' | 'reading_status' | 'is_favorite';
type RuleOperator = 'equals' | 'contains' | 'greater_than' | 'less_than' | 'in_last_days' | 'not_equals' | 'starts_with' | 'ends_with' | 'is_empty' | 'is_not_empty' | 'is_one_of';
type MatchType = 'all' | 'any';

const FIELD_OPTIONS: { value: RuleField; label: string }[] = [
  { value: 'title', label: 'Title' },
  { value: 'author', label: 'Author' },
  { value: 'tag', label: 'Tag' },
  { value: 'format', label: 'Format' },
  { value: 'series', label: 'Series' },
  { value: 'rating', label: 'Rating' },
  { value: 'publisher', label: 'Publisher' },
  { value: 'language', label: 'Language' },
  { value: 'reading_status', label: 'Reading Status' },
  { value: 'is_favorite', label: 'Favorite' },
  { value: 'added_date', label: 'Date Added' },
];

const OPERATOR_MAP: Record<RuleField, { value: RuleOperator; label: string }[]> = {
  title: [
    { value: 'contains', label: 'contains' },
    { value: 'equals', label: 'is exactly' },
    { value: 'not_equals', label: 'is not' },
    { value: 'starts_with', label: 'starts with' },
    { value: 'ends_with', label: 'ends with' },
  ],
  author: [
    { value: 'contains', label: 'contains' },
    { value: 'equals', label: 'is exactly' },
    { value: 'not_equals', label: 'is not' },
    { value: 'is_empty', label: 'is empty' },
    { value: 'is_not_empty', label: 'is not empty' },
  ],
  tag: [
    { value: 'contains', label: 'has tag' },
    { value: 'is_empty', label: 'has no tags' },
  ],
  format: [
    { value: 'equals', label: 'is' },
    { value: 'not_equals', label: 'is not' },
  ],
  series: [
    { value: 'equals', label: 'is' },
    { value: 'contains', label: 'contains' },
    { value: 'is_empty', label: 'is not in a series' },
    { value: 'is_not_empty', label: 'is in a series' },
  ],
  rating: [
    { value: 'equals', label: 'is' },
    { value: 'greater_than', label: 'is greater than' },
    { value: 'less_than', label: 'is less than' },
    { value: 'is_empty', label: 'is unrated' },
  ],
  publisher: [
    { value: 'contains', label: 'contains' },
    { value: 'equals', label: 'is exactly' },
    { value: 'is_empty', label: 'is empty' },
    { value: 'is_not_empty', label: 'is not empty' },
  ],
  language: [
    { value: 'equals', label: 'is' },
    { value: 'not_equals', label: 'is not' },
    { value: 'is_empty', label: 'is not set' },
  ],
  reading_status: [
    { value: 'equals', label: 'is' },
    { value: 'not_equals', label: 'is not' },
    { value: 'is_one_of', label: 'is one of' },
  ],
  is_favorite: [
    { value: 'equals', label: 'is' },
  ],
  added_date: [
    { value: 'in_last_days', label: 'in last N days' },
  ],
};

const FORMAT_OPTIONS = ['epub', 'pdf', 'mobi', 'azw3', 'txt', 'html'];
const READING_STATUS_OPTIONS = [
  { value: 'planning', label: 'Planning to Read' },
  { value: 'reading', label: 'Currently Reading' },
  { value: 'completed', label: 'Completed' },
  { value: 'paused', label: 'On Hold' },
  { value: 'dropped', label: 'Dropped' },
];

export const SmartShelfEditor = ({ rules, onChange }: SmartShelfEditorProps) => {
  const [matchType, setMatchType] = useState<MatchType>('all');

  const addRule = () => {
    const newRule: SmartRule = {
      field: 'author',
      operator: 'contains',
      value: '',
      matchType: matchType,
    };
    onChange([...rules, newRule]);
  };

  const removeRule = (index: number) => {
    const newRules = rules.filter((_, i) => i !== index);
    onChange(newRules);
  };

  const updateRule = (index: number, updates: Partial<SmartRule>) => {
    const newRules = rules.map((rule, i) => {
      if (i === index) {
        const updatedRule = { ...rule, ...updates };
        
        // Reset operator if field changed
        if (updates.field && updates.field !== rule.field) {
          const newField = updates.field as RuleField;
          updatedRule.operator = OPERATOR_MAP[newField][0].value;
          updatedRule.value = '';
        }
        
        return updatedRule;
      }
      return rule;
    });
    onChange(newRules);
  };

  const handleMatchTypeChange = (newMatchType: MatchType) => {
    setMatchType(newMatchType);
    // Update all rules with new match type
    const newRules = rules.map(rule => ({ ...rule, matchType: newMatchType }));
    onChange(newRules);
  };

  const renderValueInput = (rule: SmartRule, index: number) => {
    const field = rule.field as RuleField;
    const operator = rule.operator as RuleOperator;
    
    const inputClasses = "flex-1 px-4 py-2 border border-white/10 rounded-xl bg-white/5 text-sm text-white/90 placeholder:text-white/30 focus:outline-none focus:ring-1 focus:ring-white/20 focus:border-white/20 hover:border-white/20 transition-all";
    const selectClasses = "flex-1 appearance-none px-4 py-2 border border-white/10 rounded-xl bg-white/5 text-sm text-white/90 focus:outline-none focus:ring-1 focus:ring-white/20 focus:border-white/20 hover:border-white/20 transition-all";

    // Don't show value input for operators that don't need it
    if (['is_empty', 'is_not_empty'].includes(operator)) {
      return (
        <div className="flex-1 px-4 py-2 text-sm text-white/40 italic">
          No value needed
        </div>
      );
    }

    switch (field) {
      case 'format':
        return (
          <select
            value={rule.value}
            onChange={(e) => updateRule(index, { value: e.target.value })}
            className={selectClasses}
          >
            <option value="" className="bg-[#1a1a1a] text-white">Select format...</option>
            {FORMAT_OPTIONS.map((fmt) => (
              <option key={fmt} value={fmt} className="bg-[#1a1a1a] text-white">
                {fmt.toUpperCase()}
              </option>
            ))}
          </select>
        );

      case 'reading_status':
        if (operator === 'is_one_of') {
          // Multi-select for reading status
          const selectedStatuses = rule.value ? rule.value.split(',') : [];
          return (
            <div className="flex-1 flex flex-wrap gap-3 px-4 py-3 border border-white/10 rounded-xl bg-white/5">
              {READING_STATUS_OPTIONS.map((status) => (
                <label key={status.value} className="flex items-center gap-2 text-sm cursor-pointer text-white/80 hover:text-white transition-colors">
                  <input
                    type="checkbox"
                    checked={selectedStatuses.includes(status.value)}
                    onChange={(e) => {
                      const newStatuses = e.target.checked
                        ? [...selectedStatuses, status.value]
                        : selectedStatuses.filter(s => s !== status.value);
                      updateRule(index, { value: newStatuses.join(',') });
                    }}
                    className="rounded border-white/20 bg-white/10 text-white focus:ring-0 focus:ring-offset-0"
                  />
                  <span>{status.label}</span>
                </label>
              ))}
            </div>
          );
        } else {
          return (
            <select
              value={rule.value}
              onChange={(e) => updateRule(index, { value: e.target.value })}
              className={selectClasses}
            >
              <option value="" className="bg-[#1a1a1a] text-white">Select status...</option>
              {READING_STATUS_OPTIONS.map((status) => (
                <option key={status.value} value={status.value} className="bg-[#1a1a1a] text-white">
                  {status.label}
                </option>
              ))}
            </select>
          );
        }

      case 'is_favorite':
        return (
          <select
            value={rule.value}
            onChange={(e) => updateRule(index, { value: e.target.value })}
            className={selectClasses}
          >
            <option value="" className="bg-[#1a1a1a] text-white">Select...</option>
            <option value="true" className="bg-[#1a1a1a] text-white">Yes (Favorite)</option>
            <option value="false" className="bg-[#1a1a1a] text-white">No (Not Favorite)</option>
          </select>
        );

      case 'rating':
        return (
          <input
            type="number"
            min="0"
            max="5"
            step="0.5"
            value={rule.value}
            onChange={(e) => updateRule(index, { value: e.target.value })}
            placeholder="0-5"
            className={inputClasses}
          />
        );

      case 'language':
        return (
          <input
            type="text"
            value={rule.value}
            onChange={(e) => updateRule(index, { value: e.target.value })}
            placeholder="e.g., en, es, fr"
            className={inputClasses}
          />
        );

      case 'added_date':
        return (
          <input
            type="number"
            min="1"
            value={rule.value}
            onChange={(e) => updateRule(index, { value: e.target.value })}
            placeholder="Number of days"
            className={inputClasses}
          />
        );

      default:
        return (
          <input
            type="text"
            value={rule.value}
            onChange={(e) => updateRule(index, { value: e.target.value })}
            placeholder="Enter value..."
            className={inputClasses}
          />
        );
    }
  };

  const selectClasses = "appearance-none px-4 py-2 border border-white/10 rounded-xl bg-white/5 text-sm text-white/90 focus:outline-none focus:ring-1 focus:ring-white/20 focus:border-white/20 hover:border-white/20 transition-all";

  return (
    <div className="space-y-6 p-4">
      {/* Match Type Selector */}
      <div className="flex items-center gap-3 text-sm text-white/70">
        <span>Match</span>
        <select
          value={matchType}
          onChange={(e) => handleMatchTypeChange(e.target.value as MatchType)}
          className="appearance-none px-3 py-1.5 border border-white/10 rounded-lg bg-white/5 text-white focus:outline-none focus:border-white/30 cursor-pointer hover:bg-white/10 transition-colors"
        >
          <option value="all" className="bg-[#1a1a1a] text-white">ALL</option>
          <option value="any" className="bg-[#1a1a1a] text-white">ANY</option>
        </select>
        <span>of the following rules:</span>
      </div>

      {/* Rules List */}
      <div className="space-y-3">
        {rules.length === 0 ? (
          <div className="text-center py-10 border border-dashed border-white/10 rounded-2xl bg-white/5">
            <p className="text-sm text-white/50 mb-3">
              No rules added yet
            </p>
            <button
              type="button"
              onClick={addRule}
              className="text-sm text-white/70 hover:text-white border-b border-white/30 hover:border-white transition-colors"
            >
              Add your first rule
            </button>
          </div>
        ) : (
          rules.map((rule, index) => {
            const field = rule.field as RuleField;
            const operators = OPERATOR_MAP[field];

            return (
              <div key={index} className="flex flex-col sm:flex-row sm:items-center gap-3 p-4 bg-white/5 border border-white/5 rounded-2xl relative">
                {/* Field Selector */}
                <select
                  value={rule.field}
                  onChange={(e) => updateRule(index, { field: e.target.value as RuleField })}
                  className={cn(selectClasses, "w-full sm:w-32")}
                >
                  {FIELD_OPTIONS.map((opt) => (
                    <option key={opt.value} value={opt.value} className="bg-[#1a1a1a] text-white">
                      {opt.label}
                    </option>
                  ))}
                </select>

                {/* Operator Selector */}
                <select
                  value={rule.operator}
                  onChange={(e) => updateRule(index, { operator: e.target.value as RuleOperator })}
                  className={cn(selectClasses, "w-full sm:w-40")}
                >
                  {operators.map((opt) => (
                    <option key={opt.value} value={opt.value} className="bg-[#1a1a1a] text-white">
                      {opt.label}
                    </option>
                  ))}
                </select>

                {/* Value Input */}
                <div className="flex-1 w-full flex">
                  {renderValueInput(rule, index)}
                </div>

                {/* Remove Button */}
                <button
                  type="button"
                  onClick={() => removeRule(index)}
                  className="p-2 hover:bg-white/10 text-white/40 hover:text-white rounded-xl transition-colors absolute top-2 right-2 sm:relative sm:top-0 sm:right-0"
                  title="Remove rule"
                >
                  <Trash2 className="w-4 h-4" />
                </button>
              </div>
            );
          })
        )}
      </div>

      {/* Add Rule Button */}
      {rules.length > 0 && (
        <button
          type="button"
          onClick={addRule}
          className="flex items-center gap-2 px-4 py-2.5 text-sm font-medium border border-white/10 rounded-xl bg-white/5 text-white/80 hover:text-white hover:bg-white/10 transition-colors"
        >
          <Plus className="w-4 h-4" />
          Add Rule
        </button>
      )}

      {/* Preview Info */}
      {rules.length > 0 && (
        <div className="text-xs text-white/40 italic mt-2 pl-1">
          Books will be automatically added to this shelf when they match {matchType === 'all' ? 'all' : 'any'} of the rules above.
        </div>
      )}
    </div>
  );
};
