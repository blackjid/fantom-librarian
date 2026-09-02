import { useRef, useState } from "react";
import { cn } from "@/lib/utils";

const clamp = (value: number, min: number, max: number) => Math.min(Math.max(value, min), max);

/**
 * The grab strip between two panes. It owns no width of its own — the shell keeps that — and it
 * never lets a pane past the bounds that keep both sides readable.
 */
export function Resizer({
  width,
  onWidth,
  onReset,
  min,
  max,
  label,
  className,
}: {
  width: number;
  onWidth: (width: number) => void;
  /** Double-click puts the pane back where it started. */
  onReset: () => void;
  min: number;
  max: number;
  label: string;
  className?: string;
}) {
  const drag = useRef<{ x: number; width: number } | null>(null);
  const [dragging, setDragging] = useState(false);
  const [hovered, setHovered] = useState(false);
  const [focused, setFocused] = useState(false);
  // A finished drag leaves the pointer sitting on the strip. The line goes anyway, and stays
  // gone until the pointer leaves and comes back.
  const [settled, setSettled] = useState(false);
  const lit = dragging || focused || (hovered && !settled);

  return (
    <div
      role="separator"
      aria-orientation="vertical"
      aria-label={label}
      aria-valuenow={Math.round(width)}
      aria-valuemin={min}
      aria-valuemax={max}
      tabIndex={0}
      onPointerEnter={() => {
        setHovered(true);
        setSettled(false);
      }}
      onPointerLeave={() => {
        setHovered(false);
        setSettled(false);
      }}
      onPointerDown={(event) => {
        drag.current = { x: event.clientX, width };
        setDragging(true);
        // Capture, so a fast drag that outruns the strip keeps resizing.
        event.currentTarget.setPointerCapture(event.pointerId);
      }}
      onPointerMove={(event) => {
        if (!drag.current) return;
        onWidth(clamp(drag.current.width + event.clientX - drag.current.x, min, max));
      }}
      onPointerUp={(event) => {
        drag.current = null;
        setDragging(false);
        setSettled(true);
        event.currentTarget.releasePointerCapture(event.pointerId);
      }}
      onDoubleClick={onReset}
      // Pressing the strip focuses it too, and that focus outlives the drag — only a keyboard
      // landing should light the line.
      onFocus={(event) => setFocused(event.currentTarget.matches(":focus-visible"))}
      onBlur={() => setFocused(false)}
      onKeyDown={(event) => {
        const step = event.key === "ArrowLeft" ? -16 : event.key === "ArrowRight" ? 16 : 0;
        if (step === 0) return;
        event.preventDefault();
        onWidth(clamp(width + step, min, max));
      }}
      className={cn(
        "relative w-2 shrink-0 cursor-col-resize touch-none outline-none",
        className,
      )}
    >
      {/* A line of the same light the surfaces catch, not an accent, and gone at both ends. */}
      <span
        aria-hidden
        className={cn(
          "pointer-events-none absolute inset-y-0 left-1/2 w-px -translate-x-1/2 bg-linear-to-b from-transparent via-foreground/30 to-transparent transition-opacity duration-150",
          lit ? "opacity-100" : "opacity-0",
        )}
      />
    </div>
  );
}
