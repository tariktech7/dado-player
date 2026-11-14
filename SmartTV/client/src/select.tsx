import { createPortal } from "preact/compat";
import { useEffect, useLayoutEffect, useRef, useState } from "preact/hooks";
export function TVSelect({
  value,
  onChange,
  options,
}: {
  value: string;
  onChange: (v: string) => void;
  options: [string, string][];
}) {
  const [open, setOpen] = useState(false),
    [label, setLabel] = useState("Auswahl");
  const trigger = useRef<HTMLButtonElement>(null),
    popup = useRef<HTMLElement>(null);
  const close = () => {
    setOpen(false);
    requestAnimationFrame(() =>
      trigger.current?.focus({ preventScroll: true }),
    );
  };
  useEffect(() => {
    const name = trigger.current
      ?.closest("label")
      ?.querySelector("span")?.textContent;
    if (name) setLabel(name);
  }, []);
  useLayoutEffect(() => {
    if (open) {
      popup.current
        ?.querySelector<HTMLButtonElement>('[aria-selected="true"]')
        ?.focus();
    }
  }, [open]);
  return (
    <span class="select-control">
      <button
        ref={trigger}
        class="tv-select"
        type="button"
        role="combobox"
        aria-label={label}
        aria-expanded={open}
        aria-haspopup="listbox"
        onClick={() => setOpen(true)}
      >
        <span>{options.find(([v]) => v === value)?.[1] || value}</span>
        <span aria-hidden="true">⌄</span>
      </button>
      {open &&
        createPortal(
          <div
            class="scrim select-popup"
            onClick={(e) => {
              if (e.target === e.currentTarget) close();
            }}
            onKeyDown={(e) => {
              if (e.key === "Escape" || e.keyCode === 10009) {
                e.preventDefault();
                e.stopPropagation();
                close();
              }
            }}
          >
            <section
              ref={popup}
              class="modal glass selection-dialog"
              role="listbox"
              aria-label={label}
            >
              <h2>{label}</h2>
              <p class="muted">
                Mit Pfeilen auswählen · OK bestätigen · Zurück abbrechen
              </p>
              <div class="selection-options">
                {options.map(([v, text]) => (
                  <button
                    type="button"
                    role="option"
                    aria-selected={v === value}
                    class={v === value ? "selected" : ""}
                    onClick={() => {
                      onChange(v);
                      close();
                    }}
                  >
                    <span>{text}</span>
                    <span aria-hidden="true">{v === value ? "✓" : ""}</span>
                  </button>
                ))}
              </div>
              <button type="button" onClick={close}>
                Abbrechen
              </button>
            </section>
          </div>,
          document.getElementById("app")!,
        )}
    </span>
  );
}
