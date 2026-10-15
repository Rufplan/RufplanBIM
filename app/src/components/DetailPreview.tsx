import { useEffect, useState } from "react";
import type { DisplayList } from "../bindings/DisplayList";
import { ipc } from "../ipc";
import { DetailThumb } from "./DetailThumb";

// A library detail drawn from its own geometry (ADR-069), fetched once and shared by the
// Detail Library's grid, its preview pane and the enlarged viewer.

const previews = new Map<string, Promise<DisplayList>>();

export function detailPreview(id: string) {
  let p = previews.get(id);
  if (!p) {
    p = ipc.detailPreview(id);
    p.catch(() => previews.delete(id));
    previews.set(id, p);
  }
  return p;
}

/** The detail's display list once it has loaded, else null. */
export function useDetailPreview(id: string): DisplayList | null {
  const [dl, setDl] = useState<{ id: string; dl: DisplayList } | null>(null);
  useEffect(() => {
    let live = true;
    detailPreview(id).then(
      (d) => live && setDl({ id, dl: d }),
      () => {},
    );
    return () => {
      live = false;
    };
  }, [id]);
  return dl && dl.id === id ? dl.dl : null;
}

export function Preview({ id, name, big }: { id: string; name: string; big?: boolean }) {
  const dl = useDetailPreview(id);
  if (!dl) return <div className="dl-thumb dl-loading" role="img" aria-label={name} />;
  return <DetailThumb dl={dl} label={name} text={big} />;
}
