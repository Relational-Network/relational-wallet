// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { QRCodeSVG } from "qrcode.react";

interface AddressQRCodeProps {
  address: string;
  size?: number;
  label?: string;
}

export function AddressQRCode({ address, size = 160 }: AddressQRCodeProps) {
  return (
    <div className="qr-container">
      <div className="qr-frame">
        <QRCodeSVG value={address} size={size} level="M" marginSize={2} />
      </div>
    </div>
  );
}
