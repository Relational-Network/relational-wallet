// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { useEffect } from "react";
import type { ReactNode } from "react";

interface ActionDialogProps {
  open: boolean;
  title: string;
  onClose: () => void;
  children: ReactNode;
  wide?: boolean;
  dialogClassName?: string;
  bodyClassName?: string;
}

export function ActionDialog({
  open,
  title,
  onClose,
  children,
  wide = false,
  dialogClassName,
  bodyClassName,
}: ActionDialogProps) {
  useEffect(() => {
    if (!open) return;

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div className="dialog-backdrop" onClick={onClose} role="presentation">
      <div
        className={`dialog-card${wide ? " wide" : ""}${dialogClassName ? ` ${dialogClassName}` : ""}`}
        onClick={(event) => event.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-label={title}
      >
        <div className="dialog-header">
          <h2>{title}</h2>
          <button type="button" className="btn btn-ghost" onClick={onClose} aria-label="Close dialog">
            ✕
          </button>
        </div>
        <div className={`dialog-body${bodyClassName ? ` ${bodyClassName}` : ""}`}>{children}</div>
      </div>
    </div>
  );
}
