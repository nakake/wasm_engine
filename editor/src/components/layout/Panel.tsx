import type { ReactNode } from 'react';

interface PanelProps {
  title: string;
  children: ReactNode;
  onClose?: () => void;
}

export function Panel({ title, children, onClose }: PanelProps) {
  return (
    <div className="panel">
      <div className="panel-header">
        <span className="panel-title">{title}</span>
        {onClose && (
          <button className="panel-close" onClick={onClose} title="Close">
            ×
          </button>
        )}
      </div>
      <div className="panel-content">{children}</div>
    </div>
  );
}
