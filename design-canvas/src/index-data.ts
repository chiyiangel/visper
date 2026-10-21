import { beginFrame, catalog, resetCatalog } from "./catalog";
import { allFrames } from "./frames";

export function emitIndex(): string {
  resetCatalog();
  const frames = allFrames();
  for (const frame of frames) {
    beginFrame(frame.id);
    frame.render(frame.id.endsWith("-dark") ? "dark" : "light");
  }
  const elements = catalog();
  return JSON.stringify(
    {
      copyFormat: '[visper-design] frame=<frame> > el=<id> | label="<label>" | src=<file:line>',
      idConvention:
        "ids are stable ascii paths unique inside a frame. The same control keeps the same id across states. label is the visible copy. src is file:line in the Visper repo.",
      frameCount: frames.length,
      elementCount: elements.length,
      frames: frames.map((frame) => ({
        id: frame.id,
        number: frame.number,
        title: frame.title,
        section: frame.section,
        assumption: frame.assumption ?? null,
      })),
      elements,
    },
    null,
    2,
  );
}
