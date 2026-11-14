import type { JSX } from "preact";
import { useRef, useState } from "preact/hooks";
import { resizeCanvas } from "./platform";

/** TV focus never opens the IME. OK/click explicitly enters editing mode. */
export function TVInput(props: JSX.InputHTMLAttributes<HTMLInputElement>) {
  const [editing, setEditing] = useState(false),
    ref = useRef<HTMLInputElement>(null),
    internalFocus = useRef(false);
  const text = ![
    "checkbox",
    "radio",
    "range",
    "button",
    "submit",
    "hidden",
  ].includes(String(props.type || "text"));
  if (!text) return <input {...props} />;
  const activate = () => {
    const el = ref.current;
    if (!el || props.disabled || props.readOnly) return;
    internalFocus.current = true;
    setEditing(true);
    el.readOnly = false;
    el.blur();
    el.focus({ preventScroll: true });
    internalFocus.current = false;
    resizeCanvas();
  };
  const finish = () => {
    const el = ref.current;
    if (!el) return;
    internalFocus.current = true;
    setEditing(false);
    el.readOnly = true;
    el.blur();
    el.focus({ preventScroll: true });
    internalFocus.current = false;
    resizeCanvas();
  };
  return (
    <input
      {...props}
      ref={ref}
      readOnly={!editing || !!props.readOnly}
      aria-description={editing ? "Eingabe aktiv" : "OK zum Bearbeiten"}
      onClick={(e) => {
        if (!editing) activate();
        (props.onClick as any)?.(e);
      }}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.keyCode === 13) {
          e.preventDefault();
          e.stopPropagation();
          editing ? finish() : activate();
        } else if (editing && (e.key === "Escape" || e.keyCode === 10009)) {
          e.preventDefault();
          e.stopPropagation();
          finish();
        } else (props.onKeyDown as any)?.(e);
      }}
      onBlur={(e) => {
        if (!internalFocus.current) {
          setEditing(false);
          e.currentTarget.readOnly = true;
        }
        (props.onBlur as any)?.(e);
      }}
    />
  );
}
