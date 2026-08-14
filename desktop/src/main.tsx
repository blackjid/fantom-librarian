import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { TooltipProvider } from "@/components/ui/tooltip";
import App from "@/App";
import "@/styles.css";

// A webview's default right-click menu offers Reload, Back, and Inspect — none of which mean
// anything in a librarian window. Text fields keep theirs, where cut/copy/paste is the point.
document.addEventListener("contextmenu", (event) => {
  const target = event.target as HTMLElement | null;
  if (!target?.closest("input, textarea, [data-selectable]")) event.preventDefault();
});

// Dragging a name or a row would start a native file/text drag over the whole window.
document.addEventListener("dragstart", (event) => {
  const target = event.target as HTMLElement | null;
  if (!target?.closest("input, textarea, [data-selectable]")) event.preventDefault();
});

const root = document.getElementById("root");
if (!root) throw new Error("no #root element");

createRoot(root).render(
  <StrictMode>
    <TooltipProvider delayDuration={400}>
      <App />
    </TooltipProvider>
  </StrictMode>,
);
