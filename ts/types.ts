export interface Task {
  id: string;
  title: string;
  note: string;
  position: number;
  createdAt: number; // unix ms
  startedAt: number | null;
  doneAt: number | null;
}

export interface QueueState {
  version: 1;
  current: string | null;
  tasks: Task[];
}

export const EMPTY_QUEUE: QueueState = {
  version: 1,
  current: null,
  tasks: [],
};
