import { useEffect, useRef, useState } from "react";
import type { ParcelHit } from "../bindings/ParcelHit";
import { apply } from "../fileActions";
import { errorMessage, ipc } from "../ipc";
import { loadGoogleMaps, type GMap, type GoogleMaps, type GPolygon } from "../google";
import { useAppStore } from "../store";

// Site tab dialogs (ADR-023): Find Lot (Google Maps search, Regrid parcels, USGS topo) and
// API Keys.

const FT = 304.8;

export function SiteDialog() {
  const mode = useAppStore((s) => s.siteDialog);
  const setMode = useAppStore((s) => s.setSiteDialog);
  if (!mode) return null;
  return (
    <div
      className="modal-backdrop"
      role="dialog"
      aria-label={mode === "keys" ? "API Keys" : "Find Lot"}
    >
      {mode === "keys" ? (
        <KeysForm onDone={() => setMode(null)} />
      ) : (
        <FindLot onClose={() => setMode(null)} />
      )}
    </div>
  );
}

function KeysForm({ onDone }: { onDone: () => void }) {
  const [google, setGoogle] = useState("");
  const [regrid, setRegrid] = useState("");
  const [status, setStatus] = useState<{ google: boolean; regrid: boolean } | null>(null);
  useEffect(() => {
    ipc.siteKeys().then(
      (k) => setStatus({ google: !!k.googleKey, regrid: k.regrid }),
      () => setStatus({ google: false, regrid: false }),
    );
  }, []);
  const save = async () => {
    try {
      await ipc.siteSetKeys(google || null, regrid || null);
      onDone();
    } catch (e) {
      useAppStore.getState().setError(errorMessage(e));
    }
  };
  return (
    <div className="modal site-keys">
      <h2>API Keys</h2>
      <p className="muted">
        Stored in the Windows credential store on this computer, never in the project.
      </p>
      <label className="field">
        Google Maps API key {status?.google ? "(saved)" : ""}
        <input
          type="password"
          aria-label="Google Maps API key"
          placeholder={status?.google ? "Leave blank to keep" : "AIza…"}
          value={google}
          onChange={(e) => setGoogle(e.target.value)}
        />
        <span className="muted">
          Enable the Maps JavaScript API and the Geocoding API for this key in Google Cloud.
        </span>
      </label>
      <label className="field">
        Regrid API token {status?.regrid ? "(saved)" : ""}
        <input
          type="password"
          aria-label="Regrid API token"
          placeholder={status?.regrid ? "Leave blank to keep" : "Regrid token"}
          value={regrid}
          onChange={(e) => setRegrid(e.target.value)}
        />
        <span className="muted">From app.regrid.com, for parcel boundaries.</span>
      </label>
      <div className="modal-actions">
        <button className="btn-outline" onClick={onDone}>
          Cancel
        </button>
        <button className="btn-cyan" onClick={() => void save()}>
          Save
        </button>
      </div>
    </div>
  );
}

function FindLot({ onClose }: { onClose: () => void }) {
  const site = useAppStore((s) => s.app?.site ?? null);
  const setMode = useAppStore((s) => s.setSiteDialog);
  const mapEl = useRef<HTMLDivElement>(null);
  const g = useRef<{ maps: GoogleMaps; map: GMap; shape: GPolygon | null } | null>(null);
  const [query, setQuery] = useState(site?.address ?? "");
  const [status, setStatus] = useState("Loading Google Maps…");
  const [needsKey, setNeedsKey] = useState(false);
  const [parcel, setParcel] = useState<ParcelHit | null>(null);
  const [busy, setBusy] = useState(false);
  const [spacing, setSpacing] = useState(5);
  const [margin, setMargin] = useState(25);

  const outline = (ring: [number, number][], color: string) => {
    const t = g.current;
    if (!t) return;
    t.shape?.setMap(null);
    t.shape = new t.maps.Polygon({
      map: t.map,
      paths: ring.map(([lat, lng]) => ({ lat, lng })),
      strokeColor: color,
      strokeWeight: 2,
      fillColor: color,
      fillOpacity: 0.12,
      clickable: false,
    });
    const b = new t.maps.LatLngBounds();
    ring.forEach(([lat, lng]) => b.extend({ lat, lng }));
    t.map.fitBounds(b);
  };

  const lookUp = async (lat: number, lng: number) => {
    setBusy(true);
    setStatus("Looking up the parcel…");
    try {
      const p = await ipc.siteParcel(lat, lng);
      setParcel(p);
      outline(p.ring as [number, number][], "#3ECFF7");
      setStatus(p.address || "Parcel found.");
    } catch (e) {
      setParcel(null);
      setStatus(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    let live = true;
    void (async () => {
      const keys = await ipc.siteKeys().catch(() => null);
      if (!keys?.googleKey) {
        setNeedsKey(true);
        setStatus("Add your Google Maps API key to search and see the map.");
        return;
      }
      try {
        const maps = await loadGoogleMaps(keys.googleKey);
        if (!live || !mapEl.current) return;
        const map = new maps.Map(mapEl.current, {
          center: site ? { lat: site.lat, lng: site.lon } : { lat: 39.8, lng: -98.6 },
          zoom: site ? 19 : 4,
          mapTypeId: "hybrid",
          tilt: 0,
          streetViewControl: false,
          fullscreenControl: false,
        });
        g.current = { maps, map, shape: null };
        map.addListener("click", (e) => {
          if (e.latLng) void lookUp(e.latLng.lat(), e.latLng.lng());
        });
        if (site && site.ring.length >= 3) outline(site.ring as [number, number][], "#0a0a0a");
        setStatus(
          site
            ? `Current lot: ${site.address}`
            : "Search an address, then click the lot on the map.",
        );
      } catch (e) {
        setStatus(errorMessage(e));
      }
    })();
    return () => {
      live = false;
    };
    // The map is created once per opening.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const search = async () => {
    const t = g.current;
    if (!t || !query.trim()) return;
    setStatus("Searching…");
    try {
      const { results } = await new t.maps.Geocoder().geocode({
        address: query,
        componentRestrictions: { country: "US" },
      });
      const r = results[0];
      if (!r) {
        setStatus("No match in the USA for that address.");
        return;
      }
      const at = r.geometry.location;
      t.map.setCenter({ lat: at.lat(), lng: at.lng() });
      t.map.setZoom(19);
      setQuery(r.formatted_address);
      await lookUp(at.lat(), at.lng());
    } catch (e) {
      setStatus(`Search failed: ${errorMessage(e)}`);
    }
  };

  const takeLot = async () => {
    if (!parcel) return;
    setBusy(true);
    const ok = await apply(() =>
      ipc.siteSetLot(
        parcel.ring,
        parcel.apn,
        parcel.owner,
        parcel.address || query,
        parcel.acres,
        "Regrid",
      ),
    );
    setBusy(false);
    if (ok) setStatus("Lot set. Get the topography next, or close to see the Site plan.");
  };

  const topo = async () => {
    setBusy(true);
    setStatus("Getting elevations from USGS 3DEP…");
    const ok = await apply(() => ipc.siteFetchTopo(spacing * FT, margin * FT));
    setBusy(false);
    if (ok) setStatus("Topography added: see the Site plan, sections and 3D.");
  };

  return (
    <div className="modal site-find">
      <div className="site-head">
        <h2>Find Lot</h2>
        <form
          className="site-search"
          onSubmit={(e) => {
            e.preventDefault();
            void search();
          }}
        >
          <input
            aria-label="Address"
            placeholder="Address, city or APN area…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            disabled={needsKey}
          />
          <button className="btn-cyan" type="submit" disabled={needsKey || busy}>
            Search
          </button>
        </form>
        <button className="btn-ghost" onClick={onClose} aria-label="Close">
          ×
        </button>
      </div>
      <div className="site-body">
        <div ref={mapEl} className="site-map">
          {needsKey && (
            <div className="site-map-empty">
              <p>Google Maps needs your API key.</p>
              <button className="btn-cyan" onClick={() => setMode("keys")}>
                Add API Keys
              </button>
            </div>
          )}
        </div>
        <aside className="site-side">
          <div className="site-status" role="status">
            {status}
          </div>
          {parcel && (
            <dl className="site-parcel">
              <dt>Address</dt>
              <dd>{parcel.address || "—"}</dd>
              <dt>Parcel (APN)</dt>
              <dd>{parcel.apn || "—"}</dd>
              <dt>Owner</dt>
              <dd>{parcel.owner || "—"}</dd>
              <dt>Area</dt>
              <dd>{parcel.acres ? `${parcel.acres.toFixed(3)} ac` : "—"}</dd>
            </dl>
          )}
          <button className="btn-cyan" onClick={() => void takeLot()} disabled={!parcel || busy}>
            Use This Lot
          </button>
          <div className="site-topo">
            <h3>Topography</h3>
            <p className="muted">
              Preliminary ground from USGS 3DEP (1 m lidar where available). Not a survey.
            </p>
            <label className="field">
              Grid spacing
              <select
                aria-label="Grid spacing"
                value={spacing}
                onChange={(e) => setSpacing(Number(e.target.value))}
              >
                {[2, 5, 10, 20].map((v) => (
                  <option key={v} value={v}>
                    {v}&apos;
                  </option>
                ))}
              </select>
            </label>
            <label className="field">
              Beyond the lot
              <select
                aria-label="Beyond the lot"
                value={margin}
                onChange={(e) => setMargin(Number(e.target.value))}
              >
                {[0, 25, 50, 100].map((v) => (
                  <option key={v} value={v}>
                    {v}&apos;
                  </option>
                ))}
              </select>
            </label>
            <button className="btn-outline" onClick={() => void topo()} disabled={!site || busy}>
              {site?.hasTopo ? "Refresh Topography" : "Get Topography"}
            </button>
          </div>
          <button className="btn-ghost" onClick={() => setMode("keys")}>
            API Keys…
          </button>
        </aside>
      </div>
    </div>
  );
}
