// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { useEffect, useRef, useState } from "react";
import { ActionDialog } from "@/components/ActionDialog";

interface RecipientQrScannerProps {
  open: boolean;
  onClose: () => void;
  onScan: (value: string) => void;
}

export function RecipientQrScanner({ open, onClose, onScan }: RecipientQrScannerProps) {
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const frameRef = useRef<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;

    let stopped = false;

    const start = async () => {
      try {
        if (!("BarcodeDetector" in window)) {
          setError("QR scanning is not supported in this browser. Paste address manually.");
          return;
        }

        const stream = await navigator.mediaDevices.getUserMedia({
          video: { facingMode: "environment" },
          audio: false,
        });

        streamRef.current = stream;
        if (!videoRef.current) return;

        videoRef.current.srcObject = stream;
        await videoRef.current.play();

        const Detector = (window as Window & { BarcodeDetector?: new (options?: { formats?: string[] }) => { detect: (source: CanvasImageSource) => Promise<Array<{ rawValue?: string }>> } }).BarcodeDetector;
        if (!Detector) {
          setError("QR scanning is not supported in this browser. Paste address manually.");
          return;
        }

        const detector = new Detector({ formats: ["qr_code"] });

        const scan = async () => {
          if (stopped || !videoRef.current) return;

          try {
            const results = await detector.detect(videoRef.current);
            const code = results.find((item) => Boolean(item.rawValue))?.rawValue;
            if (code) {
              onScan(code);
              onClose();
              return;
            }
          } catch {
            // Keep scanning; intermittent detect errors are expected on some browsers.
          }

          frameRef.current = window.requestAnimationFrame(scan);
        };

        frameRef.current = window.requestAnimationFrame(scan);
      } catch (scanError) {
        setError(scanError instanceof Error ? scanError.message : "Unable to open camera");
      }
    };

    void start();

    return () => {
      stopped = true;
      if (frameRef.current !== null) {
        window.cancelAnimationFrame(frameRef.current);
      }
      if (streamRef.current) {
        streamRef.current.getTracks().forEach((track) => track.stop());
        streamRef.current = null;
      }
    };
  }, [open, onClose, onScan]);

  return (
    <ActionDialog open={open} onClose={onClose} title="Scan recipient QR">
      <div className="simple-stack">
        <p className="simple-muted" style={{ marginTop: 0 }}>
          Point the camera at a wallet QR code. If scanning fails, paste address manually.
        </p>
        {error ? <div className="simple-error">{error}</div> : null}
        <video ref={videoRef} className="simple-scanner-video" playsInline muted />
      </div>
    </ActionDialog>
  );
}
