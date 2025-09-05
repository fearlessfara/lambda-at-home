import { useEffect, useMemo, useRef, useState } from 'react';

export function Combobox({
  value,
  onChange,
  options,
  placeholder = 'Select…',
  searchPlaceholder = 'Search…',
  className = '',
}: {
  value: string;
  onChange: (v: string) => void;
  options: string[];
  placeholder?: string;
  searchPlaceholder?: string;
  className?: string;
}) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const ref = useRef<HTMLDivElement | null>(null);

  const filtered = useMemo(() => {
    const q = query.toLowerCase();
    return options.filter(o => o.toLowerCase().includes(q));
  }, [options, query]);

  useEffect(() => {
    const onDoc = (e: MouseEvent) => {
      if (!ref.current) return;
      if (!ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener('mousedown', onDoc);
    return () => document.removeEventListener('mousedown', onDoc);
  }, []);

  const selectedLabel = value || '';

  return (
    <div ref={ref} className={`relative ${className}`}>
      <button
        type="button"
        className="w-full border rounded px-3 py-2 text-left text-sm bg-white hover:bg-gray-50"
        onClick={() => setOpen(o => !o)}
      >
        {selectedLabel || <span className="text-gray-400">{placeholder}</span>}
      </button>
      {open && (
        <div className="absolute z-50 mt-1 w-full border bg-white rounded shadow-lg">
          <div className="p-2 border-b">
            <input
              autoFocus
              placeholder={searchPlaceholder}
              className="w-full border rounded px-2 py-1 text-sm"
              value={query}
              onChange={(e)=>setQuery(e.target.value)}
            />
          </div>
          <div className="max-h-60 overflow-auto">
            {(filtered.length ? filtered : options).map(opt => (
              <div
                key={opt}
                className={`px-3 py-2 text-sm cursor-pointer hover:bg-gray-100 ${opt===value ? 'bg-orange-50' : ''}`}
                onClick={() => { onChange(opt); setOpen(false); }}
              >
                {opt}
              </div>
            ))}
            {options.length === 0 && (
              <div className="px-3 py-2 text-sm text-gray-500">No options</div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

