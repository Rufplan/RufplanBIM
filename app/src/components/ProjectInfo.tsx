import { useEffect, useState, type ReactNode } from "react";
import { create } from "zustand";
import type { ProjectContact } from "../bindings/ProjectContact";
import type { ProjectDetails } from "../bindings/ProjectDetails";
import type { ProjectInfoState } from "../bindings/ProjectInfoState";
import { apply } from "../fileActions";
import { ipc } from "../ipc";
import { useAppStore } from "../store";

// The Project Info tab (ADR-084): everything about the job besides the model — overview,
// location, client, consultants, budget, schedule, codes and zoning, notes. Kept on the
// project's ProjectInfo element (studio-core `project`); edits save after a short pause as
// one undoable step. Sums and areas come from Rust (`project_info_get`).

type SectionId =
  "overview" | "location" | "client" | "team" | "budget" | "schedule" | "codes" | "notes";

interface Section {
  id: SectionId;
  label: string;
  short: string;
  group: string;
  icon: string;
}

export const SECTIONS: Section[] = [
  {
    id: "overview",
    label: "Overview",
    short: "Overview",
    group: "PROJECT",
    icon: "M4 4h16v16H4zM8 9h8M8 13h8M8 17h5",
  },
  {
    id: "location",
    label: "Location",
    short: "Location",
    group: "PROJECT",
    icon: "M12 21s-7-6.2-7-11.5A7 7 0 0 1 19 9.5C19 14.8 12 21 12 21zM12 12a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5z",
  },
  {
    id: "client",
    label: "Client & Owner",
    short: "Client",
    group: "PROJECT",
    icon: "M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM4 21a8 8 0 0 1 16 0",
  },
  {
    id: "team",
    label: "Consultants",
    short: "Team",
    group: "TEAM",
    icon: "M9 11a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7zM2.5 20a6.5 6.5 0 0 1 13 0M16 4.5a3.5 3.5 0 0 1 0 6.5M18 14a6.5 6.5 0 0 1 3.5 6",
  },
  {
    id: "budget",
    label: "Budget",
    short: "Budget",
    group: "COST & TIME",
    icon: "M12 3v18M17 7.5c0-1.9-2.2-3-5-3s-5 1.1-5 3 2 2.8 5 3.5 5 1.6 5 3.5-2.2 3-5 3-5-1.1-5-3",
  },
  {
    id: "schedule",
    label: "Schedule",
    short: "Schedule",
    group: "COST & TIME",
    icon: "M4 6h16v14H4zM4 10h16M8 3v5M16 3v5M8 14h3",
  },
  {
    id: "codes",
    label: "Codes & Zoning",
    short: "Codes",
    group: "REGULATORY",
    icon: "M6 3h9l4 4v14H6zM14 3v5h5M9 13h6M9 17h6",
  },
  {
    id: "notes",
    label: "Notes",
    short: "Notes",
    group: "NOTES",
    icon: "M5 4h14v12l-4 4H5zM15 20v-4h4M8 9h8M8 13h5",
  },
];

const GROUPS = ["PROJECT", "TEAM", "COST & TIME", "REGULATORY", "NOTES"];

const PRESETS = {
  status: ["Prospect", "Active", "On Hold", "Complete", "Cancelled"],
  projectType: [
    "Single-Family Residential",
    "Multifamily Residential",
    "Mixed-Use",
    "Office",
    "Retail",
    "Restaurant",
    "Hospitality",
    "Healthcare",
    "Education",
    "Civic",
    "Industrial / Warehouse",
    "Laboratory",
    "Religious",
    "Recreation",
  ],
  workType: [
    "New Construction",
    "Addition",
    "Renovation",
    "Addition & Renovation",
    "Tenant Improvement",
    "Adaptive Reuse",
    "Historic Restoration",
  ],
  delivery: [
    "Design-Bid-Build",
    "Negotiated Bid",
    "CM at Risk",
    "Design-Build",
    "Integrated Project Delivery",
    "Owner-Builder",
  ],
  code: ["2024 IBC", "2021 IBC", "2018 IBC", "2024 IRC", "2021 IRC", "2018 IRC"],
  energy: [
    "2024 IECC",
    "2021 IECC",
    "2018 IECC",
    "ASHRAE 90.1-2022",
    "ASHRAE 90.1-2019",
    "Title 24 (2022)",
  ],
  occupancy: [
    "A-1",
    "A-2",
    "A-3",
    "B",
    "E",
    "F-1",
    "I-2",
    "M",
    "R-1",
    "R-2",
    "R-3",
    "S-1",
    "S-2",
    "U",
  ],
  construction: [
    "I-A",
    "I-B",
    "II-A",
    "II-B",
    "III-A",
    "III-B",
    "IV-HT",
    "IV-A",
    "IV-B",
    "IV-C",
    "V-A",
    "V-B",
  ],
  sprinklered: ["NFPA 13 (fully sprinklered)", "NFPA 13R", "NFPA 13D", "Not sprinklered"],
};

// ---- State: the server's copy, and the draft being typed (saved after a pause) ----

interface Draft {
  name: string;
  number: string;
  details: ProjectDetails;
}

interface PiState {
  info: ProjectInfoState | null;
  draft: Draft | null;
  dirty: boolean;
  saving: boolean;
  section: SectionId;
  /** The consultant card opened for details. */
  open: number | null;
  /** A passing message for the status bar. */
  note: string | null;
}

export const usePi = create<PiState>(() => ({
  info: null,
  draft: null,
  dirty: false,
  saving: false,
  section: "overview",
  open: null,
  note: null,
}));

let timer = 0;

function fromInfo(info: ProjectInfoState): Draft {
  return {
    name: info.identity.name,
    number: info.identity.number,
    details: structuredClone(info.details),
  };
}

async function refresh() {
  try {
    const info = await ipc.projectInfoGet();
    const s = usePi.getState();
    usePi.setState({ info, ...(s.dirty ? {} : { draft: fromInfo(info) }) });
  } catch {
    // No project open.
  }
}

/** Saves the draft now (a pending pause is cut short). */
export async function commitProjectInfo() {
  window.clearTimeout(timer);
  const { draft, dirty } = usePi.getState();
  if (!draft || !dirty || !draft.name.trim()) return;
  usePi.setState({ saving: true });
  const sent = draft;
  const ok = await apply(() => ipc.projectInfoSet(sent.name, sent.number, sent.details));
  // Typing may have gone on while saving; only settle what was sent.
  const now = usePi.getState();
  usePi.setState({ saving: false, dirty: now.draft === sent ? !ok : true });
  await refresh();
}

/** Edits the draft and saves it after a pause. */
export function edit(f: (d: Draft) => void) {
  const { draft } = usePi.getState();
  if (!draft) return;
  const next = structuredClone(draft);
  f(next);
  usePi.setState({ draft: next, dirty: true });
  window.clearTimeout(timer);
  timer = window.setTimeout(() => void commitProjectInfo(), 600);
}

function useProjectInfo() {
  const revision = useAppStore((s) => s.app?.revision ?? 0);
  useEffect(() => {
    void refresh();
  }, [revision]);
  return usePi();
}

// ---- Numbers ----

function money(v: number, currency = "USD") {
  try {
    return new Intl.NumberFormat("en-US", {
      style: "currency",
      currency: currency || "USD",
      maximumFractionDigits: 0,
    }).format(v);
  } catch {
    return `${Math.round(v).toLocaleString("en-US")}`;
  }
}

const sfText = (v: number) => `${Math.round(v).toLocaleString("en-US")} SF`;

/** A number typed freely ("1,250,000", "$85"), shown formatted when not being edited. */
function NumberInput({
  value,
  onChange,
  format,
  label,
  suffix,
}: {
  value: number;
  onChange: (v: number) => void;
  format: (v: number) => string;
  label: string;
  suffix?: string;
}) {
  const [text, setText] = useState<string | null>(null);
  return (
    <span className="pi-num">
      <input
        aria-label={label}
        inputMode="decimal"
        value={text ?? (value ? format(value) : "")}
        placeholder="—"
        onFocus={() => setText(value ? String(value) : "")}
        onChange={(e) => {
          setText(e.target.value);
          const n = Number(e.target.value.replace(/[,$\s%]/g, ""));
          if (Number.isFinite(n) && n >= 0) onChange(n);
        }}
        onBlur={() => setText(null)}
      />
      {suffix && <span className="pi-suffix">{suffix}</span>}
    </span>
  );
}

// ---- Completion, per section ----

const has = (s: string | undefined) => !!s && s.trim().length > 0;

export function completion(id: SectionId, d: Draft): [number, number] {
  const x = d.details;
  const count = (xs: boolean[]): [number, number] => [xs.filter(Boolean).length, xs.length];
  switch (id) {
    case "overview":
      return count([
        has(d.name),
        has(d.number),
        has(x.overview.status),
        has(x.overview.project_type),
        has(x.overview.work_type),
        has(x.overview.delivery),
        has(x.overview.description),
        x.overview.target_area_sf > 0,
      ]);
    case "location": {
      const l = x.location;
      return count([
        has(l.street),
        has(l.city),
        has(l.state),
        has(l.zip),
        has(l.county),
        has(l.apn),
        has(l.jurisdiction),
      ]);
    }
    case "client": {
      const c = x.client;
      return count([
        has(c.company) || has(c.name),
        has(c.name),
        has(c.email),
        has(c.phone),
        has(c.address),
      ]);
    }
    case "team":
      return count(x.team.map((m) => has(m.contact.company) || has(m.contact.name)));
    case "budget":
      return count(x.budget.lines.map((l) => l.amount > 0));
    case "schedule":
      return count(x.milestones.map((m) => has(m.date)));
    case "codes": {
      const c = x.codes;
      return count(
        [
          c.building_code,
          c.energy_code,
          c.occupancy,
          c.construction_type,
          c.sprinklered,
          c.zoning_district,
          c.lot_area,
          c.far,
          c.max_height,
          c.setbacks,
          c.lot_coverage,
          c.parking,
        ].map(has),
      );
    }
    case "notes":
      return count([has(x.notes)]);
  }
}

// ---- Pieces ----

function StrokeIcon({ d, size = 18 }: { d: string; size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
    >
      <path d={d} />
    </svg>
  );
}

function Field({
  label,
  value,
  onChange,
  presets,
  multiline,
  wide,
  type,
  placeholder,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  presets?: string[];
  multiline?: boolean;
  wide?: boolean;
  type?: string;
  placeholder?: string;
}) {
  const listId = presets ? `pi-list-${label.replace(/\W+/g, "-").toLowerCase()}` : undefined;
  return (
    <label className={`pi-field${wide ? " wide" : ""}`}>
      <span>{label}</span>
      {multiline ? (
        <textarea
          rows={4}
          value={value}
          placeholder={placeholder}
          onChange={(e) => onChange(e.target.value)}
        />
      ) : (
        <input
          type={type ?? "text"}
          value={value}
          list={listId}
          placeholder={placeholder ?? (presets ? "Choose or type…" : "")}
          onChange={(e) => onChange(e.target.value)}
        />
      )}
      {presets && (
        <datalist id={listId}>
          {presets.map((p) => (
            <option key={p} value={p} />
          ))}
        </datalist>
      )}
    </label>
  );
}

function Card({
  eyebrow,
  title,
  right,
  children,
}: {
  eyebrow: string;
  title: string;
  right?: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="std-card pi-card">
      <div className="std-card-head">
        <div className="std-title">
          <span className="std-eyebrow">{eyebrow}</span>
          <h2>{title}</h2>
        </div>
        {right && <div className="std-head-right">{right}</div>}
      </div>
      <div className="pi-body">{children}</div>
    </div>
  );
}

function Stat({ label, value, note }: { label: string; value: string; note?: string }) {
  return (
    <div className="pi-stat">
      <span>{label}</span>
      <strong>{value}</strong>
      {note && <em>{note}</em>}
    </div>
  );
}

function ContactFields({
  c,
  set,
  prefix,
}: {
  c: ProjectContact;
  set: (f: (c: ProjectContact) => void) => void;
  prefix: string;
}) {
  const f = (label: string, key: keyof ProjectContact, type?: string, wide?: boolean) => (
    <Field
      label={label}
      value={c[key]}
      type={type}
      wide={wide}
      onChange={(v) => set((x) => void (x[key] = v))}
      placeholder={`${prefix} ${label.toLowerCase()}`}
    />
  );
  return (
    <div className="pi-fields">
      {f("Company", "company")}
      {f("Contact", "name")}
      {f("Title", "title")}
      {f("Email", "email", "email")}
      {f("Phone", "phone", "tel")}
      {f("Website", "website", "url")}
      {f("Address", "address", undefined, true)}
    </div>
  );
}

function Progress({ id, d }: { id: SectionId; d: Draft }) {
  const [n, all] = completion(id, d);
  const pct = all ? Math.round((n / all) * 100) : 0;
  return (
    <div className="std-progress">
      <span>
        {n}/{all} filled in
      </span>
      <div className="std-bar">
        <div style={{ width: `${pct}%` }} />
      </div>
    </div>
  );
}

// ---- Sections ----

function Overview({ d, info }: { d: Draft; info: ProjectInfoState }) {
  const o = d.details.overview;
  const set = (f: (o: ProjectDetails["overview"]) => void) => edit((x) => f(x.details.overview));
  const stage = info.stages.find((s) => s.current);
  const target = o.target_area_sf;
  return (
    <>
      <Card eyebrow="01 · PROJECT" title="Overview" right={<Progress id="overview" d={d} />}>
        <div className="pi-fields">
          <Field
            label="Project Name"
            value={d.name}
            onChange={(v) => edit((x) => void (x.name = v))}
            placeholder="Needed to save"
          />
          <Field
            label="Project Number"
            value={d.number}
            onChange={(v) => edit((x) => void (x.number = v))}
          />
          <Field
            label="Status"
            value={o.status}
            presets={PRESETS.status}
            onChange={(v) => set((x) => void (x.status = v))}
          />
          <Field
            label="Project Type"
            value={o.project_type}
            presets={PRESETS.projectType}
            onChange={(v) => set((x) => void (x.project_type = v))}
          />
          <Field
            label="Work Type"
            value={o.work_type}
            presets={PRESETS.workType}
            onChange={(v) => set((x) => void (x.work_type = v))}
          />
          <Field
            label="Delivery Method"
            value={o.delivery}
            presets={PRESETS.delivery}
            onChange={(v) => set((x) => void (x.delivery = v))}
          />
          <label className="pi-field">
            <span>Target Gross Area</span>
            <NumberInput
              label="Target Gross Area"
              value={target}
              suffix="SF"
              format={(v) => Math.round(v).toLocaleString("en-US")}
              onChange={(v) => set((x) => void (x.target_area_sf = v))}
            />
          </label>
          <Field
            label="Description"
            value={o.description}
            multiline
            wide
            placeholder="The program, in a few sentences"
            onChange={(v) => set((x) => void (x.description = v))}
          />
        </div>
        {!d.name.trim() && <div className="pi-warn">The project needs a name to save.</div>}
      </Card>
      <Card eyebrow="FROM THE MODEL" title="Project Facts">
        <div className="pi-stats">
          <Stat
            label="Gross Area"
            value={sfText(info.grossSf)}
            note={
              target > 0
                ? `${Math.round((info.grossSf / target) * 100)}% of the ${sfText(target)} target`
                : "Outside faces of the walls, all levels"
            }
          />
          <Stat label="Room Area" value={sfText(info.roomSf)} note={`${info.rooms} rooms`} />
          <Stat label="Levels" value={String(info.levels)} />
          <Stat
            label="Design Stage"
            value={stage ? stage.abbreviation || stage.name : "—"}
            note={stage?.name}
          />
          <Stat label="Rufplan" value={info.rufplan ?? "Not linked"} note="Rufplan tab › Link" />
        </div>
      </Card>
    </>
  );
}

function Location({ d, info }: { d: Draft; info: ProjectInfoState }) {
  const l = d.details.location;
  const set = (f: (l: ProjectDetails["location"]) => void) => edit((x) => f(x.details.location));
  const f = (label: string, key: keyof typeof l, wide?: boolean) => (
    <Field
      label={label}
      value={l[key]}
      wide={wide}
      onChange={(v) => set((x) => void (x[key] = v))}
    />
  );
  const site = info.site;
  return (
    <>
      <Card eyebrow="02 · PROJECT" title="Location" right={<Progress id="location" d={d} />}>
        <div className="pi-fields">
          {f("Street Address", "street", true)}
          {f("City", "city")}
          {f("State", "state")}
          {f("ZIP", "zip")}
          {f("County", "county")}
          {f("Country", "country")}
          {f("Parcel Number (APN)", "apn")}
          {f("Jurisdiction (AHJ)", "jurisdiction")}
          {f("Legal Description", "legal", true)}
        </div>
      </Card>
      <Card
        eyebrow="FROM THE SITE TAB"
        title="Site"
        right={
          site && (
            <button
              className="pi-btn"
              onClick={() =>
                set((x) => {
                  x.street = site.address;
                  if (site.apn) x.apn = site.apn;
                })
              }
            >
              Use Site Address
            </button>
          )
        }
      >
        {site ? (
          <div className="pi-stats">
            <Stat label="Address" value={site.address || "—"} />
            <Stat label="Coordinates" value={`${site.lat.toFixed(5)}, ${site.lon.toFixed(5)}`} />
            <Stat label="Parcel" value={site.apn || "—"} note={site.owner || undefined} />
            <Stat
              label="Lot"
              value={site.acres > 0 ? `${site.acres.toFixed(2)} ac` : "—"}
              note={site.acres > 0 ? sfText(site.acres * 43560) : undefined}
            />
          </div>
        ) : (
          <div className="pi-empty">Locate the lot on the Site tab to see it here.</div>
        )}
      </Card>
    </>
  );
}

function Client({ d }: { d: Draft }) {
  return (
    <>
      <Card eyebrow="03 · PROJECT" title="Client" right={<Progress id="client" d={d} />}>
        <ContactFields
          c={d.details.client}
          prefix="Client"
          set={(f) => edit((x) => f(x.details.client))}
        />
        <div className="pi-hint">The title block shows the client&apos;s company (or name).</div>
      </Card>
      <Card eyebrow="WHEN NOT THE CLIENT" title="Owner's Representative">
        <ContactFields
          c={d.details.owner_rep}
          prefix="Rep's"
          set={(f) => edit((x) => f(x.details.owner_rep))}
        />
      </Card>
    </>
  );
}

function Team({ d, info }: { d: Draft; info: ProjectInfoState }) {
  const open = usePi((s) => s.open);
  const team = d.details.team;
  const currency = d.details.budget.currency;
  const [adding, setAdding] = useState("");
  const fees = team.reduce((a, m) => a + m.fee, 0);
  const add = (discipline: string) => {
    if (!discipline) return;
    edit((x) =>
      x.details.team.push({
        discipline,
        contact: {
          company: "",
          name: "",
          title: "",
          email: "",
          phone: "",
          address: "",
          website: "",
        },
        scope: "",
        fee: 0,
        notes: "",
      }),
    );
    usePi.setState({ open: team.length });
    setAdding("");
  };
  return (
    <Card
      eyebrow="04 · TEAM"
      title="Consultants"
      right={
        <>
          {fees > 0 && <span className="pi-total">Fees {money(fees, currency)}</span>}
          <Progress id="team" d={d} />
          <select
            aria-label="Add consultant"
            className="pi-add"
            value={adding}
            onChange={(e) => add(e.target.value)}
          >
            <option value="">+ Add consultant…</option>
            {info.disciplines.map((x) => (
              <option key={x} value={x}>
                {x}
              </option>
            ))}
          </select>
        </>
      }
    >
      <div className="pi-team">
        {team.map((m, i) => {
          const filled = has(m.contact.company) || has(m.contact.name);
          const isOpen = open === i;
          const set = (f: (m: (typeof team)[number]) => void) => edit((x) => f(x.details.team[i]!));
          return (
            <div key={i} className={`pi-member${isOpen ? " open" : ""}${filled ? "" : " empty"}`}>
              <div
                className="pi-member-head"
                role="button"
                tabIndex={0}
                aria-expanded={isOpen}
                onClick={() => usePi.setState({ open: isOpen ? null : i })}
                onKeyDown={(e) => {
                  if (e.key === "Enter") usePi.setState({ open: isOpen ? null : i });
                }}
              >
                <span className={`pi-dot${filled ? " on" : ""}`} aria-hidden />
                <span className="pi-disc">{m.discipline || "Consultant"}</span>
                <span className="pi-firm">
                  {m.contact.company || m.contact.name || <em>Not yet engaged</em>}
                </span>
                <span className="pi-who">
                  {[m.contact.name && m.contact.company ? m.contact.name : "", m.contact.phone]
                    .filter(Boolean)
                    .join(" · ")}
                </span>
                {m.contact.email && (
                  <a
                    className="pi-mail"
                    href={`mailto:${m.contact.email}`}
                    onClick={(e) => e.stopPropagation()}
                  >
                    {m.contact.email}
                  </a>
                )}
                <span className="pi-chev" aria-hidden>
                  {isOpen ? "▴" : "▾"}
                </span>
              </div>
              {isOpen && (
                <div className="pi-member-body">
                  <div className="pi-fields">
                    <Field
                      label="Discipline"
                      value={m.discipline}
                      presets={info.disciplines}
                      onChange={(v) => set((x) => void (x.discipline = v))}
                    />
                  </div>
                  <ContactFields
                    c={m.contact}
                    prefix="Consultant"
                    set={(f) => set((x) => f(x.contact))}
                  />
                  <div className="pi-fields">
                    <label className="pi-field">
                      <span>Fee</span>
                      <NumberInput
                        label={`${m.discipline} fee`}
                        value={m.fee}
                        format={(v) => money(v, currency)}
                        onChange={(v) => set((x) => void (x.fee = v))}
                      />
                    </label>
                    <Field
                      label="Scope"
                      value={m.scope}
                      onChange={(v) => set((x) => void (x.scope = v))}
                      placeholder="e.g. Foundations and framing, CA"
                    />
                    <Field
                      label="Notes"
                      value={m.notes}
                      wide
                      onChange={(v) => set((x) => void (x.notes = v))}
                    />
                  </div>
                  <div className="pi-row-end">
                    <button
                      className="pi-btn danger"
                      onClick={() => {
                        edit((x) => void x.details.team.splice(i, 1));
                        usePi.setState({ open: null });
                      }}
                    >
                      Remove {m.discipline || "consultant"}
                    </button>
                  </div>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </Card>
  );
}

function Budget({ d, info }: { d: Draft; info: ProjectInfoState }) {
  const b = d.details.budget;
  const t = info.totals;
  const set = (f: (b: ProjectDetails["budget"]) => void) => edit((x) => f(x.details.budget));
  const m = (v: number) => money(v, b.currency);
  const parts = [
    { k: "hard", label: "Hard", v: t.hard },
    { k: "soft", label: "Soft", v: t.soft },
    { k: "esc", label: "Escalation", v: t.escalation },
    { k: "cont", label: "Contingency", v: t.contingency },
  ];
  const variance = t.target_hard > 0 ? t.target_hard - (t.hard + t.escalation) : 0;
  return (
    <>
      <Card
        eyebrow="05 · COST & TIME"
        title="Budget"
        right={<span className="pi-total big">{m(t.total)}</span>}
      >
        <div className="pi-stack" aria-label="Budget breakdown">
          {parts.map(
            (p) =>
              p.v > 0 && (
                <div
                  key={p.k}
                  className={`pi-seg ${p.k}`}
                  style={{ flexGrow: p.v }}
                  title={`${p.label}: ${m(p.v)}`}
                />
              ),
          )}
        </div>
        <div className="pi-stats">
          {parts.map((p) => (
            <Stat key={p.k} label={p.label} value={m(p.v)} />
          ))}
          <Stat
            label="Hard Cost / SF"
            value={t.hard_per_sf > 0 ? `${m(t.hard_per_sf)}/SF` : "—"}
            note={info.grossSf > 0 ? `over ${sfText(info.grossSf)} gross` : "No gross area yet"}
          />
        </div>
        <div className="pi-lines">
          <div className="pi-line head">
            <span>LINE</span>
            <span>TYPE</span>
            <span className="right">AMOUNT</span>
            <span>NOTES</span>
            <span />
          </div>
          {b.lines.map((l, i) => (
            <div key={i} className="pi-line">
              <input
                aria-label="Budget line"
                value={l.name}
                onChange={(e) => set((x) => void (x.lines[i]!.name = e.target.value))}
              />
              <div className="pi-seg-toggle" role="radiogroup" aria-label={`${l.name} type`}>
                {[true, false].map((hard) => (
                  <button
                    key={String(hard)}
                    role="radio"
                    aria-checked={l.hard === hard}
                    className={l.hard === hard ? "on" : undefined}
                    onClick={() => set((x) => void (x.lines[i]!.hard = hard))}
                  >
                    {hard ? "HARD" : "SOFT"}
                  </button>
                ))}
              </div>
              <NumberInput
                label={`${l.name} amount`}
                value={l.amount}
                format={m}
                onChange={(v) => set((x) => void (x.lines[i]!.amount = v))}
              />
              <input
                aria-label={`${l.name} notes`}
                value={l.notes}
                placeholder="—"
                onChange={(e) => set((x) => void (x.lines[i]!.notes = e.target.value))}
              />
              <button
                className="pi-x"
                aria-label={`Remove ${l.name}`}
                onClick={() => set((x) => void x.lines.splice(i, 1))}
              >
                ×
              </button>
            </div>
          ))}
          <button
            className="pi-btn ghost"
            onClick={() =>
              set((x) => void x.lines.push({ name: "New line", hard: false, amount: 0, notes: "" }))
            }
          >
            + Add line
          </button>
        </div>
      </Card>
      <Card eyebrow="ALLOWANCES & TARGET" title="Assumptions">
        <div className="pi-fields">
          <label className="pi-field">
            <span>Contingency</span>
            <NumberInput
              label="Contingency"
              value={b.contingency_pct}
              suffix="%"
              format={(v) => String(v)}
              onChange={(v) => set((x) => void (x.contingency_pct = Math.min(v, 100)))}
            />
          </label>
          <label className="pi-field">
            <span>Escalation (of hard cost)</span>
            <NumberInput
              label="Escalation"
              value={b.escalation_pct}
              suffix="%"
              format={(v) => String(v)}
              onChange={(v) => set((x) => void (x.escalation_pct = Math.min(v, 100)))}
            />
          </label>
          <label className="pi-field">
            <span>Target Construction Cost</span>
            <NumberInput
              label="Target cost per SF"
              value={b.target_cost_sf}
              suffix="/SF"
              format={m}
              onChange={(v) => set((x) => void (x.target_cost_sf = v))}
            />
          </label>
          <Field
            label="Currency"
            value={b.currency}
            presets={["USD", "CAD", "EUR", "GBP", "AUD", "MXN"]}
            onChange={(v) => set((x) => void (x.currency = v.toUpperCase()))}
          />
        </div>
        {t.target_hard > 0 && (
          <div className={`pi-variance${variance < 0 ? " over" : ""}`}>
            Target {m(t.target_hard)} for {sfText(info.grossSf)} ·{" "}
            {variance < 0 ? `${m(-variance)} over target` : `${m(variance)} under target`}
          </div>
        )}
      </Card>
    </>
  );
}

function Schedule({ d, info }: { d: Draft; info: ProjectInfoState }) {
  const ms = d.details.milestones;
  const set = (f: (m: ProjectDetails["milestones"]) => void) =>
    edit((x) => f(x.details.milestones));
  return (
    <>
      <Card eyebrow="06 · COST & TIME" title="Milestones" right={<Progress id="schedule" d={d} />}>
        <div className="pi-lines">
          <div className="pi-line ms head">
            <span />
            <span>MILESTONE</span>
            <span>DATE</span>
            <span />
          </div>
          {ms.map((m, i) => (
            <div key={i} className={`pi-line ms${m.done ? " done" : ""}`}>
              <button
                className={`std-check${m.done ? " on" : ""}`}
                role="checkbox"
                aria-checked={m.done}
                aria-label={`${m.name} done`}
                onClick={() => set((x) => void (x[i]!.done = !m.done))}
              >
                {m.done ? "✓" : ""}
              </button>
              <input
                aria-label="Milestone"
                value={m.name}
                list="pi-milestones"
                onChange={(e) => set((x) => void (x[i]!.name = e.target.value))}
              />
              <input
                aria-label={`${m.name} date`}
                type="date"
                value={m.date}
                onChange={(e) => set((x) => void (x[i]!.date = e.target.value))}
              />
              <button
                className="pi-x"
                aria-label={`Remove ${m.name}`}
                onClick={() => set((x) => void x.splice(i, 1))}
              >
                ×
              </button>
            </div>
          ))}
          <datalist id="pi-milestones">
            {info.milestoneNames.map((n) => (
              <option key={n} value={n} />
            ))}
          </datalist>
          <button
            className="pi-btn ghost"
            onClick={() =>
              set((x) => void x.push({ name: "New milestone", date: "", done: false }))
            }
          >
            + Add milestone
          </button>
        </div>
      </Card>
      <Card eyebrow="DESIGN STAGES" title="Stages">
        <div className="pi-lines">
          <div className="pi-line st head">
            <span>STAGE</span>
            <span>START</span>
            <span>TARGET</span>
            <span />
          </div>
          {info.stages.map((s) => (
            <div key={s.name} className={`pi-line st${s.current ? " current" : ""}`}>
              <span>
                <b>{s.abbreviation}</b> {s.name}
              </span>
              <span>{s.start || "—"}</span>
              <span>{s.target || "—"}</span>
              <span>{s.current && <span className="pi-chip">CURRENT</span>}</span>
            </div>
          ))}
        </div>
        <div className="pi-hint">
          Stages and their dates are set in the project&apos;s Properties.
        </div>
      </Card>
    </>
  );
}

function Codes({ d }: { d: Draft }) {
  const c = d.details.codes;
  const set = (f: (c: ProjectDetails["codes"]) => void) => edit((x) => f(x.details.codes));
  const f = (label: string, key: keyof typeof c, presets?: string[], wide?: boolean) => (
    <Field
      label={label}
      value={c[key]}
      presets={presets}
      wide={wide}
      onChange={(v) => set((x) => void (x[key] = v))}
    />
  );
  return (
    <>
      <Card eyebrow="07 · REGULATORY" title="Building Code" right={<Progress id="codes" d={d} />}>
        <div className="pi-fields">
          {f("Building Code", "building_code", PRESETS.code)}
          {f("Energy Code", "energy_code", PRESETS.energy)}
          {f("Occupancy Group", "occupancy", PRESETS.occupancy)}
          {f("Construction Type", "construction_type", PRESETS.construction)}
          {f("Sprinkler System", "sprinklered", PRESETS.sprinklered)}
        </div>
      </Card>
      <Card eyebrow="LAND USE" title="Zoning">
        <div className="pi-fields">
          {f("Zoning District", "zoning_district")}
          {f("Lot Area", "lot_area")}
          {f("Floor Area Ratio", "far")}
          {f("Maximum Height", "max_height")}
          {f("Lot Coverage", "lot_coverage")}
          {f("Parking", "parking")}
          {f("Setbacks", "setbacks", undefined, true)}
          {f("Code Notes", "notes", undefined, true)}
        </div>
      </Card>
    </>
  );
}

function Notes({ d }: { d: Draft }) {
  return (
    <Card eyebrow="08 · NOTES" title="Notes">
      <textarea
        className="pi-notes"
        aria-label="Project notes"
        placeholder="Decisions, open questions, anything the team should know…"
        value={d.details.notes}
        onChange={(e) => edit((x) => void (x.details.notes = e.target.value))}
      />
    </Card>
  );
}

// ---- The tab's panes ----

/** The team as text: one consultant per line, for pasting into an email. */
export function directory(d: Draft): string {
  const line = (role: string, c: ProjectContact) =>
    [role, c.company, c.name, c.phone, c.email].filter((s) => s && s.trim()).join(" — ");
  const head = `${d.name}${d.number ? ` (${d.number})` : ""}`;
  const people = [
    !isEmpty(d.details.client) && line("Client", d.details.client),
    !isEmpty(d.details.owner_rep) && line("Owner's Representative", d.details.owner_rep),
    ...d.details.team.filter((m) => !isEmpty(m.contact)).map((m) => line(m.discipline, m.contact)),
  ].filter((r): r is string => !!r);
  const address = [d.details.location.street, d.details.location.city, d.details.location.state]
    .filter(has)
    .join(", ");
  return [head, ...(address ? [address] : []), "", ...people].join("\n").trim();
}

function isEmpty(c: ProjectContact) {
  return Object.values(c).every((v) => !has(v));
}

export function ProjectInfoRibbon() {
  const { draft, section } = usePi();
  return (
    <>
      {GROUPS.map((g) => (
        <div className="rb-group" key={g}>
          <div className="rb-items">
            {SECTIONS.filter((s) => s.group === g).map((s) => (
              <button
                key={s.id}
                className={`rb-btn std-btn${s.id === section ? " active" : ""}`}
                title={s.label}
                onClick={() => usePi.setState({ section: s.id })}
              >
                <StrokeIcon d={s.icon} />
                <span>{s.short}</span>
              </button>
            ))}
          </div>
          <div className="rb-title">{g}</div>
        </div>
      ))}
      <div className="rb-group">
        <div className="rb-items">
          <button
            className="rb-btn std-btn wide"
            disabled={!draft}
            title="Copy the project directory (client and consultants) to paste into an email"
            onClick={() => {
              if (!draft) return;
              void navigator.clipboard?.writeText(directory(draft)).then(
                () => {
                  usePi.setState({ note: "Project directory copied" });
                  window.setTimeout(() => usePi.setState({ note: null }), 2500);
                },
                () => {},
              );
            }}
          >
            <StrokeIcon d="M8 8h12v12H8zM4 16V4h12" />
            <span>Copy Directory</span>
          </button>
        </div>
        <div className="rb-title">SHARE</div>
      </div>
    </>
  );
}

export function ProjectInfoBrowser() {
  const { draft, section } = useProjectInfo();
  const all: [number, number] = draft
    ? SECTIONS.reduce<[number, number]>(
        (a, s) => {
          const [n, t] = completion(s.id, draft);
          return [a[0] + n, a[1] + t];
        },
        [0, 0],
      )
    : [0, 0];
  const pct = all[1] ? Math.round((all[0] / all[1]) * 100) : 0;
  return (
    <aside className="panel browser std-browser" aria-label="Project Info Browser">
      <div className="panel-title">Project Info</div>
      <div className="std-tree">
        <div className="std-root">
          <span aria-hidden>▼</span>
          {draft?.name.toUpperCase() || "PROJECT"}
        </div>
        {SECTIONS.map((s) => {
          const [n, t] = draft ? completion(s.id, draft) : [0, 0];
          return (
            <button
              key={s.id}
              className={`std-row${s.id === section ? " on" : ""}`}
              onClick={() => usePi.setState({ section: s.id })}
            >
              <span>{s.label}</span>
              <span className={`std-count${t > 0 && n === t ? " full" : ""}`}>
                {n}/{t}
              </span>
            </button>
          );
        })}
      </div>
      <div className="std-foot">
        <div className="std-foot-head">
          <span>INFO COMPLETE</span>
          <span>{pct}%</span>
        </div>
        <div className="std-bar">
          <div style={{ width: `${pct}%` }} />
        </div>
        <div className="std-foot-line">
          {all[0]} of {all[1]} filled in
        </div>
      </div>
    </aside>
  );
}

export function ProjectInfoView() {
  const { draft, info, section } = usePi();
  // Save what's pending when leaving the tab.
  useEffect(() => () => void commitProjectInfo(), []);
  const s = SECTIONS.find((x) => x.id === section)!;
  return (
    <section className="workspace std-workspace">
      <div className="tabs" role="tablist" aria-label="Open views">
        <div className="tab active std-tab" role="tab" aria-selected>
          <span className="tab-label">
            <span className="tab-kind">PROJECT INFO</span>
            {s.label}
          </span>
        </div>
      </div>
      <div className="std-canvas pi-canvas">
        {draft && info && (
          <>
            {section === "overview" && <Overview d={draft} info={info} />}
            {section === "location" && <Location d={draft} info={info} />}
            {section === "client" && <Client d={draft} />}
            {section === "team" && <Team d={draft} info={info} />}
            {section === "budget" && <Budget d={draft} info={info} />}
            {section === "schedule" && <Schedule d={draft} info={info} />}
            {section === "codes" && <Codes d={draft} />}
            {section === "notes" && <Notes d={draft} />}
          </>
        )}
      </div>
    </section>
  );
}

/** The summary in the Properties pane: the job at a glance. */
export function ProjectInfoSummary() {
  const { draft, info } = usePi();
  if (!draft || !info) return <aside className="panel properties" aria-label="Properties" />;
  const x = draft.details;
  const engaged = x.team.filter((m) => has(m.contact.company) || has(m.contact.name)).length;
  const next = x.milestones
    .filter((m) => !m.done && m.date)
    .sort((a, b) => a.date.localeCompare(b.date))[0];
  const client = x.client.company || x.client.name;
  const address = [x.location.street, x.location.city, x.location.state].filter(has).join(", ");
  const row = (label: string, value: ReactNode) => (
    <div className="std-prop">
      <span>{label}</span>
      <span>{value}</span>
    </div>
  );
  return (
    <aside className="panel properties std-props" aria-label="Properties">
      <div className="panel-title">Project Summary</div>
      <div className="std-props-body">
        <div className="std-props-title">
          <span>{draft.number || "PROJECT"}</span>
          <strong>{draft.name || "Untitled"}</strong>
          {x.overview.status && <span className="pi-chip">{x.overview.status.toUpperCase()}</span>}
        </div>
        <div className="std-section">IDENTITY</div>
        {row("Client", client || <span className="muted">—</span>)}
        {row("Address", address || <span className="muted">—</span>)}
        {row("Type", x.overview.project_type || <span className="muted">—</span>)}
        {row("Delivery", x.overview.delivery || <span className="muted">—</span>)}
        <div className="std-section">TEAM</div>
        {row("Consultants", `${engaged} of ${x.team.length} engaged`)}
        <div className="std-section">BUDGET</div>
        {row("Total", money(info.totals.total, x.budget.currency))}
        {row(
          "Hard / SF",
          info.totals.hard_per_sf > 0
            ? `${money(info.totals.hard_per_sf, x.budget.currency)}`
            : "—",
        )}
        {row("Gross Area", sfText(info.grossSf))}
        <div className="std-section">SCHEDULE</div>
        {row("Stage", info.stages.find((s) => s.current)?.name ?? "—")}
        {row("Next", next ? `${next.name}, ${next.date}` : <span className="muted">—</span>)}
        <div className="std-section">CODE</div>
        {row(
          "Occupancy",
          [x.codes.occupancy, x.codes.construction_type].filter(has).join(" · ") || (
            <span className="muted">—</span>
          ),
        )}
      </div>
    </aside>
  );
}

export function ProjectInfoStatus() {
  const { dirty, saving, note } = usePi();
  return (
    <footer className="statusbar">
      <span className="status-prompt">
        Pick a section in the ribbon or browser. Everything saves with the project as you type and
        undoes like any edit.
      </span>
      <span className="status-cursor std-status-total">
        {note ?? (saving ? "Saving…" : dirty ? "Editing…" : "Saved")}
      </span>
    </footer>
  );
}
