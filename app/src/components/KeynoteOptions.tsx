import { useState } from "react";
import { useAppStore } from "../store";
import { KeynotePicker, useKeynotes } from "./KeynotePicker";

// The options bar for the keynote tools (ADR-081): Revit's tag type and Leader, and for a
// User keynote, the keynote to place, chosen in the picker.

export function KeynoteOptions({ user }: { user: boolean }) {
  const o = useAppStore((s) => s.options);
  const set = useAppStore((s) => s.setOption);
  const entries = useKeynotes();
  const [picking, setPicking] = useState(false);
  const chosen = entries.find((k) => k.key === o.keynoteUserKey);
  return (
    <>
      <label className="ob-field">
        Tag
        <select
          aria-label="Keynote tag type"
          value={o.keynoteStyle}
          onChange={(e) => set("keynoteStyle", e.target.value as "Key" | "KeyAndText")}
        >
          <option value="Key">Boxed key</option>
          <option value="KeyAndText">Boxed key with text</option>
        </select>
      </label>
      <label className="ob-check">
        <input
          type="checkbox"
          checked={o.keynoteLeader}
          onChange={(e) => set("keynoteLeader", e.target.checked)}
        />
        Leader
      </label>
      {user && (
        <button
          className="ob-keynote"
          aria-label="Keynote to place"
          title="Choose the keynote to place"
          onClick={() => setPicking(true)}
        >
          {chosen ? (
            <>
              <span className="kn-key">{chosen.key}</span> {chosen.text}
            </>
          ) : (
            "Choose a keynote…"
          )}
        </button>
      )}
      {picking && (
        <KeynotePicker
          title="User Keynote"
          subtitle="The keynote to place (it stays chosen for the next ones)"
          initial={o.keynoteUserKey || null}
          onCancel={() => setPicking(false)}
          onPick={(key) => {
            set("keynoteUserKey", key);
            setPicking(false);
          }}
        />
      )}
    </>
  );
}
