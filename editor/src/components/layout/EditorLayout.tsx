import type { ReactNode } from 'react';
import { useEditorStore } from '../../stores/editorStore';

interface EditorLayoutProps {
  toolbar: ReactNode;
  hierarchy: ReactNode;
  viewport: ReactNode;
  inspector: ReactNode;
}

export function EditorLayout({
  toolbar,
  hierarchy,
  viewport,
  inspector,
}: EditorLayoutProps) {
  const isHierarchyVisible = useEditorStore((s) => s.isHierarchyVisible);
  const isInspectorVisible = useEditorStore((s) => s.isInspectorVisible);

  return (
    <div className="editor-layout">
      <header className="editor-toolbar">{toolbar}</header>

      <main className="editor-main">
        {isHierarchyVisible && (
          <aside className="editor-panel editor-panel-left">{hierarchy}</aside>
        )}

        <section className="editor-viewport">{viewport}</section>

        {isInspectorVisible && (
          <aside className="editor-panel editor-panel-right">{inspector}</aside>
        )}
      </main>
    </div>
  );
}
