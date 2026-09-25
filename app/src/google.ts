// Loads the Google Maps JavaScript API with the owner's key (ADR-023). Only the parts the
// Site tab uses are typed here.

export interface LatLngLiteral {
  lat: number;
  lng: number;
}

export interface GLatLng {
  lat(): number;
  lng(): number;
}

export interface GMap {
  setCenter(c: LatLngLiteral): void;
  setZoom(z: number): void;
  fitBounds(b: GBounds): void;
  addListener(event: string, fn: (e: { latLng?: GLatLng }) => void): { remove(): void };
}

export interface GBounds {
  extend(p: LatLngLiteral): void;
}

export interface GPolygon {
  setMap(m: GMap | null): void;
}

export interface GGeocodeResult {
  formatted_address: string;
  geometry: { location: GLatLng; viewport?: GBounds };
}

export interface GoogleMaps {
  Map: new (el: HTMLElement, opts: Record<string, unknown>) => GMap;
  Polygon: new (opts: Record<string, unknown>) => GPolygon;
  Marker: new (opts: Record<string, unknown>) => GPolygon;
  LatLngBounds: new () => GBounds;
  Geocoder: new () => {
    geocode(req: Record<string, unknown>): Promise<{ results: GGeocodeResult[] }>;
  };
}

interface GoogleWindow {
  google?: { maps?: GoogleMaps };
  __rufplanMapsReady?: () => void;
  gm_authFailure?: () => void;
}

let loading: Promise<GoogleMaps> | null = null;

/** Loads the Maps JavaScript API once. Rejects if the key is refused. */
export function loadGoogleMaps(key: string): Promise<GoogleMaps> {
  const w = window as unknown as GoogleWindow;
  if (w.google?.maps) return Promise.resolve(w.google.maps);
  if (loading) return loading;
  loading = new Promise<GoogleMaps>((resolve, reject) => {
    w.__rufplanMapsReady = () => {
      if (w.google?.maps) resolve(w.google.maps);
      else reject(new Error("Google Maps did not load"));
    };
    w.gm_authFailure = () => {
      loading = null;
      reject(
        new Error(
          "Google refused the Maps key. Check it in Site > API Keys, and that the Maps JavaScript and Geocoding APIs are enabled.",
        ),
      );
    };
    const s = document.createElement("script");
    s.src = `https://maps.googleapis.com/maps/api/js?key=${encodeURIComponent(key)}&v=weekly&loading=async&callback=__rufplanMapsReady`;
    s.async = true;
    s.onerror = () => {
      loading = null;
      reject(new Error("Couldn't reach Google Maps. Check your internet connection."));
    };
    document.head.appendChild(s);
  });
  return loading;
}
