/** Paths copied from assets/icons. GPUI tints the whole SVG, so these use currentColor. */
const ICONS: Record<string, { body: string; sw?: number; fill?: boolean }> = {
  plus: { body: `<path d="M12 5v14M5 12h14"/>` },
  "plus-bold": { body: `<path d="M12 5v14M5 12h14"/>`, sw: 2.5 },
  audio: {
    body: `<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8Z"/><path d="M14 2v6h6M8 13v4m4-7v10m4-6v2"/>`,
  },
  play: { body: `<path d="m8 5 11 7-11 7Z"/>`, fill: true },
  pause: { body: `<path d="M7 5h3v14H7zm7 0h3v14h-3z"/>`, fill: true },
  check: {
    body: `<circle cx="12" cy="12" r="9"/><path d="m8 12 3 3 5-6"/>`,
  },
  warning: {
    body: `<circle cx="12" cy="12" r="9"/><path d="M12 7v6m0 3v.1"/>`,
  },
  download: {
    body: `<path d="M12 3v12m-5-5 5 5 5-5M4 16v4h16v-4"/>`,
  },
  export: {
    body: `<path d="M12 16V3m-4 4 4-4 4 4M5 11v10h14V11"/>`,
  },
  retry: {
    body: `<path d="M20 7v5h-5M4 17v-5h5"/><path d="M6 7a7 7 0 0 1 12-2l2 3M4 16l2 3a7 7 0 0 0 12-2"/>`,
  },
  more: {
    body: `<circle cx="5" cy="12" r="1.75"/><circle cx="12" cy="12" r="1.75"/><circle cx="19" cy="12" r="1.75"/>`,
    fill: true,
  },
  remove: {
    body: `<path d="M4 6h16M9 6V3h6v3M6 6l1 15h10l1-15M10 10v7m4-7v7"/>`,
  },
  lock: {
    body: `<rect x="5" y="10" width="14" height="11" rx="2"/><path d="M8 10V7a4 4 0 0 1 8 0v3M12 14v3"/>`,
  },
  waiting: {
    body: `<circle cx="12" cy="12" r="8"/><path d="M12 7v5l3 2"/>`,
  },
  cancelled: {
    body: `<circle cx="12" cy="12" r="8"/><path d="M8 12h8"/>`,
  },
  "chevron-up": { body: `<path d="m5 15 7-7 7 7"/>`, sw: 2.5 },
  tick: { body: `<path d="m5 12.5 4.5 4.5L19 7.5"/>`, sw: 2.25 },
};

export function icon(name: string, size: number): string {
  const spec = ICONS[name];
  if (!spec) return "";
  const paint = spec.fill
    ? `fill="currentColor"`
    : `fill="none" stroke="currentColor" stroke-width="${spec.sw ?? 1.5}" stroke-linecap="round" stroke-linejoin="round"`;
  return `<svg class="icon" width="${size}" height="${size}" viewBox="0 0 24 24" aria-hidden="true" ${paint}>${spec.body}</svg>`;
}

/** Mark from assets/icons/mark-vee.svg and mark-lines.svg, viewBox 180 400 910 520. */
export function mark(width: number, veeVar: string, linesVar: string, animated: boolean): string {
  const height = (width * 520) / 910;
  const lines = `<svg viewBox="180 400 910 520" aria-hidden="true"><g fill="none" stroke="${linesVar}" stroke-width="70" stroke-linecap="round" stroke-linejoin="round"><path d="M865 450H1045"/><path d="M892 557H1045"/></g></svg>`;
  const vee = `<svg class="mark-vee" viewBox="180 400 910 520" aria-hidden="true"><path d="M262 505L430 795C490 860 575 860 630 795L800 485" fill="none" stroke="${veeVar}" stroke-width="140" stroke-linecap="round" stroke-linejoin="round"/></svg>`;
  const clipClass = animated ? "mark-lines-clip can-write" : "mark-lines-clip";
  return `<div class="mark" style="width:${width}px;height:${height}px">${vee}<div class="${clipClass}">${lines}</div></div>`;
}
