export type ThemeName = "light" | "dark";
export type Lang = "auto" | "zh" | "en";
export type StatusName = "queued" | "transcribing" | "completed" | "failed" | "cancelled";

export type Segment = {
  id: string;
  time: string;
  text: string;
  playing?: boolean;
  /** Character range highlighted as a text selection. */
  select?: [number, number];
};

export type LibRow = {
  id: string;
  name: string;
  status: StatusName;
  selected?: boolean;
  extra?: string;
  extraWarning?: boolean;
  tag?: "Large" | "Turbo";
  progress?: number;
};

export type Group = { id: string; label: string; rows: LibRow[] };

export type Recording = {
  name: string;
  path: string;
  status: StatusName;
  duration?: string;
  language: Lang;
  took?: string;
  modelName?: string;
  waitingForModel?: boolean;
  exportsPrevious?: boolean;
  failure?: { title: string; body: string };
  missing?: { title: string; body: string };
  progress?: {
    headline: string;
    settled: boolean;
    finishAt?: string;
    elapsed?: string;
    processed?: string;
    speed?: string;
    fraction?: number;
    percent?: string;
  };
  segments: Segment[];
  emptyCopy?: string;
  writing?: boolean;
  hint: string;
  playback: {
    enabled: boolean;
    mode: "idle" | "playing" | "paused";
    time?: string;
    missingNote?: boolean;
    jump?: boolean;
  };
  exportEnabled: boolean;
  showRerun: boolean;
  rerunEnabled: boolean;
  moreOpen?: boolean;
  justExported?: boolean;
};

export type WindowModel = {
  theme: ThemeName;
  width: number;
  height: number;
  narrow?: boolean;
  canAdd: boolean;
  groups: Group[];
  footer: "large" | "turbo" | "none";
  popover?: { current: "large" | "turbo"; large: string; turbo: string };
  download?:
    | { state: "running"; title: string; note: string; amount: string; fraction: number }
    | { state: "failed"; title: string; error: string; switchBack?: string };
  notice?: string;
  dragOver?: boolean;
  focus?: string;
  tooltip?: "more" | "add" | "filename";
  tooltipText?: string;
  sheet?: { title: string; body: string; ok: string; cancel: string; src: string };
  view:
    | { kind: "choose-model" }
    | { kind: "ready-empty" }
    | { kind: "unselected" }
    | { kind: "recording"; rec: Recording };
};

export type MenuId = "app" | "file" | "edit" | "window" | "help";

export type FrameSpec = {
  id: string;
  number: string;
  title: string;
  section: string;
  caption: string;
  assumption?: string;
  /** Restartable motion: welcome mark or the writing indicator. */
  play?: "mark" | "writing";
  width: number;
  height: number;
  themeable: boolean;
  render: (theme: ThemeName) => string;
};
