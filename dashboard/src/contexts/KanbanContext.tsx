import { createContext, useCallback, useContext, useEffect, useState, type ReactNode } from "react";
import type { KanbanCard, TaskDispatch, WSEvent } from "../types";
import * as api from "../api/client";

// ── Context value ──

interface KanbanContextValue {
  kanbanCards: KanbanCard[];
  taskDispatches: TaskDispatch[];
  upsertKanbanCard: (card: KanbanCard) => void;
  upsertTaskDispatch: (dispatch: TaskDispatch) => void;
  setKanbanCards: React.Dispatch<React.SetStateAction<KanbanCard[]>>;
  deleteKanbanCard: (id: string) => void;
}

const KanbanContext = createContext<KanbanContextValue | null>(null);

// ── Provider ──

interface KanbanProviderProps {
  initialCards: KanbanCard[];
  initialDispatches: TaskDispatch[];
  children: ReactNode;
}

export function KanbanProvider({ initialCards, initialDispatches, children }: KanbanProviderProps) {
  const [kanbanCards, setKanbanCards] = useState<KanbanCard[]>(initialCards);
  const [taskDispatches, setTaskDispatches] = useState<TaskDispatch[]>(initialDispatches);

  const upsertKanbanCard = useCallback((card: KanbanCard) => {
    setKanbanCards((prev) => [card, ...prev.filter((p) => p.id !== card.id)]);
  }, []);

  const upsertTaskDispatch = useCallback((dispatch: TaskDispatch) => {
    setTaskDispatches((prev) => [dispatch, ...prev.filter((p) => p.id !== dispatch.id)].slice(0, 200));
  }, []);

  const deleteKanbanCard = useCallback((id: string) => {
    setKanbanCards((prev) => prev.filter((card) => card.id !== id));
  }, []);

  // ── WS event handling ──
  useEffect(() => {
    function handleWs(e: Event) {
      const event = (e as CustomEvent<WSEvent>).detail;
      switch (event.type) {
        case "kanban_card_created":
        case "kanban_card_updated":
          upsertKanbanCard(event.payload as KanbanCard);
          break;
        case "kanban_card_deleted":
          deleteKanbanCard((event.payload as { id: string }).id);
          break;
        case "task_dispatch_created":
        case "task_dispatch_updated":
          upsertTaskDispatch(event.payload as TaskDispatch);
          break;
      }
    }
    window.addEventListener("pcd-ws-event", handleWs);
    return () => window.removeEventListener("pcd-ws-event", handleWs);
  }, [upsertKanbanCard, upsertTaskDispatch, deleteKanbanCard]);

  return (
    <KanbanContext.Provider value={{ kanbanCards, taskDispatches, upsertKanbanCard, upsertTaskDispatch, setKanbanCards, deleteKanbanCard }}>
      {children}
    </KanbanContext.Provider>
  );
}

// ── Hook ──

export function useKanban(): KanbanContextValue {
  const ctx = useContext(KanbanContext);
  if (!ctx) throw new Error("useKanban must be used within KanbanProvider");
  return ctx;
}
