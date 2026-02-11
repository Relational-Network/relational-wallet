// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { QRCodeSVG } from "qrcode.react";

interface AddressQRCodeProps {
  address: string;
  /** QR code size in pixels. Default: 160 */
  size?: number;
}

/**
 * Renders a QR code for an Ethereum/Avalanche wallet address.
 *
 * Encodes the raw hex address for universal scanner compatibility.
 */
export function AddressQRCode({ address, size = 160 }: AddressQRCodeProps) {
  return (
    <div
      style={{
        display: "inline-flex",
        flexDirection: "column",
        alignItems: "center",
        gap: "0.5rem",
      }}
    >
      <div
        style={{
          padding: "0.75rem",
          background: "#fff",
          border: "1px solid #ddd",
          borderRadius: "4px",
          lineHeight: 0,
        }}
      >
        <QRCodeSVG
          value={address}
          size={size}
          level="M"
          marginSize={2}
        />
      </div>
      <span
        style={{
          fontSize: "0.6875rem",
          color: "#999",
        }}
      >
        Scan to get address
      </span>
    </div>
  );
}
