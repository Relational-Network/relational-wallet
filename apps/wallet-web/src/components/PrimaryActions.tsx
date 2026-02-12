// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { Send, QrCode, ArrowDownToLine, ArrowUpFromLine } from "lucide-react";

interface PrimaryActionsProps {
  disabled?: boolean;
  onSend: () => void;
  onReceive: () => void;
  onOnRamp: () => void;
  onOffRamp: () => void;
}

export function PrimaryActions({
  disabled = false,
  onSend,
  onReceive,
  onOnRamp,
  onOffRamp,
}: PrimaryActionsProps) {
  return (
    <div className="quick-actions" role="group" aria-label="Primary actions">
      <button type="button" disabled={disabled} onClick={onSend} className="quick-action-btn">
        <span className="icon-circle"><Send size={18} /></span>
        Send
      </button>
      <button type="button" disabled={disabled} onClick={onReceive} className="quick-action-btn">
        <span className="icon-circle"><QrCode size={18} /></span>
        Receive
      </button>
      <button type="button" disabled={disabled} onClick={onOnRamp} className="quick-action-btn">
        <span className="icon-circle"><ArrowDownToLine size={18} /></span>
        On-Ramp
      </button>
      <button type="button" disabled={disabled} onClick={onOffRamp} className="quick-action-btn">
        <span className="icon-circle"><ArrowUpFromLine size={18} /></span>
        Off-Ramp
      </button>
    </div>
  );
}
