import { useEffect, useState, type FormEvent } from "react";
import {
  errorMessage,
  ipc,
  type PublishOptions,
  type PublishResult,
  type RufplanLink,
} from "../ipc";
import {
  linkProject,
  projectUrl,
  publish,
  refreshCloud,
  signIn,
  signInGoogle,
  signOut,
} from "../rufplan";
import { useAppStore } from "../store";

function SignInForm() {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState<"" | "password" | "google">("");
  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setBusy("password");
    await signIn(email, password);
    setBusy("");
  };
  const google = async () => {
    setBusy("google");
    await signInGoogle();
    setBusy("");
  };
  return (
    <form className="rp-form" onSubmit={(e) => void submit(e)}>
      <p className="modal-message">Sign in with your Rufplan.io account.</p>
      <label className="rp-field">
        <span>Email</span>
        <input
          type="email"
          autoFocus
          autoComplete="username"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
        />
      </label>
      <label className="rp-field">
        <span>Password</span>
        <input
          type="password"
          autoComplete="current-password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
        />
      </label>
      <div className="modal-actions">
        <button className="btn-cyan" type="submit" disabled={!email || !password || !!busy}>
          {busy === "password" ? "Signing in…" : "Sign In"}
        </button>
        <button
          className="btn-outline"
          type="button"
          disabled={!!busy}
          onClick={() => void google()}
        >
          {busy === "google" ? "Waiting for browser…" : "Google"}
        </button>
      </div>
      {busy === "google" && (
        <p className="rp-note">Finish signing in in your browser, then come back here.</p>
      )}
    </form>
  );
}

function Account() {
  const cloud = useAppStore((s) => s.cloud);
  if (!cloud?.signedIn) return <SignInForm />;
  return (
    <>
      <p className="modal-message">{cloud.name}</p>
      <p className="rp-note">Signed in as {cloud.email}</p>
      <div className="modal-actions">
        <button className="btn-outline" onClick={() => void signOut()}>
          Sign Out
        </button>
      </div>
    </>
  );
}

function Link() {
  const linked = useAppStore((s) => s.app?.rufplan ?? null);
  const close = useAppStore((s) => s.setRufplan);
  const [projects, setProjects] = useState<RufplanLink[] | null>(null);
  const [picked, setPicked] = useState(linked?.id ?? "");
  const [failed, setFailed] = useState("");
  useEffect(() => {
    ipc
      .cloudProjects()
      .then((p) => {
        setProjects(p);
        setPicked((cur) => cur || p[0]?.id || "");
      })
      .catch((err) => setFailed(errorMessage(err)));
  }, []);
  const choose = async () => {
    const link = projects?.find((p) => p.id === picked);
    if (link && (await linkProject(link))) close(null);
  };
  return (
    <>
      <p className="modal-message">
        {linked ? `Linked to ${linked.name}` : "Pick the Rufplan project this model belongs to."}
      </p>
      {failed && <p className="rp-error">{failed}</p>}
      {!projects && !failed && <p className="rp-note">Loading your projects…</p>}
      {projects?.length === 0 && (
        <p className="rp-note">You don&apos;t own any projects yet. Create one on rufplan.io.</p>
      )}
      {projects && projects.length > 0 && (
        <div className="rp-list" role="radiogroup" aria-label="Rufplan projects">
          {projects.map((p) => (
            <label key={p.id} className={`rp-option${picked === p.id ? " on" : ""}`}>
              <input
                type="radio"
                name="rufplan-project"
                checked={picked === p.id}
                onChange={() => setPicked(p.id)}
              />
              <span>{p.name}</span>
              <span className="rp-sub">rufplan.io/projects/{p.slug}</span>
            </label>
          ))}
        </div>
      )}
      <div className="modal-actions">
        <button
          className="btn-cyan"
          disabled={!picked || picked === linked?.id}
          onClick={() => void choose()}
        >
          Link
        </button>
        {linked && (
          <button
            className="btn-outline"
            onClick={() => void linkProject(null).then((ok) => ok && close(null))}
          >
            Unlink
          </button>
        )}
      </div>
    </>
  );
}

function Publish() {
  const app = useAppStore((s) => s.app);
  const [options, setOptions] = useState<PublishOptions | null>(null);
  const [deliverable, setDeliverable] = useState("");
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<PublishResult | null>(null);
  useEffect(() => {
    ipc
      .publishOptions()
      .then((o) => {
        setOptions(o);
        setDeliverable(o.deliverables[0]?.id ?? "");
        setName(`${o.stage} Set`.trim());
      })
      .catch((err) => useAppStore.getState().setError(errorMessage(err)));
  }, []);
  const linked = app?.rufplan;
  if (!linked) return <p className="modal-message">Link this model to a Rufplan project first.</p>;
  if (result)
    return (
      <>
        <p className="modal-message">Published to {linked.name}</p>
        <ul className="rp-files">
          {result.published.map((f) => (
            <li key={f}>{f}</li>
          ))}
        </ul>
        {result.skipped.length > 0 && (
          <p className="rp-error">
            Not uploaded — {result.skipped.join("; ")}. Rufplan&apos;s storage currently only
            accepts PDFs and images for deliverables.
          </p>
        )}
        <p className="rp-note">{result.url}</p>
        <div className="modal-actions">
          <button className="btn-cyan" onClick={() => void ipc.openRufplan(result.url)}>
            Open in Rufplan
          </button>
        </div>
      </>
    );
  const go = async () => {
    setBusy(true);
    setResult(await publish(name, deliverable));
    setBusy(false);
  };
  return (
    <>
      <p className="modal-message">
        Publish the {options?.stage ?? ""} set to {linked.name}
      </p>
      <p className="rp-note">
        {options
          ? `${options.sheetCount} sheet${options.sheetCount === 1 ? "" : "s"} as a PDF, plus the IFC model. It's recorded as an issue on the title blocks.`
          : "Loading…"}
      </p>
      <label className="rp-field">
        <span>Deliverable</span>
        <select value={deliverable} onChange={(e) => setDeliverable(e.target.value)}>
          {options?.deliverables.map((d) => (
            <option key={d.id} value={d.id}>
              {d.label}
            </option>
          ))}
        </select>
      </label>
      <label className="rp-field">
        <span>Issue name</span>
        <input value={name} onChange={(e) => setName(e.target.value)} />
      </label>
      <div className="modal-actions">
        <button
          className="btn-cyan"
          disabled={!options || options.sheetCount === 0 || !deliverable || !name.trim() || busy}
          onClick={() => void go()}
        >
          {busy ? "Publishing…" : "Publish"}
        </button>
      </div>
      <p className="rp-note">Opens at {projectUrl(linked.slug)}</p>
    </>
  );
}

const TITLES = { account: "Rufplan Account", link: "Link Project", publish: "Publish" };

export function RufplanDialog() {
  const mode = useAppStore((s) => s.rufplan);
  const cloud = useAppStore((s) => s.cloud);
  const setRufplan = useAppStore((s) => s.setRufplan);
  useEffect(() => {
    if (mode && !useAppStore.getState().cloud) void refreshCloud();
  }, [mode]);
  if (!mode) return null;
  const close = () => setRufplan(null);
  let body;
  if (cloud && !cloud.configured) {
    body = (
      <p className="modal-message">
        This build can&apos;t connect to Rufplan (it was built without the Rufplan key).
      </p>
    );
  } else if (!cloud) {
    body = <p className="rp-note">Connecting…</p>;
  } else if (mode === "account" || !cloud.signedIn) {
    body = <Account />;
  } else if (mode === "link") {
    body = <Link />;
  } else {
    body = <Publish />;
  }
  return (
    <div
      className="modal-backdrop"
      role="dialog"
      aria-modal
      aria-label={TITLES[mode]}
      onKeyDown={(e) => e.key === "Escape" && close()}
    >
      <div className="modal rp-modal">
        <div className="rp-head">
          <div className="modal-kicker">{TITLES[mode]}</div>
          <button className="rp-close" aria-label="Close" onClick={close}>
            ×
          </button>
        </div>
        {body}
      </div>
    </div>
  );
}
